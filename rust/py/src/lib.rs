//! PyO3 綁定 —— Python 調用方的入口。算法在 `yuhao_metrics_core`，這裏只做類型轉換。
//!
//! 用法：
//!
//! ```python
//! import yuhao_metrics as ym
//! table = dict(zip(df["char"], df["code"]))
//! freq  = dict(zip(df["char"], df["freq_simp"]))
//! ym.dynamic_selection(table, freq)["rate"]
//! ```
//!
//! 傳 `dict` 是最省事的，但 **`dict` 沒有「碼表順序」可言**——需要
//! `as_listed=True` 時要傳 `list[tuple[str, str]]`。

use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use yuhao_metrics_core as core;
use yuhao_metrics_core::code_table::{CodeTable, Frequency};

/// 碼表：`dict[str, str]`、`dict[str, list[str]]`，或保序的 `list[tuple]`。
#[derive(FromPyObject)]
enum TableArg {
    #[pyo3(transparent, annotation = "list")]
    Ordered(Vec<(String, CodesArg)>),
    #[pyo3(transparent, annotation = "dict")]
    Unordered(HashMap<String, CodesArg>),
}

#[derive(FromPyObject)]
enum CodesArg {
    #[pyo3(transparent, annotation = "str")]
    One(String),
    #[pyo3(transparent, annotation = "list")]
    Many(Vec<String>),
}

impl From<CodesArg> for Vec<String> {
    fn from(codes: CodesArg) -> Self {
        match codes {
            CodesArg::One(s) => vec![s],
            CodesArg::Many(v) => v,
        }
    }
}

impl From<TableArg> for CodeTable {
    fn from(arg: TableArg) -> Self {
        let mut table = CodeTable::new();
        match arg {
            TableArg::Ordered(rows) => {
                for (character, codes) in rows {
                    table.push(character, codes.into());
                }
            }
            TableArg::Unordered(map) => {
                for (character, codes) in map {
                    table.push(character, codes.into());
                }
            }
        }
        table
    }
}

fn candidate_order(as_listed: bool) -> core::duplicate::CandidateOrder {
    if as_listed {
        core::duplicate::CandidateOrder::AsListed
    } else {
        core::duplicate::CandidateOrder::ByFrequency
    }
}

fn selection<'py>(py: Python<'py>, value: core::duplicate::DynamicSelection) -> Bound<'py, PyDict> {
    let d = PyDict::new(py);
    d.set_item("duplicate_mass", value.duplicate_mass).unwrap();
    d.set_item("total_mass", value.total_mass).unwrap();
    d.set_item("rate", value.rate).unwrap();
    d
}

/// 靜態重碼。`charset=None` 統計全表。
#[pyfunction]
#[pyo3(signature = (code_table, charset = None))]
fn static_duplicates<'py>(
    py: Python<'py>,
    code_table: TableArg,
    charset: Option<HashSet<String>>,
) -> PyResult<Bound<'py, PyDict>> {
    let stats = core::duplicate::static_duplicates(&code_table.into(), charset.as_ref());
    let d = PyDict::new(py);
    d.set_item("total_chars", stats.total_chars)?;
    d.set_item("duplicate_count", stats.duplicate_count)?;
    d.set_item("duplicate_groups", stats.duplicate_groups)?;
    d.set_item("duplicate_rate", stats.duplicate_rate)?;
    d.set_item("unique_codes", stats.unique_codes)?;
    d.set_item("duplicate_chars", stats.duplicate_chars)?;
    Ok(d)
}

/// 動態選重。返回 `{"duplicate_mass", "total_mass", "rate"}`。
///
/// `quant.get_dynamic_dup_rate` 返回的是 `duplicate_mass`，TS 那份返回的是
/// `rate`；兩個都在這裏，選哪個由調用方寫明。
#[pyfunction]
#[pyo3(signature = (code_table, frequency, as_listed = false))]
fn dynamic_selection<'py>(
    py: Python<'py>,
    code_table: TableArg,
    frequency: Frequency,
    as_listed: bool,
) -> Bound<'py, PyDict> {
    let value = core::duplicate::dynamic_selection(
        &code_table.into(),
        &frequency,
        candidate_order(as_listed),
    );
    selection(py, value)
}

