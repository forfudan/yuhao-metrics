/**
 * 集成測試：使用 yu 倉庫的真實數據驗證計算正確性
 *
 * 期望結果：
 * 靜態重碼：GB2312: 373, 通規: 572, 國字: 79, 通+國: 907, CJK: 5396
 * 動態選重：知乎: 2.47‱, 北語: 4.15‱, 臺繁: 4.65‱, 古籍: 22.14‱, 繁簡: 7.73‱
 */

import { describe, it, expect, beforeAll } from 'vitest'
import { calculateDynamicSelection, calculateStaticDuplicates } from '../ts/stats'
import { filterByCharset } from '../ts/charset'
import { loadCharsetData, loadCharFrequencies, parseCSV } from '../ts/data/loaders'

/** 字根 → 字根編碼。本測試自己造碼表用，不是本庫的類型（本庫不知道什麼是字根）。 */
type Zigen = { zigen: string; code: string; notes: string }

let zigenMap: Map<string, Zigen>
let codeTable: Map<string, string>
let charsetData: Record<string, any>
let charFrequencies: Record<string, Record<string, number>>

beforeAll(async () => {
  console.log('\n加載真實數據…\n')

  charsetData = await loadCharsetData()
  console.log('✓ 字符集數據')

  const freqData = await loadCharFrequencies()
  charFrequencies = {
    zhihu: freqData.zhihu,
    sc: freqData.sc,
    tc: freqData.tc,
    guji: freqData.guji
  }
  console.log('✓ 字頻數據')

  const zigenUrl = 'https://cdn.jsdelivr.net/gh/forfudan/yu@main/src/public/zigen-ling.csv'
  const zigenRes = await fetch(zigenUrl)
  if (!zigenRes.ok) {
    throw new Error(`Failed to load zigen table: ${zigenRes.status}`)
  }
  const zigenText = await zigenRes.text()
  const zigenRows = parseCSV(zigenText, true)

  zigenMap = new Map()
  zigenRows.forEach(row => {
    const [zigen, code, notes = ''] = row
    if (zigen && code) {
      zigenMap.set(zigen, { zigen, code, notes })
    }
  })
  console.log(`✓ 字根表 (${zigenMap.size} 個)`)

  const chaifenUrl = 'https://cdn.jsdelivr.net/gh/forfudan/yu@main/src/public/chaifen.csv'
  const chaifenRes = await fetch(chaifenUrl)
  if (!chaifenRes.ok) {
    throw new Error(`Failed to load chaifen table: ${chaifenRes.status}`)
  }
  const chaifenText = await chaifenRes.text()
  const chaifenRows = parseCSV(chaifenText, true)

  codeTable = new Map()
  chaifenRows.forEach(row => {
    const [char, division] = row
    if (char && division) {
      const code = makeCodesFromDivision(division, zigenMap)
      if (code && !code.includes('?')) {
        codeTable.set(char, code)
      }
    }
  })
  console.log(`✓ 編碼生成 (${codeTable.size} 個)\n`)
}, 60000)

function makeCodesFromDivision(division: string, zigenMap: Map<string, Zigen>): string {
  const divisionArray = [...division]
  const lenRoots = divisionArray.length

  const rootA = divisionArray[0]
  const rootB = divisionArray[1] || ''
  const rootC = divisionArray[2] || ''
  const rootZ = divisionArray[lenRoots - 1]

  const getMa = (root: string) => zigenMap.get(root)?.code || ''
  const maA = getMa(rootA)
  const maZ = getMa(rootZ)

  const isZhugen = (ma: string) => ma.length === 2
  const getDama = (ma: string) => ma[0] || ''
  const getShengma = (ma: string) => ma.length === 3 ? ma[1] : ''
  const getYunma = (ma: string) => ma[ma.length - 1] || ''

  const aIsZhugen = isZhugen(maA)
  const zIsZhugen = isZhugen(maZ)

  const aD = getDama(maA)
  const aS = getShengma(maA)
  const aY = getYunma(maA)
  const bD = getDama(getMa(rootB))
  const cD = getDama(getMa(rootC))
  const zD = getDama(maZ)
  const zS = getShengma(maZ)
  const zY = getYunma(maZ)

  let code = ''

  if (aIsZhugen) {
    if (lenRoots === 1) {
      code = aD + aY
    } else if (lenRoots === 2) {
      code = zIsZhugen ? aD + zD + zY : aD + zD + zS + zY
    } else if (lenRoots === 3) {
      code = zIsZhugen ? aD + bD + zD + zY : aD + bD + zD + zS
    } else {
      code = aD + bD + cD + zD
    }
  } else {
    if (lenRoots === 1) {
      code = aD + aS + aY
    } else if (lenRoots === 2) {
      code = zIsZhugen ? aD + aS + zD + zY : aD + aS + zD + zS
    } else {
      code = aD + aS + bD + zD
    }
  }

  return code.toLowerCase()
}

