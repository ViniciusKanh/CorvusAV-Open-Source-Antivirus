//! Biblioteca do shell desktop do CorvusAV: inicializa o banco de dados, o
//! adaptador ClamAV e o orquestrador de verificações, e expõe comandos
//! Tauri para a interface React consumir.

mod commands;
mod state;

use std::sync::Arc;

use corvus_clamav::ClamAvAdapter;
use corvus_scanner::{ScanEvent, ScannerService};
use corvus_storage::Database;
use tauri::Emitter;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let db_path = corvus_storage::resolve_db_path()
        .expect("não foi possível determinar o diretório de dados do CorvusAV");
    let db = Arc::new(
        Database::open(&db_path).expect("falha ao abrir/migrar o banco de dados do CorvusAV"),
    );

    let clamav = ClamAvAdapter::from_system();
    if clamav.is_none() {
        tracing::warn!(
            "clamscan não foi encontrado no sistema; a verificação ficará indisponível \
             até que o ClamAV seja instalado (ver docs/development)."
        );
    }

    let scanner = Arc::new(ScannerService::new(db, clamav));
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel::<ScanEvent>();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { scanner, event_tx })
        .setup(move |app| {
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                while let Some(event) = event_rx.recv().await {
                    if let Err(error) = app_handle.emit("corvusav://scan-event", &event) {
                        tracing::error!(
                            %error,
                            "falha ao emitir evento de verificação para a interface"
                        );
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::engine_health,
            commands::start_quick_scan,
            commands::start_full_scan,
            commands::start_custom_scan,
            commands::cancel_scan,
            commands::recent_jobs,
            commands::job_results,
            commands::pick_custom_scan_targets,
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o CorvusAV");
}
