//! Conexão SQLite e operações de persistência do CorvusAV.

use std::path::Path;
use std::sync::Mutex;

use corvus_core::{
    Detection, DetectionId, EngineHealth, QuarantineItem, QuarantineItemId, ScanJob, ScanJobId,
    ScanResult, ScanResultId, ScanStatus, VerdictCounts,
};
use std::collections::HashMap;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::codec::*;
use crate::error::{StorageError, StorageResult};
use crate::migrations::run_migrations;

/// Evento de auditoria persistido (mudanças de política, restauração de
/// quarentena, atualização de componentes e outras ações sensíveis).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub occurred_at: OffsetDateTime,
    pub event_type: String,
    pub description: String,
    pub metadata: serde_json::Value,
}

/// Camada de persistência SQLite do CorvusAV.
///
/// Protegida por um `Mutex` interno: esta é a única conexão do processo e o
/// SQLite não foi desenhado para escrita concorrente vinda de múltiplas
/// conexões sobre o mesmo arquivo. Chamadores assíncronos (Tauri commands)
/// devem invocar estes métodos via `tokio::task::spawn_blocking`.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Abre (criando se necessário) o banco em `path`, aplica pragmas de
    /// integridade/concorrência e roda as migrations pendentes.
    pub fn open(path: &Path) -> StorageResult<Self> {
        let mut conn = Connection::open(path)?;
        Self::configure(&mut conn)?;
        run_migrations(&mut conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Banco em memória, usado em testes e desenvolvimento isolado.
    pub fn open_in_memory() -> StorageResult<Self> {
        let mut conn = Connection::open_in_memory()?;
        Self::configure(&mut conn)?;
        run_migrations(&mut conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    fn configure(conn: &mut Connection) -> StorageResult<()> {
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // scan_jobs
    // ---------------------------------------------------------------

    pub fn insert_scan_job(&self, job: &ScanJob) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.execute(
            "INSERT INTO scan_jobs (
                id, kind, status, target_paths, created_at, started_at, finished_at,
                current_file, files_scanned, files_total, threats_found, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                job.id.to_string(),
                scan_kind_to_str(job.kind),
                scan_status_to_str(job.status),
                serde_json::to_string(&job.target_paths)?,
                to_rfc3339(job.created_at)?,
                job.started_at.map(to_rfc3339).transpose()?,
                job.finished_at.map(to_rfc3339).transpose()?,
                job.current_file,
                job.files_scanned as i64,
                job.files_total.map(|v| v as i64),
                job.threats_found as i64,
                job.error_message,
            ],
        )?;
        Ok(())
    }

    /// Atualiza os campos mutáveis de uma tarefa existente (status,
    /// progresso, horários). Falha se o registro não existir: evita
    /// mascarar um bug de sincronização como sucesso silencioso.
    pub fn update_scan_job(&self, job: &ScanJob) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let affected = conn.execute(
            "UPDATE scan_jobs SET
                status = ?2, started_at = ?3, finished_at = ?4, current_file = ?5,
                files_scanned = ?6, files_total = ?7, threats_found = ?8, error_message = ?9
             WHERE id = ?1",
            params![
                job.id.to_string(),
                scan_status_to_str(job.status),
                job.started_at.map(to_rfc3339).transpose()?,
                job.finished_at.map(to_rfc3339).transpose()?,
                job.current_file,
                job.files_scanned as i64,
                job.files_total.map(|v| v as i64),
                job.threats_found as i64,
                job.error_message,
            ],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound(format!("scan_job {}", job.id)));
        }
        Ok(())
    }

    pub fn get_scan_job(&self, id: ScanJobId) -> StorageResult<Option<ScanJob>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.query_row(
            "SELECT id, kind, status, target_paths, created_at, started_at, finished_at,
                    current_file, files_scanned, files_total, threats_found, error_message
             FROM scan_jobs WHERE id = ?1",
            params![id.to_string()],
            Self::row_to_scan_job,
        )
        .optional()
        .map_err(StorageError::from)
    }

    /// Tarefas mais recentes primeiro — alimenta a tabela de histórico da
    /// Central de Verificação.
    pub fn list_recent_jobs(&self, limit: u32) -> StorageResult<Vec<ScanJob>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let mut stmt = conn.prepare(
            "SELECT id, kind, status, target_paths, created_at, started_at, finished_at,
                    current_file, files_scanned, files_total, threats_found, error_message
             FROM scan_jobs ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], Self::row_to_scan_job)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn row_to_scan_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanJob> {
        let id: String = row.get(0)?;
        let kind: String = row.get(1)?;
        let status: String = row.get(2)?;
        let target_paths: String = row.get(3)?;
        let created_at: String = row.get(4)?;
        let started_at: Option<String> = row.get(5)?;
        let finished_at: Option<String> = row.get(6)?;
        let files_total: Option<i64> = row.get(9)?;

        Ok(ScanJob {
            id: ScanJobId::from_uuid(uuid::Uuid::parse_str(&id).map_err(sqlite_decode_err)?),
            kind: scan_kind_from_str(&kind).map_err(sqlite_decode_err)?,
            status: scan_status_from_str(&status).map_err(sqlite_decode_err)?,
            target_paths: serde_json::from_str(&target_paths).map_err(sqlite_decode_err)?,
            created_at: parse_rfc3339(&created_at).map_err(sqlite_decode_err)?,
            started_at: started_at
                .map(|s| parse_rfc3339(&s))
                .transpose()
                .map_err(sqlite_decode_err)?,
            finished_at: finished_at
                .map(|s| parse_rfc3339(&s))
                .transpose()
                .map_err(sqlite_decode_err)?,
            current_file: row.get(7)?,
            files_scanned: row.get::<_, i64>(8)? as u64,
            files_total: files_total.map(|v| v as u64),
            threats_found: row.get::<_, i64>(10)? as u64,
            error_message: row.get(11)?,
        })
    }

    /// Finaliza como `Failed` qualquer tarefa que tenha ficado presa em um
    /// estado não terminal (`queued`/`preparing`/`running`/`cancel_requested`)
    /// de uma execução anterior do aplicativo — por exemplo, se o CorvusAV
    /// foi fechado (ou travou) no meio de uma verificação. Sem isso, essas
    /// tarefas permaneceriam exibindo "Verificando" no histórico para
    /// sempre, mesmo que nenhum processo esteja de fato em execução.
    /// Chamada uma vez na inicialização do aplicativo, antes de qualquer
    /// nova verificação ser iniciada. Retorna quantas tarefas foram
    /// corrigidas.
    pub fn reconcile_interrupted_jobs(&self) -> StorageResult<u64> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let now = to_rfc3339(OffsetDateTime::now_utc())?;
        let affected = conn.execute(
            "UPDATE scan_jobs SET
                status = ?1,
                finished_at = COALESCE(finished_at, ?2),
                error_message = ?3
             WHERE status IN (?4, ?5, ?6, ?7)",
            params![
                scan_status_to_str(ScanStatus::Failed),
                now,
                "Interrompida: o CorvusAV foi encerrado antes da conclusão desta verificação.",
                scan_status_to_str(ScanStatus::Queued),
                scan_status_to_str(ScanStatus::Preparing),
                scan_status_to_str(ScanStatus::Running),
                scan_status_to_str(ScanStatus::CancelRequested),
            ],
        )?;
        Ok(affected as u64)
    }

    /// Remove do histórico as tarefas cujo status esteja em `statuses`.
    /// `scan_results`/`detections` associados são removidos em cascata
    /// (ver `ON DELETE CASCADE` no esquema). Deliberadamente não aceita
    /// estados não terminais (ver `ScanStatus::is_terminal`): o chamador
    /// (`ScannerService::clear_history`) é responsável por restringir a
    /// operação a estados seguros de apagar.
    pub fn delete_scan_jobs_by_status(&self, statuses: &[ScanStatus]) -> StorageResult<u64> {
        if statuses.is_empty() {
            return Ok(0);
        }
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let placeholders = statuses.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!("DELETE FROM scan_jobs WHERE status IN ({placeholders})");
        let values: Vec<&str> = statuses.iter().map(|s| scan_status_to_str(*s)).collect();
        let affected = conn.execute(&sql, rusqlite::params_from_iter(values))?;
        Ok(affected as u64)
    }

    // ---------------------------------------------------------------
    // scan_results + detections
    // ---------------------------------------------------------------

    /// Insere um resultado e suas detecções em uma única transação.
    pub fn insert_scan_result(&self, result: &ScanResult) -> StorageResult<()> {
        let mut conn = self.conn.lock().expect("mutex do banco envenenado");
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO scan_results (
                id, scan_job_id, file_path, file_hash, engines_used, verdict,
                duration_ms, examined_at, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                result.id.to_string(),
                result.scan_job_id.to_string(),
                result.file_path,
                result.file_hash,
                serde_json::to_string(&result.engines_used)?,
                verdict_to_str(result.verdict),
                result.duration_ms as i64,
                to_rfc3339(result.examined_at)?,
                result.error_message,
            ],
        )?;

        for detection in &result.detections {
            tx.execute(
                "INSERT INTO detections (
                    id, scan_result_id, engine_name, engine_version, signature_id, metadata
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    detection.id.to_string(),
                    detection.scan_result_id.to_string(),
                    detection.engine_name,
                    detection.engine_version,
                    detection.signature_id,
                    serde_json::to_string(&detection.metadata)?,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Contagem de vereditos por tarefa, para as tarefas em `job_ids`.
    /// Tarefas sem nenhum resultado registrado (ex.: falharam antes de
    /// examinar qualquer arquivo) simplesmente não aparecem no mapa — o
    /// chamador deve tratar a ausência como "nenhum resultado", nunca como
    /// `clean`.
    pub fn verdict_counts_for_jobs(
        &self,
        job_ids: &[ScanJobId],
    ) -> StorageResult<HashMap<ScanJobId, VerdictCounts>> {
        if job_ids.is_empty() {
            return Ok(HashMap::new());
        }
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let placeholders = job_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        let sql = format!(
            "SELECT scan_job_id, verdict FROM scan_results WHERE scan_job_id IN ({placeholders})"
        );
        let id_strings: Vec<String> = job_ids.iter().map(|id| id.to_string()).collect();
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(id_strings.iter()), |row| {
            let job_id: String = row.get(0)?;
            let verdict: String = row.get(1)?;
            Ok((job_id, verdict))
        })?;

        let mut counts: HashMap<ScanJobId, VerdictCounts> = HashMap::new();
        for row in rows {
            let (job_id_str, verdict_str) = row?;
            let job_id = ScanJobId::from_uuid(
                uuid::Uuid::parse_str(&job_id_str).map_err(sqlite_decode_err)?,
            );
            let verdict = verdict_from_str(&verdict_str).map_err(sqlite_decode_err)?;
            counts.entry(job_id).or_default().add(verdict);
        }
        Ok(counts)
    }

    pub fn list_results_for_job(&self, job_id: ScanJobId) -> StorageResult<Vec<ScanResult>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let mut stmt = conn.prepare(
            "SELECT id, scan_job_id, file_path, file_hash, engines_used, verdict,
                    duration_ms, examined_at, error_message
             FROM scan_results WHERE scan_job_id = ?1 ORDER BY examined_at ASC",
        )?;
        let mut results = stmt
            .query_map(params![job_id.to_string()], Self::row_to_scan_result)?
            .collect::<Result<Vec<_>, _>>()?;

        for result in &mut results {
            let mut dstmt = conn.prepare(
                "SELECT id, scan_result_id, engine_name, engine_version, signature_id, metadata
                 FROM detections WHERE scan_result_id = ?1",
            )?;
            result.detections = dstmt
                .query_map(params![result.id.to_string()], Self::row_to_detection)?
                .collect::<Result<Vec<_>, _>>()?;
        }

        Ok(results)
    }

    /// Busca um único resultado (com suas detecções) pelo seu id — usado,
    /// por exemplo, para localizar o arquivo de origem antes de colocá-lo
    /// em quarentena.
    pub fn get_scan_result(&self, id: ScanResultId) -> StorageResult<Option<ScanResult>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let mut result = conn
            .query_row(
                "SELECT id, scan_job_id, file_path, file_hash, engines_used, verdict,
                        duration_ms, examined_at, error_message
                 FROM scan_results WHERE id = ?1",
                params![id.to_string()],
                Self::row_to_scan_result,
            )
            .optional()?;

        if let Some(result) = result.as_mut() {
            let mut dstmt = conn.prepare(
                "SELECT id, scan_result_id, engine_name, engine_version, signature_id, metadata
                 FROM detections WHERE scan_result_id = ?1",
            )?;
            result.detections = dstmt
                .query_map(params![result.id.to_string()], Self::row_to_detection)?
                .collect::<Result<Vec<_>, _>>()?;
        }

        Ok(result)
    }

    fn row_to_scan_result(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanResult> {
        let id: String = row.get(0)?;
        let scan_job_id: String = row.get(1)?;
        let engines_used: String = row.get(4)?;
        let verdict: String = row.get(5)?;
        let examined_at: String = row.get(7)?;

        Ok(ScanResult {
            id: ScanResultId::from_uuid(uuid::Uuid::parse_str(&id).map_err(sqlite_decode_err)?),
            scan_job_id: ScanJobId::from_uuid(
                uuid::Uuid::parse_str(&scan_job_id).map_err(sqlite_decode_err)?,
            ),
            file_path: row.get(2)?,
            file_hash: row.get(3)?,
            engines_used: serde_json::from_str(&engines_used).map_err(sqlite_decode_err)?,
            verdict: verdict_from_str(&verdict).map_err(sqlite_decode_err)?,
            detections: Vec::new(),
            duration_ms: row.get::<_, i64>(6)? as u64,
            examined_at: parse_rfc3339(&examined_at).map_err(sqlite_decode_err)?,
            error_message: row.get(8)?,
        })
    }

    fn row_to_detection(row: &rusqlite::Row<'_>) -> rusqlite::Result<Detection> {
        let id: String = row.get(0)?;
        let scan_result_id: String = row.get(1)?;
        let metadata: String = row.get(5)?;

        Ok(Detection {
            id: DetectionId::from_uuid(uuid::Uuid::parse_str(&id).map_err(sqlite_decode_err)?),
            scan_result_id: ScanResultId::from_uuid(
                uuid::Uuid::parse_str(&scan_result_id).map_err(sqlite_decode_err)?,
            ),
            engine_name: row.get(2)?,
            engine_version: row.get(3)?,
            signature_id: row.get(4)?,
            metadata: serde_json::from_str(&metadata).map_err(sqlite_decode_err)?,
        })
    }

    // ---------------------------------------------------------------
    // engine_status
    // ---------------------------------------------------------------

    pub fn upsert_engine_status(&self, health: &EngineHealth) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.execute(
            "INSERT INTO engine_status (
                engine_name, availability, engine_version, signature_version,
                signatures_updated_at, last_error, checked_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (engine_name) DO UPDATE SET
                availability = excluded.availability,
                engine_version = excluded.engine_version,
                signature_version = excluded.signature_version,
                signatures_updated_at = excluded.signatures_updated_at,
                last_error = excluded.last_error,
                checked_at = excluded.checked_at",
            params![
                health.engine_name,
                engine_availability_to_str(health.availability),
                health.engine_version,
                health.signature_version,
                health.signatures_updated_at.map(to_rfc3339).transpose()?,
                health.last_error,
                to_rfc3339(OffsetDateTime::now_utc())?,
            ],
        )?;
        Ok(())
    }

    pub fn get_engine_status(&self, engine_name: &str) -> StorageResult<Option<EngineHealth>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.query_row(
            "SELECT engine_name, availability, engine_version, signature_version,
                    signatures_updated_at, last_error
             FROM engine_status WHERE engine_name = ?1",
            params![engine_name],
            |row| {
                let availability: String = row.get(1)?;
                let signatures_updated_at: Option<String> = row.get(4)?;
                Ok(EngineHealth {
                    engine_name: row.get(0)?,
                    availability: engine_availability_from_str(&availability)
                        .map_err(sqlite_decode_err)?,
                    engine_version: row.get(2)?,
                    signature_version: row.get(3)?,
                    signatures_updated_at: signatures_updated_at
                        .map(|s| parse_rfc3339(&s))
                        .transpose()
                        .map_err(sqlite_decode_err)?,
                    last_error: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(StorageError::from)
    }

    // ---------------------------------------------------------------
    // application_settings
    // ---------------------------------------------------------------

    pub fn get_setting(&self, key: &str) -> StorageResult<Option<String>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.query_row(
            "SELECT value FROM application_settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::from)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.execute(
            "INSERT INTO application_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, to_rfc3339(OffsetDateTime::now_utc())?],
        )?;
        Ok(())
    }

    // ---------------------------------------------------------------
    // audit_events
    // ---------------------------------------------------------------

    pub fn insert_audit_event(
        &self,
        event_type: &str,
        description: &str,
        metadata: serde_json::Value,
    ) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.execute(
            "INSERT INTO audit_events (id, occurred_at, event_type, description, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                to_rfc3339(OffsetDateTime::now_utc())?,
                event_type,
                description,
                serde_json::to_string(&metadata)?,
            ],
        )?;
        Ok(())
    }

    pub fn list_recent_audit_events(&self, limit: u32) -> StorageResult<Vec<AuditEvent>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let mut stmt = conn.prepare(
            "SELECT id, occurred_at, event_type, description, metadata
             FROM audit_events ORDER BY occurred_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map(params![limit], |row| {
                let occurred_at: String = row.get(1)?;
                let metadata: String = row.get(4)?;
                Ok(AuditEvent {
                    id: row.get(0)?,
                    occurred_at: parse_rfc3339(&occurred_at).map_err(sqlite_decode_err)?,
                    event_type: row.get(2)?,
                    description: row.get(3)?,
                    metadata: serde_json::from_str(&metadata).map_err(sqlite_decode_err)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ---------------------------------------------------------------
    // quarantine_items
    // ---------------------------------------------------------------

    pub fn insert_quarantine_item(&self, item: &QuarantineItem) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.execute(
            "INSERT INTO quarantine_items (
                id, scan_result_id, original_path, internal_name, state, reason,
                file_hash_before, file_hash_after, quarantined_at, restored_at, last_error
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                item.id.to_string(),
                item.scan_result_id.map(|id| id.to_string()),
                item.original_path,
                item.internal_name,
                quarantine_state_to_str(item.state),
                item.reason,
                item.file_hash_before,
                item.file_hash_after,
                to_rfc3339(item.quarantined_at)?,
                item.restored_at.map(to_rfc3339).transpose()?,
                item.last_error,
            ],
        )?;
        Ok(())
    }

    /// Atualiza os campos que mudam após a quarentena inicial (estado,
    /// hash pós-restauração, data de restauração e o último erro real).
    /// Falha se o item não existir, pelo mesmo motivo de `update_scan_job`:
    /// nunca mascarar uma dessincronização como sucesso silencioso.
    pub fn update_quarantine_item(&self, item: &QuarantineItem) -> StorageResult<()> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let affected = conn.execute(
            "UPDATE quarantine_items SET
                state = ?2, file_hash_after = ?3, restored_at = ?4, last_error = ?5
             WHERE id = ?1",
            params![
                item.id.to_string(),
                quarantine_state_to_str(item.state),
                item.file_hash_after,
                item.restored_at.map(to_rfc3339).transpose()?,
                item.last_error,
            ],
        )?;
        if affected == 0 {
            return Err(StorageError::NotFound(format!("quarantine_item {}", item.id)));
        }
        Ok(())
    }

    pub fn get_quarantine_item(&self, id: QuarantineItemId) -> StorageResult<Option<QuarantineItem>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        conn.query_row(
            "SELECT id, scan_result_id, original_path, internal_name, state, reason,
                    file_hash_before, file_hash_after, quarantined_at, restored_at, last_error
             FROM quarantine_items WHERE id = ?1",
            params![id.to_string()],
            Self::row_to_quarantine_item,
        )
        .optional()
        .map_err(StorageError::from)
    }

    /// Lista os itens em quarentena, mais recentes primeiro. Sem paginação
    /// nesta versão — volume esperado baixo (isolamento é uma ação manual
    /// por arquivo, nunca em massa).
    pub fn list_quarantine_items(&self) -> StorageResult<Vec<QuarantineItem>> {
        let conn = self.conn.lock().expect("mutex do banco envenenado");
        let mut stmt = conn.prepare(
            "SELECT id, scan_result_id, original_path, internal_name, state, reason,
                    file_hash_before, file_hash_after, quarantined_at, restored_at, last_error
             FROM quarantine_items ORDER BY quarantined_at DESC",
        )?;
        let rows = stmt
            .query_map([], Self::row_to_quarantine_item)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn row_to_quarantine_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<QuarantineItem> {
        let id: String = row.get(0)?;
        let scan_result_id: Option<String> = row.get(1)?;
        let state: String = row.get(4)?;
        let quarantined_at: String = row.get(8)?;
        let restored_at: Option<String> = row.get(9)?;

        Ok(QuarantineItem {
            id: QuarantineItemId::from_uuid(uuid::Uuid::parse_str(&id).map_err(sqlite_decode_err)?),
            scan_result_id: scan_result_id
                .map(|s| uuid::Uuid::parse_str(&s).map(ScanResultId::from_uuid))
                .transpose()
                .map_err(sqlite_decode_err)?,
            original_path: row.get(2)?,
            internal_name: row.get(3)?,
            state: quarantine_state_from_str(&state).map_err(sqlite_decode_err)?,
            reason: row.get(5)?,
            file_hash_before: row.get(6)?,
            file_hash_after: row.get(7)?,
            quarantined_at: parse_rfc3339(&quarantined_at).map_err(sqlite_decode_err)?,
            restored_at: restored_at
                .map(|s| parse_rfc3339(&s))
                .transpose()
                .map_err(sqlite_decode_err)?,
            last_error: row.get(10)?,
        })
    }
}

/// Converte um erro de decodificação (UUID, enum, timestamp, JSON) em um
/// `rusqlite::Error`, para poder ser propagado de dentro de closures de
/// `query_row`/`query_map`, preservando a mensagem original.
fn sqlite_decode_err(err: impl std::fmt::Display) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, err.to_string())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use corvus_core::{EngineAvailability, ScanKind, ScanStatus, Verdict};

    #[test]
    fn insert_and_fetch_scan_job_roundtrips() {
        let db = Database::open_in_memory().unwrap();
        let mut job = ScanJob::new(ScanKind::Quick, vec![]);
        db.insert_scan_job(&job).unwrap();

        job.transition_to(ScanStatus::Preparing).unwrap();
        job.transition_to(ScanStatus::Running).unwrap();
        job.files_scanned = 42;
        db.update_scan_job(&job).unwrap();

        let fetched = db.get_scan_job(job.id).unwrap().expect("job deveria existir");
        assert_eq!(fetched.status, ScanStatus::Running);
        assert_eq!(fetched.files_scanned, 42);
    }

    #[test]
    fn update_nonexistent_job_fails_instead_of_silently_succeeding() {
        let db = Database::open_in_memory().unwrap();
        let job = ScanJob::new(ScanKind::Quick, vec![]);
        let err = db.update_scan_job(&job).unwrap_err();
        assert!(matches!(err, StorageError::NotFound(_)));
    }

    #[test]
    fn scan_result_with_detections_roundtrips_and_lists_by_job() {
        let db = Database::open_in_memory().unwrap();
        let job = ScanJob::new(ScanKind::Custom, vec!["C:/Users/alguem/Downloads".into()]);
        db.insert_scan_job(&job).unwrap();

        let result_id = ScanResultId::new();
        let detection = Detection {
            id: DetectionId::new(),
            scan_result_id: result_id,
            engine_name: "clamav".into(),
            engine_version: Some("1.3.0".into()),
            signature_id: "Eicar-Test-Signature".into(),
            metadata: serde_json::json!({ "origem": "clamscan" }),
        };
        let result = ScanResult {
            id: result_id,
            scan_job_id: job.id,
            file_path: "C:/Users/alguem/Downloads/eicar.com".into(),
            file_hash: Some("44d88612fea8a8f36de82e1278abb02f".into()),
            engines_used: vec!["clamav".into()],
            verdict: Verdict::Malicious,
            detections: vec![detection],
            duration_ms: 120,
            examined_at: OffsetDateTime::now_utc(),
            error_message: None,
        };
        db.insert_scan_result(&result).unwrap();

        let fetched = db.list_results_for_job(job.id).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].verdict, Verdict::Malicious);
        assert_eq!(fetched[0].detections.len(), 1);
        assert_eq!(fetched[0].detections[0].signature_id, "Eicar-Test-Signature");
    }

    #[test]
    fn engine_status_upsert_overwrites_previous_value() {
        let db = Database::open_in_memory().unwrap();
        let health = EngineHealth {
            engine_name: "clamav".into(),
            availability: EngineAvailability::Unavailable,
            engine_version: None,
            signature_version: None,
            signatures_updated_at: None,
            last_error: Some("executável não encontrado".into()),
        };
        db.upsert_engine_status(&health).unwrap();

        let mut updated = health.clone();
        updated.availability = EngineAvailability::Available;
        updated.engine_version = Some("1.3.0".into());
        updated.last_error = None;
        db.upsert_engine_status(&updated).unwrap();

        let fetched = db.get_engine_status("clamav").unwrap().unwrap();
        assert_eq!(fetched.availability, EngineAvailability::Available);
        assert_eq!(fetched.engine_version.as_deref(), Some("1.3.0"));
    }

    #[test]
    fn settings_roundtrip() {
        let db = Database::open_in_memory().unwrap();
        assert_eq!(db.get_setting("tema").unwrap(), None);
        db.set_setting("tema", "dark").unwrap();
        assert_eq!(db.get_setting("tema").unwrap(), Some("dark".to_string()));
    }

    #[test]
    fn quarantine_item_roundtrips_and_updates() {
        use corvus_core::QuarantineState;

        let db = Database::open_in_memory().unwrap();
        let item = QuarantineItem {
            id: QuarantineItemId::new(),
            scan_result_id: None,
            original_path: "C:/Users/alguem/Downloads/eicar.com".into(),
            internal_name: "a1b2c3d4.bin".into(),
            state: QuarantineState::Quarantined,
            reason: "Ameaça encontrada: Eicar-Test-Signature".into(),
            file_hash_before: Some("44d88612fea8a8f36de82e1278abb02f".into()),
            file_hash_after: Some("44d88612fea8a8f36de82e1278abb02f".into()),
            quarantined_at: OffsetDateTime::now_utc(),
            restored_at: None,
            last_error: None,
        };
        db.insert_quarantine_item(&item).unwrap();

        let fetched = db.get_quarantine_item(item.id).unwrap().expect("item deveria existir");
        assert_eq!(fetched.state, QuarantineState::Quarantined);
        assert_eq!(fetched.original_path, item.original_path);

        let mut restored = fetched;
        restored.state = QuarantineState::Restored;
        restored.restored_at = Some(OffsetDateTime::now_utc());
        db.update_quarantine_item(&restored).unwrap();

        let after_restore = db.get_quarantine_item(item.id).unwrap().unwrap();
        assert_eq!(after_restore.state, QuarantineState::Restored);
        assert!(after_restore.restored_at.is_some());

        let all = db.list_quarantine_items().unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn update_nonexistent_quarantine_item_fails_instead_of_silently_succeeding() {
        use corvus_core::QuarantineState;

        let db = Database::open_in_memory().unwrap();
        let item = QuarantineItem {
            id: QuarantineItemId::new(),
            scan_result_id: None,
            original_path: "C:/x".into(),
            internal_name: "x.bin".into(),
            state: QuarantineState::Quarantined,
            reason: "teste".into(),
            file_hash_before: None,
            file_hash_after: None,
            quarantined_at: OffsetDateTime::now_utc(),
            restored_at: None,
            last_error: None,
        };
        let err = db.update_quarantine_item(&item).unwrap_err();
        assert!(matches!(err, StorageError::NotFound(_)));
    }

    #[test]
    fn audit_events_are_recorded_most_recent_first() {
        let db = Database::open_in_memory().unwrap();
        db.insert_audit_event("settings_changed", "Tema alterado", serde_json::json!({})).unwrap();
        db.insert_audit_event("scan_started", "Verificação rápida iniciada", serde_json::json!({})).unwrap();

        let events = db.list_recent_audit_events(10).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "scan_started");
    }
}
