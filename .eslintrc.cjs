module.exports = {
  root: true,
  env: { es2022: true, browser: true, node: true },
  parser: '@typescript-eslint/parser',
  parserOptions: { ecmaVersion: 2022, sourceType: 'module' },
  plugins: ['@typescript-eslint'],
  extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended'],
  rules: {
    // wasm 邊界上的值本來就是 any，逐個標註不如認下來
    '@typescript-eslint/no-explicit-any': 'off',
    'no-trailing-spaces': 'error',
  },
  ignorePatterns: ['dist/', 'wasm/', 'wasm-node/', 'node_modules/'],
}
