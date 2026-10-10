//! Adaptador experimental para `clamd`, o daemon do ClamAV, gerenciado pelo
//! próprio CorvusAV como processo-filho — nunca instalado como serviço do
//! Windows, nunca compartilhado com uma instância de `clamd` já existente no
//! sistema (ver corvusav-security-engineer skill, seção de arquitetura de
//! referência: "CorvusAV gerencia seu próprio processo `clamd.exe` como
//! processo-filho do app").
//!
//! Diferente do `clamscan` (processo único, de uma só thread, que recarrega
//! a base de assinaturas a cada execução), o `clamd` pode varrer com
//! múltiplas threads (`MaxThreads`) sobre a base já carregada em memória —
//! o que tende a ser bem mais rápido para verificações com muitos arquivos.
//!
//! **Escopo desta primeira versão**: o CorvusAV sobe uma instância *nova* de
//! `clamd`, dedicada, para cada verificação — não um daemon de longa duração
//! reaproveitado entre verificações. Isso significa que o custo de carregar
//! a base de assinaturas na memória ainda é pago uma vez por verificação,
//! exatamente como acontece hoje com `clamscan`; o que esta versão resolve é
//! a *verificação individual demorar menos* (ao paralelizar a varredura em
//! várias threads), não o custo de iniciar o mecanismo a cada verificação.
//! Reaproveitar uma instância de `clamd` entre verificações diferentes (o
//! que resolveria também esse segundo ponto) fica para uma fase futura.
//!
//! Se o `clamd` não puder ser localizado, não conseguir subir ou não
//! responder a tempo, a varredura cai automaticamente para o `clamscan` —
//! nunca falha a verificação só porque o `clamd` não está disponível (ver
//! `ClamdError::is_startup_failure` e `crate::engine::ScanEngineAdapter`).
//! Uma falha real depois que o `clamd` já respondeu ao PING nunca é
//! escondida atrás de uma nova tentativa com `clamscan`: isso duplicaria o
//! tempo de espera do usuário e poderia mascarar um problema real em vez de
//! reportá-lo.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::watch;

use crate::adapter::{escape_regex_literal, parse_clamscan_line, RawScanEntry};

/// Intervalo entre tentativas de PING enquanto o `clamd` ainda está
/// inicializando (carregando a base de assinaturas).
const PING_INTERVAL: Duration = Duration::from_millis(200);

