//! Tipos espelhando os contratos serializados pelo backend Rust
//! (`corvus-core`, `corvus-scanner`). Mantenha em sincronia com
//! `crates/corvus-core/src` e `crates/corvus-scanner/src/events.rs`.

export type ScanKind = 'quick' | 'full' | 'custom'

export type ScanStatus =
  | 'queued'
  | 'preparing'
  | 'running'
  | 'cancel_requested'
  | 'completed'
  | 'failed'
  | 'cancelled'

export type Verdict = 'clean' | 'suspicious' | 'malicious' | 'unknown' | 'error'

export type EngineAvailability =
  | 'available'
  | 'unavailable'
  | 'signatures_missing'
  | 'signatures_outdated'
  | 'scan_in_progress'
  | 'initialization_failed'

export interface EngineHealth {
  engine_name: string
  availability: EngineAvailability
  engine_version: string | null
  signature_version: string | null
  signatures_updated_at: string | null
  last_error: string | null
}

export interface ScanJob {
  id: string
  kind: ScanKind
  status: ScanStatus
  target_paths: string[]
  created_at: string
  started_at: string | null
  finished_at: string | null
  current_file: string | null
  files_scanned: number
  files_total: number | null
  threats_found: number
  error_message: string | null
}

export interface Detection {
  id: string
  scan_result_id: string
  engine_name: string
  engine_version: string | null
  signature_id: string
  metadata: unknown
}

export interface ScanResult {
  id: string
  scan_job_id: string
  file_path: string
  file_hash: string | null
  engines_used: string[]
  verdict: Verdict
  detections: Detection[]
  duration_ms: number
  examined_at: string
  error_message: string | null
}

// O backend serializa `ScanEvent` como enum internamente marcado
// (`#[serde(tag = "type")]`); como as variantes carregam structs, os campos
// de `ScanJob`/`ScanResult` ficam no mesmo nível de `type`, não aninhados.
export type ScanEvent =
  | ({ type: 'job_updated' } & ScanJob)
  | ({ type: 'file_result' } & ScanResult)

export const TERMINAL_SCAN_STATUSES: ReadonlySet<ScanStatus> = new Set([
  'completed',
  'failed',
  'cancelled',
])
