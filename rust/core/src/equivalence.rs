//! 速度當量 (speed equivalent) —— 敲完一個編碼要花多少「時間單位」。
//!
//! 當量表給出任意兩個相鄰按鍵的代價（1.0 是最快的異手交替，2.1 左右是最慢的同指
//! 跳行）。一個編碼的當量是它相鄰碼對的均值，一個方案的當量是按字頻加權的總平均。
//!
//! # 收攏了哪幾份實現
//!
//! | 原先在哪 | 聚合方式 | 未知碼對 | 補位 |
//! |---|---|---|---|
//! | `quant.py:get_equiv_score` | 逐字 | 計 0，仍佔分母 | 短碼補 `_` 到 `max_len` |
//! | `quant.py:get_equiv_score_for_pop` | 逐字 | 計 0，仍佔分母 | 不補 |
//! | `speedEquivalentService.ts:從碼表計算加權速度當量` | 逐碼對 | 跳過，不佔分母 | 不補，改爲替換選重鍵 |
//! | `yuhao_build/metrics.py:equivalent_score` | 逐字 | 按左右手兜底（[`hand_fallback`]） | 短於四碼補一個 `_` |
//!
//! **這兩種聚合不是同一個數。** 逐字聚合先把一個字的碼對取均值再按字頻加權；逐碼
//! 對聚合把全表的碼對放在一起按出現頻率取均值。碼長一律相同且字頻歸一化時二者相
//! 等，碼長不齊時就不等——而簡碼表的碼長從來不齊。所以兩個都留着，由
//! [`Aggregation`] 選，不預設哪個「對」。詳見 `docs/RECONCILIATION.md`。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::code_table::{freq_of, CodeTable, Frequency};

/// 碼對 → 當量值。鍵是兩個字符的字符串，如 `"df"`。
pub type EquivTable = HashMap<String, f64>;

/// 第 n 選 → 實際按鍵。碼表生成時第 n 選的編碼末尾補的是數字 n，但多數方案用
/// `;` 選二重、`'` 選三重，而這兩個鍵在基準行、數字鍵在上排，擊鍵成本差很多。
pub type SelectionKeys = HashMap<String, String>;

/// 當量表裏查不到的碼對怎麼算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownPair {
    /// 計 0，但仍然佔着分母的一格 —— `quant.py` 的行爲（polars 的
    /// `sum_horizontal` 跳過 null，除數卻是 `len_code - 1`）。
    #[default]
    CountAsZero,
    /// 整個跳過，分子分母都不計 —— TS 那份的行爲。
    Skip,
    /// 按左右手與同鍵兜底，見 [`hand_fallback`] —— 碼表生成器那一派
    /// （`yuhao_build/metrics.py::equivalent_score`，原先五支生成器各寫一份）。
    HandFallback,
}

/// [`UnknownPair::HandFallback`] 的三個兜底值：左手接右手、同鍵、其餘。
pub const HAND_FALLBACK_CROSS: f64 = 1.1;
/// 見 [`HAND_FALLBACK_CROSS`]。
pub const HAND_FALLBACK_SAME_KEY: f64 = 1.3;
/// 見 [`HAND_FALLBACK_CROSS`]。
pub const HAND_FALLBACK_OTHER: f64 = 1.5;

const LEFT_KEYS: &str = "qwertasdfgzxcvb";
const RIGHT_KEYS: &str = "yuiophjklnm";

/// 未知碼對的兜底當量。`None` ＝ 這個碼對一格都不佔（含空格的那一族）。
///
/// ⚠️ 只有**左手接右手**算 1.1，右手接左手不算 —— 這是被收攏的那份實現的行爲，
/// 不是打字學上的對稱。判斷的次序也照它：先空格、再異手、再同鍵、最後其餘。
pub fn hand_fallback(pair: &str) -> Option<f64> {
    let mut chars = pair.chars();
    let (first, second) = (chars.next()?, chars.next()?);
    if first == ' ' || second == ' ' {
        None
    } else if LEFT_KEYS.contains(first) && RIGHT_KEYS.contains(second) {
        Some(HAND_FALLBACK_CROSS)
    } else if first == second {
        Some(HAND_FALLBACK_SAME_KEY)
    } else {
        Some(HAND_FALLBACK_OTHER)
    }
}

