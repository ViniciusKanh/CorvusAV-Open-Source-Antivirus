//! Wrapper fino sobre `invoke`/`listen` do Tauri, isolando a interface do
//! detalhe de transporte e dos nomes exatos dos comandos do backend.

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  EngineHealth,
  FreshclamOutcome,
  QuarantineItem,
  ScanEvent,
  ScanJob,
  ScanResult,
  VerdictCounts,
} from '../types/corvus'

const SCAN_EVENT_CHANNEL = 'corvusav://scan-event'

export function fetchEngineHealth(): Promise<EngineHealth> {
  return invoke<EngineHealth>('engine_health')
}

export function startQuickScan(): Promise<string> {
  return invoke<string>('start_quick_scan')
}

export function startFullScan(): Promise<string> {
  return invoke<string>('start_full_scan')
}

export function startCustomScan(paths: string[]): Promise<string> {
  return invoke<string>('start_custom_scan', { paths })
}

export function cancelScan(jobId: string): Promise<void> {
  return invoke<void>('cancel_scan', { jobId })
}

export function fetchRecentJobs(limit: number): Promise<ScanJob[]> {
  return invoke<ScanJob[]>('recent_jobs', { limit })
}

export function fetchJobResults(jobId: string): Promise<ScanResult[]> {
  return invoke<ScanResult[]>('job_results', { jobId })
}

/// Remove do histórico as verificações com falha ou canceladas. Retorna
/// quantas entradas foram removidas.
export function clearScanHistory(): Promise<number> {
  return invoke<number>('clear_scan_history')
}

export function pickCustomScanTargets(): Promise<string[]> {
  return invoke<string[]>('pick_custom_scan_targets')
}

/// Retorna, para cada ID de verificação informado, a contagem de
/// resultados por veredito (limpo/suspeito/malicioso/indeterminado/erro).
/// Verificações sem nenhum resultado registrado simplesmente não aparecem
/// no mapa retornado.
export function fetchVerdictSummaries(jobIds: string[]): Promise<Record<string, VerdictCounts>> {
  return invoke<Record<string, VerdictCounts>>('verdict_summaries', { jobIds })
}

/// Lê a lista de caminhos excluídos das verificações.
export function getScanExclusions(): Promise<string[]> {
  return invoke<string[]>('get_scan_exclusions')
}

/// Substitui a lista de caminhos excluídos das verificações.
export function setScanExclusions(paths: string[]): Promise<void> {
  return invoke<void>('set_scan_exclusions', { paths })
}

/// Dispara uma atualização manual da base de assinaturas via freshclam.
/// Nunca é chamado automaticamente — só sob ação explícita do usuário.
export function updateSignatures(): Promise<FreshclamOutcome> {
  return invoke<FreshclamOutcome>('update_signatures')
}

/// Move o arquivo de um resultado de verificação para a quarentena. O
/// backend busca o caminho real a partir de `scanResultId` — nunca
/// enviamos um caminho de arquivo diretamente daqui.
export function quarantineResult(scanResultId: string): Promise<QuarantineItem> {
  return invoke<QuarantineItem>('quarantine_result', { scanResultId })
}

/// Lista os itens atualmente registrados em quarentena (qualquer estado).
export function listQuarantineItems(): Promise<QuarantineItem[]> {
  return invoke<QuarantineItem[]>('list_quarantine_items')
}

/// Restaura um item da quarentena para seu caminho original.
export function restoreQuarantineItem(itemId: string): Promise<QuarantineItem> {
  return invoke<QuarantineItem>('restore_quarantine_item', { itemId })
}

export function onScanEvent(handler: (event: ScanEvent) => void): Promise<UnlistenFn> {
  return listen<ScanEvent>(SCAN_EVENT_CHANNEL, (message) => handler(message.payload))
}
