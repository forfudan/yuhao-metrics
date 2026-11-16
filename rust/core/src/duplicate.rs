//! 重碼 (duplicate codes) —— 靜態重碼與動態選重率。
//!
//! # 收攏了哪幾份實現
//!
//! | 指標 | 原先在哪 |
//! |---|---|
//! | 靜態重碼 | `quant.py:get_static_dup_rate` · 網站的 `duplicateAnalysisService.ts` |
//! | 動態選重 | `quant.py:get_dynamic_dup_rate` · 網站的 `duplicateAnalysisService.ts` |
//!
//! ⚠️ **詞語動態重碼還沒收**：舊實現裏有（`quant.py:get_dynamic_word_dup_rate`），
//! 本模塊**沒有**對應函數，別來這裏找。
//!
//! 這幾份不是同一個函數的幾個抄本，它們算出來的數不一樣。處理的辦法是：**差異
//! 全部變成參數或者變成返回結構的一個字段，一個都不丟**，不預設哪一份「對」。
//!
//! - `quant.py` 的動態重碼返回的是**未歸一化的頻率質量**（分子），
//!   TS 那份返回的是**比率**（分子 ÷ 全表字頻和）。所以
//!   [`dynamic_selection`] 兩個都返回 —— 見 [`DynamicSelection`]。
//! - TS 那份有 `是否按字頻排序`：`false` 時照碼表列出的候選次序算首選。
//!   這是 [`CandidateOrder`]。
//! - `quant.py` 的靜態重碼有 `charset` 開關，TS 那份也有；統一成
//!   [`static_duplicates`] 的 `charset: Option<&HashSet<String>>`。

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::code_table::{freq_of, CodeTable, Frequency};

/// 重碼組裏誰算「首選」（不必按鍵選重的那一個）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CandidateOrder {
    /// 按字頻降序重排，最高頻的那個是首選。
    ///
    /// 這是 `quant.py` 和舊 Rust 版唯一支持的算法，也是評估「這個方案理論上能有
    /// 多好」時該用的：它假設碼表按字頻排過序。
    #[default]
    ByFrequency,
    /// 照碼表列出的次序，先出現的是首選。
    ///
    /// 評估「這個方案**現在**發出去是什麼樣」時該用的：真實的 Rime 碼表候選次序
    /// 未必等於字頻次序。TS 那份的 `是否按字頻排序 = false`。
    AsListed,
}

/// 動態選重的結果。
///
/// 分子和分母都給出來，是因爲兩份舊實現各只返回其中一個，而它們的差別無法從數值
/// 上看出來——`quant.py` 傳歸一化字頻時 `mass` 恰好等於 `rate`，傳絕對頻數時就
/// 相差一個總頻數的倍數。合成一個函數而只留一個返回值，等於在這裏挑一邊。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DynamicSelection {
    /// 分子：需要按鍵選重的那些字符的字頻之和。`quant.py` 返回的就是這個。
    pub duplicate_mass: f64,
    /// 分母：碼表內全部字符的字頻之和。
    pub total_mass: f64,
    /// `duplicate_mass / total_mass`，總和爲 0 時記 0。TS 那份返回的是這個。
    pub rate: f64,
}

impl DynamicSelection {
    fn new(duplicate_mass: f64, total_mass: f64) -> Self {
        Self {
            duplicate_mass,
            total_mass,
            rate: if total_mass > 0.0 {
                duplicate_mass / total_mass
            } else {
                0.0
            },
        }
    }
}

