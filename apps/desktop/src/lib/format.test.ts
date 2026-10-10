import { describe, expect, it } from 'vitest'
import { formatDurationBetween, formatDurationMs, truncatePath } from './format'

describe('formatDurationMs', () => {
  it('formats sub-minute durations as seconds only', () => {
    expect(formatDurationMs(45_000)).toBe('45s')
  })

  it('formats durations over a minute as "Xm YYs"', () => {
    expect(formatDurationMs(154_000)).toBe('2m 34s')
  })

  it('clamps negative durations to zero', () => {
    expect(formatDurationMs(-500)).toBe('0s')
  })
})

describe('formatDurationBetween', () => {
  it('returns an em dash when either timestamp is missing', () => {
    expect(formatDurationBetween(null, '2026-01-01T00:00:10Z')).toBe('—')
    expect(formatDurationBetween('2026-01-01T00:00:00Z', null)).toBe('—')
  })

  it('computes the duration between two ISO timestamps', () => {
    expect(formatDurationBetween('2026-01-01T00:00:00Z', '2026-01-01T00:01:05Z')).toBe('1m 05s')
  })

  it('returns an em dash for an inverted (invalid) interval', () => {
    expect(formatDurationBetween('2026-01-01T00:01:00Z', '2026-01-01T00:00:00Z')).toBe('—')
  })
})

describe('truncatePath', () => {
  it('returns the original path when within the limit', () => {
    expect(truncatePath('C:\\curto.txt', 32)).toBe('C:\\curto.txt')
  })

  it('truncates a long path while preserving the file name', () => {
    const long = 'C:\\Users\\alguem\\Downloads\\pasta\\muito\\profunda\\arquivo-suspeito.exe'
    const result = truncatePath(long, 32)
    expect(result.length).toBeLessThanOrEqual(32)
    expect(result.endsWith('arquivo-suspeito.exe')).toBe(true)
  })
})
