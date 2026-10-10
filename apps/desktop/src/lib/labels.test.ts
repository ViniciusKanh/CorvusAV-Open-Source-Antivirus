import { describe, expect, it } from 'vitest'
import {
  engineAvailabilityLabel,
  scanKindLabel,
  scanStatusLabel,
  verdictLabel,
  verdictTone,
} from './labels'

describe('verdict labels', () => {
  it('never labels an error or unknown verdict as clean', () => {
    expect(verdictLabel('error')).not.toMatch(/nenhuma ameaça/i)
    expect(verdictLabel('unknown')).not.toMatch(/nenhuma ameaça/i)
  })

  it('maps malicious to the danger tone', () => {
    expect(verdictTone('malicious')).toBe('danger')
  })

  it('maps clean to the success tone', () => {
    expect(verdictTone('clean')).toBe('success')
  })
})

describe('scan status and kind labels', () => {
  it('labels every scan kind in Portuguese', () => {
    expect(scanKindLabel('quick')).toBe('Verificação rápida')
    expect(scanKindLabel('full')).toBe('Verificação completa')
    expect(scanKindLabel('custom')).toBe('Verificação personalizada')
  })

  it('labels the cancel_requested status distinctly from cancelled', () => {
    expect(scanStatusLabel('cancel_requested')).not.toBe(scanStatusLabel('cancelled'))
  })
})

describe('engine availability labels', () => {
  it('never reuses the "available" label for a degraded state', () => {
    const available = engineAvailabilityLabel('available')
    expect(engineAvailabilityLabel('unavailable')).not.toBe(available)
    expect(engineAvailabilityLabel('signatures_missing')).not.toBe(available)
    expect(engineAvailabilityLabel('initialization_failed')).not.toBe(available)
  })
})