/// Tempo máximo de espera pela primeira resposta ao PING antes de desistir
/// e cair para o `clamscan`. Independente do `timeout` geral da
/// verificação (nunca consome todo o orçamento de tempo só na
/// inicialização).
const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Error)]
pub enum ClamdError {
    #[error("executável do clamd não foi encontrado")]
    BinaryNotFound,
    #[error("não foi possível localizar o diretório de assinaturas desta instalação do ClamAV")]
    DatabaseDirUnknown,
    #[error("falha de E/S ao gerenciar o processo clamd: {0}")]
    Io(#[from] std::io::Error),
    #[error("clamd não respondeu dentro do tempo limite de inicialização ({0:?})")]
    StartupTimedOut(Duration),
    #[error("verificação cancelada pelo usuário")]
    Cancelled,
    #[error("tempo limite da verificação foi excedido")]
    TimedOut,
}

impl ClamdError {
    /// `true` quando o erro ocorreu antes de qualquer varredura real
    /// começar (clamd não encontrado, não subiu, não respondeu ao PING a
    /// tempo) — o único caso em que é seguro cair de volta para o
    /// `clamscan` sem risco de duplicar trabalho ou esconder uma falha real
    /// em andamento. Qualquer outra variante representa uma falha depois
    /// que o `clamd` já estava respondendo, e deve ser reportada como está.
    pub fn is_startup_failure(&self) -> bool {
        matches!(
            self,
            ClamdError::BinaryNotFound
                | ClamdError::DatabaseDirUnknown
                | ClamdError::StartupTimedOut(_)
        )
    }
}

/// Configuração de uma instância de `clamd` gerenciada pelo CorvusAV,
/// dedicada a uma única verificação.
#[derive(Debug, Clone)]
struct ClamdConfig {
    database_dir: PathBuf,
    port: u16,
    max_threads: u32,
    exclude_dirs: Vec<String>,
    log_file: PathBuf,
}

/// Gera o conteúdo de um `clamd.conf` mínimo para uma instância efêmera e
/// isolada: escuta só em loopback (nunca exposta na rede), roda em primeiro
/// plano (o CorvusAV gerencia seu ciclo de vida como processo-filho comum,
/// sem depender de fork/daemonização) e nunca é instalada como serviço do
/// Windows. Função pura, sem E/S — só monta texto, o que a torna testável
/// sem precisar de um binário `clamd` real.
fn render_conf(config: &ClamdConfig) -> String {
    let mut conf = String::new();
    conf.push_str("# Gerado automaticamente pelo CorvusAV — não editar manualmente.\n");
    conf.push_str("Foreground yes\n");
    conf.push_str(&format!(
        "DatabaseDirectory {}\n",
        config.database_dir.display()
    ));
    conf.push_str("TCPAddr 127.0.0.1\n");
    conf.push_str(&format!("TCPSocket {}\n", config.port));
    conf.push_str(&format!("MaxThreads {}\n", config.max_threads));
    conf.push_str("MaxConnectionQueueLength 1\n");
    conf.push_str("ReadTimeout 300\n");
    conf.push_str(&format!("LogFile {}\n", config.log_file.display()));
    conf.push_str("LogTime yes\n");
    for dir in &config.exclude_dirs {
        conf.push_str(&format!("ExcludePath {}\n", escape_regex_literal(dir)));
    }
    conf
}

/// Escolhe uma porta TCP de loopback para esta instância dedicada de
/// `clamd`. Heurística simples (faixa alta, derivada do PID do processo
/// atual), sem nenhuma garantia de estar livre — se já estiver em uso, a
/// inicialização do `clamd` falha e a verificação cai para o `clamscan`
/// (ver `ClamdError::is_startup_failure`), nunca trava a verificação à
/// espera de uma porta. Uma estratégia de nova tentativa com outra porta
/// fica para uma fase futura, se a colisão se mostrar um problema real na
/// prática.
fn pick_port() -> u16 {
    const BASE: u32 = 20000;
    const RANGE: u32 = 20000;
    (BASE + (std::process::id() % RANGE)) as u16
}

/// Número de threads de varredura do `clamd`: uma por núcleo lógico
/// disponível, com um teto conservador para não competir demais com o
/// resto do sistema do usuário em máquinas com muitos núcleos.
fn default_max_threads() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .min(16)
}

async fn ping(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)).await else {
        return false;
    };
    if stream.write_all(b"nPING\n").await.is_err() {
        return false;
    }
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match reader.read_line(&mut line).await {
        Ok(0) => false,
        Ok(_) => line.trim() == "PONG",
        Err(_) => false,
    }
}

/// Encerra a instância de `clamd` desta verificação. Pede um encerramento
/// gracioso via protocolo primeiro (para o `clamd` fechar seu arquivo de
/// log com limpeza), mas nunca depende só disso: o processo é
/// explicitamente derrubado em seguida de qualquer forma, e o
/// `kill_on_drop` configurado na criação do processo garante que ele nunca
/// fica órfão mesmo que este encerramento gracioso falhe ou trave.
async fn shutdown_child(child: &mut Child, port: u16) {
    if let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)).await {
        let _ = stream.write_all(b"nSHUTDOWN\n").await;
    }
    let _ = tokio::time::timeout(Duration::from_secs(3), child.wait()).await;
    let _ = child.start_kill();
    let _ = child.wait().await;
}