/// 動態選重率：打一個字時，有多大比例的機會要多按一次選重鍵。
///
/// 定義：每個重碼組裏，除首選以外的字符的字頻之和 ÷ 全表字頻之和。首選由
/// `order` 決定。
pub fn dynamic_selection(
    table: &CodeTable,
    frequency: &Frequency,
    order: CandidateOrder,
) -> DynamicSelection {
    // 按首選編碼分組，組內保持碼表順序——`AsListed` 依賴它。
    let mut groups: Vec<Vec<f64>> = Vec::new();
    let mut position: HashMap<&str, usize> = HashMap::new();
    let mut total_mass = 0.0;

    for entry in table {
        let Some(code) = entry.codes.first() else {
            continue;
        };
        let freq = freq_of(frequency, &entry.character);
        total_mass += freq;

        match position.get(code.as_str()) {
            Some(&i) => groups[i].push(freq),
            None => {
                position.insert(code.as_str(), groups.len());
                groups.push(vec![freq]);
            }
        }
    }

    let mut duplicate_mass = 0.0;
    for freqs in groups {
        if freqs.len() < 2 {
            continue;
        }
        let first = match order {
            // f64 沒有全序（NaN），所以不能用 sort/max 的 Ord 版本。字頻不該是
            // NaN，但一個壞掉的輸入不應該讓整個計算 panic。
            CandidateOrder::ByFrequency => freqs.iter().copied().fold(f64::MIN, f64::max),
            CandidateOrder::AsListed => freqs[0],
        };
        let sum: f64 = freqs.iter().sum();
        duplicate_mass += sum - first;
    }

    DynamicSelection::new(duplicate_mass, total_mass)
}

/// 從**已帶選重鍵**的碼表算動態選重率。
///
/// 適用於 `fullWithSelection` / `shortWithSelection` 這類碼表：第 n 選的編碼末尾
/// 已經補了數字 n，所以不必再分組，末位是數字就是要選重。
///
/// 對應 `duplicateAnalysisService.ts:計算原始碼表的動態選重率`。它和
/// [`dynamic_selection`] 不是同一個算法的兩種寫法：這一個信任碼表裏已有的候選
/// 次序，[`CandidateOrder::AsListed`] 則是自己重新分組得出同一結論。兩者在碼表
/// 自洽時應當相等，不等就說明選重鍵標錯了——那是有用的信息，不是重複。
pub fn dynamic_selection_from_selection_keys(
    table: &CodeTable,
    frequency: &Frequency,
) -> DynamicSelection {
    let mut duplicate_mass = 0.0;
    let mut total_mass = 0.0;

    for entry in table {
        let Some(code) = entry.codes.first() else {
            continue;
        };
        let freq = freq_of(frequency, &entry.character);
        total_mass += freq;
        if code.chars().next_back().is_some_and(|c| c.is_ascii_digit()) {
            duplicate_mass += freq;
        }
    }

    DynamicSelection::new(duplicate_mass, total_mass)
}

/// 靜態重碼的統計結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticDuplicates {
    /// 參與統計的字符數（落在字符集內、且有編碼的）。
    pub total_chars: usize,
    /// 重碼字數：所有處在重碼組裏的字符的個數。
    ///
    /// 注意這數的是**字符**不是**組**：「的」「得」同碼算 2 不算 1。三份舊實現
    /// 在這一點上是一致的（RECONCILIATION #7），組數另見 `duplicate_groups`。
    pub duplicate_count: usize,
    /// 重碼組數：有多少個編碼上掛了不止一個字。
    pub duplicate_groups: usize,
    /// `duplicate_count / total_chars`。
    pub duplicate_rate: f64,
    /// 不同編碼的個數。
    pub unique_codes: usize,
    /// 全部重碼字，按碼表順序。
    pub duplicate_chars: Vec<String>,
}

