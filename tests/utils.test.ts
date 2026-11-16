import { describe, it, expect } from 'vitest'
import { normalizeFrequency, mergeFrequencies } from '../ts/utils'

describe('normalizeFrequency', () => {
  it('歸一化之後總和是 1', () => {
    const freq = { '的': 100, '了': 50, '在': 50 }
    const normalized = normalizeFrequency(freq)

    expect(normalized['的']).toBeCloseTo(0.5)
    expect(normalized['了']).toBeCloseTo(0.25)
    expect(normalized['在']).toBeCloseTo(0.25)

    const sum = Object.values(normalized).reduce((a, b) => a + b, 0)
    expect(sum).toBeCloseTo(1)
  })

  it('空表不炸', () => {
    const normalized = normalizeFrequency({})
    expect(normalized).toEqual({})
  })
})

describe('mergeFrequencies', () => {
  it('合併多份字頻表', () => {
    const freq1 = { '的': 10, '了': 5 }
    const freq2 = { '的': 5, '在': 3 }

    const merged = mergeFrequencies(freq1, freq2)

    expect(merged['的']).toBe(15)
    expect(merged['了']).toBe(5)
    expect(merged['在']).toBe(3)
  })
})