/// 聚合方式：先按字平均再加權，還是把全表碼對混在一起加權平均。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Aggregation {
    /// 逐字：`Σ_字 字頻 × mean(該字的碼對當量)`。`quant.py` 的行爲。
    ///
    /// 字頻歸一化時這是個加權平均；不歸一化時它是個質量，數值隨總頻數縮放。
    #[default]
    PerCharacter,
    /// 逐碼對：`Σ_碼對 當量 × 碼對頻率 ÷ Σ_碼對 碼對頻率`。TS 那份的行爲。
    ///
    /// 天然歸一化，字頻是不是歸一化的都一樣。
    PerPair,
}

/// 算當量之前怎麼把編碼整理成「實際要敲的按鍵」。
#[derive(Debug, Clone, Default)]
pub struct CodeNormalization {
    /// 把末尾的選重數字換成方案實際用的鍵。空表則不換。
    pub selection_keys: SelectionKeys,
    /// 短於此長度的編碼末尾補一個 `_`（空格鍵），表示要敲空格上屏。
    /// `None` 則不補。`quant.py:get_equiv_score` 用 `Some(max_len)`。
    pub pad_to: Option<usize>,
}

impl CodeNormalization {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_selection_keys(selection_keys: SelectionKeys) -> Self {
        Self {
            selection_keys,
            pad_to: None,
        }
    }

    pub fn apply(&self, code: &str) -> String {
        let code = replace_selection_key(code, &self.selection_keys);
        match self.pad_to {
            Some(n) if code.chars().count() < n && !code.ends_with('_') => code + "_",
            _ => code,
        }
    }
}

/// 把編碼末尾成串的選重數字換成方案實際使用的按鍵。
///
/// 只在整串數字都在表裏有對應項時才替換，免得把「第 12 選」的 `12` 誤傷成 `1;`。
/// 表裏沒有的位次保持數字鍵原樣。
pub fn replace_selection_key(code: &str, keys: &SelectionKeys) -> String {
    if keys.is_empty() {
        return code.to_string();
    }
    let digits_start = code
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit())
        .map(|(i, _)| i)
        .last();

    let Some(start) = digits_start else {
        return code.to_string();
    };
    match keys.get(&code[start..]) {
        Some(replacement) if !replacement.is_empty() => {
            format!("{}{}", &code[..start], replacement)
        }
        _ => code.to_string(),
    }
}

/// 相鄰碼對的頻率：每個字的字頻，記到它編碼裏每一對相鄰按鍵上。
pub fn pair_frequencies(
    table: &CodeTable,
    frequency: &Frequency,
    normalization: &CodeNormalization,
) -> HashMap<String, f64> {
    let mut pairs: HashMap<String, f64> = HashMap::new();

    for entry in table {
        let Some(raw) = entry.codes.first() else {
            continue;
        };
        let freq = freq_of(frequency, &entry.character);
        if freq == 0.0 {
            continue;
        }
        let code = normalization.apply(raw);
        for pair in adjacent_pairs(&code) {
            *pairs.entry(pair).or_insert(0.0) += freq;
        }
    }

    pairs
}

fn adjacent_pairs(code: &str) -> impl Iterator<Item = String> + '_ {
    let chars: Vec<char> = code.chars().collect();
    (0..chars.len().saturating_sub(1)).map(move |i| chars[i..i + 2].iter().collect())
}

