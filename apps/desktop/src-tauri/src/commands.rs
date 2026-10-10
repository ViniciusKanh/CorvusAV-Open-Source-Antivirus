//! Comandos Tauri expostos à interface. Cada comando valida e converte seus
//! próprios parâmetros; nenhuma chamada privilegiada ocorre sem a validação
//! correspondente nas camadas internas (`corvus-policy`, `corvus-scanner`).

use corvus_clamav::FreshclamOutcome;
use corvus_core::{
    EngineHealth, QuarantineItem, QuarantineItemId, ScanJob, ScanJobId, ScanKind, ScanResult,
    ScanResultId, VerdictCounts,
};
use std::collections::HashMap;
use tauri::State;

use crate::state::AppState;

fn parse_job_id(raw: &str) -> Result<ScanJobId, String> {
    uuid::Uuid::parse_str(raw)
        .map(ScanJobId::from_uuid)
        .map_err(|e| format!("identificador de tarefa inválido: {e}"))
}

fn parse_scan_result_id(raw: &str) -> Result<ScanResultId, String> {
    uuid::Uuid::parse_str(raw)
        .map(ScanResultId::from_uuid)
        .map_err(|e| format!("identificador de resultado inválido: {e}"))
}

fn parse_quarantine_item_id(raw: &str) -> Result<QuarantineItemId, String> {
    uuid::Uuid::parse_str(raw)
        .map(QuarantineItemId::from_uuid)
        .map_err(|e| format!("identificador de item de quarentena inválido: {e}"))
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

/// Remove do histórico as verificações com falha ou canceladas. Nunca
/// apaga uma tarefa em andamento (ver `ScannerService::clear_history`).
/// Retorna quantas entradas foram removidas.
#[tauri::command]
pub async fn clear_scan_history(state: State<'_, AppState>) -> Result<u64, String> {
    state.scanner.clear_history().await.map_err(|e| e.to_string())
}

/// Contagem de vereditos por tarefa, para as tarefas informadas (ver
/// `VerdictCounts`). Usado pelo histórico para mostrar um resumo de
/// qualidade de cada verificação sem buscar todos os resultados.
#[tauri::command]
pub async fn verdict_summaries(
    state: State<'_, AppState>,
    job_ids: Vec<String>,
) -> Result<HashMap<ScanJobId, VerdictCounts>, String> {
    let ids = job_ids
        .into_iter()
        .map(|id| parse_job_id(&id))
        .collect::<Result<Vec<_>, _>>()?;
    state.scanner.verdict_summaries(ids).await.map_err(|e| e.to_string())
}

/// Pastas atualmente excluídas de todas as verificações.
#[tauri::command]
pub async fn get_scan_exclusions(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    state.scanner.get_scan_exclusions().await.map_err(|e| e.to_string())
}

/// Substitui a lista de pastas excluídas das verificações.
#[tauri::command]
pub async fn set_scan_exclusions(
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> Result<(), String> {
    state.scanner.set_scan_exclusions(paths).await.map_err(|e| e.to_string())
}

/// Dispara uma atualização manual da base de assinaturas via `freshclam`.
/// Nunca é chamado automaticamente — só sob ação explícita do usuário na
/// tela de Atualizações. O resultado reflete exatamente o que o processo
/// `freshclam` reportou (código de saída e saída de texto), incluindo o
/// caso comum de falha por permissão de escrita quando o ClamAV está
/// instalado em "Program Files" e o CorvusAV roda sem elevação.
#[tauri::command]
pub async fn update_signatures(state: State<'_, AppState>) -> Result<FreshclamOutcome, String> {
    state.scanner.update_signatures().await.map_err(|e| e.to_string())
}

/// Move o arquivo de um resultado de verificação para a quarentena. O
/// `scan_result_id` é validado e o caminho real é buscado no banco pelo
/// backend — a interface nunca informa um caminho de arquivo diretamente
/// aqui.
#[tauri::command]
pub async fn quarantine_result(
    state: State<'_, AppState>,
    scan_result_id: String,
) -> Result<QuarantineItem, String> {
    let id = parse_scan_result_id(&scan_result_id)?;
    state.scanner.quarantine_scan_result(id).await.map_err(|e| e.to_string())
}

/// Lista os itens atualmente registrados em quarentena (qualquer estado),
/// mais recentes primeiro.
#[tauri::command]
pub async fn list_quarantine_items(state: State<'_, AppState>) -> Result<Vec<QuarantineItem>, String> {
    state.scanner.list_quarantine_items().await.map_err(|e| e.to_string())
}

/// Restaura um item da quarentena para seu caminho original. Recusa a
/// operação (sem mover nada) se o destino já existir ou se a integridade do
/// arquivo isolado não puder ser confirmada — ver `QuarantineManager::restore_item`.
#[tauri::command]
pub async fn restore_quarantine_item(
    state: State<'_, AppState>,
    item_id: String,
) -> Result<QuarantineItem, String> {
    let id = parse_quarantine_item_id(&item_id)?;
    state.scanner.restore_quarantine_item(id).await.map_err(|e| e.to_string())
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
