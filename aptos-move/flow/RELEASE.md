# move-flow Release Guide

This document describes how to release and install move-flow binaries.

## For Users: Installing move-flow

### Quick Install

**Unix/Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- move-flow
```

**Windows (PowerShell):**
```powershell
iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 -OutFile install.ps1
.\install.ps1 -BinaryName move-flow
```

### Build from source

```bash
cargo install --git https://github.com/aptos-labs/aptos-core --locked aptos-move-flow
```

### Install Specific Version

**Unix/Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- move-flow --version 1.0.4
```

**Windows:**
```powershell
.\install.ps1 -BinaryName move-flow -Version 1.0.4
```

### Manual Installation

1. Go to the [releases page](https://github.com/aptos-labs/aptos-core/releases)
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

### Release Process

1. Go to [Actions → Release move-flow](../../.github/workflows/move-flow-release.yaml)
2. Click "Run workflow"
3. Fill in the parameters:
   - **release_version**: `1.0.4` (must match Cargo.toml version)
   - **source_git_ref_override**: (optional) specific branch/commit to build from
   - **release_title**: (optional) custom title, defaults to "move-flow v1.0.4"
   - **dry_run**: Uncheck this for a real release, keep checked for testing
   - **skip_checks**: Only check if you want to bypass version validation

4. Click "Run workflow"

### What Gets Created

For version `1.0.4`, the workflow creates:

**Release Tag:**
```
move-flow-v1.0.4
```

**Artifacts (5 platforms):**
```
move-flow-v1.0.4-x86_64-unknown-linux-gnu.zip
move-flow-v1.0.4-aarch64-unknown-linux-gnu.zip
move-flow-v1.0.4-x86_64-apple-darwin.zip
move-flow-v1.0.4-aarch64-apple-darwin.zip
move-flow-v1.0.4-x86_64-pc-windows-msvc.zip
```

**Checksums:**
```
move-flow-v1.0.4-x86_64-unknown-linux-gnu.zip.sha256
move-flow-v1.0.4-aarch64-unknown-linux-gnu.zip.sha256
move-flow-v1.0.4-x86_64-apple-darwin.zip.sha256
move-flow-v1.0.4-aarch64-apple-darwin.zip.sha256
move-flow-v1.0.4-x86_64-pc-windows-msvc.zip.sha256
SHA256SUMS (combined checksums file)
```

### Testing Before Release

Always test with `dry_run: true` first:

1. Set `dry_run: true`
2. Run the workflow
3. Check that all builds succeed
4. Verify artifacts are created (they won't be published)
5. Once confirmed, run again with `dry_run: false`

### Required Secrets

`APTOS_BOT_GH_PAT_APTOS_AI_PLUGIN_PUBLISHER` — purpose-scoped PAT with
write access to `aptos-labs/aptos-ai` only. Used by the `publish-plugin`
job to push the plugin branch and open/refresh the PR. The job skips
cleanly (emitting a `::notice::`) when the secret is absent, so a missing
PAT does not block the release.

## Build Profile

move-flow uses the `tool` build profile:
- Optimized for binary size (`opt-level = "z"`)
- Link-time optimization enabled
- Debug symbols stripped
- Fast startup time
- Ideal for command-line tools

## Troubleshooting

### Version mismatch error
Update the version in `aptos-move/flow/Cargo.toml` to match your release version, or use `skip_checks: true` (not recommended).

### Release already exists
Check the [releases page](https://github.com/aptos-labs/aptos-core/releases). If the tag `move-flow-v1.0.4` already exists, you need to either:
- Use a different version number
- Delete the existing release (requires admin permissions)

The release workflow's pre-flight job now performs an automatic tag-collision
check before any build runs: if `move-flow-v<release_version>` already exists
as a tag or GitHub release, the workflow fails fast with a clear message
instead of failing later at the publish step.

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
- [cargo-binstall Configuration](../../scripts/binary_release/CARGO_BINSTALL.md)
- [Quick Start Guide](../../scripts/binary_release/QUICKSTART.md)

## Support

For issues with move-flow itself, see the main [move-flow README](README.md).

For issues with the release process or installation, create an issue on GitHub.
