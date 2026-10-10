//! Eventos emitidos durante a execução de uma verificação, destinados a
//! alimentar a interface em tempo real (via comandos/eventos Tauri).

use corvus_core::{ScanJob, ScanResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScanEvent {
    /// Estado/progresso da tarefa mudou (status, contadores, arquivo atual).
    JobUpdated(ScanJob),
    /// Um arquivo terminou de ser examinado.
    FileResult(ScanResult),
}
