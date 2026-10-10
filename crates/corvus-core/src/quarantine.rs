//! Estados de itens movidos para a quarentena supervisionada.

use crate::ids::{QuarantineItemId, ScanResultId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Estado de um item em quarentena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineState {
    Quarantined,
    RestoreRequested,
    Restored,
    RestoreFailed,
}

/// Um arquivo isolado após detecção, aguardando decisão do usuário.
///
/// A primeira versão do CorvusAV não executa remoção definitiva nem
/// quarentena automática sem aprovação explícita (ver limitações do MVP);
/// este tipo já modela o ciclo de vida completo para quando essa função
/// for implementada.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuarantineItem {
    pub id: QuarantineItemId,
    pub scan_result_id: Option<ScanResultId>,
    pub original_path: String,
    /// Nome interno aleatório usado no armazenamento da quarentena (nunca o
    /// nome original, para reduzir risco de execução acidental).
    pub internal_name: String,
    pub state: QuarantineState,
    pub reason: String,
    pub file_hash_before: Option<String>,
    pub file_hash_after: Option<String>,
    pub quarantined_at: OffsetDateTime,
    pub restored_at: Option<OffsetDateTime>,
    /// Motivo real da última falha de restauração (estado `RestoreFailed`).
    /// `None` em qualquer outro estado — nunca inventamos uma explicação
    /// quando a operação deu certo ou ainda não foi tentada.
    pub last_error: Option<String>,
}
