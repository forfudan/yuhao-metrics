/**
 * 速度當量 —— 敲完一個編碼要花多少「時間單位」。
 */

import type { CharFrequency, CodeTable, EquivTable, SelectionKeys } from '../types/code-table.js'
import type { EquivBucket } from '../types/stats.js'
import { getWasmModule, serializeCodeTable } from '../wasm.js'
import type { UnknownPair } from '../wasm.js'

/**
 * 聚合方式。**兩者不是同一個數。**
 *
 * - `perCharacter`：先把一個字的碼對取均值，再按字頻加權。`quant.py` 的行爲。
 * - `perPair`：把全表的碼對放在一起按出現頻率取均值。yuhao-assess 的行爲。
 *
 * 碼長一律相同且字頻歸一化時二者相等，碼長不齊時就不等——而簡碼表的碼長從來不齊。
 */
export type EquivAggregation = 'perCharacter' | 'perPair'

/** 算當量之前怎麼把編碼整理成「實際要敲的按鍵」。 */
export interface CodeNormalization {
  /** 把末尾的選重數字換成方案實際用的鍵。 */
  selectionKeys?: SelectionKeys
  /** 短於此長度的編碼末尾補一個 `_`（空格鍵）。 */
  padTo?: number
}

export interface EquivalentOptions extends CodeNormalization {
  aggregation?: EquivAggregation
  /** 當量表裏查不到的碼對怎麼算。默認 `'zero'`（記 0 但仍佔分母）。 */
  unknown?: UnknownPair
}

function keysJson(normalization: CodeNormalization): string | undefined {
  return normalization.selectionKeys ? JSON.stringify(normalization.selectionKeys) : undefined
}

/** 相鄰碼對的頻率：每個字的字頻，記到它編碼裏每一對相鄰按鍵上。 */
export async function calculatePairFrequencies(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  normalization: CodeNormalization = {}
): Promise<Record<string, number>> {
  const wasm = await getWasmModule()
  return wasm.pair_frequencies(
    serializeCodeTable(codeTable),
    JSON.stringify(charFrequency),
    keysJson(normalization),
    normalization.padTo
  ) as Record<string, number>
}

/** 字頻加權速度當量。 */
export async function calculateWeightedEquivalent(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  equivTable: EquivTable,
  options: EquivalentOptions = {}
): Promise<number> {
  const wasm = await getWasmModule()
  return wasm.weighted_equivalent(
    serializeCodeTable(codeTable),
    JSON.stringify(charFrequency),
    JSON.stringify(equivTable),
    options.aggregation === 'perPair',
    keysJson(options),
    options.padTo,
    options.unknown
  )
}

/** 單個編碼的速度當量：相鄰碼對當量的均值。 */
export async function calculateCodeEquivalent(
  code: string,
  equivTable: EquivTable,
  unknown?: UnknownPair
): Promise<number> {
  const wasm = await getWasmModule()
  return wasm.code_equivalent(code, JSON.stringify(equivTable), unknown)
}

/** 把碼對按當量值分檔（默認 1.0–2.1，每 0.1 一檔），按當量降序返回。 */
export async function calculateEquivalentDistribution(
  pairFrequencies: Record<string, number>,
  equivTable: EquivTable,
  range: [number, number] = [1.0, 2.1],
  step = 0.1
): Promise<EquivBucket[]> {
  const wasm = await getWasmModule()
  const raw = wasm.equivalent_distribution(
    JSON.stringify(pairFrequencies),
    JSON.stringify(equivTable),
    range[0],
    range[1],
    step
  ) as any[]
  return raw.map(item => ({
    equivValue: item.equiv_value,
    keyPairs: item.key_pairs,
    frequencyRatio: item.frequency_ratio,
  }))
}

/**
 * 把編碼末尾成串的選重數字換成方案實際使用的按鍵。
 *
 * 只在整串數字都在表裏有對應項時才替換，免得把「第 12 選」的 `12` 誤傷成 `1;`。
 */
export async function replaceSelectionKey(
  code: string,
  selectionKeys: SelectionKeys
): Promise<string> {
  const wasm = await getWasmModule()
  return wasm.replace_selection_key(code, JSON.stringify(selectionKeys))
}
