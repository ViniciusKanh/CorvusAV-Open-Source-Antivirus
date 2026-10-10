//! Saúde e disponibilidade de um mecanismo de varredura (ex.: ClamAV).

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Disponibilidade observada de um mecanismo de varredura.
///
/// Corresponde diretamente aos estados exigidos para o painel de status da
/// Central de Verificação. `Available` só deve ser reportado após validação
/// real (executável encontrado, versão obtida, assinaturas presentes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineAvailability {
    Available,
    Unavailable,
    SignaturesMissing,
    SignaturesOutdated,
    ScanInProgress,
    InitializationFailed,
}

/// Estado de saúde de um mecanismo de varredura em um dado momento.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHealth {
    pub engine_name: String,
    pub availability: EngineAvailability,
    pub engine_version: Option<String>,
    pub signature_version: Option<String>,
    pub signatures_updated_at: Option<OffsetDateTime>,
    /// Última mensagem de erro observada, quando aplicável.
    pub last_error: Option<String>,
}

impl EngineHealth {
    /// Verdadeiro apenas quando o mecanismo pode efetivamente ser usado
    /// para iniciar uma nova verificação agora.
    pub fn is_ready_to_scan(&self) -> bool {
        matches!(self.availability, EngineAvailability::Available)
    }
}
