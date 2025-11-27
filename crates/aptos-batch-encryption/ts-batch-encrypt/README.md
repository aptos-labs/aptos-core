<h1 align="center" style="text-align: center; width: fit-content; margin-left: auto; margin-right: auto;">ts-base</h1>

<p align="center">
  <a href="https://github.com/bgub/ts-base/actions">CI</a>
  ·
  <a href="https://github.com/bgub/ts-base/releases">Releases</a>
  ·
  <a href="https://github.com/bgub/ts-base/issues">Issues</a>
</p>

<span align="center">

[![npm](https://img.shields.io/npm/v/%40bgub%2Fts-base?logo=npm&label=npm)](https://www.npmjs.com/package/@bgub/ts-base)
[![CI](https://github.com/bgub/ts-base/actions/workflows/ci.yml/badge.svg)](https://github.com/bgub/ts-base/actions)
[![Codecov](https://codecov.io/github/bgub/ts-base/branch/main/graph/badge.svg)](https://codecov.io/github/bgub/ts-base)
[![Sponsor](https://img.shields.io/badge/sponsor-%E2%9D%A4-ff69b4)](https://github.com/sponsors/bgub)

</span>

TypeScript library starter that works out-of-the-box with Node, Deno, Bun, and the browser. Batteries included: linting, testing, bundling, size-limit, and automated releases.

## Features

- **Biome**: lint and format with a single tool
- **Vitest**: fast tests with coverage and thresholds
- **Size Limit**: keep bundles tiny, with CI checks
- **tsdown**: ESM builds for Node and a separate browser bundle
- **CI**: lint, typecheck, test, coverage, and size comments/badges
- **Release Please**: automated release PRs and changelogs
- **Commit Linting**: conventional commits enforced in CI
- **Deno-friendly**: `.ts` source imports for direct consumption
- **Multi-runtime**: `src/internal.ts` is runtime-agnostic; `src/index.ts` (Node) and `src/browser.ts` (browser) wire runtime-specific APIs
- **OIDC + Provenance**: publish to npm and JSR via manual CI release

## Usage

Install dependencies and run scripts:

```bash
pnpm i
pnpm lint
pnpm test
pnpm build
```

Node usage:

```ts
import { add, greet, getSecureRandomId } from "@bgub/ts-base";

console.log(add(2, 3));
console.log(greet("Ada"));
console.log(getSecureRandomId());
```

Browser usage (bundled or via import maps):

```ts
import { add, greet, getSecureRandomId } from "@bgub/ts-base/browser";

add(1, 2);
greet("Linus");
getSecureRandomId();
```

Deno usage (import from `src` if desired):

```ts
import { add, greet } from "https://jsr.io/@bgub/ts-base/<version>/src/index.ts";
```

## Project Structure

- `src/internal.ts`: core logic, no Node/browser APIs
- `src/index.ts`: Node adapter (e.g., `crypto.randomBytes`)
- `src/browser.ts`: browser adapter (e.g., `crypto.getRandomValues`)
- `tsdown.config.ts`: builds Node entry and browser `core` bundle
- `vitest.config.ts`: coverage config and thresholds

## Releasing

- Merge the automated Release PR created by Release Please
- Manually run the "Release" workflow to publish to npm and JSR with provenance

## License

MIT © [bgub](https://github.com/bgub)
