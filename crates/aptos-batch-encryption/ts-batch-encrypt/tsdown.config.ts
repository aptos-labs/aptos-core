import { defineConfig } from "tsdown";

export default defineConfig([
  {
    entry: ["./src/index.ts"],
    platform: "node",
    dts: true,
    sourcemap: true,
  },
  {
    entry: ["./src/browser.ts"],
    platform: "browser",
    dts: true,
    minify: true,
    sourcemap: true,
  },
]);
