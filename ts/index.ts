// 公開 API。計算一律在 Rust 裏，這裏導出的都是它的包裝。
export * from './types/index.js'
export * from './stats/index.js'
export * from './charset/index.js'
export * from './data/index.js'
export * from './utils/index.js'

export { getWasmModule, isWasmLoaded, serializeCodeTable } from './wasm.js'
export type { WasmModule, CodeTableJson } from './wasm.js'
