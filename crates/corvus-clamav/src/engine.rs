//! `ScanEngineAdapter`: ponto único de entrada para execução de
//! verificações, que tenta `clamd` primeiro (quando disponível) e cai para
//! `clamscan` automaticamente se o `clamd` não puder ser usado — ver
//! `crate::clamd` para o que conta como "não pôde ser usado" e por que uma
//! falha real durante a varredura nunca é escondida atrás dessa
//! alternância. `corvus-scanner` depende só deste tipo, nunca diretamente
//! de `ClamdAdapter`/`ClamAvAdapter` — a escolha de motor é um detalhe de
//! implementação interno a este crate.

use std::path::PathBuf;
use std::time::Duration;

use corvus_core::EngineHealth;
use tokio::sync::watch;

use crate::adapter::{ClamAvAdapter, ClamAvError, RawScanEntry};
use crate::clamd::{ClamdAdapter, ClamdError};

pub struct ScanEngineAdapter {
    clamscan: ClamAvAdapter,
    clamd: Option<ClamdAdapter>,
}

impl ScanEngineAdapter {
    /// Localiza o `clamscan` desta instalação (obrigatório — sem ele não há
    /// mecanismo nenhum) e, opcionalmente, o `clamd` da mesma instalação
    /// (quando presente, verificações tentam usá-lo primeiro). Retorna
    /// `None` só quando nem o `clamscan` é encontrado, mantendo o mesmo
    /// comportamento de antes desta migração para quem não tem `clamd`
    /// instalado.
    pub fn from_system() -> Option<Self> {
        let clamscan = ClamAvAdapter::from_system()?;
        let clamd = ClamdAdapter::from_system();
        if clamd.is_some() {
            tracing::info!(
                "clamd encontrado nesta instalação do ClamAV — verificações tentarão \
                 usá-lo antes de cair para o clamscan"
            );
        }
        Some(Self { clamscan, clamd })
    }

    pub fn executable_path(&self) -> &std::path::Path {
        self.clamscan.executable_path()
    }

    /// Saúde do mecanismo: continua baseada no `clamscan` (mesma instalação
    /// do ClamAV, mesma versão/base de assinaturas independentemente de
    /// qual dos dois é usado para varrer) — nenhuma mudança de
    /// comportamento aqui.
    pub fn probe_health(&self) -> EngineHealth {
        self.clamscan.probe_health()
    }

    pub async fn run_scan(
        &self,
        targets: &[PathBuf],
        exclude_dirs: &[String],
        cancel: watch::Receiver<bool>,
        timeout: Duration,
        mut on_entry: impl FnMut(&RawScanEntry) + Send,
    ) -> Result<Vec<RawScanEntry>, ClamAvError> {
        if let Some(clamd) = &self.clamd {
            match clamd
                .run_scan(targets, exclude_dirs, cancel.clone(), timeout, &mut on_entry)
                .await
            {
                Ok(entries) => return Ok(entries),
                Err(e) if e.is_startup_failure() => {
                    // Nenhuma entrada pode ter sido reportada ainda (por
                    // definição de "falha de inicialização"), então cair
                    // para o clamscan aqui nunca duplica nem omite
                    // resultados já emitidos ao chamador.
                    tracing::warn!(
                        error = %e,
                        "clamd não pôde ser usado nesta verificação; usando clamscan"
                    );
                }
                Err(e) => return Err(translate_clamd_error(e)),
            }
        }

        self.clamscan.run_scan(targets, exclude_dirs, cancel, timeout, on_entry).await
    }
}

/// Traduz uma falha do `clamd` ocorrida depois da inicialização (nunca uma
/// falha de inicialização — essas são tratadas internamente com fallback,
/// não chegam aqui) para o mesmo tipo de erro que `corvus-scanner` já sabe
/// interpretar vindo do `clamscan`, preservando o motivo original na
/// mensagem em vez de genericalizar a causa.
fn translate_clamd_error(error: ClamdError) -> ClamAvError {
    match error {
        ClamdError::Cancelled => ClamAvError::Cancelled,
        ClamdError::TimedOut => ClamAvError::TimedOut,
        ClamdError::Io(e) => ClamAvError::Io(e),
        // Inalcançável em uso normal (is_startup_failure() já filtrou
        // essas variantes antes de chegar aqui), mas tratado explicitamente
        // em vez de um `unreachable!()` para nunca derrubar o processo por
        // causa de uma mudança futura nesta função.
        other => ClamAvError::Unavailable(other.to_string()),
    }
}
