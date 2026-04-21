---
name: aptos-cli-release
description: Use when cutting or preparing a new Aptos CLI release, bumping the `aptos` crate version, or editing `crates/aptos/CHANGELOG.md` for a release
---

# Aptos CLI release (crate `aptos`)

## Files

| File | Change |
|------|--------|
| `crates/aptos/Cargo.toml` | Set `[package] version = ...` to the new semver. |
| `crates/aptos/CHANGELOG.md` | Follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and repo style: `# Unreleased` at top, then `## [X.Y.Z]` sections newest first. |

## Versioning

- **Patch** (9.1.0 → 9.1.1): bugfixes only.
- **Minor** (9.1.0 → 9.2.0): new features or non-breaking additions.
- **Major** (9.x → 10.0.0): breaking CLI or documented compatibility breaks.

Match the requested bump type to the version field and to how you group notes under the new `## [version]` heading.

## Changelog workflow

1. Under `# Unreleased`, collect bullet notes for changes since the last tagged CLI release (or move existing unreleased bullets).
2. When releasing, add `## [<new version>]` immediately below `# Unreleased` and move the bullets for this release under it (newest release section stays directly under `Unreleased`).
3. If nothing is pending after a release, keep one placeholder bullet under `# Unreleased` (for example `- _No changes yet._`) so the section is clearly intentional, not an oversight.

## Verification

After edits, run:

```bash
cargo check -p aptos
```

## Common mistakes

- Bumping `Cargo.toml` without adding a matching `## [version]` block (or leaving released notes only under `Unreleased`).
- Forgetting that the CLI version lives only in `crates/aptos/Cargo.toml` for this workflow (not the workspace root `Cargo.toml`).
