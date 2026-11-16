import type { CharFrequencies, CharFrequency } from '../types/index.js'

/**
 * 字頻數據的默認基址。數據本身不在本包裏，由這個站點提供。
 *
 * ⚠️ 下面那兩支加載函數**會聯網**。要換數據源就傳 `baseUrl`；完全不想聯網就自己
 * 準備 `Record<字, 數>` 餵給各指標——本庫的所有函數都只吃現成的碼表與字頻。
 */
export const ASSESS_DATA_BASE = 'https://zhuyuhao.com/yuhao-assess-data/data'

/**
 * 加載四份字頻（知乎語料、簡體、繁體、古籍），返回 `{ zhihu, sc, tc, guji }`。
 *
 * 檔裏是**頻數**（絕對計數）不是頻率；各指標按總和歸一，所以兩者等價。
 */
export async function loadCharFrequencies(
  baseUrl: string = ASSESS_DATA_BASE
): Promise<CharFrequencies> {
  const files = [
    'charAbsoluteFrequencyZhihu.json',
    'charAbsoluteFrequencySC.json',
    'charAbsoluteFrequencyTC.json',
    'charAbsoluteFrequencyGuji.json'
  ]

  const results = await Promise.all(
    files.map(async (file) => {
      const url = `${baseUrl}/${file}`
      try {
        const res = await fetch(url)
        if (!res.ok) {
          throw new Error(`HTTP ${res.status}: ${res.statusText}`)
        }
        return res
      } catch (err) {
        throw new Error(`Failed to fetch ${file}: ${err instanceof Error ? err.message : String(err)}`)
      }
    })
  )

  const [zhihuRes, scRes, tcRes, gujiRes] = results

  const [zhihu, sc, tc, guji] = await Promise.all([
    zhihuRes.json() as Promise<CharFrequency>,
    scRes.json() as Promise<CharFrequency>,
    tcRes.json() as Promise<CharFrequency>,
    gujiRes.json() as Promise<CharFrequency>
  ])

  return { zhihu, sc, tc, guji }
}

/** 加載字集定義（哪些字算在 GB2312／通規／國字…裏）。同樣會聯網。 */
export async function loadCharsetData(
  url: string = `${ASSESS_DATA_BASE}/charsets.json`
): Promise<Record<string, any>> {
  const response = await fetch(url)
  if (!response.ok) {
    throw new Error(`Failed to load charset data: ${response.status} ${url}`)
  }
  return response.json() as Promise<Record<string, any>>
}

/**
 * 把 CSV 解析成記錄數組。**只按逗號切，不處理引號與轉義**——夠這個庫自己讀
 * 那幾份字集表用，不是一個通用 CSV 解析器。
 */
export function parseCSV(text: string, hasHeader: boolean = true): string[][] {
  const lines = text.split('\n').filter(line => line.trim())
  const startIndex = hasHeader ? 1 : 0

  return lines.slice(startIndex).map(line => line.split(','))
}
