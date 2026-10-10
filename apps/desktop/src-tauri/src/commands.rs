//! Comandos Tauri expostos à interface. Cada comando valida e converte seus
//! próprios parâmetros; nenhuma chamada privilegiada ocorre sem a validação
//! correspondente nas camadas internas (`corvus-policy`, `corvus-scanner`).

use corvus_core::{EngineHealth, ScanJob, ScanJobId, ScanKind, ScanResult};
use tauri::State;

use crate::state::AppState;

fn parse_job_id(raw: &str) -> Result<ScanJobId, String> {
    uuid::Uuid::parse_str(raw)
        .map(ScanJobId::from_uuid)
        .map_err(|e| format!("identificador de tarefa inválido: {e}"))
}

#[tauri::command]
pub async fn engine_health(state: State<'_, AppState>) -> Result<EngineHealth, String> {
    Ok(state.scanner.engine_health().await)
}

#[tauri::command]
pub async fn start_quick_scan(state: State<'_, AppState>) -> Result<String, String> {
    state
        .scanner
        .start_scan(ScanKind::Quick, None, state.event_tx.clone())
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_full_scan(state: State<'_, AppState>) -> Result<String, String> {
    // Espera-se falhar com uma mensagem clara nesta versão: a verificação
    // completa ainda não está implementada (ver corvus-policy::full_scan_targets).
    state
        .scanner
        .start_scan(ScanKind::Full, None, state.event_tx.clone())
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_custom_scan(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<String, String> {
    let targets = paths.into_iter().map(std::path::PathBuf::from).collect();
    state
        .scanner
        .start_scan(ScanKind::Custom, Some(targets), state.event_tx.clone())
        .await
        .map(|id| id.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>, job_id: String) -> Result<(), String> {
    let id = parse_job_id(&job_id)?;
    state.scanner.cancel_scan(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn recent_jobs(state: State<'_, AppState>, limit: u32) -> Result<Vec<ScanJob>, String> {
    state.scanner.recent_jobs(limit).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn job_results(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Vec<ScanResult>, String> {
    let id = parse_job_id(&job_id)?;
    state.scanner.job_results(id).await.map_err(|e| e.to_string())
}

/// Abre o diálogo nativo de seleção de pastas para a verificação
/// personalizada. Retorna a lista de caminhos escolhidos (vazia se o
/// usuário cancelar).
#[tauri::command]
pub async fn pick_custom_scan_targets(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folders(move |folders| {
        let _ = tx.send(folders);
    });

    let folders = rx.await.map_err(|e| e.to_string())?;
    Ok(folders
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.to_string())
        .collect())
}
