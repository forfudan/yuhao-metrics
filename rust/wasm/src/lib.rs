//! wasm 綁定 —— 只做 JSON 進出，算法在 `yuhao_metrics_core`。
//!
//! 每個導出函數都是「解析 → 調 core → 序列化」三行。看見這裏出現 `if` 或者算術，
//! 就是有東西寫錯了地方。

use std::collections::{HashMap, HashSet};

use serde::Deserialize;
use wasm_bindgen::prelude::*;
use yuhao_metrics_core as core;
use yuhao_metrics_core::code_table::{CodeTable, Frequency};

#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

fn parse<T: for<'de> Deserialize<'de>>(what: &str, json: &str) -> Result<T, JsValue> {
    serde_json::from_str(json).map_err(|e| JsValue::from_str(&format!("{what} 解析失敗: {e}")))
}

/// 序列化到 JS。
///
/// `serialize_maps_as_objects(true)` 是必須的：serde_wasm_bindgen 默認把
/// `HashMap` 出成 JS 的 `Map`，於是 `counts.a` 是 `undefined` 而不是報錯——一個
/// 只在讀取時才發作、且悄無聲息的錯。這裏的 map 全是「字符串鍵 → 數」，出成普通
/// 對象才對得上 TS 那邊的 `Record<string, number>`。
fn emit<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    let serializer = serde_wasm_bindgen::Serializer::new().serialize_maps_as_objects(true);
    value
        .serialize(&serializer)
        .map_err(|e| JsValue::from_str(&format!("序列化失敗: {e}")))
}

/// 碼表的 JSON 形態：`{"的": ["dwke"], …}`，或者每字一碼的 `{"的": "dwke"}`。
#[derive(Deserialize)]
#[serde(untagged)]
enum CodesJson {
    Many(Vec<String>),
    One(String),
}

/// **JSON 對象的鍵是無序的**，而動態選重率的 `as_listed` 算法依賴碼表順序。所以
/// 需要保序時傳數組形態 `[["的", ["dwke"]], …]`，傳對象則只能用 `by_frequency`。
#[derive(Deserialize)]
#[serde(untagged)]
enum TableJson {
    Ordered(Vec<(String, CodesJson)>),
    Unordered(HashMap<String, CodesJson>),
}

fn to_code_table(json: &str) -> Result<CodeTable, JsValue> {
    let parsed: TableJson = parse("碼表", json)?;
    let mut table = CodeTable::new();
    let mut push = |character: String, codes: CodesJson| {
        table.push(
            character,
            match codes {
                CodesJson::Many(v) => v,
                CodesJson::One(s) => vec![s],
            },
        );
    };
    match parsed {
        TableJson::Ordered(rows) => rows.into_iter().for_each(|(c, k)| push(c, k)),
        TableJson::Unordered(map) => map.into_iter().for_each(|(c, k)| push(c, k)),
    }
    Ok(table)
}

fn candidate_order(as_listed: bool) -> core::duplicate::CandidateOrder {
    if as_listed {
        core::duplicate::CandidateOrder::AsListed
    } else {
        core::duplicate::CandidateOrder::ByFrequency
    }
}

// ── 重碼 ─────────────────────────────────────────────────────────────────────

/// 靜態重碼。`charset_json` 傳 `null` 統計全表，或傳一個字符數組。
#[wasm_bindgen]
pub fn static_duplicates(
    code_table_json: &str,
    charset_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let table = to_code_table(code_table_json)?;
    let charset: Option<HashSet<String>> = charset_json
        .as_deref()
        .map(|j| parse("字符集", j))
        .transpose()?;
    emit(&core::duplicate::static_duplicates(
        &table,
        charset.as_ref(),
    ))
}

/// 動態選重率。返回 `{duplicate_mass, total_mass, rate}`。
#[wasm_bindgen]
pub fn dynamic_selection(
    code_table_json: &str,
    frequency_json: &str,
    as_listed: bool,
) -> Result<JsValue, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    emit(&core::duplicate::dynamic_selection(
        &table,
        &frequency,
        candidate_order(as_listed),
    ))
}

/// 從已帶選重鍵的碼表算動態選重率。
#[wasm_bindgen]
pub fn dynamic_selection_from_selection_keys(
    code_table_json: &str,
    frequency_json: &str,
) -> Result<JsValue, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    emit(&core::duplicate::dynamic_selection_from_selection_keys(
        &table, &frequency,
    ))
}

/// 非首選重碼字明細，按字頻降序。
#[wasm_bindgen]
pub fn non_first_duplicates(
    code_table_json: &str,
    frequency_json: &str,
    as_listed: bool,
) -> Result<JsValue, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    emit(&core::duplicate::non_first_duplicates(
        &table,
        &frequency,
        candidate_order(as_listed),
    ))
}

