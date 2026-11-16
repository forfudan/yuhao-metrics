/** 靜態重碼統計。 */
export interface StaticDuplicates {
  /** 參與統計的字符數（落在字符集內、且有編碼的）。 */
  totalChars: number
  /** 重碼字數：處在重碼組裏的**字符**個數，不是組數。 */
  duplicateCount: number
  /** 重碼組數：有多少個編碼上掛了不止一個字。 */
  duplicateGroups: number
  /** `duplicateCount / totalChars`。 */
  duplicateRate: number
  /** 不同編碼的個數。 */
  uniqueCodes: number
  /** 全部重碼字，按碼表順序。 */
  duplicateChars: string[]
}

/**
 * 動態選重的結果。
 *
 * 分子分母都給出來，是因爲此前 Python 那份返回 `duplicateMass`、TS 那份返回
 * `rate`，而兩者的差別無法從數值上看出來。
 */
export interface DynamicSelection {
  /** 需要按鍵選重的那些字符的字頻之和。 */
  duplicateMass: number
  /** 碼表內全部字符的字頻之和。 */
  totalMass: number
  /** `duplicateMass / totalMass`。 */
  rate: number
}

/** 非首選重碼字的明細。 */
export interface NonFirstDuplicate {
  character: string
  code: string
  frequency: number
  /** 該編碼上的全部字符，按候選次序。 */
  candidates: string[]
  /** 排在第幾選（首選是 1）。 */
  rank: number
}

/** 某一檔當量值上落了哪些碼對、佔多少頻率。 */
export interface EquivBucket {
  equivValue: number
  keyPairs: string[]
  frequencyRatio: number
}

/** 簡碼效率曲線上的一個點。 */
export interface EfficiencyPoint {
  /** 實際用上簡碼的字數。 */
  shortCodeChars: number
  /** 字頻加權平均碼長，越小越好。 */
  weightedCodeLength: number
}

/** 簡碼效率曲線。 */
export interface ShortCodeEfficiency {
  /** 按收益降序排出的簡碼字。各 N 值的字串是它的前綴。 */
  rankedCharacters: string
  points: EfficiencyPoint[]
}
