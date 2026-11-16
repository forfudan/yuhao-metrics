/**
 * 人機工程 —— 按鍵分佈、鍵位分數、單手率。
 */

import type { CharFrequency, CodeTable, SelectionKeys } from '../types/code-table.js'
import { getWasmModule, serializeCodeTable } from '../wasm.js'

/**
 * 鍵位分數的參數集。
 *
 * - `zigen`：字根碼位用（`quant.py:get_ergo_score`）。
 * - `shortCode`：簡碼用（`quant.py:get_ergo_score_for_pop`）。
 */
export type ErgoProfile = 'zigen' | 'shortCode'

/**
 * 按鍵計數（熱力圖）：每個按鍵被敲到的字頻加權次數。
 *
 * 數的是**全部編碼**，不只首選。`_` 記作 `space`，大寫按小寫算。
 */
export async function calculateKeyCounts(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  selectionKeys?: SelectionKeys
): Promise<Record<string, number>> {
  const wasm = await getWasmModule()
  return wasm.key_counts(
    serializeCodeTable(codeTable),
    JSON.stringify(charFrequency),
    selectionKeys ? JSON.stringify(selectionKeys) : undefined
  ) as Record<string, number>
}

/**
 * 鍵位分數：實測鍵頻分佈與理想分佈的偏離度，越小越好。
 *
 * `noZ` 只對 `shortCode` 有意義：把 z 的理想權重歸零。
 */
export async function calculateErgoScore(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  profile: ErgoProfile = 'zigen',
  noZ = false
): Promise<number> {
  const wasm = await getWasmModule()
  return wasm.ergo_score(
    serializeCodeTable(codeTable),
    JSON.stringify(charFrequency),
    profile === 'shortCode' ? 'short_code' : 'zigen',
    noZ
  )
}

/**
 * 單手率：編碼的所有按鍵落在同一隻手上的字符佔多少字頻。
 *
 * 返回的是**頻率質量**——字頻歸一化時它就是比例，不歸一化時隨總頻數縮放。
 */
export async function calculateSingleHandMass(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  minLen = 4
): Promise<number> {
  const wasm = await getWasmModule()
  return wasm.single_hand_mass(
    serializeCodeTable(codeTable),
    JSON.stringify(charFrequency),
    minLen
  )
}