/// 靜態重碼：不看字頻，只看有多少個字符跟別人撞了編碼。
///
/// `charset` 爲 `None` 時統計全表；給定時只統計落在該字符集裏的字符，**並且重碼
/// 判定也只在該字符集內進行**——集外的字跟集內的字同碼不算重碼。這一點是三份舊
/// 實現一致的，也是唯一講得通的：GB2312 的重碼率不該被生僻字影響。
pub fn static_duplicates(table: &CodeTable, charset: Option<&HashSet<String>>) -> StaticDuplicates {
    let in_charset = |c: &str| charset.is_none_or(|set| set.contains(c));

    let mut groups: Vec<Vec<&str>> = Vec::new();
    let mut position: HashMap<&str, usize> = HashMap::new();

    for entry in table {
        if !in_charset(&entry.character) {
            continue;
        }
        let Some(code) = entry.codes.first() else {
            continue;
        };
        match position.get(code.as_str()) {
            Some(&i) => groups[i].push(&entry.character),
            None => {
                position.insert(code.as_str(), groups.len());
                groups.push(vec![&entry.character]);
            }
        }
    }

    let total_chars: usize = groups.iter().map(Vec::len).sum();
    let unique_codes = groups.len();
    let mut duplicate_count = 0;
    let mut duplicate_groups = 0;
    let mut duplicate_chars = Vec::new();

    for chars in &groups {
        if chars.len() > 1 {
            duplicate_groups += 1;
            duplicate_count += chars.len();
            duplicate_chars.extend(chars.iter().map(|c| (*c).to_string()));
        }
    }

    StaticDuplicates {
        total_chars,
        duplicate_count,
        duplicate_groups,
        duplicate_rate: if total_chars > 0 {
            duplicate_count as f64 / total_chars as f64
        } else {
            0.0
        },
        unique_codes,
        duplicate_chars,
    }
}

/// 非首選重碼字的明細——UI 要拿它列表，優化時要拿它找該調哪個字。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NonFirstDuplicate {
    pub character: String,
    pub code: String,
    pub frequency: f64,
    /// 該編碼上的全部字符，按 `order` 決定的候選次序。
    pub candidates: Vec<String>,
    /// 該字符排在第幾選（首選是 1）。
    pub rank: usize,
}