/// 從已帶選重鍵的碼表算動態選重。
#[pyfunction]
fn dynamic_selection_from_selection_keys<'py>(
    py: Python<'py>,
    code_table: TableArg,
    frequency: Frequency,
) -> Bound<'py, PyDict> {
    let value =
        core::duplicate::dynamic_selection_from_selection_keys(&code_table.into(), &frequency);
    selection(py, value)
}

/// 非首選重碼字明細。
#[pyfunction]
#[pyo3(signature = (code_table, frequency, as_listed = false))]
fn non_first_duplicates<'py>(
    py: Python<'py>,
    code_table: TableArg,
    frequency: Frequency,
    as_listed: bool,
) -> PyResult<Bound<'py, PyList>> {
    let items = core::duplicate::non_first_duplicates(
        &code_table.into(),
        &frequency,
        candidate_order(as_listed),
    );
    let list = PyList::empty(py);
    for item in items {
        let d = PyDict::new(py);
        d.set_item("character", item.character)?;
        d.set_item("code", item.code)?;
        d.set_item("frequency", item.frequency)?;
        d.set_item("candidates", item.candidates)?;
        d.set_item("rank", item.rank)?;
        list.append(d)?;
    }
    Ok(list)
}

fn normalization(
    selection_keys: Option<HashMap<String, String>>,
    pad_to: Option<usize>,
) -> core::equivalence::CodeNormalization {
    core::equivalence::CodeNormalization {
        selection_keys: selection_keys.unwrap_or_default(),
        pad_to,
    }
}

/// 當量表查不到的碼對怎麼算：`"zero"` 計 0 仍佔分母、`"skip"` 整個跳過、
/// `"hand"` 按左右手兜底（碼表生成器那一派）。
///
/// 認不出來的值當場報錯，不默默回退到某一種——三種算法得出的是三個不同的數。
fn unknown_pair(unknown: &str) -> PyResult<core::equivalence::UnknownPair> {
    match unknown {
        "zero" => Ok(core::equivalence::UnknownPair::CountAsZero),
        "skip" => Ok(core::equivalence::UnknownPair::Skip),
        "hand" => Ok(core::equivalence::UnknownPair::HandFallback),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "unknown={other:?}：只有 \"zero\"、\"skip\"、\"hand\""
        ))),
    }
}

/// 字頻加權速度當量。
///
/// 默認 `per_pair=False` + `skip_unknown_pairs=False`，即 `quant.get_equiv_score`
/// 的行爲；配上 `pad_to=max_len` 就與它完全一致。
#[pyfunction]
#[pyo3(signature = (
    code_table, frequency, equiv_table,
    per_pair = false, selection_keys = None, pad_to = None, unknown = "zero",
))]
#[allow(clippy::too_many_arguments)]
fn weighted_equivalent(
    code_table: TableArg,
    frequency: Frequency,
    equiv_table: core::equivalence::EquivTable,
    per_pair: bool,
    selection_keys: Option<HashMap<String, String>>,
    pad_to: Option<usize>,
    unknown: &str,
) -> PyResult<f64> {
    Ok(core::equivalence::weighted_equivalent(
        &code_table.into(),
        &frequency,
        &equiv_table,
        if per_pair {
            core::equivalence::Aggregation::PerPair
        } else {
            core::equivalence::Aggregation::PerCharacter
        },
        &normalization(selection_keys, pad_to),
        unknown_pair(unknown)?,
    ))
}

/// 單個編碼的速度當量。對應 `quant.get_equiv_score_for_pop_code`。
#[pyfunction]
#[pyo3(signature = (code, equiv_table, unknown = "zero", pad_to = None))]
fn code_equivalent(
    code: &str,
    equiv_table: core::equivalence::EquivTable,
    unknown: &str,
    pad_to: Option<usize>,
) -> PyResult<f64> {
    let norm = normalization(None, pad_to);
    Ok(core::equivalence::code_equivalent(
        &norm.apply(code),
        &equiv_table,
        unknown_pair(unknown)?,
    ))
}

