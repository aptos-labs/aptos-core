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

## Release tags

Installed CLIs compare their version against the newest `aptos-cli-vX.Y.Z` tag that has a published GitHub release (not a pre-release, with assets), and tell users to upgrade within 3 days of it appearing. Homebrew installs compare against the Homebrew formula instead, so they are only told once the homebrew-core bump PR from the release workflow merges.

- Only the "Release CLI" workflow should create `aptos-cli-v*` tags. The lookup tries just the 3 newest stable tags, so stray tags above the real release slow it down, and three of them break both the notice and `aptos update aptos`.
- The workflow builds and tags one commit (`source_git_ref_override` if given, otherwise the dispatched commit), so `crates/aptos/Cargo.toml` at that commit must equal the release version (its `preflight` job enforces this). Binaries whose version differs from their tag are told to upgrade on every check.
- The release job publishes only after every binary is uploaded, so a failed run leaves at most a draft release and no tag. Delete that draft before re-running.
- The release job never replaces an existing release. To re-release a version, first run `gh release delete aptos-cli-vX.Y.Z --cleanup-tag`.

## Verification

After edits, run:

```bash
cargo check -p aptos
```

## Common mistakes

- Bumping `Cargo.toml` without adding a matching `## [version]` block (or leaving released notes only under `Unreleased`).
- Forgetting that the CLI version lives only in `crates/aptos/Cargo.toml` for this workflow (not the workspace root `Cargo.toml`).
