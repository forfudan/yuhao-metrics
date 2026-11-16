//! 碼表 (code table) 與字頻 (character frequency) —— 全部指標的兩個輸入。

use std::collections::HashMap;

/// 一個字符及其全部編碼。首選編碼是 `codes[0]`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub character: String,
    pub codes: Vec<String>,
}

/// 碼表。
///
/// **順序是有意義的，所以這裏不是 `HashMap`。** 動態選重率有兩種算法：一種按字頻
/// 重排候選，一種照碼表原有的候選次序算 ([`CandidateOrder::AsListed`])。後者只要
/// 換成哈希表就再也算不對，而且錯得無聲無息——數值仍然合理，只是每次運行都不同。
///
/// [`CandidateOrder::AsListed`]: crate::duplicate::CandidateOrder::AsListed
#[derive(Debug, Clone, Default)]
pub struct CodeTable {
    entries: Vec<Entry>,
    index: HashMap<String, usize>,
}

impl CodeTable {
    pub fn new() -> Self {
        Self::default()
    }

    /// 追加一個字符。同一個字符再次出現時，編碼併入既有項，位置不變。
    pub fn push(&mut self, character: impl Into<String>, codes: Vec<String>) {
        let character = character.into();
        match self.index.get(&character) {
            Some(&i) => self.entries[i].codes.extend(codes),
            None => {
                self.index.insert(character.clone(), self.entries.len());
                self.entries.push(Entry { character, codes });
            }
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Entry> {
        self.entries.iter()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, character: &str) -> Option<&Entry> {
        self.index.get(character).map(|&i| &self.entries[i])
    }

    /// 首選編碼——除按鍵計數 (它要數全部編碼) 外，每一項指標用的都是它。
    pub fn primary_code(&self, character: &str) -> Option<&str> {
        self.get(character)
            .and_then(|e| e.codes.first())
            .map(String::as_str)
    }

    /// 每個字符只有一個編碼的碼表。`(字符, 編碼)` 對按給定順序保留。
    pub fn from_pairs<K, V, I>(pairs: I) -> Self
    where
        K: Into<String>,
        V: Into<String>,
        I: IntoIterator<Item = (K, V)>,
    {
        let mut table = Self::new();
        for (character, code) in pairs {
            table.push(character, vec![code.into()]);
        }
        table
    }
}

impl<'a> IntoIterator for &'a CodeTable {
    type Item = &'a Entry;
    type IntoIter = std::slice::Iter<'a, Entry>;
    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter()
    }
}

/// 字頻。絕對頻數還是歸一化頻率都可以——凡返回「率」的函數都自己除以總和；凡返回
/// 「頻率質量」的函數原樣加總，各函數文檔裏寫明是哪一種。
pub type Frequency = HashMap<String, f64>;

/// 取字頻，缺字記 0。
pub(crate) fn freq_of(frequency: &Frequency, character: &str) -> f64 {
    frequency.get(character).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_character_merges_into_the_existing_entry() {
        let mut table = CodeTable::new();
        table.push("的", vec!["de".into()]);
        table.push("的", vec!["de2".into()]);
        assert_eq!(table.len(), 1);
        assert_eq!(table.get("的").unwrap().codes, vec!["de", "de2"]);
        assert_eq!(table.primary_code("的"), Some("de"));
    }

    #[test]
    fn insertion_order_survives() {
        let table = CodeTable::from_pairs([("丙", "c"), ("甲", "a"), ("乙", "b")]);
        let order: Vec<&str> = table.iter().map(|e| e.character.as_str()).collect();
        assert_eq!(order, vec!["丙", "甲", "乙"]);
    }
}
