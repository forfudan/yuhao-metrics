//! 簡碼效率 (short-code efficiency) —— 挑多少個字配簡碼，平均碼長能降到多少。
//!
//! 收攏 `shortCodeEfficiencyService.ts:計算指定字頻下之簡碼效率`。此前只有 TS 一
//! 份，Python 那邊沒有對應實現，所以這是搬家。

use serde::{Deserialize, Serialize};

use crate::code_table::Frequency;

/// 某個 N 值下的結果。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EfficiencyPoint {
    /// 實際用上簡碼的字數（`min(n, 可用簡碼字數)`）。
    pub short_code_chars: usize,
    /// 字頻加權平均碼長，越小越好。
    pub weighted_code_length: f64,
}

/// 簡碼效率曲線。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortCodeEfficiency {
    /// 按收益降序排出的簡碼字，取最大 N 個。各 N 值的字串是它的前綴。
    pub ranked_characters: String,
    pub points: Vec<EfficiencyPoint>,
}

struct Candidate {
    character: String,
    short_len: usize,
    full_len: usize,
    frequency: f64,
    gain: f64,
}

/// 計算簡碼效率曲線。
///
/// `short` / `full` 是字符到簡碼／全碼的映射，都應當已經帶上選重鍵——碼長算的是
/// 實際擊鍵數。`max_code_length` 用來判定「這還算不算簡碼」：長度等於全碼最大碼長
/// 的不算。
///
/// 收益 = 字頻 ×（全碼長 − 簡碼長）。按收益降序取前 N 個配簡碼，其餘用全碼。
pub fn short_code_efficiency(
    frequency: &Frequency,
    short: &dyn Fn(&str) -> Option<String>,
    full: &dyn Fn(&str) -> Option<String>,
    max_code_length: usize,
    ns: &[usize],
) -> ShortCodeEfficiency {
    let mut candidates: Vec<Candidate> = Vec::new();

    for (character, &freq) in frequency {
        if freq <= 0.0 {
            continue;
        }
        let (Some(s), Some(f)) = (short(character), full(character)) else {
            continue;
        };
        let short_len = s.chars().count();
        let full_len = f.chars().count();
        candidates.push(Candidate {
            character: character.clone(),
            short_len,
            full_len,
            frequency: freq,
            gain: freq * (full_len as f64 - short_len as f64),
        });
    }

    // 全碼基線：所有字都用全碼時的加權碼長。
    let baseline: f64 = candidates
        .iter()
        .map(|c| c.frequency * c.full_len as f64)
        .sum();

    let mut eligible: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.short_len < c.full_len && c.short_len < max_code_length)
        .collect();
    // 收益相同時按字符排序，好讓結果不依賴 HashMap 的遍歷次序。
    eligible.sort_by(|a, b| {
        b.gain
            .total_cmp(&a.gain)
            .then_with(|| a.character.cmp(&b.character))
    });

    let max_n = ns.iter().copied().max().unwrap_or(0).min(eligible.len());
    let ranked_characters: String = eligible[..max_n]
        .iter()
        .map(|c| c.character.as_str())
        .collect();

    // 前綴和：前 k 個字改用簡碼省下的加權碼長。
    let mut saved = Vec::with_capacity(eligible.len() + 1);
    saved.push(0.0);
    for c in &eligible {
        saved.push(saved.last().unwrap() + c.gain);
    }

    let points = ns
        .iter()
        .map(|&n| {
            let k = n.min(eligible.len());
            EfficiencyPoint {
                short_code_chars: k,
                weighted_code_length: baseline - saved[k],
            }
        })
        .collect();

    ShortCodeEfficiency {
        ranked_characters,
        points,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn lookup(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn taking_more_short_codes_never_makes_it_worse() {
        let frequency: Frequency = [("的", 0.5), ("是", 0.3), ("我", 0.2)]
            .into_iter()
            .map(|(c, f)| (c.to_string(), f))
            .collect();
        let short = lookup(&[("的", "e"), ("是", "i"), ("我", "o")]);
        let full = lookup(&[("的", "dwke"), ("是", "jfhi"), ("我", "trap")]);

        let result = short_code_efficiency(
            &frequency,
            &|c| short.get(c).cloned(),
            &|c| full.get(c).cloned(),
            4,
            &[0, 1, 2, 3],
        );

        // 全用全碼：碼長 4
        assert!((result.points[0].weighted_code_length - 4.0).abs() < 1e-12);
        // 收益最大的是 的（0.5 × 3 = 1.5）
        assert_eq!(result.ranked_characters.chars().next(), Some('的'));
        assert!((result.points[1].weighted_code_length - 2.5).abs() < 1e-12);
        assert!((result.points[3].weighted_code_length - 1.0).abs() < 1e-12);

        let lengths: Vec<f64> = result
            .points
            .iter()
            .map(|p| p.weighted_code_length)
            .collect();
        assert!(lengths.windows(2).all(|w| w[1] <= w[0] + 1e-12));
    }

    #[test]
    fn a_short_code_that_is_not_shorter_is_not_a_short_code() {
        let frequency: Frequency = [("的".to_string(), 1.0)].into_iter().collect();
        let short = lookup(&[("的", "dwke")]);
        let full = lookup(&[("的", "dwke")]);
        let result = short_code_efficiency(
            &frequency,
            &|c| short.get(c).cloned(),
            &|c| full.get(c).cloned(),
            4,
            &[0, 10],
        );
        assert_eq!(result.ranked_characters, "");
        assert!((result.points[1].weighted_code_length - 4.0).abs() < 1e-12);
    }
}
