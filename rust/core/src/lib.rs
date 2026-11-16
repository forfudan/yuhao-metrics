//! # yuhao-metrics-core
//!
//! 中文輸入法方案的評分核心：重碼 · 當量 · 選重 · 人機工程 · 簡碼效率。
//!
//! **與方案無關 (schema-agnostic)。** 這裏不知道什麼是字根、什麼是拆分——輸入只有
//! 碼表和字頻，所以五筆、倉頡、虎碼一樣算得。字根與拆分那一層是 `yuhao-core`，它
//! 產出碼表之後調用這裏（`docs/RESTRUCTURE.md` §2）。
//!
//! ## 這個 crate 裏沒有綁定
//!
//! 沒有 `wasm_bindgen`，沒有 `pyo3`，於是 `cargo test` 測得到每一個算法。綁定在
//! 隔壁的 `yuhao-metrics-wasm` 和 `yuhao-metrics-py`，它們只做序列化。
//!
//! ## 分歧當參數，不當分叉
//!
//! 同一個指標此前在 `yulib/quant.py`、`yuhao-assess/src/services/*.ts` 和這裏各有
//! 一份，算出來的數不一樣。差別一律變成顯式參數
//! （[`duplicate::CandidateOrder`]、[`equivalence::Aggregation`]、
//! [`equivalence::UnknownPair`]……），一個都不丟，也不預設哪個「對」。
//! 裁決記錄在 `docs/RECONCILIATION.md`。

pub mod charset;
pub mod code_table;
pub mod duplicate;
pub mod equivalence;
pub mod ergonomics;
pub mod short_code;

pub use code_table::{CodeTable, Entry, Frequency};
