//! Conexão SQLite e operações de persistência do CorvusAV.

use std::path::Path;
use std::sync::Mutex;

use corvus_core::{Detection, DetectionId, EngineHealth, ScanJob, ScanJobId, ScanResult, ScanResultId};
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
    fn audit_events_are_recorded_most_recent_first() {
        let db = Database::open_in_memory().unwrap();
        db.insert_audit_event("settings_changed", "Tema alterado", serde_json::json!({})).unwrap();
        db.insert_audit_event("scan_started", "Verificação rápida iniciada", serde_json::json!({})).unwrap();

        let events = db.list_recent_audit_events(10).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "scan_started");
    }
}
