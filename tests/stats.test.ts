import { describe, it, expect } from 'vitest'
import {
  calculateCodeEquivalent,
  calculateDynamicSelection,
  calculateErgoScore,
  calculateKeyCounts,
  calculateShortCodeEfficiency,
  calculateSingleHandMass,
  calculateStaticDuplicates,
  calculateWeightedEquivalent,
  calculateNonFirstDuplicates,
  replaceSelectionKey,
} from '../ts/stats'
import type { CodeTable, EquivTable } from '../ts/types'

const 碼表: CodeTable = new Map([
  ['的', 'de'],
  ['得', 'de'],
  ['了', 'le'],
])
const 字頻 = { 的: 0.05, 得: 0.02, 了: 0.03 }

describe('靜態重碼', () => {
  it('數的是字符不是組', async () => {
    const s = await calculateStaticDuplicates(碼表)
    expect(s.totalChars).toBe(3)
    expect(s.duplicateCount).toBe(2)
    expect(s.duplicateGroups).toBe(1)
    expect(s.uniqueCodes).toBe(2)
    expect(s.duplicateRate).toBeCloseTo(2 / 3)
    expect(s.duplicateChars).toEqual(['的', '得'])
  })

  it('字符集之外的字不製造重碼', async () => {
    const s = await calculateStaticDuplicates(碼表, ['的', '了'])
    expect(s.totalChars).toBe(2)
    expect(s.duplicateCount).toBe(0)
  })

  it('空碼表不是 NaN', async () => {
    const s = await calculateStaticDuplicates(new Map())
    expect(s.totalChars).toBe(0)
    expect(s.duplicateRate).toBe(0)
  })
})

describe('動態選重率', () => {
  it('分子分母都返回', async () => {
    const r = await calculateDynamicSelection(碼表, 字頻)
    expect(r.duplicateMass).toBeCloseTo(0.02)
    expect(r.totalMass).toBeCloseTo(0.1)
    expect(r.rate).toBeCloseTo(0.2)
  })

  it('絕對頻數與歸一化頻率的 rate 相同、mass 不同', async () => {
    const 頻數 = { 的: 500, 得: 200, 了: 300 }
    const a = await calculateDynamicSelection(碼表, 頻數)
    const b = await calculateDynamicSelection(碼表, 字頻)
    expect(a.rate).toBeCloseTo(b.rate)
    expect(a.duplicateMass).toBe(200)
    expect(b.duplicateMass).toBeCloseTo(0.02)
  })

  it('asListed 取碼表第一行，不取最高頻', async () => {
    const 亂序: CodeTable = new Map([
      ['得', 'de'],
      ['的', 'de'],
    ])
    const listed = await calculateDynamicSelection(亂序, 字頻, 'asListed')
    const byFreq = await calculateDynamicSelection(亂序, 字頻, 'byFrequency')
    expect(listed.duplicateMass).toBeCloseTo(0.05)
    expect(byFreq.duplicateMass).toBeCloseTo(0.02)
  })

  it('列得出要選重的是哪些字', async () => {
    const d = await calculateNonFirstDuplicates(碼表, 字頻)
    expect(d).toHaveLength(1)
    expect(d[0].character).toBe('得')
    expect(d[0].rank).toBe(2)
    expect(d[0].candidates).toEqual(['的', '得'])
  })
})

describe('速度當量', () => {
  const 當量表: EquivTable = { df: 1.0, fg: 2.0, gh: 1.5 }

  it('單碼當量是相鄰碼對的均值', async () => {
    expect(await calculateCodeEquivalent('dfg', 當量表, 'skip')).toBeCloseTo(1.5)
  })

  it('查不到的碼對，三種算法是三個數', async () => {
    // fz 查不到：跳過 → 只剩 df；記零 → df 與 fz 各佔分母一格；
    // 按手兜底 → fz 兩鍵同在左手，算 1.5。
    expect(await calculateCodeEquivalent('dfz', 當量表, 'skip')).toBeCloseTo(1.0)
    expect(await calculateCodeEquivalent('dfz', 當量表, 'zero')).toBeCloseTo(0.5)
    expect(await calculateCodeEquivalent('dfz', 當量表, 'hand')).toBeCloseTo(1.25)
  })

  it('認不出來的 unknown 當場報錯，不默默回退', async () => {
    await expect(
      calculateCodeEquivalent('dfg', 當量表, 'bogus' as never)
    ).rejects.toThrow()
  })

  it('碼長不齊時兩種聚合不相等', async () => {
    const t: CodeTable = new Map([
      ['甲', 'df'],
      ['乙', 'fgh'],
    ])
    const f = { 甲: 0.5, 乙: 0.5 }
    const perChar = await calculateWeightedEquivalent(t, f, 當量表, {
      aggregation: 'perCharacter',
      unknown: 'skip',
    })
    const perPair = await calculateWeightedEquivalent(t, f, 當量表, {
      aggregation: 'perPair',
      unknown: 'skip',
    })
    expect(perChar).toBeCloseTo(1.375)
    expect(perPair).toBeCloseTo(1.5)
  })

  it('選重鍵只在整串數字有對應項時替換', async () => {
    const keys = { '2': ';', '3': "'" }
    expect(await replaceSelectionKey('abc2', keys)).toBe('abc;')
    expect(await replaceSelectionKey('abc12', keys)).toBe('abc12')
  })
})

describe('人機工程', () => {
  it('單手率要同時滿足單手與碼長', async () => {
    const t: CodeTable = new Map([
      ['甲', 'asdf'],
      ['乙', 'hjkl'],
      ['丙', 'asdj'],
      ['丁', 'as'],
    ])
    const f = { 甲: 0.1, 乙: 0.2, 丙: 0.3, 丁: 0.4 }
    expect(await calculateSingleHandMass(t, f, 4)).toBeCloseTo(0.3)
    expect(await calculateSingleHandMass(t, f, 2)).toBeCloseTo(0.7)
  })

  it('按鍵計數折疊大小寫，空格記作 space', async () => {
    const counts = await calculateKeyCounts(new Map([['甲', 'AS_']]), { 甲: 1 })
    expect(counts.a).toBe(1)
    expect(counts.s).toBe(1)
    expect(counts.space).toBe(1)
  })

  it('鍵位分數是個非負數', async () => {
    const score = await calculateErgoScore(碼表, 字頻)
    expect(score).toBeGreaterThanOrEqual(0)
    expect(Number.isFinite(score)).toBe(true)
  })
})

describe('簡碼效率', () => {
  it('多配簡碼不會讓平均碼長變長', async () => {
    const r = await calculateShortCodeEfficiency(
      { 的: 0.5, 是: 0.3, 我: 0.2 },
      { 的: 'e', 是: 'i', 我: 'o' },
      { 的: 'dwke', 是: 'jfhi', 我: 'trap' },
      4,
      [0, 1, 2, 3]
    )
    expect(r.points[0].weightedCodeLength).toBeCloseTo(4)
    expect(r.rankedCharacters[0]).toBe('的')
    const lengths = r.points.map(p => p.weightedCodeLength)
    expect(lengths).toEqual([...lengths].sort((a, b) => b - a))
  })
})
