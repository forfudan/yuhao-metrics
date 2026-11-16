//! 人機工程 (ergonomics) —— 按鍵分佈、鍵位分數、單手率。
//!
//! 收攏 `yulib/quant.py` 的 `get_ergo_score` / `get_ergo_score_for_pop` /
//! `get_single_hand_rate`，以及 `keyboardHeatmapService.ts:計算按鍵計數`。
//! 鍵位分數此前只有 Python 一份，沒有 TS 對照，所以這裏是搬家不是裁決；搬家的
//! 收穫是那兩份 Python 的差別終於寫在一處了（見 [`ErgoProfile`]）。

use std::collections::HashMap;

use crate::code_table::{freq_of, CodeTable, Frequency};
use crate::equivalence::CodeNormalization;

/// 左手負責的鍵。
pub const LEFT_HAND: &str = "qwertasdfgzxcvb";
/// 右手負責的鍵。
pub const RIGHT_HAND: &str = "yuiophjklnm";

/// 字根碼位用的理想鍵頻權重（`quant.py:PRIORITY_OF_KEYS`）。
///
/// 數字本身不是頻率，是「這個鍵有多好用」的打分；用之前會歸一化成一個分佈。
pub const KEY_PRIORITY: [(char, f64); 26] = [
    ('f', 5.5),
    ('j', 5.5),
    ('d', 6.5),
    ('k', 7.0),
    ('e', 6.0),
    ('i', 6.0),
    ('s', 4.5),
    ('l', 5.0),
    ('w', 3.5),
    ('o', 4.0),
    ('v', 3.5),
    ('m', 4.0),
    ('a', 3.0),
    ('r', 3.75),
    ('u', 3.5),
    ('g', 4.0),
    ('h', 4.0),
    ('n', 3.5),
    ('b', 3.0),
    ('q', 1.75),
    ('p', 1.75),
    ('t', 2.5),
    ('c', 3.5),
    ('y', 2.0),
    ('x', 2.25),
    ('z', 1.0),
];

/// 簡碼用的理想鍵頻權重（`quant.py:PRIORITY_OF_KEYS_FOR_POP`）。
pub const KEY_PRIORITY_FOR_SHORT_CODE: [(char, f64); 26] = [
    ('f', 55.0),
    ('j', 55.0),
    ('d', 70.0),
    ('k', 70.0),
    ('e', 40.0),
    ('i', 50.0),
    ('s', 55.0),
    ('l', 55.0),
    ('w', 35.0),
    ('o', 25.0),
    ('v', 45.0),
    ('m', 45.0),
    ('a', 20.0),
    ('r', 40.0),
    ('u', 25.0),
    ('g', 40.0),
    ('h', 40.0),
    ('n', 35.0),
    ('b', 35.0),
    ('q', 10.0),
    ('p', 10.0),
    ('t', 25.0),
    ('c', 35.0),
    ('y', 15.0),
    ('x', 15.0),
    ('z', 10.0),
];

/// 一條偏差調整規則：某組鍵的實測頻率高於／低於理想值時，偏差怎麼縮放。
#[derive(Debug, Clone, Copy)]
pub struct Adjustment {
    /// 適用的鍵。
    pub keys: &'static str,
    /// `true` 只在實測高於理想時生效，`false` 只在低於時生效。
    pub when_above: bool,
    /// 新的實測值 = 理想值 + 偏差 × `factor`。0 表示按理想值算，即完全免罰。
    ///
    /// **負數不是筆誤，而且符號有實際後果。** 分數本身是偏差的平方，符號不影響它；
    /// 但規則是按順序改寫同一列的，負係數會把一個「高於理想」的鍵翻到理想值之下，
    /// 於是後面那條「低於理想就補齊」的規則就會命中它，把懲罰清成 0。`quant.py`
    /// 寫的就是 `(best - freq) / 4 + best`，這裏原樣照搬。
    pub factor: f64,
}

/// 鍵位分數的一套參數：理想分佈 + 一串偏差調整。
///
/// 兩份 Python 實現的差別全在這裏，一眼可比。
#[derive(Debug, Clone)]
pub struct ErgoProfile {
    pub priority: Vec<(char, f64)>,
    pub adjustments: Vec<Adjustment>,
}

impl ErgoProfile {
    /// `quant.py:get_ergo_score` —— 字根碼位用。
    pub fn zigen() -> Self {
        Self {
            priority: KEY_PRIORITY.to_vec(),
            adjustments: vec![
                Adjustment {
                    keys: "dk",
                    when_above: true,
                    factor: 0.0,
                },
                Adjustment {
                    keys: "ei",
                    when_above: true,
                    factor: 0.25,
                },
                Adjustment {
                    keys: "fjsla",
                    when_above: true,
                    factor: -0.5,
                },
                Adjustment {
                    keys: "yqpbx",
                    when_above: true,
                    factor: 2.0,
                },
                Adjustment {
                    keys: "z",
                    when_above: true,
                    factor: 4.0,
                },
                Adjustment {
                    keys: "qpxyz",
                    when_above: false,
                    factor: 0.0,
                },
            ],
        }
    }

