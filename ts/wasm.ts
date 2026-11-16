/**
 * WASM 加載器 —— 本庫的計算全部在 Rust 裏，這裏只負責把模塊取來。
 *
 * 沒有 JS 回退。同一個公式寫兩遍，遲早有一遍是錯的，而錯的那遍平時不跑，
 * 誰也不知道它錯了（見 docs/RESTRUCTURE.md §8.2）。
 */

/** 碼表的傳輸形態。數組形態保序，對象形態不保序——`asListed` 要用前者。 */
export type CodeTableJson = Record<string, string | string[]> | Array<[string, string | string[]]>

/**
 * 當量表查不到的碼對怎麼算。三個名字與 Rust 的 `UnknownPair` 及 PyO3 綁定相同。
 *
 * - `zero`：計 0，但仍佔分母一格（默認）。
 * - `skip`：整個跳過，分子分母都不計。
 * - `hand`：按左右手兜底（左手接右手 1.1／同鍵 1.3／其餘 1.5，含空格的碼對不計）。
 */
export type UnknownPair = 'zero' | 'skip' | 'hand'

/** Rust 側 `#[wasm_bindgen]` 導出的函數。簽名與 `rust/wasm/src/lib.rs` 一一對應。 */
interface WasmModule {
  static_duplicates(codeTableJson: string, charsetJson: string | null): unknown
  dynamic_selection(codeTableJson: string, frequencyJson: string, asListed: boolean): unknown
  dynamic_selection_from_selection_keys(codeTableJson: string, frequencyJson: string): unknown
  non_first_duplicates(codeTableJson: string, frequencyJson: string, asListed: boolean): unknown

  pair_frequencies(
    codeTableJson: string,
    frequencyJson: string,
    selectionKeysJson: string | undefined,
    padTo: number | undefined
  ): unknown
  weighted_equivalent(
    codeTableJson: string,
    frequencyJson: string,
    equivTableJson: string,
    perPair: boolean,
    selectionKeysJson: string | undefined,
    padTo: number | undefined,
    unknown: UnknownPair | undefined
  ): number
  code_equivalent(code: string, equivTableJson: string, unknown: UnknownPair | undefined): number
  equivalent_distribution(
    pairFrequenciesJson: string,
    equivTableJson: string,
    low: number,
    high: number,
    step: number
  ): unknown
  replace_selection_key(code: string, selectionKeysJson: string): string

  key_counts(
    codeTableJson: string,
    frequencyJson: string,
    selectionKeysJson: string | undefined
  ): unknown
  ergo_score(
    codeTableJson: string,
    frequencyJson: string,
    profile: string,
    noZ: boolean
  ): number
  single_hand_mass(codeTableJson: string, frequencyJson: string, minLen: number): number

  short_code_efficiency(
    frequencyJson: string,
    shortCodesJson: string,
    fullCodesJson: string,
    maxCodeLength: number,
    nsJson: string
  ): unknown

  cjk_blocks(): unknown
  cumulative_block_order(): unknown
  is_in_block(character: string, blockId: string): boolean
  blocks_of(character: string): unknown
  is_in_charset(character: string, charset: string, charsetDataJson: string | undefined): boolean
  filter_by_charset(
    charactersJson: string,
    charset: string,
    charsetDataJson: string | undefined
  ): unknown
}

let wasmModule: WasmModule | null = null
let wasmInitPromise: Promise<WasmModule> | null = null

/** 是否跑在 Node 裏。 */
function isNode(): boolean {
  return typeof process !== 'undefined' && process.versions?.node != null
}

/**
 * wasm-pack 出兩份產物，這裏按運行時挑一份：
 *
 * - `wasm/`（`--target web`）：瀏覽器用，`default()` 自己去 fetch 那個 .wasm。
 * - `wasm-node/`（`--target nodejs`）：Node 用，require 時就把二進制讀進來了，
 *   不必初始化。
 *
 * 之前這裏只用 web 那份，在 Node 裏手動讀字節餵給它——`require`/`__dirname` 在
 * ESM 包裏根本不存在，那條路從來沒跑通過；換成 `node:fs` 之後又觸發 vite 把
 * .wasm 當資源內聯成 187 KB 的 data: URL。兩份產物本來就是爲這件事準備的。
 */
async function initWasm(): Promise<WasmModule> {
  if (wasmModule) {
    return wasmModule
  }
  if (wasmInitPromise) {
    return wasmInitPromise
  }

  wasmInitPromise = (async () => {
    try {
      if (isNode()) {
        const mod: any = await import('../wasm-node/yuhao_metrics.js')
        wasmModule = (mod.default ?? mod) as WasmModule
      } else {
        const mod: any = await import('../wasm/yuhao_metrics.js')
        await mod.default()
        wasmModule = mod as WasmModule
      }
      return wasmModule
    } catch (error) {
      wasmInitPromise = null
      throw new Error(
        `WASM 加載失敗: ${error instanceof Error ? error.message : String(error)}`
      )
    }
  })()

  return wasmInitPromise
}

/** 取已初始化的 WASM 模塊，未初始化則先初始化。 */
export async function getWasmModule(): Promise<WasmModule> {
  return initWasm()
}

/** WASM 是否已經加載完。 */
export function isWasmLoaded(): boolean {
  return wasmModule !== null
}

/**
 * 把 `Map` 形態的碼表轉成傳輸形態，**保序**。
 *
 * `Map` 的遍歷順序是插入順序，JSON 對象的鍵順序在規範上不保證，所以這裏一律出
 * 數組——`asListed` 的動態選重率就靠它。
 */
export function serializeCodeTable(codeTable: Map<string, string | string[]>): string {
  return JSON.stringify(Array.from(codeTable.entries()))
}

export type { WasmModule }
