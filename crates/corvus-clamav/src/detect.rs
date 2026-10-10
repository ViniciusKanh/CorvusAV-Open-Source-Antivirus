//! Localização do executável `clamscan` e sondagem de disponibilidade do
//! mecanismo, sem jamais assumir que o ClamAV está instalado.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

use time::OffsetDateTime;

use corvus_core::{EngineAvailability, EngineHealth};

const ENGINE_NAME: &str = "ClamAV";
const PATH_OVERRIDE_VAR: &str = "CORVUSAV_CLAMAV_PATH";

/// Informações de versão extraídas de `clamscan --version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineVersionInfo {
    pub engine_version: String,
    pub signature_version: Option<String>,
}

/// Procura o executável `clamscan` nesta ordem: variável de ambiente
/// `CORVUSAV_CLAMAV_PATH` (configurável pelo usuário em Configurações),
/// diretórios do `PATH`, e por fim locais de instalação padrão no Windows.
/// Retorna `None` sem lançar erro quando nada é encontrado — cabe à camada
/// de UI comunicar essa condição ao usuário.
pub fn locate_clamscan() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "clamscan.exe" } else { "clamscan" };

    if let Some(custom) = std::env::var_os(PATH_OVERRIDE_VAR) {
        let path = PathBuf::from(custom);
        if path.is_file() {
            return Some(path);
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if cfg!(windows) {
        for dir in ["C:\\Program Files\\ClamAV", "C:\\ClamAV"] {
            let candidate = PathBuf::from(dir).join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Procura o executável `freshclam` desta instalação do ClamAV. Como o
/// `freshclam` é distribuído junto com o `clamscan` no mesmo diretório,
/// reaproveitamos o diretório de `CORVUSAV_CLAMAV_PATH` (quando configurado)
/// antes de cair para `PATH` e os locais padrão de instalação no Windows —
/// mas nunca assumimos que ele existe sem checar o sistema de arquivos.
pub fn locate_freshclam() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "freshclam.exe" } else { "freshclam" };

    if let Some(custom_clamscan) = std::env::var_os(PATH_OVERRIDE_VAR) {
        let clamscan_path = PathBuf::from(custom_clamscan);
        if let Some(dir) = clamscan_path.parent() {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if cfg!(windows) {
        for dir in ["C:\\Program Files\\ClamAV", "C:\\ClamAV"] {
            let candidate = PathBuf::from(dir).join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Procura o executável `clamd` desta instalação do ClamAV. Mesma lógica de
/// busca de `locate_freshclam` (mesmo diretório do `clamscan`, depois
/// `PATH`, depois locais padrão de instalação no Windows) — nunca assume
/// que o `clamd` existe sem checar o sistema de arquivos. Usado apenas pela
/// migração opcional para `clamd` (ver `crate::clamd`); sua ausência não é
/// um erro, só faz a varredura cair de volta para o `clamscan`.
pub fn locate_clamd() -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "clamd.exe" } else { "clamd" };

    if let Some(custom_clamscan) = std::env::var_os(PATH_OVERRIDE_VAR) {
        let clamscan_path = PathBuf::from(custom_clamscan);
        if let Some(dir) = clamscan_path.parent() {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if cfg!(windows) {
        for dir in ["C:\\Program Files\\ClamAV", "C:\\ClamAV"] {
            let candidate = PathBuf::from(dir).join(exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Interpreta a saída de `clamscan --version`, no formato
/// `ClamAV <versao>/<versao_assinaturas>/<data>`.
pub fn parse_version_output(raw: &str) -> Option<EngineVersionInfo> {
    let first_line = raw.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }
    let mut parts = first_line.split('/');
    let engine_version = parts.next()?.trim().to_string();
    let signature_version = parts.next().map(|s| s.trim().to_string());

    if engine_version.is_empty() {
        return None;
    }

    Some(EngineVersionInfo { engine_version, signature_version })
}

/// Nomes-base dos arquivos de assinatura oficiais do ClamAV. Cada um pode
/// existir em formato compactado (`.cvd`, como distribuído pelo FreshClam)
/// ou incremental (`.cld`, após atualizações parciais).
const SIGNATURE_DB_BASE_NAMES: [&str; 3] = ["daily", "main", "bytecode"];

/// Diretórios candidatos a conter a base de assinaturas de uma instalação
/// do ClamAV, a partir do executável (`clamscan`/`clamd`/`freshclam`, tanto
/// faz — todos vivem na mesma instalação): o subdiretório `database` ao
/// lado do executável (layout padrão do instalador oficial no Windows) e o
/// próprio diretório do executável (layout alternativo visto em algumas
/// distribuições).
fn candidate_database_dirs(exe: &Path) -> Option<[PathBuf; 2]> {
    let exe_dir = exe.parent()?;
    Some([exe_dir.join("database"), exe_dir.to_path_buf()])
}

/// Localiza o diretório que de fato contém os arquivos de assinatura
/// (`daily`/`main`/`bytecode`, em `.cvd` ou `.cld`) desta instalação do
/// ClamAV, verificando a presença real de pelo menos um desses arquivos no
/// disco — nunca assume um caminho padrão sem confirmar. Usado para
/// preencher `DatabaseDirectory` no `clamd.conf` gerado pela migração
/// opcional para `clamd` (ver `crate::clamd`).
pub fn resolve_database_dir(clamav_exe: &Path) -> Option<PathBuf> {
    let candidate_dirs = candidate_database_dirs(clamav_exe)?;
    for dir in &candidate_dirs {
        for base_name in SIGNATURE_DB_BASE_NAMES {
            for extension in ["cvd", "cld"] {
                if dir.join(format!("{base_name}.{extension}")).is_file() {
                    return Some(dir.clone());
                }
            }
        }
    }
    None
}

/// Data/hora real da assinatura mais recentemente atualizada no disco, lida
/// da data de modificação dos próprios arquivos de base (nunca inventada
/// nem copiada de uma mensagem de log). Procura tanto no diretório
/// `database` ao lado do executável (layout padrão do instalador oficial
/// no Windows) quanto no próprio diretório do executável, e retorna `None`
/// silenciosamente se nada for encontrado ou legível — a ausência desse
/// dado não deve impedir o resto da sondagem de saúde do mecanismo.
fn signatures_last_updated(clamscan_exe: &Path) -> Option<OffsetDateTime> {
    let exe_dir = clamscan_exe.parent()?;
    let candidate_dirs = [exe_dir.join("database"), exe_dir.to_path_buf()];

    let mut most_recent: Option<OffsetDateTime> = None;
    for dir in &candidate_dirs {
        for base_name in SIGNATURE_DB_BASE_NAMES {
            for extension in ["cvd", "cld"] {
                let path = dir.join(format!("{base_name}.{extension}"));
                let Ok(metadata) = std::fs::metadata(&path) else { continue };
                let Ok(modified) = metadata.modified() else { continue };
                let Ok(duration) = modified.duration_since(UNIX_EPOCH) else { continue };
                let Ok(timestamp) = OffsetDateTime::from_unix_timestamp(duration.as_secs() as i64) else {
                    continue;
                };
                let is_more_recent = match most_recent {
                    Some(current) => timestamp > current,
                    None => true,
                };
                if is_more_recent {
                    most_recent = Some(timestamp);
                }
            }
        }
    }
    most_recent
}

/// Sonda a disponibilidade real do mecanismo executando `--version`.
/// Nunca retorna `Available` sem ter obtido uma resposta válida do
/// executável. Esta função é bloqueante (usa `std::process::Command`
/// diretamente) por ser rápida; chamadores assíncronos devem envolvê-la em
/// `tokio::task::spawn_blocking`.
pub fn probe_health(executable: Option<&Path>) -> EngineHealth {
    let Some(exe) = executable else {
        return EngineHealth {
            engine_name: ENGINE_NAME.to_string(),
            availability: EngineAvailability::Unavailable,
            engine_version: None,
            signature_version: None,
            signatures_updated_at: None,
            last_error: Some(format!(
                "Executável do ClamAV (clamscan) não foi encontrado. Instale o ClamAV \
                 e/ou defina a variável de ambiente {PATH_OVERRIDE_VAR} apontando para \
                 clamscan.exe."
            )),
        };
    };

    match Command::new(exe).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            match parse_version_output(&stdout) {
                Some(info) => EngineHealth {
                    engine_name: ENGINE_NAME.to_string(),
                    availability: EngineAvailability::Available,
                    engine_version: Some(info.engine_version),
                    signature_version: info.signature_version,
                    signatures_updated_at: signatures_last_updated(exe),
                    last_error: None,
                },
                None => EngineHealth {
                    engine_name: ENGINE_NAME.to_string(),
                    availability: EngineAvailability::InitializationFailed,
                    engine_version: None,
                    signature_version: None,
                    signatures_updated_at: None,
                    last_error: Some(format!(
                        "Não foi possível interpretar a saída de versão do ClamAV: {stdout:?}"
                    )),
                },
            }
        }
        Ok(output) => EngineHealth {
            engine_name: ENGINE_NAME.to_string(),
            availability: EngineAvailability::InitializationFailed,
            engine_version: None,
            signature_version: None,
            signatures_updated_at: None,
            last_error: Some(format!(
                "clamscan --version retornou código {:?}: {}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr)
            )),
        },
        Err(e) => EngineHealth {
            engine_name: ENGINE_NAME.to_string(),
            availability: EngineAvailability::InitializationFailed,
            engine_version: None,
            signature_version: None,
            signatures_updated_at: None,
            last_error: Some(format!("Falha ao executar clamscan: {e}")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_version_line() {
        let info = parse_version_output("ClamAV 1.3.0/27425/Wed Oct  9 09:12:00 2026\n").unwrap();
        assert_eq!(info.engine_version, "ClamAV 1.3.0");
        assert_eq!(info.signature_version.as_deref(), Some("27425"));
    }

    #[test]
    fn empty_output_yields_none() {
        assert!(parse_version_output("").is_none());
        assert!(parse_version_output("\n").is_none());
    }

    #[test]
    fn missing_executable_is_reported_as_unavailable_not_available() {
        let health = probe_health(None);
        assert_eq!(health.availability, EngineAvailability::Unavailable);
        assert!(health.last_error.is_some());
    }

    #[test]
    fn finds_signature_date_from_database_subdirectory() {
        let dir = std::env::temp_dir().join(format!("corvusav-test-sigs-found-{}", std::process::id()));
        let db_dir = dir.join("database");
        std::fs::create_dir_all(&db_dir).expect("falha ao preparar diretório de teste");
        let exe = dir.join("clamscan.exe");
        std::fs::write(&exe, b"").expect("falha ao escrever executável fictício de teste");
        std::fs::write(db_dir.join("daily.cvd"), b"assinatura ficticia de teste")
            .expect("falha ao escrever arquivo de assinatura fictício");

        let found = signatures_last_updated(&exe);

        std::fs::remove_dir_all(&dir).ok();
        assert!(found.is_some());
    }

    #[test]
    fn missing_signature_files_yield_none_instead_of_a_guess() {
        let dir = std::env::temp_dir().join(format!("corvusav-test-sigs-missing-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("falha ao preparar diretório de teste");
        let exe = dir.join("clamscan.exe");
        std::fs::write(&exe, b"").expect("falha ao escrever executável fictício de teste");

        let found = signatures_last_updated(&exe);

        std::fs::remove_dir_all(&dir).ok();
        assert!(found.is_none());
    }
}
