---
name: script-composer-npm-release
description: Use when building and publishing a new version of @aptos-labs/aptos-dynamic-transaction-composer to npm. Covers wasm-pack build, version bump, dry-run, and publish.
---

# Script Composer npm release

Package: `@aptos-labs/aptos-dynamic-transaction-composer`
Source: `aptos-move/script-composer/`
Registry: https://www.npmjs.com/package/@aptos-labs/aptos-dynamic-transaction-composer

## When to publish

**In most cases, only publish from the `main` branch (mainnet).** The wasm package encodes the Move bytecode format — when the on-chain bytecode version changes (new VM version shipped to mainnet), a new npm package must be released to support it. Publishing from a feature branch or devnet/testnet branch may produce a package incompatible with mainnet.

Exception: use a pre-release tag (see below) when testing unreleased VM changes from a non-main branch.

## Files

| File | Change |
|------|--------|
| `aptos-move/script-composer/pkg/package.json` | Bump `version` field — this is what npm publishes |

Note: `Cargo.toml` version (`0.1.4`) is **separate** and does not need to match the npm version. wasm-pack regenerates `pkg/package.json` from Cargo.toml on every build, so the version will be reset — always bump it **after** building.

## Versioning

### Step 0: Check what's already published
```bash
# See latest stable version
npm view @aptos-labs/aptos-dynamic-transaction-composer dist-tags.latest

# See ALL dist-tags (latest, beta, rc, etc.)
npm view @aptos-labs/aptos-dynamic-transaction-composer dist-tags
```

Follow semver:
- **Patch** (0.1.6 → 0.1.7): bug fixes, no API change
- **Minor** (0.1.7 → 0.2.0): new wasm exports or API additions
- **Pre-release** (0.1.7-beta.1): testing / unreleased VM changes — see Pre-release section

## Stable release workflow

All commands run from the **repo root** unless noted.

### 1. Sync to latest main
```bash
git fetch origin && git checkout main && git pull origin main
```

### 2. Build wasm
```bash
wasm-pack build --target web --scope aptos-labs --release aptos-move/script-composer
```
- `--target web`: ESM output with explicit `init()` / `initSync()` exports (matches existing package format)
- `--scope aptos-labs`: sets npm package name to `@aptos-labs/aptos-dynamic-transaction-composer`
- `--release`: optimized wasm binary
- Output lands in `aptos-move/script-composer/pkg/` (overwrites existing files including `package.json`)

### 3. Bump version in pkg/package.json
Edit `aptos-move/script-composer/pkg/package.json` — set `"version"` to the new semver.

### 4. Dry-run (simulate, no upload)
```bash
cd aptos-move/script-composer/pkg && npm publish --dry-run
```
Verify output: `.wasm`, `.js`, `.d.ts` all listed; version is correct.

### 5. Publish (user runs this step)
```bash
cd aptos-move/script-composer/pkg && npm publish --access public
```

### 6. Verify
```bash
npm view @aptos-labs/aptos-dynamic-transaction-composer dist-tags.latest
```
Should return the new version.

## Pre-release publishing (beta / rc / next)

Use this when testing changes that aren't ready for `latest`, or when publishing from a non-main branch (e.g., a new VM bytecode version being tested on devnet/testnet).

### Pre-release version formats
- `0.1.7-beta.1`, `0.1.7-beta.2` — beta iterations
- `0.1.7-rc.1` — release candidate
- `0.1.7-next.0` — for testing unreleased VM/bytecode changes

### Pre-release workflow

Same build steps (1–3 above), but with a pre-release version string. Then:

```bash
# Dry-run first
cd aptos-move/script-composer/pkg && npm publish --dry-run --tag beta

# Publish with a dist-tag — does NOT touch `latest`
cd aptos-move/script-composer/pkg && npm publish --access public --tag beta
# Other tags: --tag rc  /  --tag next
```

### Promote a pre-release to `latest` when ready
```bash
npm dist-tag add @aptos-labs/aptos-dynamic-transaction-composer@0.1.7-beta.1 latest
```

### View / remove dist-tags
```bash
# List all tags
npm view @aptos-labs/aptos-dynamic-transaction-composer dist-tags

# Remove a tag
npm dist-tag rm @aptos-labs/aptos-dynamic-transaction-composer beta
```

## Common mistakes

- **Publishing from wrong branch**: always publish stable from `main`; use a pre-release tag if publishing from another branch
- **Wrong target**: always `--target web` — `bundler` or `nodejs` produce a different module format
- **Missing `--scope aptos-labs`**: wasm-pack would generate `name: "aptos-dynamic-transaction-composer"` (no `@aptos-labs/` prefix)
- **Forgetting to bump version after build**: wasm-pack resets `pkg/package.json` to the Cargo.toml version on every build
- **Pre-release without `--tag`**: omitting `--tag` on a pre-release version (e.g. `0.1.7-beta.1`) still updates `latest` and breaks users on stable
- **Wrong working directory for publish**: must `cd aptos-move/script-composer/pkg`, not the crate root
