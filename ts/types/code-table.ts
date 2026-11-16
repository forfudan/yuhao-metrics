/**
 * 碼表：字符 → 編碼。
 *
 * 用 `Map` 不用普通對象，是因爲**順序有意義**：動態選重率的 `asListed` 算法把碼
 * 表列出的第一個候選當首選，而 JSON 對象的鍵順序在規範上不保證。
 *
 * 一個字符有多個編碼時值傳數組，首選是 `[0]`。
 */
export type CodeTable = Map<string, string | string[]>

/** 字頻。絕對頻數或歸一化頻率都可以。 */
export type CharFrequency = Record<string, number>

/** 碼對 → 當量值。鍵是兩個字符的字符串，如 `"df"`。 */
export type EquivTable = Record<string, number>

/**
 * 第 n 選 → 實際按鍵。
 *
 * 碼表生成時第 n 選的編碼末尾補的是數字 n，但多數方案用 `;` 選二重、`'` 選三重，
 * 這兩個鍵在基準行、數字鍵在上排，擊鍵成本差很多。
 */
export type SelectionKeys = Record<string, string>
