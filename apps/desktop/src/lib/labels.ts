import type { EngineAvailability, ScanKind, ScanStatus, Verdict } from '../types/corvus'

export function engineAvailabilityLabel(availability: EngineAvailability): string {
  switch (availability) {
    case 'available':
      return 'Motor disponível'
    case 'unavailable':
      return 'Motor indisponível'
    case 'signatures_missing':
      return 'Assinaturas ausentes'
    case 'signatures_outdated':
      return 'Assinaturas desatualizadas'
    case 'scan_in_progress':
      return 'Verificação em andamento'
    case 'initialization_failed':
      return 'Falha de inicialização'
  }
}

export function verdictLabel(verdict: Verdict): string {
  switch (verdict) {
    case 'clean':
      return 'Nenhuma ameaça encontrada'
    case 'suspicious':
      return 'Item suspeito'
    case 'malicious':
      return 'Ameaça encontrada'
    case 'unknown':
      return 'Resultado indeterminado'
    case 'error':
      return 'Erro ao examinar'
  }
}

export type Tone = 'success' | 'warning' | 'danger' | 'neutral'

export function verdictTone(verdict: Verdict): Tone {
  switch (verdict) {
    case 'clean':
      return 'success'
    case 'suspicious':
      return 'warning'
    case 'malicious':
      return 'danger'
    case 'unknown':
    case 'error':
      return 'neutral'
  }
}

export function scanStatusLabel(status: ScanStatus): string {
  switch (status) {
    case 'queued':
      return 'Na fila'
    case 'preparing':
      return 'Preparando'
    case 'running':
      return 'Verificando'
    case 'cancel_requested':
      return 'Cancelamento solicitado'
    case 'completed':
      return 'Concluída'
    case 'failed':
      return 'Falha'
    case 'cancelled':
      return 'Cancelada'
  }
}

export function scanStatusTone(status: ScanStatus): Tone {
  switch (status) {
    case 'completed':
      return 'success'
    case 'failed':
      return 'danger'
    case 'cancelled':
      return 'warning'
    default:
      return 'neutral'
  }
}

export function scanKindLabel(kind: ScanKind): string {
  switch (kind) {
    case 'quick':
      return 'Verificação rápida'
    case 'full':
      return 'Verificação completa'
    case 'custom':
      return 'Verificação personalizada'
  }
}
