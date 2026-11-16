/**
 * 來自不同語料庫的字頻。
 *
 * 此前這裏還有 `Zigen` 和 `Chaifen` 兩個接口。它們是宇浩方案的概念，而
 * `yuhao-metrics` 是與方案無關的一層——它要能給五筆、倉頡、虎碼算分，就不該知道
 * 什麼是字根（docs/RESTRUCTURE.md §2）。兩個接口都沒有使用者，已刪；真需要時它們
 * 屬於 `yuhao-core`。
 */

import type { CharFrequency } from './code-table.js'

export interface CharFrequencies {
  /** 知乎簡體 */
  zhihu: CharFrequency
  /** 北語簡體 */
  sc: CharFrequency
  /** 台標繁體 */
  tc: CharFrequency
  /** 古籍繁體 */
  guji: CharFrequency
}
