module.exports = {
  env: {
    browser: true,
    es2021: true,
    jest: true,
    webextensions: true
  },
  extends: [
    'airbnb-base',
    'airbnb-typescript/base',
    'plugin:typescript-sort-keys/recommended'
  ],
  ignorePatterns: [
    '*.js',
  ],
  parser: '@typescript-eslint/parser',
  parserOptions: {
    tsconfigRootDir: __dirname,
    project: ["tsconfig.json"],
    ecmaVersion: 'latest',
    sourceType: 'module'
  },
  plugins: [
    'sort-class-members',
    'typescript-sort-keys',
    'sort-keys-fix',
    'sort-destructure-keys',
    'react',
    '@typescript-eslint'
  ],
  rules: {
    "react/require-default-props": 0,
    "sort-destructure-keys/sort-destructure-keys": 2,
    "sort-keys-fix/sort-keys-fix": "warn",
    "sort-keys": ["error", "asc", { caseSensitive: true, minKeys: 2, natural: false }]
  }
}
