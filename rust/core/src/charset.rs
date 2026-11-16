//! 字集歸屬 (charset membership) —— CJK 區塊。
//!
//! **這裏是唯一的一份 CJK 區塊定義（RECONCILIATION #4）。** 此前它有兩份：一處
//! 寫死的 `0x4E00..=0x9FFF`，和一份 JSON 表。兩份都對，但沒有機制保證它們一直都
//! 對——擴展 J 區的上界在 Unicode 17.0 動過，動的只是 JSON 那份。
//!
//! 現在 TS 那一側由 wasm 導出（`cjk_blocks()`），JSON 不再隨包發佈。
//!
//! **收攏的價值 2026-09-02 兌現了一次**：Unicode 18.0 只動一格（擴展 D 的填充上界
//! 加一），而全樹只有這一處要改——`grep 2B81D` 剩下的全是產物裏的數據行。從前那是
//! 兩處，而擴展 J 那一次只改到了其中一處。
//!
//! 通用規範漢字表、GB2312 這類**按表**定義的字集不在這裏——它們是數據不是常量，
//! 由調用方傳一個 `HashSet<String>` 進來（見 [`crate::duplicate::static_duplicates`]）。

use serde::{Deserialize, Serialize};

/// 一個 Unicode 區塊。`start` / `end` 都是閉區間。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub id: &'static str,
    pub name: &'static str,
    pub start: u32,
    pub end: u32,
}

/// Unicode 18.0。`end` 記的是該區塊**實際填充到**的碼位，不是區塊的名義上界——
/// 判斷一個字是不是漢字時，名義上界會把未分配的碼位也算進來。
pub const CJK_BLOCKS: &[Block] = &[
    Block {
        id: "cjk_basic",
        name: "CJK基本區",
        start: 0x4E00,
        end: 0x9FFF,
    },
    Block {
        id: "cjk_a",
        name: "CJK擴展A區",
        start: 0x3400,
        end: 0x4DBF,
    },
    Block {
        id: "cjk_b",
        name: "CJK擴展B區",
        start: 0x20000,
        end: 0x2A6DF,
    },
    Block {
        id: "cjk_c",
        name: "CJK擴展C區",
        start: 0x2A700,
        end: 0x2B73F,
    },
    Block {
        id: "cjk_d",
        name: "CJK擴展D區",
        start: 0x2B740,
        // Unicode 18.0 給這一區加了一個字（U+2B81E 𫠞），是 18.0 對這張表的**全部**
        // 改動——十一個區塊逐一對過 `Unihan_IRGSources-18.0.0`，其餘十個的填充上界一
        // 個都沒動。小篆那一區（U+3D000〜U+3FC3F）不進這張表：它不是 CJK 統一表意文
        // 字，進來會改變「這個字算不算漢字」，而今天全樹沒有一處要處理篆字。
        end: 0x2B81E,
    },
    Block {
        id: "cjk_e",
        name: "CJK擴展E區",
        start: 0x2B820,
        end: 0x2CEAD,
    },
    Block {
        id: "cjk_f",
        name: "CJK擴展F區",
        start: 0x2CEB0,
        end: 0x2EBE0,
    },
    Block {
        id: "cjk_i",
        name: "CJK擴展I區",
        start: 0x2EBF0,
        end: 0x2EE5D,
    },
    Block {
        id: "cjk_g",
        name: "CJK擴展G區",
        start: 0x30000,
        end: 0x3134A,
    },
    Block {
        id: "cjk_h",
        name: "CJK擴展H區",
        start: 0x31350,
        end: 0x323AF,
    },
    Block {
        id: "cjk_j",
        name: "CJK擴展J區",
        start: 0x323B0,
        end: 0x33479,
    },
];

pub fn block(id: &str) -> Option<&'static Block> {
    CJK_BLOCKS.iter().find(|b| b.id == id)
}

/// 字符（取首個碼位）是否落在指定區塊裏。
pub fn is_in_block(character: &str, block_id: &str) -> bool {
    let Some(block) = block(block_id) else {
        return false;
    };
    character
        .chars()
        .next()
        .is_some_and(|c| (block.start..=block.end).contains(&(c as u32)))
}

/// 字符落在哪些區塊裏。區塊互不重疊，所以最多一個——返回 `Vec` 是爲了跟 TS 那份
/// 的簽名一致，將來若加入重疊的兼容區也不必改調用方。
pub fn blocks_of(character: &str) -> Vec<&'static str> {
    let Some(c) = character.chars().next() else {
        return Vec::new();
    };
    let code = c as u32;
    CJK_BLOCKS
        .iter()
        .filter(|b| (b.start..=b.end).contains(&code))
        .map(|b| b.id)
        .collect()
}

/// 累積字集 `cjk_to_*` 的區塊次序。
///
/// **不是碼位次序。** 擴展 I 區的碼位（U+2EBF0）落在 F 和 G 之間，但「到 G 區」
/// 指的是基本區加擴展 A–G，不含 I。按碼位排會把 I 悄悄算進去。
pub const CUMULATIVE_ORDER: &[&str] = &[
    "cjk_basic",
    "cjk_a",
    "cjk_b",
    "cjk_c",
    "cjk_d",
    "cjk_e",
    "cjk_f",
    "cjk_g",
    "cjk_h",
    "cjk_i",
    "cjk_j",
];

/// 一個字符在按表定義的字集裏的歸屬。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharsetRecord {
    #[serde(default)]
    pub is_gb2312: bool,
    #[serde(default)]
    pub is_tonggui: bool,
    #[serde(default)]
    pub is_guozi: bool,
}

/// 按表定義的字集數據：字符 → 歸屬。
pub type CharsetData = std::collections::HashMap<String, CharsetRecord>;

