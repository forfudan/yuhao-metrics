/**
 * 一個 CJK 統一漢字區塊。定義在 Rust 側（`rust/core/src/charset.rs`），
 * 這裏只是它的形狀。
 */
export interface CJKBlock {
  /** 區塊 id，如 `cjk_basic`、`cjk_a`。 */
  id: string
  /** 區塊名稱，如「CJK基本區」。 */
  name: string
  /** Unicode 起始碼位（閉區間）。 */
  start: number
  /**
   * Unicode 結束碼位（閉區間）。
   *
   * 記的是該區塊**實際填充到**的碼位，不是名義上界——判斷一個字是不是漢字時，
   * 名義上界會把未分配的碼位也算進來。
   */
  end: number
}
