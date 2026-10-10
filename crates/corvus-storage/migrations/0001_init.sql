-- Esquema inicial do CorvusAV (v0.1.0).
-- Convenções: IDs são UUIDs em texto; timestamps são RFC 3339 em UTC;
-- valores JSON (listas/objetos) são armazenados como TEXT serializado.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS scan_jobs (
    id              TEXT PRIMARY KEY,
    kind            TEXT NOT NULL,
    status          TEXT NOT NULL,
    target_paths    TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    started_at      TEXT,
    finished_at     TEXT,
    current_file    TEXT,
    files_scanned   INTEGER NOT NULL DEFAULT 0,
    files_total     INTEGER,
    threats_found   INTEGER NOT NULL DEFAULT 0,
    error_message   TEXT
);

CREATE INDEX IF NOT EXISTS idx_scan_jobs_created_at ON scan_jobs (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_scan_jobs_status ON scan_jobs (status);

CREATE TABLE IF NOT EXISTS scan_results (
    id              TEXT PRIMARY KEY,
    scan_job_id     TEXT NOT NULL REFERENCES scan_jobs (id) ON DELETE CASCADE,
    file_path       TEXT NOT NULL,
    file_hash       TEXT,
    engines_used    TEXT NOT NULL,
    verdict         TEXT NOT NULL,
    duration_ms     INTEGER NOT NULL,
    examined_at     TEXT NOT NULL,
    error_message   TEXT
);

CREATE INDEX IF NOT EXISTS idx_scan_results_job ON scan_results (scan_job_id);
CREATE INDEX IF NOT EXISTS idx_scan_results_verdict ON scan_results (verdict);

CREATE TABLE IF NOT EXISTS detections (
    id              TEXT PRIMARY KEY,
    scan_result_id  TEXT NOT NULL REFERENCES scan_results (id) ON DELETE CASCADE,
    engine_name     TEXT NOT NULL,
    engine_version  TEXT,
    signature_id    TEXT NOT NULL,
    metadata        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_detections_result ON detections (scan_result_id);

CREATE TABLE IF NOT EXISTS engine_status (
    engine_name             TEXT PRIMARY KEY,
    availability            TEXT NOT NULL,
    engine_version          TEXT,
    signature_version       TEXT,
    signatures_updated_at   TEXT,
    last_error              TEXT,
    checked_at              TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS application_settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS audit_events (
    id              TEXT PRIMARY KEY,
    occurred_at     TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    description     TEXT NOT NULL,
    metadata        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_events_time ON audit_events (occurred_at DESC);