/// 單個編碼的速度當量：相鄰碼對當量的均值。單碼（無相鄰對）記 0。
pub fn code_equivalent(code: &str, equiv: &EquivTable, unknown: UnknownPair) -> f64 {
    let mut total = 0.0;
    let mut count = 0usize;

    for pair in adjacent_pairs(code) {
        match (equiv.get(&pair), unknown) {
            (Some(value), _) => {
                total += value;
                count += 1;
            }
            (None, UnknownPair::CountAsZero) => count += 1,
            (None, UnknownPair::Skip) => {}
            (None, UnknownPair::HandFallback) => {
                if let Some(value) = hand_fallback(&pair) {
                    total += value;
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

/// 字頻加權速度當量。
///
/// `aggregation` 決定聚合方式，`normalization` 決定編碼怎麼整理，`unknown` 決定
/// 當量表查不到的碼對怎麼算——三個開關就是三份舊實現的全部差別。
pub fn weighted_equivalent(
    table: &CodeTable,
    frequency: &Frequency,
    equiv: &EquivTable,
    aggregation: Aggregation,
    normalization: &CodeNormalization,
    unknown: UnknownPair,
) -> f64 {
    match aggregation {
        Aggregation::PerCharacter => table
            .iter()
            .filter_map(|entry| {
                let code = entry.codes.first()?;
                let freq = freq_of(frequency, &entry.character);
                Some(freq * code_equivalent(&normalization.apply(code), equiv, unknown))
            })
            .sum(),
        Aggregation::PerPair => {
            let pairs = pair_frequencies(table, frequency, normalization);
            equivalent_from_pairs(&pairs, equiv, unknown)
        }
    }
}

/// 從已算好的碼對頻率求加權當量（逐碼對聚合）。
pub fn equivalent_from_pairs(
    pairs: &HashMap<String, f64>,
    equiv: &EquivTable,
    unknown: UnknownPair,
) -> f64 {
    let mut weighted = 0.0;
    let mut total = 0.0;

    for (pair, freq) in pairs {
        match (equiv.get(pair), unknown) {
            (Some(value), _) => {
                weighted += value * freq;
                total += freq;
            }
            (None, UnknownPair::CountAsZero) => total += freq,
            (None, UnknownPair::Skip) => {}
            (None, UnknownPair::HandFallback) => {
                if let Some(value) = hand_fallback(pair) {
                    weighted += value * freq;
                    total += freq;
                }
            }
        }
    }

    if total > 0.0 {
        weighted / total
    } else {
        0.0
    }
}

/// 當量分檔：某一檔當量值上落了哪些碼對、佔多少頻率。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EquivBucket {
    pub equiv_value: f64,
    pub key_pairs: Vec<String>,
    pub frequency_ratio: f64,
}

/// 把碼對按當量值分檔（默認 1.0–2.1，每 0.1 一檔），按當量降序返回。
///
/// 超出範圍的當量值夾到兩端——這是 TS 那份的行爲，保留：當量表本來就只在這個區間
/// 裏取值，越界說明表壞了，而夾住比靜靜丟掉更容易在圖上看出來。
///
/// ⚠️ 兩處與 [`weighted_equivalent`] 不同，畫圖之前要知道：
///
/// - **查不到的碼對一律跳過**，沒有 [`UnknownPair`] 這個開關。所以按
///   [`UnknownPair::HandFallback`] 算分時，圖上會缺掉那些兜底的碼對。
/// - `frequency_ratio` 的分母是**全部**碼對的頻率和（含被跳過的），所以各檔之和
///   可以不到 1。
pub fn equivalent_distribution(
    pairs: &HashMap<String, f64>,
    equiv: &EquivTable,
    range: (f64, f64),
    step: f64,
) -> Vec<EquivBucket> {
    // ⚠️ 傳反了也不許 panic：`f64::clamp` 在 `low > high` 時直接炸，而在 wasm 裏
    // 一次 panic 就把整個模塊實例弄廢，後面每一次調用都跟着失敗。公開函數不能有
    // 這種雷，所以這裏先排好序。
    let (low, high) = if range.0 <= range.1 {
        range
    } else {
        (range.1, range.0)
    };
    let mut buckets: HashMap<i64, (Vec<String>, f64)> = HashMap::new();
    let total: f64 = pairs.values().sum();
    let key = |v: f64| (v / step).round() as i64;

    for (pair, freq) in pairs {
        let Some(&value) = equiv.get(pair) else {
            continue;
        };
        let clamped = (value / step).round() * step;
        let clamped = clamped.clamp(low, high);
        let bucket = buckets
            .entry(key(clamped))
            .or_insert_with(|| (Vec::new(), 0.0));
        bucket.0.push(pair.clone());
        bucket.1 += freq;
    }

    let mut out: Vec<EquivBucket> = buckets
        .into_iter()
        .map(|(k, (mut key_pairs, freq))| {
            key_pairs.sort();
            EquivBucket {
                equiv_value: k as f64 * step,
                key_pairs,
                frequency_ratio: if total > 0.0 { freq / total } else { 0.0 },
            }
        })
        .collect();

    out.sort_by(|a, b| b.equiv_value.total_cmp(&a.equiv_value));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equiv() -> EquivTable {
        [("df", 1.0), ("fg", 2.0), ("gh", 1.5)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn keys() -> SelectionKeys {
        [("2", ";"), ("3", "'")]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn selection_digits_are_replaced_only_as_a_whole_run() {
        assert_eq!(replace_selection_key("abc2", &keys()), "abc;");
        // 「第 12 選」表裏沒有，整串保持原樣——不能拆成 1 + ";"。
        assert_eq!(replace_selection_key("abc12", &keys()), "abc12");
        assert_eq!(replace_selection_key("abcd", &keys()), "abcd");
    }

    #[test]
    fn code_equivalent_is_the_mean_over_adjacent_pairs() {
        // dfg → df(1.0) + fg(2.0)，均值 1.5
        assert!((code_equivalent("dfg", &equiv(), UnknownPair::Skip) - 1.5).abs() < 1e-12);
        // 單碼沒有相鄰對
        assert_eq!(code_equivalent("d", &equiv(), UnknownPair::Skip), 0.0);
    }

    #[test]
    fn unknown_pairs_change_the_answer_and_that_is_the_point() {
        // dfz → df(1.0) 已知，fz 未知。
        let t = equiv();
        assert!((code_equivalent("dfz", &t, UnknownPair::Skip) - 1.0).abs() < 1e-12);
        assert!((code_equivalent("dfz", &t, UnknownPair::CountAsZero) - 0.5).abs() < 1e-12);
        // 兜底那一派：fz 兩個都是左手，不是異手、不同鍵，所以 1.5。
        assert!((code_equivalent("dfz", &t, UnknownPair::HandFallback) - 1.25).abs() < 1e-12);
    }

    #[test]
    fn hand_fallback_is_asymmetric_and_skips_spaces() {
        // 左手接右手 1.1，反過來不是——被收攏的那份實現只認一個方向。
        assert_eq!(hand_fallback("dj"), Some(HAND_FALLBACK_CROSS));
        assert_eq!(hand_fallback("jd"), Some(HAND_FALLBACK_OTHER));
        assert_eq!(hand_fallback("dd"), Some(HAND_FALLBACK_SAME_KEY));
        assert_eq!(hand_fallback("jj"), Some(HAND_FALLBACK_SAME_KEY));
        // 空格那一族一格都不佔，所以是 None 而不是某個值。
        assert_eq!(hand_fallback("d "), None);
        assert_eq!(hand_fallback(" d"), None);
        // 補位用的 `_` 不是空格：它兩邊都不屬於，落在「其餘」。
        assert_eq!(hand_fallback("d_"), Some(HAND_FALLBACK_OTHER));
        // 不足兩個字符就沒有碼對。
        assert_eq!(hand_fallback("d"), None);
    }

    #[test]
    fn hand_fallback_skips_space_pairs_in_the_denominator_too() {
        let t = equiv();
        // "df z"：df(1.0) 已知，"f " 與 " z" 都含空格——分母只有 df 與 fz 兩格。
        let score = code_equivalent("df z", &t, UnknownPair::HandFallback);
        assert!((score - 1.0).abs() < 1e-12);
        // 全部碼對都含空格時一格都不剩，回 0.0（而不是除零）。
        assert_eq!(code_equivalent("a b", &t, UnknownPair::HandFallback), 0.0);
    }

    #[test]
    fn a_reversed_range_is_sorted_instead_of_panicking() {
        let pairs: HashMap<String, f64> = [("df", 1.0)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let 正 = equivalent_distribution(&pairs, &equiv(), (1.0, 2.1), 0.1);
        let 反 = equivalent_distribution(&pairs, &equiv(), (2.1, 1.0), 0.1);
        assert_eq!(正, 反);
    }

    #[test]
    fn hand_fallback_also_governs_the_per_pair_aggregation() {
        // 逐碼對那條路（`weighted_equivalent` 的另一半）同樣要認兜底與空格規則。
        let t = equiv();
        let pairs: HashMap<String, f64> = [("df", 1.0), ("f ", 1.0), (" z", 1.0)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        // 兩個含空格的碼對分子分母都不佔，所以結果就是 df 那一格的當量。
        let score = equivalent_from_pairs(&pairs, &t, UnknownPair::HandFallback);
        assert!((score - 1.0).abs() < 1e-12);
        // 對照：`Skip` 在這張表上同值（都只剩 df），而 `CountAsZero` 把兩格算進分母。
        assert!((equivalent_from_pairs(&pairs, &t, UnknownPair::Skip) - 1.0).abs() < 1e-12);
        let zero = equivalent_from_pairs(&pairs, &t, UnknownPair::CountAsZero);
        assert!((zero - 1.0 / 3.0).abs() < 1e-12, "得到 {zero}");
    }

    #[test]
    fn padding_adds_the_space_key_only_when_the_code_is_short() {
        let norm = CodeNormalization {
            selection_keys: SelectionKeys::new(),
            pad_to: Some(4),
        };
        assert_eq!(norm.apply("df"), "df_");
        assert_eq!(norm.apply("dfgh"), "dfgh");
        assert_eq!(norm.apply("df_"), "df_");
    }

    #[test]
    fn the_two_aggregations_agree_when_every_code_is_the_same_length() {
        let table = CodeTable::from_pairs([("甲", "df"), ("乙", "fg")]);
        let frequency: Frequency = [("甲".to_string(), 0.5), ("乙".to_string(), 0.5)]
            .into_iter()
            .collect();
        let t = equiv();
        let norm = CodeNormalization::none();

        let per_char = weighted_equivalent(
            &table,
            &frequency,
            &t,
            Aggregation::PerCharacter,
            &norm,
            UnknownPair::Skip,
        );
        let per_pair = weighted_equivalent(
            &table,
            &frequency,
            &t,
            Aggregation::PerPair,
            &norm,
            UnknownPair::Skip,
        );
        assert!((per_char - 1.5).abs() < 1e-12, "per_char = {per_char}");
        assert!((per_pair - per_char).abs() < 1e-12);
    }

    #[test]
    fn the_two_aggregations_disagree_when_code_lengths_differ() {
        // 甲 是二碼（一個碼對），乙 是三碼（兩個碼對）。逐字聚合給兩個字同樣的
        // 權重，逐碼對聚合給 乙 兩倍的權重。
        let table = CodeTable::from_pairs([("甲", "df"), ("乙", "fgh")]);
        let frequency: Frequency = [("甲".to_string(), 0.5), ("乙".to_string(), 0.5)]
            .into_iter()
            .collect();
        let t = equiv();
        let norm = CodeNormalization::none();

        let per_char = weighted_equivalent(
            &table,
            &frequency,
            &t,
            Aggregation::PerCharacter,
            &norm,
            UnknownPair::Skip,
        );
        let per_pair = weighted_equivalent(
            &table,
            &frequency,
            &t,
            Aggregation::PerPair,
            &norm,
            UnknownPair::Skip,
        );
        // 逐字：0.5×1.0 + 0.5×mean(2.0,1.5) = 0.5 + 0.875 = 1.375
        assert!((per_char - 1.375).abs() < 1e-12, "per_char = {per_char}");
        // 逐碼對：(1.0×0.5 + 2.0×0.5 + 1.5×0.5) / 1.5 = 1.5
        assert!((per_pair - 1.5).abs() < 1e-12, "per_pair = {per_pair}");
    }

    #[test]
    fn distribution_buckets_by_equivalent_value() {
        let pairs: HashMap<String, f64> = [("df", 3.0), ("fg", 1.0)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let d = equivalent_distribution(&pairs, &equiv(), (1.0, 2.1), 0.1);
        assert_eq!(d.len(), 2);
        assert!((d[0].equiv_value - 2.0).abs() < 1e-9);
        assert!((d[0].frequency_ratio - 0.25).abs() < 1e-12);
        assert!((d[1].equiv_value - 1.0).abs() < 1e-9);
        assert!((d[1].frequency_ratio - 0.75).abs() < 1e-12);
    }
}
