// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    coverage: {
      provider: "istanbul",
      reporter: ["lcov"],
      include: ["src/**/*.ts"],
      thresholds: {
        branches: 0,
        functions: 0,
        lines: 0,
        statements: 0,
      },
    },
  },
});