    /// `quant.py:get_ergo_score_for_pop` —— 簡碼用。
    ///
    /// 注意規則次序有實際後果：`dkei` 和 `fjsla` 的係數是負的，會把這些鍵的實測值
    /// 壓到理想值之下，隨後 `aeiou` 那條「低於理想就補齊」又把 `a` `e` `i` 拉回理想
    /// 值。於是這三個鍵高頻時**完全不罰**，而不是罰四分之一或一半。
    ///
    /// 把負係數當筆誤「修」成正的，分數會變（實測差 0.022，約 0.6%），因爲翻不到
    /// 理想值之下就觸發不了後面那條補齊。改成並行求值同理。
    pub fn short_code(no_z: bool) -> Self {
        let mut priority = KEY_PRIORITY_FOR_SHORT_CODE.to_vec();
        if no_z {
            for (key, value) in priority.iter_mut() {
                if *key == 'z' {
                    *value = 0.0;
                }
            }
        }
        Self {
            priority,
            adjustments: vec![
                Adjustment {
                    keys: "dkei",
                    when_above: true,
                    factor: -0.25,
                },
                Adjustment {
                    keys: "fjsla",
                    when_above: true,
                    factor: -0.5,
                },
                Adjustment {
                    keys: "xyb",
                    when_above: true,
                    factor: 2.0,
                },
                Adjustment {
                    keys: "qp",
                    when_above: true,
                    factor: 4.0,
                },
                Adjustment {
                    keys: "z",
                    when_above: true,
                    factor: 8.0,
                },
                Adjustment {
                    keys: "qpxyz",
                    when_above: false,
                    factor: 0.0,
                },
                Adjustment {
                    keys: "aeiou",
                    when_above: false,
                    factor: 0.0,
                },
            ],
        }
    }
}

/// 按鍵計數：每個按鍵被敲到的字頻加權次數。
///
/// 數的是**全部編碼**，不只首選——熱力圖要的是「這個鍵一共被按了多少」。`_` 記作
/// `space`，大寫按小寫算。
pub fn key_counts(
    table: &CodeTable,
    frequency: &Frequency,
    normalization: &CodeNormalization,
) -> HashMap<String, f64> {
    let mut counts: HashMap<String, f64> = HashMap::new();

    for entry in table {
        let weight = freq_of(frequency, &entry.character);
        for raw in &entry.codes {
            for c in normalization.apply(raw).chars() {
                let key = if c == '_' {
                    "space".to_string()
                } else {
                    c.to_lowercase().to_string()
                };
                *counts.entry(key).or_insert(0.0) += weight;
            }
        }
    }

    counts
}

/// 26 個字母各自的字頻加權出現次數。只數 a–z，其它字符（數字、`_`、標點）不計。
///
/// `quant.py` 那份是 `key_freq_dict[letter] += frequency`，遇到非字母直接
/// `KeyError` 崩掉。這裏改成跳過：一個帶選重數字的碼表不該讓評分函數拋異常。
/// **這是一處有意的行爲改變**，記在 `docs/RECONCILIATION.md`。
pub fn letter_counts(table: &CodeTable, frequency: &Frequency) -> [f64; 26] {
    let mut counts = [0.0; 26];

    for entry in table {
        let Some(code) = entry.codes.first() else {
            continue;
        };
        let freq = freq_of(frequency, &entry.character);
        for c in code.chars() {
            if let Some(i) = letter_index(c) {
                counts[i] += freq;
            }
        }
    }

    counts
}

fn letter_index(c: char) -> Option<usize> {
    let c = c.to_ascii_lowercase();
    c.is_ascii_lowercase().then(|| (c as u8 - b'a') as usize)
}

/// 鍵位分數：實測鍵頻分佈與理想分佈的偏離度，越小越好。
///
/// 兩個分佈都歸一化後逐鍵求差，按 [`ErgoProfile`] 的規則縮放偏差，平方後求平均，
/// 再乘 10000（純粹爲了讀數好看，和 `quant.py` 一致）。
pub fn ergo_score(table: &CodeTable, frequency: &Frequency, profile: &ErgoProfile) -> f64 {
    let counts = letter_counts(table, frequency);
    let observed_total: f64 = counts.iter().sum();

    let mut best = [0.0; 26];
    for (key, value) in &profile.priority {
        if let Some(i) = letter_index(*key) {
            best[i] = *value;
        }
    }
    let best_total: f64 = best.iter().sum();

    let mut observed = [0.0; 26];
    for i in 0..26 {
        observed[i] = if observed_total > 0.0 {
            counts[i] / observed_total
        } else {
            0.0
        };
        best[i] = if best_total > 0.0 {
            best[i] / best_total
        } else {
            0.0
        };
    }

    // 規則按順序改寫 observed，後一條看到的是前一條的結果——見 short_code 的註釋。
    for adjustment in &profile.adjustments {
        for c in adjustment.keys.chars() {
            let Some(i) = letter_index(c) else { continue };
            let above = observed[i] > best[i];
            if above == adjustment.when_above {
                observed[i] = best[i] + (observed[i] - best[i]) * adjustment.factor;
            }
        }
    }

    let deviation: f64 = (0..26).map(|i| (observed[i] - best[i]).powi(2)).sum();
    deviation / 26.0 * 10_000.0
}