describe('Yu Repository Real Data Integration Test', () => {
  it('四份數據都拉得到', () => {
    expect(zigenMap.size).toBeGreaterThan(0)
    expect(codeTable.size).toBeGreaterThan(0)
    expect(charsetData).toBeDefined()
    expect(charFrequencies).toBeDefined()
  })

  it('靜態重碼與期望值相符', async () => {
    console.log('\n=== 靜態重碼統計 ===\n')

    const expected = {
      gb2312: 373,
      tonggui: 572,
      guozi: 79,
      tonggui_guozi: 907,
      cjk_basic: 5396,
    }

    console.log('字符集      | 實際值 | 期望值 | 狀態')
    console.log('------------|--------|--------|------')

    const chars = [...codeTable.keys()]
    for (const [charset, expectedValue] of Object.entries(expected)) {
      const inCharset = await filterByCharset(chars, charset, charsetData)
      const stats = await calculateStaticDuplicates(codeTable, inCharset)
      const actualValue = stats.duplicateCount
      const status = actualValue === expectedValue ? '✓' : '✗'
      console.log(
        `${charset.padEnd(12)} | ${actualValue.toString().padStart(6)} | ${expectedValue.toString().padStart(6)} | ${status}`
      )
      expect(actualValue).toBe(expectedValue)
    }
  })

  it('動態選重率與期望值相符', async () => {
    console.log('\n=== 動態選重率統計 ===\n')

    // 創建簡繁聯合頻率數據（合併 sc 和 tc）
    const unified: Record<string, number> = {}
    for (const [char, freq] of Object.entries(charFrequencies.sc)) {
      unified[char] = (unified[char] || 0) + freq
    }
    for (const [char, freq] of Object.entries(charFrequencies.tc)) {
      unified[char] = (unified[char] || 0) + freq
    }

    const expected = {
      zhihu: 2.47,
      sc: 4.15,
      tc: 4.65,
      guji: 22.14,
      unified: 7.73,
    }

    console.log('語料    | 實際值(‱) | 期望值(‱) | 誤差(‱) | 狀態')
    console.log('--------|-----------|-----------|---------|------')

    const tolerance = 0.5

    // 測試各語料庫
    const tests = [
      { corpus: 'zhihu', freq: charFrequencies.zhihu },
      { corpus: 'sc', freq: charFrequencies.sc },
      { corpus: 'tc', freq: charFrequencies.tc },
      { corpus: 'guji', freq: charFrequencies.guji },
      { corpus: 'unified', freq: unified }
    ]

    // 這裏原先是 `tests.forEach(async …)`：回調返回的 Promise 沒人等，於是裏面的
    // 斷言失敗也不會讓測試變紅。改成 for…of。
    for (const { corpus, freq } of tests) {
      const { rate } = await calculateDynamicSelection(codeTable, freq)
      const actualInWanfenbi = rate * 10000
      const expectedValue = expected[corpus as keyof typeof expected]
      const diff = Math.abs(actualInWanfenbi - expectedValue)
      const match = diff <= tolerance
      const status = match ? '✓' : '✗'

      console.log(
        `${corpus.padEnd(8)} | ${actualInWanfenbi.toFixed(2).padStart(9)} | ${expectedValue.toFixed(2).padStart(9)} | ${diff.toFixed(2).padStart(7)} | ${status}`
      )

      expect(diff).toBeLessThan(tolerance)
    }
  })
})
