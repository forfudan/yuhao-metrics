import { defineConfig } from 'vite'
import { configDefaults } from 'vitest/config'
import dts from 'vite-plugin-dts'
import { resolve } from 'path'

export default defineConfig({
  test: {
    // 集成測試要聯網，而且拿的是 yu / yuhao-assess-data 的線上數據——那兩份數據
    // 會動，於是它既測不出本庫的回歸，又會讓 `just downstream-green` 隨別人的
    // 提交變紅。單元測試離線、隨時可跑；集成測試用 `pnpm test:integration` 顯式跑。
    exclude: [...configDefaults.exclude, 'tests/integration-*.test.ts'],
  },
  plugins: [
    dts({
      // ⚠️ **別開 `rollupTypes`**：這個庫是多入口（index ＋ stats ＋ charset ＋
      // data ＋ types），而 rollup 只攤平得了一個入口——開着它 `dist/index.d.ts`
      // 裏只剩介面、一個函數都沒有，`dist/stats/index.d.ts` 又 re-export 了
      // dist 裏並不存在的 `./duplicate`。runtime 的導出是齊的，所以這個洞只有
      // 裝上去用 TypeScript 的人纔撞得到（`has no exported member
      // 'calculateKeyCounts'`）。逐檔出聲明反而對：入口映射本來就按目錄走。
      rollupTypes: false,
      tsconfigPath: './tsconfig.json'
    })
  ],
  build: {
    lib: {
      entry: {
        index: resolve(__dirname, 'ts/index.ts'),
        'stats/index': resolve(__dirname, 'ts/stats/index.ts'),
        'charset/index': resolve(__dirname, 'ts/charset/index.ts'),
        'data/index': resolve(__dirname, 'ts/data/index.ts'),
        'types/index': resolve(__dirname, 'ts/types/index.ts')
      }
    },
    rollupOptions: {
      // 兩個 wasm-pack 產物目錄都不進 bundle：它們由 wasm-pack 生成並隨包發佈。
      external: [/^\.\.\/wasm(-node)?\//],
      // ⚠️ **擴展名必須按格式分開**：本包是 `"type": "module"`，Node 判格式只看
      // 擴展名 ＋ 最近一份 package.json 的 `type`。CJS 產物叫 `*.cjs.js` 的話會被
      // 當成 ESM，`require()` 的人當場 `exports is not defined`，而打包器按內容
      // 判、本機測試又只走 ESM，所以這個洞不會被任何本地測試照出來。
      // 兩個格式各一份 output，纔給得出 `.js` 與 `.cjs` 兩種擴展名。
      output: [
        {
          format: 'es',
          preserveModules: false,
          exports: 'named',
          entryFileNames: ({ name }) =>
            name.includes('/') ? `${name.replace('/index', '')}/index.js` : `${name}.js`,
          // 共用塊單獨放，免得跟入口撞名撞出 index2.js 這種東西
          chunkFileNames: 'chunks/[name]-[hash].js'
        },
        {
          format: 'cjs',
          preserveModules: false,
          exports: 'named',
          entryFileNames: ({ name }) =>
            name.includes('/') ? `${name.replace('/index', '')}/index.cjs` : `${name}.cjs`,
          chunkFileNames: 'chunks/[name]-[hash].cjs'
        }
      ]
    }
  }
})