// ── 當量 ─────────────────────────────────────────────────────────────────────

fn normalization(
    selection_keys_json: Option<String>,
    pad_to: Option<usize>,
) -> Result<core::equivalence::CodeNormalization, JsValue> {
    Ok(core::equivalence::CodeNormalization {
        selection_keys: selection_keys_json
            .as_deref()
            .map(|j| parse("選重鍵表", j))
            .transpose()?
            .unwrap_or_default(),
        pad_to,
    })
}

/// 當量表查不到的碼對怎麼算：`"zero"` 計 0 仍佔分母、`"skip"` 整個跳過、
/// `"hand"` 按左右手兜底。三個名字與 PyO3 綁定那一側完全相同。
///
/// 從前這裏是個 `bool`，於是 `HandFallback` 這一檔在 JS 這一側根本到不了——而
/// core 的文檔把「三份舊實現的差別全變成顯式參數」當作本庫存在的理由。
fn unknown_pair(unknown: Option<String>) -> Result<core::equivalence::UnknownPair, JsValue> {
    match unknown.as_deref().unwrap_or("zero") {
        "zero" => Ok(core::equivalence::UnknownPair::CountAsZero),
        "skip" => Ok(core::equivalence::UnknownPair::Skip),
        "hand" => Ok(core::equivalence::UnknownPair::HandFallback),
        other => Err(JsValue::from_str(&format!(
            "unknown={other:?}：只有 \"zero\"、\"skip\"、\"hand\""
        ))),
    }
}

/// 相鄰碼對頻率。
#[wasm_bindgen]
pub fn pair_frequencies(
    code_table_json: &str,
    frequency_json: &str,
    selection_keys_json: Option<String>,
    pad_to: Option<usize>,
) -> Result<JsValue, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    let norm = normalization(selection_keys_json, pad_to)?;
    emit(&core::equivalence::pair_frequencies(
        &table, &frequency, &norm,
    ))
}

/// 字頻加權速度當量。`per_pair` 選聚合方式，兩者不是同一個數——見 core 的文檔。
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn weighted_equivalent(
    code_table_json: &str,
    frequency_json: &str,
    equiv_table_json: &str,
    per_pair: bool,
    selection_keys_json: Option<String>,
    pad_to: Option<usize>,
    unknown: Option<String>,
) -> Result<f64, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    let equiv: core::equivalence::EquivTable = parse("當量表", equiv_table_json)?;
    let norm = normalization(selection_keys_json, pad_to)?;
    Ok(core::equivalence::weighted_equivalent(
        &table,
        &frequency,
        &equiv,
        if per_pair {
            core::equivalence::Aggregation::PerPair
        } else {
            core::equivalence::Aggregation::PerCharacter
        },
        &norm,
        unknown_pair(unknown)?,
    ))
}

/// 單個編碼的速度當量。
#[wasm_bindgen]
pub fn code_equivalent(
    code: &str,
    equiv_table_json: &str,
    unknown: Option<String>,
) -> Result<f64, JsValue> {
    let equiv: core::equivalence::EquivTable = parse("當量表", equiv_table_json)?;
    Ok(core::equivalence::code_equivalent(
        code,
        &equiv,
        unknown_pair(unknown)?,
    ))
}

/// 當量分檔，按當量降序。
#[wasm_bindgen]
pub fn equivalent_distribution(
    pair_frequencies_json: &str,
    equiv_table_json: &str,
    low: f64,
    high: f64,
    step: f64,
) -> Result<JsValue, JsValue> {
    let pairs: HashMap<String, f64> = parse("碼對頻率", pair_frequencies_json)?;
    let equiv: core::equivalence::EquivTable = parse("當量表", equiv_table_json)?;
    emit(&core::equivalence::equivalent_distribution(
        &pairs,
        &equiv,
        (low, high),
        step,
    ))
}

/// 把編碼末尾的選重數字換成方案實際使用的按鍵。
#[wasm_bindgen]
pub fn replace_selection_key(code: &str, selection_keys_json: &str) -> Result<String, JsValue> {
    let keys: core::equivalence::SelectionKeys = parse("選重鍵表", selection_keys_json)?;
    Ok(core::equivalence::replace_selection_key(code, &keys))
}

// ── 人機工程 ─────────────────────────────────────────────────────────────────

/// 按鍵計數（熱力圖）。數全部編碼，`_` 記作 `space`。
#[wasm_bindgen]
pub fn key_counts(
    code_table_json: &str,
    frequency_json: &str,
    selection_keys_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    let norm = normalization(selection_keys_json, None)?;
    emit(&core::ergonomics::key_counts(&table, &frequency, &norm))
}

