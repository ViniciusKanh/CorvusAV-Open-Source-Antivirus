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

/** Contagem de resultados por veredito dentro de uma verificação. */
export interface VerdictCounts {
  clean: number
  suspicious: number
  malicious: number
  unknown: number
  error: number
}

/** Resultado real de uma tentativa de atualização de assinaturas via
 * `freshclam`, disparada manualmente pelo usuário na tela de Atualizações.
 * `output` é o texto bruto (stdout+stderr) do processo — nunca uma
 * mensagem genérica — para que falhas por permissão de escrita, por
 * exemplo, sejam explicadas com o motivo real relatado pelo próprio
 * freshclam. */
export interface FreshclamOutcome {
  success: boolean
  exit_code: number | null
  output: string
}

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

export type QuarantineState = 'quarantined' | 'restore_requested' | 'restored' | 'restore_failed'

/** Um arquivo isolado após detecção (ver crates/corvus-quarantine). O
 * CorvusAV nunca executa nem abre o arquivo original ou o isolado — só
 * move e, sob pedido explícito, restaura. */
export interface QuarantineItem {
  id: string
  scan_result_id: string | null
  original_path: string
  internal_name: string
  state: QuarantineState
  reason: string
  file_hash_before: string | null
  file_hash_after: string | null
  quarantined_at: string
  restored_at: string | null
  last_error: string | null
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