/// 列出所有要按鍵選重的字符，按字頻降序。
pub fn non_first_duplicates(
    table: &CodeTable,
    frequency: &Frequency,
    order: CandidateOrder,
) -> Vec<NonFirstDuplicate> {
    let mut groups: Vec<Vec<(&str, f64)>> = Vec::new();
    let mut position: HashMap<&str, usize> = HashMap::new();

    for entry in table {
        let Some(code) = entry.codes.first() else {
            continue;
        };
        let item = (
            entry.character.as_str(),
            freq_of(frequency, &entry.character),
        );
        match position.get(code.as_str()) {
            Some(&i) => groups[i].push(item),
            None => {
                position.insert(code.as_str(), groups.len());
                groups.push(vec![item]);
            }
        }
    }

    let code_of: HashMap<usize, &str> = position.into_iter().map(|(c, i)| (i, c)).collect();

    let mut out = Vec::new();
    for (i, mut candidates) in groups.into_iter().enumerate() {
        if candidates.len() < 2 {
            continue;
        }
        if order == CandidateOrder::ByFrequency {
            // 同頻時保持碼表順序：sort_by 是穩定排序，這裏依賴這一點。
            candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
        }
        let names: Vec<String> = candidates.iter().map(|(c, _)| (*c).to_string()).collect();
        for (rank, (character, freq)) in candidates.into_iter().enumerate().skip(1) {
            out.push(NonFirstDuplicate {
                character: character.to_string(),
                code: code_of[&i].to_string(),
                frequency: freq,
                candidates: names.clone(),
                rank: rank + 1,
            });
        }
    }

    out.sort_by(|a, b| b.frequency.total_cmp(&a.frequency));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(pairs: &[(&str, &str)]) -> CodeTable {
        CodeTable::from_pairs(pairs.iter().copied())
    }

    fn freqs(pairs: &[(&str, f64)]) -> Frequency {
        pairs.iter().map(|(c, f)| ((*c).to_string(), *f)).collect()
    }

    #[test]
    fn only_the_non_first_candidates_count() {
        // 的/得 同碼，得 較低頻，所以要選重：0.02 / 0.10
        let t = table(&[("的", "de"), ("得", "de"), ("了", "le")]);
        let f = freqs(&[("的", 0.05), ("得", 0.02), ("了", 0.03)]);

        let r = dynamic_selection(&t, &f, CandidateOrder::ByFrequency);
        assert!((r.rate - 0.2).abs() < 1e-12, "got {}", r.rate);
        assert!((r.duplicate_mass - 0.02).abs() < 1e-12);
        assert!((r.total_mass - 0.10).abs() < 1e-12);
    }

    #[test]
    fn absolute_counts_and_shares_give_the_same_rate_but_not_the_same_mass() {
        // 這一條就是 quant.py 和 TS 兩份實現差在哪裏：mass 差 10000 倍，rate 相同。
        let t = table(&[("的", "de"), ("得", "de"), ("了", "le")]);
        let counts = freqs(&[("的", 500.0), ("得", 200.0), ("了", 300.0)]);
        let shares = freqs(&[("的", 0.05), ("得", 0.02), ("了", 0.03)]);

        let a = dynamic_selection(&t, &counts, CandidateOrder::ByFrequency);
        let b = dynamic_selection(&t, &shares, CandidateOrder::ByFrequency);
        assert!((a.rate - b.rate).abs() < 1e-12);
        assert!((a.duplicate_mass - 200.0).abs() < 1e-12);
        assert!((b.duplicate_mass - 0.02).abs() < 1e-12);
    }

    #[test]
    fn as_listed_takes_the_first_row_not_the_most_frequent() {
        // 碼表把低頻的 得 排在前面：按碼表次序算，要選重的是高頻的 的。
        let t = table(&[("得", "de"), ("的", "de")]);
        let f = freqs(&[("的", 0.05), ("得", 0.02)]);

        let listed = dynamic_selection(&t, &f, CandidateOrder::AsListed);
        let by_freq = dynamic_selection(&t, &f, CandidateOrder::ByFrequency);
        assert!((listed.duplicate_mass - 0.05).abs() < 1e-12);
        assert!((by_freq.duplicate_mass - 0.02).abs() < 1e-12);
    }

    #[test]
    fn no_duplicates_means_zero_and_empty_input_is_not_nan() {
        let t = table(&[("的", "de"), ("了", "le")]);
        let f = freqs(&[("的", 0.05), ("了", 0.03)]);
        assert_eq!(
            dynamic_selection(&t, &f, CandidateOrder::default()).rate,
            0.0
        );

        let empty = dynamic_selection(
            &CodeTable::new(),
            &Frequency::new(),
            CandidateOrder::default(),
        );
        assert_eq!(empty.rate, 0.0);
        assert_eq!(empty.total_mass, 0.0);
    }

    #[test]
    fn selection_keys_agree_with_regrouping_when_the_table_is_consistent() {
        let t = table(&[("的", "de"), ("得", "de2")]);
        let f = freqs(&[("的", 0.05), ("得", 0.02)]);
        assert!(
            (dynamic_selection_from_selection_keys(&t, &f).duplicate_mass - 0.02).abs() < 1e-12
        );
    }

    #[test]
    fn counts_characters_not_groups() {
        let t = table(&[("的", "de"), ("得", "de"), ("底", "de"), ("了", "le")]);
        let s = static_duplicates(&t, None);
        assert_eq!(s.total_chars, 4);
        assert_eq!(s.duplicate_count, 3);
        assert_eq!(s.duplicate_groups, 1);
        assert_eq!(s.unique_codes, 2);
        assert_eq!(s.duplicate_chars, vec!["的", "得", "底"]);
    }

    #[test]
    fn characters_outside_the_charset_do_not_create_duplicates() {
        let t = table(&[("的", "de"), ("得", "de")]);
        let charset: HashSet<String> = ["的".to_string()].into_iter().collect();
        let s = static_duplicates(&t, Some(&charset));
        assert_eq!(s.total_chars, 1);
        assert_eq!(s.duplicate_count, 0);
        assert_eq!(s.duplicate_rate, 0.0);
    }

    #[test]
    fn non_first_lists_every_candidate_after_the_first() {
        let t = table(&[("的", "de"), ("得", "de"), ("底", "de")]);
        let f = freqs(&[("的", 0.05), ("得", 0.02), ("底", 0.01)]);
        let d = non_first_duplicates(&t, &f, CandidateOrder::ByFrequency);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].character, "得");
        assert_eq!(d[0].rank, 2);
        assert_eq!(d[1].character, "底");
        assert_eq!(d[1].rank, 3);
        assert_eq!(d[0].candidates, vec!["的", "得", "底"]);
    }
}
