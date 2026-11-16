/**
 * 字集歸屬 —— CJK 區塊與按表定義的字集。
 *
 * **這裏沒有任何區塊常量。** 從前 `cjk-blocks.json` 一份、`duplicate.rs` 裏寫死
 * 一份，而擴展 J 區的上界在 Unicode 17.0 動過，只動了其中一份。現在唯一的定義在
 * `rust/core/src/charset.rs`，這一層只是把它取過來（RECONCILIATION #4）。
 *
 * 也沒有 `isInCJKBasic` / `isInCJKExtA` / … 那 21 個幾乎一樣的函數。字集名字是
 * 數據，不該是函數名。
 */

import type { CJKBlock } from '../types/cjk-blocks.js'
import { getWasmModule } from '../wasm.js'

/**
 * 按表定義的字集數據：字符 → 歸屬標記。
 *
 * 只有 `gb2312` / `tonggui` / `guozi` / `tonggui_guozi` 用得上。
 */
export type CharsetData = Record<string, { is_gb2312?: boolean; is_tonggui?: boolean; is_guozi?: boolean }>

let blocks: CJKBlock[] | null = null

/** 全部 CJK 區塊定義。 */
export async function getCJKBlocks(): Promise<CJKBlock[]> {
  if (!blocks) {
    const wasm = await getWasmModule()
    blocks = wasm.cjk_blocks() as CJKBlock[]
  }
  return blocks
}

/** 取一個區塊。id 不存在時返回 `undefined`。 */
export async function getCJKBlock(blockId: string): Promise<CJKBlock | undefined> {
  return (await getCJKBlocks()).find(b => b.id === blockId)
}

/** 全部區塊 id。 */
export async function getAllBlockIds(): Promise<string[]> {
  return (await getCJKBlocks()).map(b => b.id)
}

/**
 * 累積字集 `cjk_to_*` 的區塊次序。
 *
 * **不是碼位次序**——擴展 I 區的碼位落在 F 和 G 之間，但「到 G 區」不含它。
 */
export async function getCumulativeBlockOrder(): Promise<string[]> {
  const wasm = await getWasmModule()
  return wasm.cumulative_block_order() as string[]
}

/** 字符是否落在指定區塊裏。 */
export async function isInCJKBlock(character: string, blockId: string): Promise<boolean> {
  const wasm = await getWasmModule()
  return wasm.is_in_block(character, blockId)
}

/** 字符落在哪些區塊裏。區塊互不重疊，所以最多一個。 */
export async function getCharCJKBlocks(character: string): Promise<string[]> {
  const wasm = await getWasmModule()
  return wasm.blocks_of(character) as string[]
}

/**
 * 字符是否屬於某個字集。
 *
 * 支持三類名字：
 *
 * - `cjk_basic` / `cjk_a`…`cjk_j`（也接受舊寫法 `cjk_ext_a`）——單個區塊。
 * - `cjk_to_a`…`cjk_to_j` —— 基本區起的累積範圍。
 * - `gb2312` / `tonggui` / `guozi` / `tonggui_guozi` —— 按表定義，需要 `data`。
 *
 * 名字不認識或者該給的表沒給，一律 `false`。
 */
export async function isInCharset(
  character: string,
  charset: string,
  data?: CharsetData
): Promise<boolean> {
  const wasm = await getWasmModule()
  return wasm.is_in_charset(character, charset, data ? JSON.stringify(data) : undefined)
}

/** 過濾出屬於某字集的字符，保持輸入順序。 */
export async function filterByCharset(
  characters: string[],
  charset: string,
  data?: CharsetData
): Promise<string[]> {
  const wasm = await getWasmModule()
  return wasm.filter_by_charset(
    JSON.stringify(characters),
    charset,
    data ? JSON.stringify(data) : undefined
  ) as string[]
}

export type { CJKBlock }