async fn cleanup_temp_files(conf_path: &Path, log_file: &Path) {
    let _ = tokio::fs::remove_file(conf_path).await;
    let _ = tokio::fs::remove_file(log_file).await;
}

/// Adaptador fino sobre uma instância de `clamd` gerenciada pelo CorvusAV.
pub struct ClamdAdapter {
    executable: PathBuf,
    database_dir: PathBuf,
}

impl ClamdAdapter {
    pub fn new(executable: PathBuf, database_dir: PathBuf) -> Self {
        Self {
            executable,
            database_dir,
        }
    }

    /// Localiza o `clamd` e a base de assinaturas da instalação atual do
    /// ClamAV. Retorna `None` sem assumir nada quando qualquer um dos dois
    /// não for encontrado — nesse caso a varredura usa só o `clamscan` (ver
    /// `crate::engine::ScanEngineAdapter`).
    pub fn from_system() -> Option<Self> {
        let executable = crate::detect::locate_clamd()?;
        let database_dir = crate::detect::resolve_database_dir(&executable)?;
        Some(Self::new(executable, database_dir))
    }

    pub fn executable_path(&self) -> &Path {
        &self.executable
    }

    /// Executa uma verificação recursiva sobre os caminhos informados,
    /// subindo uma instância dedicada de `clamd` para esta verificação e
    /// encerrando-a ao final (ver documentação do módulo sobre o escopo
    /// desta primeira versão). Mesma semântica de `cancel`/`timeout`/
    /// `on_entry` de `ClamAvAdapter::run_scan`.
    pub async fn run_scan(
        &self,
        targets: &[PathBuf],
        exclude_dirs: &[String],
        mut cancel: watch::Receiver<bool>,
        timeout: Duration,
        mut on_entry: impl FnMut(&RawScanEntry) + Send,
    ) -> Result<Vec<RawScanEntry>, ClamdError> {
        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let port = pick_port();
        let temp_dir = std::env::temp_dir();
        let instance_id = uuid::Uuid::new_v4();
        let log_file = temp_dir.join(format!("corvusav-clamd-{instance_id}.log"));
        let conf_path = temp_dir.join(format!("corvusav-clamd-{instance_id}.conf"));

        let config = ClamdConfig {
            database_dir: self.database_dir.clone(),
            port,
            max_threads: default_max_threads(),
            exclude_dirs: exclude_dirs.to_vec(),
            log_file: log_file.clone(),
        };
        tokio::fs::write(&conf_path, render_conf(&config)).await?;

        let mut child = match Command::new(&self.executable)
            .arg(format!("--config-file={}", conf_path.display()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                cleanup_temp_files(&conf_path, &log_file).await;
                return Err(ClamdError::Io(e));
            }
        };

        let startup_deadline = tokio::time::sleep(timeout.min(STARTUP_TIMEOUT));
        tokio::pin!(startup_deadline);

        let ready = loop {
            tokio::select! {
                biased;

                changed = cancel.changed() => {
                    if changed.is_ok() && *cancel.borrow() {
                        shutdown_child(&mut child, port).await;
                        cleanup_temp_files(&conf_path, &log_file).await;
                        return Err(ClamdError::Cancelled);
                    }
                }

                _ = &mut startup_deadline => {
                    break false;
                }

                _ = tokio::time::sleep(PING_INTERVAL) => {
                    if ping(port).await {
                        break true;
                    }
                }
            }
        };

        if !ready {
            let _ = child.start_kill();
            let _ = child.wait().await;
            cleanup_temp_files(&conf_path, &log_file).await;
            return Err(ClamdError::StartupTimedOut(STARTUP_TIMEOUT));
        }

        let overall_deadline = tokio::time::sleep(timeout);
        tokio::pin!(overall_deadline);

        let mut entries = Vec::new();
        let mut scan_error = None;

        'targets: for target in targets {
            let mut stream = match TcpStream::connect(("127.0.0.1", port)).await {
                Ok(s) => s,
                Err(e) => {
                    scan_error = Some(ClamdError::Io(e));
                    break;
                }
            };
            let command = format!("nMULTISCAN {}\n", target.display());
            if let Err(e) = stream.write_all(command.as_bytes()).await {
                scan_error = Some(ClamdError::Io(e));
                break;
            }

            let mut lines = BufReader::new(stream).lines();
            loop {
                tokio::select! {
                    biased;

                    changed = cancel.changed() => {
                        if changed.is_ok() && *cancel.borrow() {
                            scan_error = Some(ClamdError::Cancelled);
                            break 'targets;
                        }
                    }

                    _ = &mut overall_deadline => {
                        scan_error = Some(ClamdError::TimedOut);
                        break 'targets;
                    }

                    line = lines.next_line() => {
                        match line {
                            Ok(Some(raw_line)) => {
                                if let Some(entry) = parse_clamscan_line(&raw_line) {
                                    on_entry(&entry);
                                    entries.push(entry);
                                }
                            }
                            Ok(None) => break,
                            Err(e) => {
                                scan_error = Some(ClamdError::Io(e));
                                break 'targets;
                            }
                        }
                    }
                }
            }
        }

