/**
 * 簡碼效率 —— 挑多少個字配簡碼，平均碼長能降到多少。
 */

import type { CharFrequency } from '../types/code-table.js'
import type { ShortCodeEfficiency } from '../types/stats.js'
import { getWasmModule } from '../wasm.js'

/**
 * 計算簡碼效率曲線。
 *
 * `shortCodes` / `fullCodes` 都應當已經帶上選重鍵——碼長算的是實際擊鍵數。
 * 收益 = 字頻 ×（全碼長 − 簡碼長），按收益降序取前 N 個配簡碼。
 */
export async function calculateShortCodeEfficiency(
  charFrequency: CharFrequency,
  shortCodes: Map<string, string> | Record<string, string>,
  fullCodes: Map<string, string> | Record<string, string>,
  maxCodeLength: number,
  ns: number[]
): Promise<ShortCodeEfficiency> {
  const wasm = await getWasmModule()
  const asObject = (m: Map<string, string> | Record<string, string>) =>
    m instanceof Map ? Object.fromEntries(m) : m

  const raw: any = wasm.short_code_efficiency(
    JSON.stringify(charFrequency),
    JSON.stringify(asObject(shortCodes)),
    JSON.stringify(asObject(fullCodes)),
    maxCodeLength,
    JSON.stringify(ns)
  )
  return {
    rankedCharacters: raw.ranked_characters,
    points: raw.points.map((p: any) => ({
      shortCodeChars: p.short_code_chars,
      weightedCodeLength: p.weighted_code_length,
    })),
  }
}
