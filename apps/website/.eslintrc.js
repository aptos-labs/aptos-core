module.exports = {
  env: {
    browser: true,
    es2021: true,
    jest: true,
    webextensions: true,
  },
  extends: [
    '@aptos-wallet/eslint-config',
    'next/core-web-vitals'
  ],
  rules: {
    "react/function-component-definition": 0,
  },
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaFeatures: {
      jsx: true
    },
    tsconfigRootDir: __dirname,
    project: ["tsconfig.json"],
    ecmaVersion: 'latest',
    sourceType: 'module'
  },
}
