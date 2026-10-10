//! Localização do executável `clamscan` e sondagem de disponibilidade do
//! mecanismo, sem jamais assumir que o ClamAV está instalado.

use std::path::{Path, PathBuf};
use std::process::Command;

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
                    signatures_updated_at: None,
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
}
