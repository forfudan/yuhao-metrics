/**
 * 重碼 —— 靜態重碼與動態選重率。計算在 Rust 裏，這裏只做序列化與命名轉換。
 */

import type { CharFrequency, CodeTable } from '../types/code-table.js'
import type { DynamicSelection, NonFirstDuplicate, StaticDuplicates } from '../types/stats.js'
import { getWasmModule, serializeCodeTable } from '../wasm.js'

/**
 * 重碼組裏誰算首選。
 *
 * - `byFrequency`：按字頻降序重排，最高頻的是首選。評估「這方案理論上能多好」。
 * - `asListed`：照碼表列出的次序。評估「這方案現在發出去是什麼樣」。
 */
export type CandidateOrder = 'byFrequency' | 'asListed'

function selection(raw: any): DynamicSelection {
  return {
    duplicateMass: raw.duplicate_mass,
    totalMass: raw.total_mass,
    rate: raw.rate,
  }
}

/**
 * 靜態重碼：不看字頻，只看有多少字符跟別人撞了編碼。
 *
 * `charset` 給定時，重碼判定**只在該字符集內進行**——集外的字跟集內的字同碼不算
 * 重碼。GB2312 的重碼率不該被生僻字影響。
 */
export async function calculateStaticDuplicates(
  codeTable: CodeTable,
  charset?: Iterable<string>
): Promise<StaticDuplicates> {
  const wasm = await getWasmModule()
  const raw: any = wasm.static_duplicates(
    serializeCodeTable(codeTable),
    charset ? JSON.stringify(Array.from(charset)) : null
  )
  return {
    totalChars: raw.total_chars,
    duplicateCount: raw.duplicate_count,
    duplicateGroups: raw.duplicate_groups,
    duplicateRate: raw.duplicate_rate,
    uniqueCodes: raw.unique_codes,
    duplicateChars: raw.duplicate_chars,
  }
}

/**
 * 動態選重率：打一個字時，有多大比例的機會要多按一次選重鍵。
 */
export async function calculateDynamicSelection(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  order: CandidateOrder = 'byFrequency'
): Promise<DynamicSelection> {
  const wasm = await getWasmModule()
  return selection(
    wasm.dynamic_selection(
      serializeCodeTable(codeTable),
      JSON.stringify(charFrequency),
      order === 'asListed'
    )
  )
}

/**
 * 從**已帶選重鍵**的碼表算動態選重率——末位是數字就是要選重，不必重新分組。
 *
 * 碼表自洽時應當與 `calculateDynamicSelection(..., 'asListed')` 相等；不等說明選
 * 重鍵標錯了。
 */
export async function calculateDynamicSelectionFromSelectionKeys(
  codeTable: CodeTable,
  charFrequency: CharFrequency
): Promise<DynamicSelection> {
  const wasm = await getWasmModule()
  return selection(
    wasm.dynamic_selection_from_selection_keys(
      serializeCodeTable(codeTable),
      JSON.stringify(charFrequency)
    )
  )
}

/** 列出所有要按鍵選重的字符，按字頻降序。 */
export async function calculateNonFirstDuplicates(
  codeTable: CodeTable,
  charFrequency: CharFrequency,
  order: CandidateOrder = 'byFrequency'
): Promise<NonFirstDuplicate[]> {
  const wasm = await getWasmModule()
  const raw = wasm.non_first_duplicates(
    serializeCodeTable(codeTable),
    JSON.stringify(charFrequency),
    order === 'asListed'
  ) as any[]
  return raw.map(item => ({
    character: item.character,
    code: item.code,
    frequency: item.frequency,
    candidates: item.candidates,
    rank: item.rank,
  }))
}
