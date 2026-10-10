//! Conversões explícitas entre tipos de domínio (`corvus-core`) e a
//! representação textual usada nas colunas SQLite.
//!
//! Evitamos serializar enums via `serde_json` diretamente em colunas TEXT
//! (o que produziria valores entre aspas, ex.: `"quick"`) para manter o
//! banco legível e consultável, e para falhar explicitamente diante de um
//! valor corrompido em vez de silenciosamente aceitar qualquer string.

use corvus_core::{EngineAvailability, ScanKind, ScanStatus, Verdict};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::error::{StorageError, StorageResult};

pub fn scan_kind_to_str(kind: ScanKind) -> &'static str {
    match kind {
        ScanKind::Quick => "quick",
        ScanKind::Full => "full",
        ScanKind::Custom => "custom",
    }
}

pub fn scan_kind_from_str(s: &str) -> StorageResult<ScanKind> {
    match s {
        "quick" => Ok(ScanKind::Quick),
        "full" => Ok(ScanKind::Full),
        "custom" => Ok(ScanKind::Custom),
        other => Err(StorageError::Database(rusqlite::Error::InvalidColumnType(
            0,
            format!("scan_kind desconhecido: {other}"),
            rusqlite::types::Type::Text,
        ))),
    }
}

pub fn scan_status_to_str(status: ScanStatus) -> &'static str {
    match status {
        ScanStatus::Queued => "queued",
        ScanStatus::Preparing => "preparing",
        ScanStatus::Running => "running",
        ScanStatus::CancelRequested => "cancel_requested",
        ScanStatus::Completed => "completed",
        ScanStatus::Failed => "failed",
        ScanStatus::Cancelled => "cancelled",
    }
}

pub fn scan_status_from_str(s: &str) -> StorageResult<ScanStatus> {
    match s {
        "queued" => Ok(ScanStatus::Queued),
        "preparing" => Ok(ScanStatus::Preparing),
        "running" => Ok(ScanStatus::Running),
        "cancel_requested" => Ok(ScanStatus::CancelRequested),
        "completed" => Ok(ScanStatus::Completed),
        "failed" => Ok(ScanStatus::Failed),
        "cancelled" => Ok(ScanStatus::Cancelled),
        other => Err(StorageError::Database(rusqlite::Error::InvalidColumnType(
            0,
            format!("scan_status desconhecido: {other}"),
            rusqlite::types::Type::Text,
        ))),
    }
}

pub fn verdict_to_str(v: Verdict) -> &'static str {
    match v {
        Verdict::Clean => "clean",
        Verdict::Suspicious => "suspicious",
        Verdict::Malicious => "malicious",
        Verdict::Unknown => "unknown",
        Verdict::Error => "error",
    }
}

pub fn verdict_from_str(s: &str) -> StorageResult<Verdict> {
    match s {
        "clean" => Ok(Verdict::Clean),
        "suspicious" => Ok(Verdict::Suspicious),
        "malicious" => Ok(Verdict::Malicious),
        "unknown" => Ok(Verdict::Unknown),
        "error" => Ok(Verdict::Error),
        other => Err(StorageError::Database(rusqlite::Error::InvalidColumnType(
            0,
            format!("verdict desconhecido: {other}"),
            rusqlite::types::Type::Text,
        ))),
    }
}

pub fn engine_availability_to_str(a: EngineAvailability) -> &'static str {
    match a {
        EngineAvailability::Available => "available",
        EngineAvailability::Unavailable => "unavailable",
        EngineAvailability::SignaturesMissing => "signatures_missing",
        EngineAvailability::SignaturesOutdated => "signatures_outdated",
        EngineAvailability::ScanInProgress => "scan_in_progress",
        EngineAvailability::InitializationFailed => "initialization_failed",
    }
}

pub fn engine_availability_from_str(s: &str) -> StorageResult<EngineAvailability> {
    match s {
        "available" => Ok(EngineAvailability::Available),
        "unavailable" => Ok(EngineAvailability::Unavailable),
        "signatures_missing" => Ok(EngineAvailability::SignaturesMissing),
        "signatures_outdated" => Ok(EngineAvailability::SignaturesOutdated),
        "scan_in_progress" => Ok(EngineAvailability::ScanInProgress),
        "initialization_failed" => Ok(EngineAvailability::InitializationFailed),
        other => Err(StorageError::Database(rusqlite::Error::InvalidColumnType(
            0,
            format!("engine_availability desconhecido: {other}"),
            rusqlite::types::Type::Text,
        ))),
    }
}

pub fn to_rfc3339(dt: OffsetDateTime) -> StorageResult<String> {
    dt.format(&Rfc3339)
        .map_err(|e| StorageError::InvalidTimestamp(e.to_string()))
}

pub fn parse_rfc3339(s: &str) -> StorageResult<OffsetDateTime> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|e| StorageError::InvalidTimestamp(e.to_string()))
}
