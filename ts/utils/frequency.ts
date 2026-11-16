/**
 * 歸一化字符頻率使其總和為 1
 */
export function normalizeFrequency(freq: Record<string, number>): Record<string, number> {
  const total = Object.values(freq).reduce((sum, val) => sum + val, 0)
  if (total === 0) return {}

  const normalized: Record<string, number> = {}
  for (const [char, value] of Object.entries(freq)) {
    normalized[char] = value / total
  }
  return normalized
}

/**
 * 合併多個頻率表
 */
export function mergeFrequencies(
  ...freqs: Record<string, number>[]
): Record<string, number> {
  const merged: Record<string, number> = {}

  for (const freq of freqs) {
    for (const [char, value] of Object.entries(freq)) {
      merged[char] = (merged[char] || 0) + value
    }
  }

  return merged
}

