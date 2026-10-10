export function formatDateTime(iso: string | null): string {
  if (!iso) return '—'
  const date = new Date(iso)
  if (Number.isNaN(date.getTime())) return '—'
  return new Intl.DateTimeFormat('pt-BR', {
    dateStyle: 'short',
    timeStyle: 'short',
  }).format(date)
}

/** Formata a duração entre dois ISO timestamps como "Xm YYs". */
export function formatDurationBetween(startIso: string | null, endIso: string | null): string {
  if (!startIso || !endIso) return '—'
  const start = new Date(startIso).getTime()
  const end = new Date(endIso).getTime()
  if (Number.isNaN(start) || Number.isNaN(end) || end < start) return '—'
  return formatDurationMs(end - start)
}

/** Formata uma duração em milissegundos como "Xm YYs" (ou apenas "Ys"). */
export function formatDurationMs(ms: number): string {
  const totalSeconds = Math.max(0, Math.round(ms / 1000))
  const minutes = Math.floor(totalSeconds / 60)
  const seconds = totalSeconds % 60
  if (minutes === 0) return `${seconds}s`
  return `${minutes}m ${seconds.toString().padStart(2, '0')}s`
}

/** Encurta um caminho de arquivo longo para exibição, preservando o nome
 * do arquivo (com uma reticência indicando que o início foi omitido). */
export function truncatePath(path: string, maxLength = 64): string {
  if (path.length <= maxLength) return path

  const separator = path.includes('\\') ? '\\' : '/'
  const parts = path.split(/[\\/]/).filter(Boolean)
  const fileName = parts.pop() ?? path
  const prefix = `…${separator}`

  if (fileName.length + prefix.length >= maxLength) {
    return `…${fileName.slice(-(maxLength - 1))}`
  }
  return `${prefix}${fileName}`
}