        shutdown_child(&mut child, port).await;
        cleanup_temp_files(&conf_path, &log_file).await;

        match scan_error {
            Some(e) => Err(e),
            None => Ok(entries),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_conf_with_expected_directives() {
        let config = ClamdConfig {
            database_dir: PathBuf::from("C:/ClamAV/database"),
            port: 23417,
            max_threads: 4,
            exclude_dirs: vec!["C:/Users/alguem/node_modules".to_string()],
            log_file: PathBuf::from("C:/temp/corvusav-clamd-teste.log"),
        };
        let conf = render_conf(&config);
        assert!(conf.contains("Foreground yes"));
        assert!(conf.contains("DatabaseDirectory C:/ClamAV/database"));
        assert!(conf.contains("TCPAddr 127.0.0.1"));
        assert!(conf.contains("TCPSocket 23417"));
        assert!(conf.contains("MaxThreads 4"));
        assert!(conf.contains("ExcludePath"));
    }

    #[test]
    fn renders_conf_without_exclude_path_when_no_exclusions() {
        let config = ClamdConfig {
            database_dir: PathBuf::from("C:/ClamAV/database"),
            port: 23417,
            max_threads: 4,
            exclude_dirs: Vec::new(),
            log_file: PathBuf::from("C:/temp/corvusav-clamd-teste.log"),
        };
        let conf = render_conf(&config);
        assert!(!conf.contains("ExcludePath"));
    }

    #[test]
    fn pick_port_stays_within_declared_range() {
        let port = pick_port();
        assert!((20000..40000).contains(&port));
    }

    #[test]
    fn default_max_threads_is_never_zero_and_respects_cap() {
        let threads = default_max_threads();
        assert!(threads >= 1);
        assert!(threads <= 16);
    }

    #[test]
    fn is_startup_failure_covers_only_pre_scan_errors() {
        assert!(ClamdError::BinaryNotFound.is_startup_failure());
        assert!(ClamdError::DatabaseDirUnknown.is_startup_failure());
        assert!(ClamdError::StartupTimedOut(STARTUP_TIMEOUT).is_startup_failure());
        assert!(!ClamdError::Cancelled.is_startup_failure());
        assert!(!ClamdError::TimedOut.is_startup_failure());
    }

    #[tokio::test]
    async fn empty_target_list_short_circuits_without_spawning() {
        let adapter = ClamdAdapter::new(PathBuf::from("clamd-inexistente"), PathBuf::from("."));
        let (_tx, rx) = watch::channel(false);
        let result = adapter
            .run_scan(&[], &[], rx, Duration::from_secs(1), |_| {})
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
