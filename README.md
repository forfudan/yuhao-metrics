# @forfudan/yuhao-metrics

**中文輸入法方案的評分庫** —— 重碼 · 當量 · 選重 · 人機工程 · 簡碼效率。

與方案無關 (schema-agnostic)：輸入只有**碼表**和**字頻**，所以五筆、倉頡、虎碼一
樣算得。這裏不知道什麼是字根、什麼是拆分——那一層是 `yuhao-core`，它產出碼表之後
調用這裏。

## 結構

算法只有一份，在 Rust 裏。TypeScript 和 Python 都是它的綁定，不是它的副本。

```raw
rust/
├── core/    yuhao-metrics-core   純算法。不依賴 wasm-bindgen，也不依賴 pyo3
├── wasm/    yuhao-metrics        #[wasm_bindgen] 殼子 → wasm/ · wasm-node/
└── py/      yuhao-metrics-py     PyO3 殼子           → import yuhao_metrics
ts/                               TypeScript 包裝：序列化 + 命名轉換，沒有計算
```

`core` 不含任何綁定，是有原因的：`#[wasm_bindgen]` 的函數在非 wasm 目標上一調就
panic，所以只要算法和綁定住在一個 crate 裏，`cargo test` 就測不到算法。分開之後
`core` 的每個模塊都有真的在算的測試。

**指標的公式只許寫在 `rust/core/` 裏。** `ts/` 與 `rust/wasm/src/lib.rs` 只做序列
化與參數轉換，看見一行指標算術就是有東西寫錯了地方。（`ts/utils/frequency.ts` 的
歸一化／合併是例外：那幾支是給 TS 調用方的便利函數，不是指標。）

## 安裝

```bash
pnpm add @forfudan/yuhao-metrics
```

Python 側：`cd rust/py && maturin develop --release`，之後 `import yuhao_metrics`。

⚠️ **發包之前先 `npm pack --dry-run` 確認 `wasm/` 與 `wasm-node/` 在包裏**：
wasm-pack 在這兩個 out-dir 裏各寫一個內容是 `*` 的 `.gitignore`，而 npm 是**按目錄**
認 ignore 檔的，於是 `files` 白名單裏的它們一個檔都進不了包，裝上去就是「WASM 加載
失敗」。`prepack` 那一句為此刪掉它們（下次 build 自己重生），CI 另有一道 grep 兜着。

## 用起來

```typescript
import {
  calculateStaticDuplicates,
  calculateDynamicSelection,
  calculateWeightedEquivalent,
  loadCharFrequencies,
} from '@forfudan/yuhao-metrics'

const frequencies = await loadCharFrequencies()

// 碼表由調用方提供：本庫只評分，不編碼。
// 用 Map 不用普通對象——順序有意義，見下。
const codeTable = new Map([
  ['的', 'dwke'],
  ['得', 'dwkf'],
])

const dup = await calculateStaticDuplicates(codeTable)

// 速度當量要一張碼對當量表：鍵是相鄰的兩個按鍵，值是這一跳的相對耗時
//（最快的異手交替約 1.0，最慢的同指跳行約 2.1）。**表由調用方提供**，本庫不帶
// 數據；查不到的碼對怎麼算由 `unknown` 決定（`'zero'`／`'skip'`／`'hand'`）。
const equiv = await calculateWeightedEquivalent(codeTable, frequencies.sc, {
  dw: 1.0,
  wk: 1.5,
  ke: 1.4,
  kf: 1.3,
})
console.log(dup.duplicateCount, dup.duplicateGroups, dup.duplicateRate)

const dyn = await calculateDynamicSelection(codeTable, frequencies.zhihu)
console.log(dyn.rate) // 也有 dyn.duplicateMass / dyn.totalMass，見下
```

```python
import yuhao_metrics as ym

table = dict(zip(df["char"], df["code"]))
freq = dict(zip(df["char"], df["freq_simp"]))

ym.dynamic_selection(table, freq)["rate"]
ym.weighted_equivalent(table, freq, equiv_table, pad_to=4)
ym.ergo_score(table, freq, "zigen")
```

## 三件容易踩的事

**碼表順序有意義。** 動態選重率有兩種算法：`byFrequency` 按字頻重排候選，
`asListed` 照碼表列出的次序。後者評估的是「這方案**現在**發出去是什麼樣」，而它
只要換成哈希表就再也算不對——數值仍然合理，只是每次運行都不同。所以 TS 這邊用
`Map`，傳到 wasm 是數組不是對象；Python 那邊要 `as_listed=True` 時傳
`list[tuple]` 而不是 `dict`。

**動態選重返回的是「質量」不只是「比率」。** `{duplicateMass, totalMass, rate}`
三個都給，是因爲此前 Python 那份返回分子、TS 那份返回比率，而字頻歸一化時兩者恰好
相等——差別看不出來，於是誰也沒發現有兩種。單手率只返回一個質量（`number`）：字頻
歸一化時它就是比例，不歸一化時隨總頻數縮放。

**速度當量有兩種聚合方式，它們不是同一個數。** `perCharacter` 先把一個字的碼對
取均值再按字頻加權，`perPair` 把全表碼對放一起按出現頻率取均值。碼長一律相同且
字頻歸一化時二者相等，碼長不齊時就不等——而簡碼表的碼長從來不齊。默認
`perCharacter`。

這三件的來龍去脈記在作者工作區的裁決記錄裏（#11、#12、#14），見下。

**關於註釋裏指向別處的那些名字。** 本庫是從同一組公式的若干份舊實現（Python、
TypeScript、Rust）收攏出來的，每一處分歧怎麼裁的都有編號記錄。凡形如
`RECONCILIATION #n`、`§n`、`quant.py`、`yulib`、`yuhao_build/…`、`yuhao-assess` 的
`*Service.ts`、以及「本檔舊版」的指稱，說的都是**那些舊實現所在的位置與那份裁決
記錄**——它們在作者的工作區裏，**不隨本庫發佈**，所以這個倉裏找不到。留着這些名字
是為了讓「為什麼這裏是個參數而不是一個默認值」有據可查，不是漏了檔。

## 開發

```bash
pnpm build              # wasm ×2 → tsc → vite
pnpm test               # vitest（離線）
pnpm run test:rust      # cargo test --workspace
pnpm run lint           # eslint
pnpm run test:integration   # 要聯網，拿的是會變的線上數據
```

Python 綁定：`cd rust/py && maturin develop --release`，之後 `import yuhao_metrics`。

> 作者的工作區另有幾條 `just` 快捷命令（構建、測試、裝 Python 綁定、拿舊的 Python
> 實現在 12 萬字的真實碼表上逐項對數）。那些命令不在本庫裏，上面這幾條是全部。

## 授權

Apache-2.0