/// 字符是否屬於某個字集。
///
/// 支持三類名字：
///
/// - `cjk_basic` / `cjk_a`…`cjk_j`（也接受 `cjk_ext_a` 這種舊寫法）——單個區塊。
/// - `cjk_to_a`…`cjk_to_j` —— 基本區起的累積範圍。
/// - `gb2312` / `tonggui` / `guozi` / `tonggui_guozi` —— 按表定義，需要 `data`。
///
/// 名字不認識或者該給的表沒給，一律返回 `false`。此前這段邏輯有兩份：TS 那邊 21
/// 個幾乎一樣的過濾函數，Rust 這邊一份只認 `cjk_basic` 與 `tonggui_guozi` 的縮水
/// 版——兩份對 `cjk_to_g` 的答案本來就不一樣。
pub fn is_in_charset(character: &str, charset: &str, data: Option<&CharsetData>) -> bool {
    let normalized = normalize_charset_name(charset);

    if let Some(id) = normalized.strip_prefix("cjk_to_") {
        let target = format!("cjk_{id}");
        let Some(upto) = CUMULATIVE_ORDER.iter().position(|b| *b == target) else {
            return false;
        };
        return CUMULATIVE_ORDER[..=upto]
            .iter()
            .any(|id| is_in_block(character, id));
    }

    if normalized.starts_with("cjk_") {
        return is_in_block(character, &normalized);
    }

    let Some(record) = data.and_then(|d| d.get(character)) else {
        return false;
    };
    match normalized.as_str() {
        "gb2312" => record.is_gb2312,
        "tonggui" => record.is_tonggui,
        "guozi" => record.is_guozi,
        "tonggui_guozi" => record.is_tonggui || record.is_guozi,
        _ => false,
    }
}

fn normalize_charset_name(charset: &str) -> String {
    match charset.strip_prefix("cjk_ext_") {
        Some(rest) => format!("cjk_{rest}"),
        None => charset.to_string(),
    }
}

/// 某字集裏的全部字符，順序保持輸入順序。
pub fn filter_by_charset<'a>(
    characters: impl IntoIterator<Item = &'a str>,
    charset: &str,
    data: Option<&CharsetData>,
) -> Vec<String> {
    characters
        .into_iter()
        .filter(|c| is_in_charset(c, charset, data))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_block_contains_common_characters() {
        assert!(is_in_block("的", "cjk_basic"));
        assert!(is_in_block("一", "cjk_basic"));
        assert!(!is_in_block("a", "cjk_basic"));
        assert!(!is_in_block("", "cjk_basic"));
    }

    #[test]
    fn an_unknown_block_id_is_false_not_a_panic() {
        assert!(!is_in_block("的", "cjk_z"));
        assert_eq!(block("cjk_z"), None);
    }

    #[test]
    fn blocks_do_not_overlap() {
        for (i, a) in CJK_BLOCKS.iter().enumerate() {
            assert!(a.start <= a.end, "{} 的區間是反的", a.id);
            for b in &CJK_BLOCKS[i + 1..] {
                assert!(
                    a.end < b.start || b.end < a.start,
                    "{} 和 {} 重疊了",
                    a.id,
                    b.id
                );
            }
        }
    }

    #[test]
    fn extension_b_is_found_by_its_supplementary_code_point() {
        // U+20000，擴展 B 區第一個字
        assert_eq!(blocks_of("\u{20000}"), vec!["cjk_b"]);
    }

    #[test]
    fn unicode_18_extended_block_d_by_one() {
        // U+2B81E 𫠞（日部四畫）是 Unicode 18.0 唯一一個新的 CJK 統一表意文字，
        // 而它落在擴展 D 區的尾巴上——17.0 的填充上界是 U+2B81D。這一條同時釘住
        // 「填充上界不是名義上界」：名義上界 U+2B81F 至今仍空着。
        assert!(is_in_charset("\u{2B81E}", "cjk_d", None));
        assert_eq!(blocks_of("\u{2B81E}"), vec!["cjk_d"]);
        assert!(blocks_of("\u{2B81F}").is_empty());
    }

    #[test]
    fn cumulative_charsets_stop_where_they_say_they_do() {
        // U+2EBF0 是擴展 I 區第一個字。它的碼位在 F 和 G 之間，但「到 G 區」不含它。
        let ext_i = "\u{2EBF0}";
        assert!(is_in_charset(ext_i, "cjk_i", None));
        assert!(!is_in_charset(ext_i, "cjk_to_g", None));
        assert!(is_in_charset(ext_i, "cjk_to_i", None));
        assert!(is_in_charset(ext_i, "cjk_to_j", None));
    }

    #[test]
    fn the_old_cjk_ext_a_spelling_still_works() {
        let ext_a = "\u{3400}";
        assert!(is_in_charset(ext_a, "cjk_a", None));
        assert!(is_in_charset(ext_a, "cjk_ext_a", None));
        assert!(!is_in_charset(ext_a, "cjk_basic", None));
    }

    #[test]
    fn table_charsets_need_the_table_and_say_false_without_it() {
        let mut data = CharsetData::new();
        data.insert(
            "的".to_string(),
            CharsetRecord {
                is_gb2312: true,
                is_tonggui: true,
                is_guozi: false,
            },
        );
        assert!(is_in_charset("的", "gb2312", Some(&data)));
        assert!(is_in_charset("的", "tonggui_guozi", Some(&data)));
        assert!(!is_in_charset("的", "guozi", Some(&data)));
        assert!(!is_in_charset("的", "gb2312", None));
        assert!(!is_in_charset("龘", "gb2312", Some(&data)));
    }

    #[test]
    fn an_unknown_charset_name_is_false() {
        assert!(!is_in_charset("的", "big5", None));
        assert!(!is_in_charset("的", "cjk_to_z", None));
    }
}
