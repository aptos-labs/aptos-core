// @ts-check

import eslint from '@eslint/js';
import { defineConfig } from 'eslint/config';
import tseslint from 'typescript-eslint';
import unusedImports from "eslint-plugin-unused-imports";

export default defineConfig({
  files: ["**/*.ts"],
  plugins: {
    unusedImports: unusedImports,
  },
  rules: {
    "no-unused-vars": "error",
    "unusedImports/no-unused-imports": "error"
  }
});