/// 鍵位分數。`profile` 取 `"zigen"`（字根碼位）或 `"short_code"`（簡碼）。
#[wasm_bindgen]
pub fn ergo_score(
    code_table_json: &str,
    frequency_json: &str,
    profile: &str,
    no_z: bool,
) -> Result<f64, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    let profile = match profile {
        "zigen" => core::ergonomics::ErgoProfile::zigen(),
        "short_code" => core::ergonomics::ErgoProfile::short_code(no_z),
        other => return Err(JsValue::from_str(&format!("未知的鍵位分數參數集: {other}"))),
    };
    Ok(core::ergonomics::ergo_score(&table, &frequency, &profile))
}

/// 單手率（頻率質量；字頻歸一化時就是比例）。
#[wasm_bindgen]
pub fn single_hand_mass(
    code_table_json: &str,
    frequency_json: &str,
    min_len: usize,
) -> Result<f64, JsValue> {
    let table = to_code_table(code_table_json)?;
    let frequency: Frequency = parse("字頻", frequency_json)?;
    Ok(core::ergonomics::single_hand_mass(
        &table, &frequency, min_len,
    ))
}

// ── 簡碼效率 ─────────────────────────────────────────────────────────────────

/// 簡碼效率曲線。
#[wasm_bindgen]
pub fn short_code_efficiency(
    frequency_json: &str,
    short_codes_json: &str,
    full_codes_json: &str,
    max_code_length: usize,
    ns_json: &str,
) -> Result<JsValue, JsValue> {
    let frequency: Frequency = parse("字頻", frequency_json)?;
    let short: HashMap<String, String> = parse("簡碼表", short_codes_json)?;
    let full: HashMap<String, String> = parse("全碼表", full_codes_json)?;
    let ns: Vec<usize> = parse("N 值列表", ns_json)?;
    emit(&core::short_code::short_code_efficiency(
        &frequency,
        &|c| short.get(c).cloned(),
        &|c| full.get(c).cloned(),
        max_code_length,
        &ns,
    ))
}

// ── 字集 ─────────────────────────────────────────────────────────────────────

/// CJK 區塊定義。TS 那邊不再自帶 JSON，就是爲了只有這一份（RECONCILIATION #4）。
#[wasm_bindgen]
pub fn cjk_blocks() -> Result<JsValue, JsValue> {
    emit(&core::charset::CJK_BLOCKS)
}

/// 字符是否落在指定 CJK 區塊裏。
#[wasm_bindgen]
pub fn is_in_block(character: &str, block_id: &str) -> bool {
    core::charset::is_in_block(character, block_id)
}

/// 字符落在哪些 CJK 區塊裏。區塊互不重疊，所以最多一個。
#[wasm_bindgen]
pub fn blocks_of(character: &str) -> Result<JsValue, JsValue> {
    emit(&core::charset::blocks_of(character))
}

/// 字符是否屬於某個字集。名字見 core 的 `is_in_charset` 文檔。
///
/// `charset_data_json` 只有 `gb2312` / `tonggui` / `guozi` / `tonggui_guozi`
/// 用得上，形如 `{"的": {"is_gb2312": true, …}}`。
#[wasm_bindgen]
pub fn is_in_charset(
    character: &str,
    charset: &str,
    charset_data_json: Option<String>,
) -> Result<bool, JsValue> {
    let data: Option<core::charset::CharsetData> = charset_data_json
        .as_deref()
        .map(|j| parse("字集數據", j))
        .transpose()?;
    Ok(core::charset::is_in_charset(
        character,
        charset,
        data.as_ref(),
    ))
}

/// 過濾出屬於某字集的字符，保持輸入順序。
#[wasm_bindgen]
pub fn filter_by_charset(
    characters_json: &str,
    charset: &str,
    charset_data_json: Option<String>,
) -> Result<JsValue, JsValue> {
    let characters: Vec<String> = parse("字符列表", characters_json)?;
    let data: Option<core::charset::CharsetData> = charset_data_json
        .as_deref()
        .map(|j| parse("字集數據", j))
        .transpose()?;
    emit(&core::charset::filter_by_charset(
        characters.iter().map(String::as_str),
        charset,
        data.as_ref(),
    ))
}

/// 累積字集 `cjk_to_*` 的區塊次序。
#[wasm_bindgen]
pub fn cumulative_block_order() -> Result<JsValue, JsValue> {
    emit(&core::charset::CUMULATIVE_ORDER)
}
