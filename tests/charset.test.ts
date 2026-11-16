import { describe, it, expect } from 'vitest'
import { filterByCharset, getCJKBlocks, getCumulativeBlockOrder, isInCharset } from '../ts/charset'

describe('isInCharset', () => {
  it('認 CJK 基本區', async () => {
    expect(await isInCharset('漢', 'cjk_basic')).toBe(true)
    expect(await isInCharset('中', 'cjk_basic')).toBe(true)
    expect(await isInCharset('a', 'cjk_basic')).toBe(false)
    expect(await isInCharset('！', 'cjk_basic')).toBe(false)
  })

  it('擴展區的字不算基本區', async () => {
    expect(await isInCharset('𠀀', 'cjk_basic')).toBe(false)
    expect(await isInCharset('𠀀', 'cjk_b')).toBe(true)
  })

  it('舊寫法 cjk_ext_a 仍然認', async () => {
    expect(await isInCharset('㐀', 'cjk_a')).toBe(true)
    expect(await isInCharset('㐀', 'cjk_ext_a')).toBe(true)
  })

  it('累積字集按區塊次序而不是碼位次序', async () => {
    // U+2EBF0 是擴展 I 區第一個字，碼位落在 F 和 G 之間，但「到 G 區」不含它。
    const extI = '\u{2EBF0}'
    expect(await isInCharset(extI, 'cjk_to_g')).toBe(false)
    expect(await isInCharset(extI, 'cjk_to_i')).toBe(true)
  })

  it('按表定義的字集，沒給表就一律 false', async () => {
    const data = { 的: { is_gb2312: true, is_tonggui: true } }
    expect(await isInCharset('的', 'gb2312', data)).toBe(true)
    expect(await isInCharset('的', 'guozi', data)).toBe(false)
    expect(await isInCharset('的', 'tonggui_guozi', data)).toBe(true)
    expect(await isInCharset('的', 'gb2312')).toBe(false)
  })

  it('不認識的字集名字返回 false，不拋異常', async () => {
    expect(await isInCharset('的', 'big5')).toBe(false)
  })
})

describe('filterByCharset', () => {
  it('保持輸入順序', async () => {
    const filtered = await filterByCharset(['漢', '字', 'a', '1', '中'], 'cjk_basic')
    expect(filtered).toEqual(['漢', '字', '中'])
  })
})

describe('getCJKBlocks', () => {
  it('區塊定義只有 Rust 一份，這裏取得到', async () => {
    const blocks = await getCJKBlocks()
    const basic = blocks.find(b => b.id === 'cjk_basic')
    expect(basic?.start).toBe(0x4e00)
    expect(basic?.end).toBe(0x9fff)
  })

  it('累積次序覆蓋每一個區塊', async () => {
    const blocks = await getCJKBlocks()
    const order = await getCumulativeBlockOrder()
    expect(new Set(order)).toEqual(new Set(blocks.map(b => b.id)))
  })
})