/// 一批編碼各自的速度當量，次序與傳進來的一樣。
///
/// 與逐個調用 [`code_equivalent`] 同值；當量表只過一次 FFI，所以碼位成千上萬時
/// 快得多。`yuhao_build/metrics.py` 用的是 `unknown="hand", pad_to=4`。
#[pyfunction]
#[pyo3(signature = (codes, equiv_table, unknown = "zero", pad_to = None))]
fn code_equivalents(
    codes: Vec<String>,
    equiv_table: core::equivalence::EquivTable,
    unknown: &str,
    pad_to: Option<usize>,
) -> PyResult<Vec<f64>> {
    let unknown = unknown_pair(unknown)?;
    let norm = normalization(None, pad_to);
    Ok(codes
        .iter()
        .map(|code| {
            core::equivalence::code_equivalent(&norm.apply(code), &equiv_table, unknown)
        })
        .collect())
}

/// 相鄰碼對頻率。
#[pyfunction]
#[pyo3(signature = (code_table, frequency, selection_keys = None, pad_to = None))]
fn pair_frequencies(
    code_table: TableArg,
    frequency: Frequency,
    selection_keys: Option<HashMap<String, String>>,
    pad_to: Option<usize>,
) -> HashMap<String, f64> {
    core::equivalence::pair_frequencies(
        &code_table.into(),
        &frequency,
        &normalization(selection_keys, pad_to),
    )
}

/// 把編碼末尾的選重數字換成方案實際使用的按鍵。
#[pyfunction]
fn replace_selection_key(code: &str, selection_keys: HashMap<String, String>) -> String {
    core::equivalence::replace_selection_key(code, &selection_keys)
}

/// 按鍵計數（熱力圖）。
#[pyfunction]
#[pyo3(signature = (code_table, frequency, selection_keys = None))]
fn key_counts(
    code_table: TableArg,
    frequency: Frequency,
    selection_keys: Option<HashMap<String, String>>,
) -> HashMap<String, f64> {
    core::ergonomics::key_counts(
        &code_table.into(),
        &frequency,
        &normalization(selection_keys, None),
    )
}

/// 鍵位分數。`profile` 取 `"zigen"` 或 `"short_code"`。
#[pyfunction]
#[pyo3(signature = (code_table, frequency, profile = "zigen", no_z = false))]
fn ergo_score(
    code_table: TableArg,
    frequency: Frequency,
    profile: &str,
    no_z: bool,
) -> PyResult<f64> {
    let profile = match profile {
        "zigen" => core::ergonomics::ErgoProfile::zigen(),
        "short_code" => core::ergonomics::ErgoProfile::short_code(no_z),
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "未知的鍵位分數參數集: {other}"
            )))
        }
    };
    Ok(core::ergonomics::ergo_score(
        &code_table.into(),
        &frequency,
        &profile,
    ))
}

/// 單手率（頻率質量）。對應 `quant.get_single_hand_rate`。
#[pyfunction]
#[pyo3(signature = (code_table, frequency, min_len = 4))]
fn single_hand_mass(code_table: TableArg, frequency: Frequency, min_len: usize) -> f64 {
    core::ergonomics::single_hand_mass(&code_table.into(), &frequency, min_len)
}

/// 簡碼效率曲線。
#[pyfunction]
fn short_code_efficiency<'py>(
    py: Python<'py>,
    frequency: Frequency,
    short_codes: HashMap<String, String>,
    full_codes: HashMap<String, String>,
    max_code_length: usize,
    ns: Vec<usize>,
) -> PyResult<Bound<'py, PyDict>> {
    let result = core::short_code::short_code_efficiency(
        &frequency,
        &|c| short_codes.get(c).cloned(),
        &|c| full_codes.get(c).cloned(),
        max_code_length,
        &ns,
    );
    let points = PyList::empty(py);
    for point in result.points {
        let d = PyDict::new(py);
        d.set_item("short_code_chars", point.short_code_chars)?;
        d.set_item("weighted_code_length", point.weighted_code_length)?;
        points.append(d)?;
    }
    let out = PyDict::new(py);
    out.set_item("ranked_characters", result.ranked_characters)?;
    out.set_item("points", points)?;
    Ok(out)
}

