import { defineConfig } from 'vitest/config'

/**
 * 集成測試用的配置。獨立一份，不從 `vite.config.ts` 合併。
 *
 * `vite.config.ts` 把 `tests/integration-*.test.ts` 排除在默認測試之外（它要聯網、
 * 拿的是會變的線上數據）。而 vitest 的 `--exclude` 和 `mergeConfig` 對數組都是
 * **追加**不是覆蓋，於是排除規則怎麼都甩不掉，顯式點名運行的結果是
 * 「No test files found」外加退出碼 1——一條看起來像「跑了但沒東西」的失敗，
 * 比不跑更糟。
 */
export default defineConfig({
  test: {
    include: ['tests/integration-*.test.ts'],
    // 線上取數據，60 s 的 beforeAll 在默認 5 s 超時下必紅。
    testTimeout: 120_000,
    hookTimeout: 120_000,
  },
})
