//! Wrapper fino sobre `invoke`/`listen` do Tauri, isolando a interface do
//! detalhe de transporte e dos nomes exatos dos comandos do backend.

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type { EngineHealth, ScanEvent, ScanJob, ScanResult } from '../types/corvus'

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

export function pickCustomScanTargets(): Promise<string[]> {
  return invoke<string[]>('pick_custom_scan_targets')
}

export function onScanEvent(handler: (event: ScanEvent) => void): Promise<UnlistenFn> {
  return listen<ScanEvent>(SCAN_EVENT_CHANNEL, (message) => handler(message.payload))
}
