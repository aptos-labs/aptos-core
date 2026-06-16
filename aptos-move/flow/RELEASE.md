# move-flow Release Guide

This document describes how to prepare a move-flow source tag in `aptos-core`.
Published binaries and plugin marketplace updates are owned by
[`aptos-ai`](https://github.com/aptos-labs/aptos-ai).

## For Users: Installing move-flow

### Quick Install

**Unix/Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-ai/main/install.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://raw.githubusercontent.com/aptos-labs/aptos-ai/main/install.ps1 | iex
```

### Build from source

```bash
cargo install --git https://github.com/aptos-labs/aptos-core --locked aptos-move-flow
```

### Install Specific Version

**Unix/Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-ai/main/install.sh | sh -s -- --version 1.0.4
```

**Windows:**
```powershell
irm https://raw.githubusercontent.com/aptos-labs/aptos-ai/main/install.ps1 -OutFile install.ps1
.\install.ps1 -Version 1.0.4
```

### Manual Installation

1. Go to the [`aptos-ai` releases page](https://github.com/aptos-labs/aptos-ai/releases)
2. Download the appropriate file for your platform:
   - Linux x86_64: `move-flow-v1.0.4-x86_64-unknown-linux-gnu.zip`
   - Linux ARM64: `move-flow-v1.0.4-aarch64-unknown-linux-gnu.zip`
   - macOS x86_64: `move-flow-v1.0.4-x86_64-apple-darwin.zip`
   - macOS ARM64: `move-flow-v1.0.4-aarch64-apple-darwin.zip`
   - Windows x86_64: `move-flow-v1.0.4-x86_64-pc-windows-msvc.zip`
3. Download the corresponding `.sha256` checksum file
4. Verify the checksum:
   ```bash
   # Unix/Linux/macOS
   shasum -a 256 -c move-flow-v1.0.4-x86_64-unknown-linux-gnu.zip.sha256

   # Windows (PowerShell)
   $expected = (Get-Content move-flow-v1.0.4-x86_64-pc-windows-msvc.zip.sha256).Split()[0]
   $actual = (Get-FileHash move-flow-v1.0.4-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLower()
   if ($expected -eq $actual) { "OK" } else { "FAILED" }
   ```
5. Extract and move the binary to a directory in your PATH

## For Maintainers: Creating a Release

### Prerequisites

1. Update the version in `aptos-move/flow/Cargo.toml`:
   ```toml
   [package]
   name = "aptos-move-flow"
   version = "1.0.4"  # Update this
   ```

2. Update `aptos-move/flow/CHANGELOG.md`:
   - Promote the items currently under `## [Unreleased]` into a new
     `## [<new-version>] - YYYY-MM-DD` heading (use today's date).
   - Leave an empty `## [Unreleased]` section at the top for future work.
   - Update the link references at the bottom of the file so
     `[Unreleased]` compares against the new tag and a fresh
     `[<new-version>]` link points at the new release.

   A CHANGELOG entry is required for every version bump; the release
   workflow assumes the changelog reflects the version being shipped.

3. Commit the version and changelog changes together:
   ```bash
   git add aptos-move/flow/Cargo.toml aptos-move/flow/CHANGELOG.md
   git commit -m "[move-flow] Bump version to 1.0.4"
   git push
   ```

### Source Tag Process

1. Go to [Actions → Release move-flow](../../.github/workflows/move-flow-release.yaml)
2. Click "Run workflow"
3. Fill in the parameters (the release version is read from
   `aptos-move/flow/Cargo.toml`):
   - **source_git_ref_override**: (optional) specific branch/commit to build from
   - **dry_run**: Uncheck this to create the `move-flow-v<version>` source tag

4. Click "Run workflow"

Real releases must be dispatched from the `main` workflow definition. If
`source_git_ref_override` is set for a real release, the resolved commit must be
reachable from `main` or an `aptos-release-v*` branch.

After the source tag is created, run the `aptos-ai` move-flow release workflow
with the matching version. That workflow consumes the `aptos-core` tag, builds
the binaries, publishes the `aptos-ai` GitHub Release, and opens/updates the
plugin tree PR in `aptos-ai`.

### What Gets Created Here

For version `1.0.4`, the workflow creates:

**Release Tag:**
```
move-flow-v1.0.4
```

### Testing Before Release

Always test with `dry_run: true` first:

1. Set `dry_run: true`
2. Run the workflow
3. Confirm version resolution, source reachability, and tag-collision checks are correct
4. Once confirmed, run again with `dry_run: false`

## Build Profile

The `aptos-ai` release pipeline builds move-flow with the `cli` profile:
- Optimized for binary size (`opt-level = "z"`)
- Link-time optimization enabled
- Debug symbols stripped
- Fast startup time
- Ideal for command-line tools

## Troubleshooting

### Tag already exists
Check the existing `aptos-core` tags. If the tag `move-flow-v1.0.4` already exists, you need to either:
- Bump the version in `aptos-move/flow/Cargo.toml`
- Delete the existing tag (requires admin permissions)

The source-tag workflow performs an automatic tag-collision check before
creating the tag: if `move-flow-v<version>` already exists, the workflow
fails fast with a clear message instead of creating a duplicate.

### Binary not found after installation
Ensure the installation directory is in your PATH:
```bash
# Unix/Linux/macOS (add to ~/.bashrc or ~/.zshrc)
export PATH="$HOME/.local/bin:$PATH"

# Windows (the installer should handle this, but if not)
# Add the installation directory to your system PATH environment variable
```

### Checksum verification fails
Re-download both the archive and checksum file. If the issue persists, report it as an issue.

## Related Documentation

- [Binary Release Workflow](../../scripts/binary_release/README.md)
- [`aptos-ai` installer documentation](https://github.com/aptos-labs/aptos-ai#readme)

## Support

For issues with move-flow itself, see the main [move-flow README](README.md).

For issues with the release process or installation, create an issue on GitHub.
