//! Execução de verificações com `clamscan` como processo externo isolado,
//! com suporte a progresso incremental, cancelamento cooperativo e limite
//! de tempo. Nunca usa shell para montar o comando (sem interpolação de
//! string) e nunca executa os arquivos examinados.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;

use corvus_core::EngineHealth;

use crate::detect::probe_health;

#[derive(Debug, Error)]
pub enum ClamAvError {
    #[error("ClamAV não está disponível: {0}")]
    Unavailable(String),
    #[error("falha de E/S ao executar o clamscan: {0}")]
    Io(#[from] std::io::Error),
    #[error("verificação cancelada pelo usuário")]
    Cancelled,
    #[error("tempo limite da verificação foi excedido")]
    TimedOut,
}

/// Veredito bruto do ClamAV para um único arquivo, antes de passar pelo
/// motor de política (`corvus-policy`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawVerdict {
    Clean,
    Infected { signature: String },
    /// Arquivo não pôde ser examinado (permissão negada, E/S, corrompido
    /// etc.). Nunca deve ser tratado como limpo pelas camadas superiores.
    Error { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawScanEntry {
    pub path: String,
    pub verdict: RawVerdict,
}

/// Interpreta uma linha de saída verbosa do clamscan:
/// `<caminho>: OK`, `<caminho>: <assinatura> FOUND` ou
/// `<caminho>: <mensagem> ERROR`. Linhas de resumo/cabeçalho são ignoradas
/// (retornam `None`) em vez de geradas como falsos resultados.
pub fn parse_clamscan_line(line: &str) -> Option<RawScanEntry> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (path, status) = line.rsplit_once(": ")?;
    let status = status.trim();

    if status == "OK" {
        return Some(RawScanEntry { path: path.to_string(), verdict: RawVerdict::Clean });
    }
    if let Some(signature) = status.strip_suffix(" FOUND") {
        return Some(RawScanEntry {
            path: path.to_string(),
            verdict: RawVerdict::Infected { signature: signature.trim().to_string() },
        });
    }
    if let Some(message) = status.strip_suffix(" ERROR") {
        return Some(RawScanEntry {
            path: path.to_string(),
            verdict: RawVerdict::Error { message: message.trim().to_string() },
        });
    }

    None
}

/// Adaptador fino sobre o executável `clamscan`.
pub struct ClamAvAdapter {
    executable: PathBuf,
}

impl ClamAvAdapter {
    pub fn new(executable: PathBuf) -> Self {
        Self { executable }
    }

    /// Localiza o `clamscan` no sistema. Retorna `None` sem assumir que o
    /// ClamAV está instalado.
    pub fn from_system() -> Option<Self> {
        crate::detect::locate_clamscan().map(Self::new)
    }

    pub fn executable_path(&self) -> &std::path::Path {
        &self.executable
    }

    /// Sonda a saúde/disponibilidade real do mecanismo (bloqueante; use
    /// `tokio::task::spawn_blocking` a partir de contexto assíncrono).
    pub fn probe_health(&self) -> EngineHealth {
        probe_health(Some(&self.executable))
    }

    /// Executa uma verificação recursiva sobre os caminhos informados.
    ///
    /// - `cancel`: canal de cancelamento cooperativo; `true` solicita o
    ///   encerramento imediato (e seguro) do processo `clamscan`.
    /// - `timeout`: limite superior de duração total da verificação.
    /// - `on_entry`: chamado para cada arquivo processado, permitindo à
    ///   camada de orquestração atualizar progresso em tempo real.
    ///
    /// Nunca trata um arquivo com erro/indeterminado como limpo: cada linha
    /// de saída é propagada com o veredito bruto exatamente como relatado.
    pub async fn run_scan(
        &self,
        targets: &[PathBuf],
        mut cancel: watch::Receiver<bool>,
        timeout: Duration,
        mut on_entry: impl FnMut(&RawScanEntry) + Send,
    ) -> Result<Vec<RawScanEntry>, ClamAvError> {
        if targets.is_empty() {
            return Ok(Vec::new());
        }

        let mut child = Command::new(&self.executable)
            .arg("--recursive")
            .arg("--verbose")
            .arg("--no-summary")
            .args(targets)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        let stdout = child.stdout.take().expect("stdout configurado como piped");
        let mut lines = BufReader::new(stdout).lines();

        let mut entries = Vec::new();
        let deadline = tokio::time::sleep(timeout);
        tokio::pin!(deadline);

        loop {
            tokio::select! {
                biased;

                changed = cancel.changed() => {
                    // Sender descartado é tratado como "não cancelar": o
                    // valor mais recente do canal continua sendo honrado.
                    if changed.is_ok() && *cancel.borrow() {
                        let _ = child.start_kill();
                        let _ = child.wait().await;
                        return Err(ClamAvError::Cancelled);
                    }
                }

                _ = &mut deadline => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    return Err(ClamAvError::TimedOut);
                }

                line = lines.next_line() => {
                    match line? {
                        Some(raw_line) => {
                            if let Some(entry) = parse_clamscan_line(&raw_line) {
                                on_entry(&entry);
                                entries.push(entry);
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        let status = child.wait().await?;
        // Códigos de saída do clamscan: 0 = nenhuma ameaça; 1 = ameaça(s)
        // encontrada(s) (ambos são execuções bem-sucedidas); 2 = erro de
        // execução. Um código 2 sem nenhuma entrada processada indica falha
        // real (ex.: argumento inválido, base de assinaturas ausente).
        if status.code() == Some(2) && entries.is_empty() {
            return Err(ClamAvError::Unavailable(
                "clamscan encerrou com código de erro (2) sem processar nenhum arquivo".into(),
            ));
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clean_line() {
        let entry = parse_clamscan_line("C:\\Users\\alguem\\arquivo.txt: OK").unwrap();
        assert_eq!(entry.path, "C:\\Users\\alguem\\arquivo.txt");
        assert_eq!(entry.verdict, RawVerdict::Clean);
    }

    #[test]
    fn parses_infected_line() {
        let entry =
            parse_clamscan_line("C:\\tmp\\eicar.com: Win.Test.EICAR_HDB-1 FOUND").unwrap();
        assert_eq!(
            entry.verdict,
            RawVerdict::Infected { signature: "Win.Test.EICAR_HDB-1".to_string() }
        );
    }

    #[test]
    fn parses_error_line_without_treating_as_clean() {
        let entry =
            parse_clamscan_line("C:\\protegido\\arquivo.dat: Permission denied ERROR").unwrap();
        assert_eq!(
            entry.verdict,
            RawVerdict::Error { message: "Permission denied".to_string() }
        );
    }

    #[test]
    fn ignores_blank_and_unrecognized_lines() {
        assert!(parse_clamscan_line("").is_none());
        assert!(parse_clamscan_line("----------- SCAN SUMMARY -----------").is_none());
    }

    #[tokio::test]
    async fn empty_target_list_short_circuits_without_spawning() {
        let adapter = ClamAvAdapter::new(PathBuf::from("clamscan-inexistente"));
        let (_tx, rx) = watch::channel(false);
        let result = adapter
            .run_scan(&[], rx, Duration::from_secs(1), |_| {})
            .await
            .unwrap();
        assert!(result.is_empty());
    }
}