/// 字符是否落在指定 CJK 區塊裏。
#[pyfunction]
fn is_in_block(character: &str, block_id: &str) -> bool {
    core::charset::is_in_block(character, block_id)
}

/// 按表定義的字集數據，Python 側形態：`{"的": {"is_gb2312": True, …}}`。
///
/// 缺的鍵當 `False`，多的鍵忽略——上游那幾張表的列並不整齊。
type CharsetDataArg = HashMap<String, HashMap<String, bool>>;

fn charset_data(arg: Option<CharsetDataArg>) -> Option<core::charset::CharsetData> {
    arg.map(|map| {
        map.into_iter()
            .map(|(character, flags)| {
                let flag = |key: &str| flags.get(key).copied().unwrap_or(false);
                (
                    character,
                    core::charset::CharsetRecord {
                        is_gb2312: flag("is_gb2312"),
                        is_tonggui: flag("is_tonggui"),
                        is_guozi: flag("is_guozi"),
                    },
                )
            })
            .collect()
    })
}

/// 字符是否屬於某個字集。
#[pyfunction]
#[pyo3(signature = (character, charset, data = None))]
fn is_in_charset(character: &str, charset: &str, data: Option<CharsetDataArg>) -> bool {
    core::charset::is_in_charset(character, charset, charset_data(data).as_ref())
}

/// 過濾出屬於某字集的字符，保持輸入順序。
#[pyfunction]
#[pyo3(signature = (characters, charset, data = None))]
fn filter_by_charset(
    characters: Vec<String>,
    charset: &str,
    data: Option<CharsetDataArg>,
) -> Vec<String> {
    core::charset::filter_by_charset(
        characters.iter().map(String::as_str),
        charset,
        charset_data(data).as_ref(),
    )
}

/// CJK 區塊定義。
#[pyfunction]
fn cjk_blocks(py: Python<'_>) -> PyResult<Bound<'_, PyList>> {
    let list = PyList::empty(py);
    for block in core::charset::CJK_BLOCKS {
        let d = PyDict::new(py);
        d.set_item("id", block.id)?;
        d.set_item("name", block.name)?;
        d.set_item("start", block.start)?;
        d.set_item("end", block.end)?;
        list.append(d)?;
    }
    Ok(list)
}

#[pymodule]
fn yuhao_metrics(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(static_duplicates, m)?)?;
    m.add_function(wrap_pyfunction!(dynamic_selection, m)?)?;
    m.add_function(wrap_pyfunction!(dynamic_selection_from_selection_keys, m)?)?;
    m.add_function(wrap_pyfunction!(non_first_duplicates, m)?)?;
    m.add_function(wrap_pyfunction!(weighted_equivalent, m)?)?;
    m.add_function(wrap_pyfunction!(code_equivalent, m)?)?;
    m.add_function(wrap_pyfunction!(code_equivalents, m)?)?;
    m.add_function(wrap_pyfunction!(pair_frequencies, m)?)?;
    m.add_function(wrap_pyfunction!(replace_selection_key, m)?)?;
    m.add_function(wrap_pyfunction!(key_counts, m)?)?;
    m.add_function(wrap_pyfunction!(ergo_score, m)?)?;
    m.add_function(wrap_pyfunction!(single_hand_mass, m)?)?;
    m.add_function(wrap_pyfunction!(short_code_efficiency, m)?)?;
    m.add_function(wrap_pyfunction!(is_in_block, m)?)?;
    m.add_function(wrap_pyfunction!(is_in_charset, m)?)?;
    m.add_function(wrap_pyfunction!(filter_by_charset, m)?)?;
    m.add_function(wrap_pyfunction!(cjk_blocks, m)?)?;
    Ok(())
}
