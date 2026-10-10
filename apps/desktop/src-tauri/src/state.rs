use std::sync::Arc;

use corvus_scanner::{ScanEvent, ScannerService};
use tokio::sync::mpsc::UnboundedSender;

/// Estado compartilhado do aplicativo, injetado nos comandos Tauri.
pub struct AppState {
    pub scanner: Arc<ScannerService>,
    /// Canal usado por `ScannerService::start_scan` para publicar eventos;
    /// uma tarefa em `lib::run` os encaminha para a interface como eventos
    /// Tauri (`corvusav://scan-event`).
    pub event_tx: UnboundedSender<ScanEvent>,
}
