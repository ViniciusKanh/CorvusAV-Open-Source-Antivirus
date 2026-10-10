-- Itens isolados em quarentena (ver crates/corvus-quarantine).
-- scan_result_id é opcional e nunca propaga exclusão: perder o resultado de
-- origem não deve apagar o registro de um arquivo que ainda está isolado em
-- disco — isso quebraria a auditoria do próprio isolamento.
CREATE TABLE IF NOT EXISTS quarantine_items (
    id                 TEXT PRIMARY KEY,
    scan_result_id     TEXT REFERENCES scan_results (id) ON DELETE SET NULL,
    original_path      TEXT NOT NULL,
    internal_name      TEXT NOT NULL,
    state              TEXT NOT NULL,
    reason             TEXT NOT NULL,
    file_hash_before   TEXT,
    file_hash_after    TEXT,
    quarantined_at     TEXT NOT NULL,
    restored_at        TEXT,
    last_error         TEXT
);

CREATE INDEX IF NOT EXISTS idx_quarantine_items_state ON quarantine_items (state);
CREATE INDEX IF NOT EXISTS idx_quarantine_items_quarantined_at ON quarantine_items (quarantined_at DESC);