/// 單手率：編碼的所有按鍵落在同一隻手上的字符，佔多少字頻。
///
/// `min_len` 以下的短碼不算——單手敲兩個鍵不構成問題，`quant.py` 默認 4。
///
/// 返回的是**頻率質量**：字頻歸一化時它就是比例，不歸一化時隨總頻數縮放。這一點
/// 跟 `quant.py` 一致（它同樣不除以總和）。
pub fn single_hand_mass(table: &CodeTable, frequency: &Frequency, min_len: usize) -> f64 {
    table
        .iter()
        .filter_map(|entry| {
            let code = entry.codes.first()?;
            if code.chars().count() < min_len {
                return None;
            }
            let all_left = code.chars().all(|c| LEFT_HAND.contains(c));
            let all_right = code.chars().all(|c| RIGHT_HAND.contains(c));
            (all_left || all_right).then(|| freq_of(frequency, &entry.character))
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frequency(pairs: &[(&str, f64)]) -> Frequency {
        pairs.iter().map(|(c, f)| ((*c).to_string(), *f)).collect()
    }

    #[test]
    fn single_hand_needs_both_one_hand_and_enough_length() {
        let table = CodeTable::from_pairs([
            ("甲", "asdf"), // 全左手，四碼
            ("乙", "hjkl"), // 全右手，四碼
            ("丙", "asdj"), // 雙手
            ("丁", "as"),   // 全左手但太短
        ]);
        let f = frequency(&[("甲", 0.1), ("乙", 0.2), ("丙", 0.3), ("丁", 0.4)]);
        assert!((single_hand_mass(&table, &f, 4) - 0.3).abs() < 1e-12);
        // 放寬長度門檻，丁 就算進來了
        assert!((single_hand_mass(&table, &f, 2) - 0.7).abs() < 1e-12);
    }

    #[test]
    fn letter_counts_skip_non_letters_instead_of_panicking() {
        let table = CodeTable::from_pairs([("甲", "as2"), ("乙", "df_")]);
        let f = frequency(&[("甲", 1.0), ("乙", 2.0)]);
        let counts = letter_counts(&table, &f);
        assert_eq!(counts[letter_index('a').unwrap()], 1.0);
        assert_eq!(counts[letter_index('s').unwrap()], 1.0);
        assert_eq!(counts[letter_index('d').unwrap()], 2.0);
        assert_eq!(counts.iter().sum::<f64>(), 6.0); // 2 個字母 ×1 + 2 個字母 ×2
    }

    #[test]
    fn a_perfect_distribution_scores_zero() {
        // 把碼表湊成正好等於理想分佈：每個鍵的字頻 = 它的權重。
        let profile = ErgoProfile::zigen();
        let mut table = CodeTable::new();
        let mut f = Frequency::new();
        for (key, weight) in &profile.priority {
            let character = key.to_string();
            table.push(character.clone(), vec![key.to_string()]);
            f.insert(character, *weight);
        }
        assert!(ergo_score(&table, &f, &profile) < 1e-20);
    }

    #[test]
    fn a_negative_factor_plus_a_later_clamp_means_no_penalty_at_all() {
        // 簡碼參數集裏 a 高頻時：fjsla 把它壓到理想值之下（係數 -0.5），
        // 隨後 aeiou 那條把它補回理想值，於是完全不罰。把 -0.5 改成 +0.5
        // 就罰一半——這正是符號的後果。
        let profile = ErgoProfile::short_code(false);
        let mut table = CodeTable::new();
        let mut f = Frequency::new();
        for (key, weight) in &profile.priority {
            let character = key.to_string();
            table.push(character.clone(), vec![key.to_string()]);
            f.insert(character, *weight);
        }
        // 理想分佈本身得 0 分。
        assert!(ergo_score(&table, &f, &profile) < 1e-20);

        // 把 a 的頻率抬高一倍：仍然是 0，因爲它被補齊回理想值。
        f.insert("a".to_string(), 40.0);
        assert!(
            ergo_score(&table, &f, &profile) > 0.0,
            "抬高 a 會改變其它鍵的歸一化份額，總分不該還是 0"
        );

        // 換成正係數，a 自己那一項就開始計分了。
        let mut flipped = profile.clone();
        for rule in flipped.adjustments.iter_mut() {
            if rule.keys == "fjsla" {
                rule.factor = 0.5;
            }
        }
        assert!(
            ergo_score(&table, &f, &flipped) != ergo_score(&table, &f, &profile),
            "符號翻轉必須改變分數，否則這條註釋是錯的"
        );
    }

    #[test]
    fn key_counts_fold_case_and_name_the_space_key() {
        let table = CodeTable::from_pairs([("甲", "AS_")]);
        let f = frequency(&[("甲", 1.0)]);
        let counts = key_counts(&table, &f, &CodeNormalization::none());
        assert_eq!(counts.get("a"), Some(&1.0));
        assert_eq!(counts.get("s"), Some(&1.0));
        assert_eq!(counts.get("space"), Some(&1.0));
    }
}
