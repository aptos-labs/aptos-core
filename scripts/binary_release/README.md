# Binary Release Workflow

This directory contains scripts and workflows for building and releasing any binary from the aptos-core repository.

## Overview

The binary release workflow allows you to build and release any executable target in the codebase for multiple platforms using standard Rust target triples. It includes:

- ✅ Multi-platform builds (Linux, macOS, Windows, ARM + x86)
- ✅ SHA256 checksums for all artifacts
- ✅ cargo-binstall support (for crates published to crates.io)
- ✅ Download scripts for easy installation
- ✅ Standard Rust target triple naming

## Quick Start

### Install a Released Binary

**Unix/Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- <binary-name>
```

**Windows (PowerShell):**
```powershell
iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 -OutFile install.ps1; .\install.ps1 -BinaryName <binary-name>
```

**Using cargo-binstall (if crate is on crates.io):**
```bash
cargo binstall <crate-name>
```

See [CARGO_BINSTALL.md](./CARGO_BINSTALL.md) for configuration details.

## Supported Platforms

The workflow builds binaries for the following platforms using standard Rust target triples:

- **Linux x86_64**: `x86_64-unknown-linux-gnu`
- **Linux ARM64**: `aarch64-unknown-linux-gnu`
- **macOS x86_64**: `x86_64-apple-darwin`
- **macOS ARM64**: `aarch64-apple-darwin`
- **Windows x86_64**: `x86_64-pc-windows-msvc`

## Build Profiles

Two build profiles are supported:

### `tool` Profile
- Optimized for binary size
- Uses `opt-level = "z"` (optimize for size)
- Includes LTO (Link Time Optimization) with "thin" mode
- Strips debug symbols
- Best for: Command-line tools, utilities, standalone binaries
- Similar to the CLI build profile

### `performance` Profile
- Optimized for runtime performance
- Uses `opt-level = 3` (maximum optimization)
- Includes LTO with "thin" mode
- Keeps debug info for profiling
- Best for: Long-running services, performance-critical applications (e.g., aptos-node)

## Release Artifacts

Each release includes:

### Binary Archives
Named using the format: `<binary-name>-v<version>-<target-triple>.zip`

Examples:
- `aptos-node-v1.2.3-x86_64-unknown-linux-gnu.zip`
- `aptos-node-v1.2.3-aarch64-apple-darwin.zip`
- `aptos-debugger-v1.0.0-x86_64-pc-windows-msvc.zip`

### Checksums
- Individual checksums: `<binary-name>-v<version>-<target-triple>.zip.sha256`
- Combined checksums: `SHA256SUMS` (contains all checksums in one file)

### Release Tags
Format: `<binary-name>-v<version>`

Examples:
- `aptos-node-v1.2.3`
- `aptos-debugger-v1.0.0`

## Creating a Release

### Via GitHub Actions

1. Go to the [Actions tab](../../actions) in the GitHub repository
2. Select "Binary Release" workflow
3. Click "Run workflow"
4. Fill in the parameters:
   - **binary_name**: Name of the output binary (e.g., `aptos-node`)
   - **crate_name**: Cargo package name to build (e.g., `aptos-node`)
   - **build_profile**: Choose `tool` or `performance`
   - **release_version**: Version number (e.g., `1.2.3`)
   - **source_git_ref_override** (optional): Git ref to build from
   - **release_title** (optional): Custom release title
   - **dry_run**: Check to test without creating a release
   - **skip_checks**: Skip version validation checks

5. Click "Run workflow"

### Local Build (Unix/Linux/macOS)

```bash
# From the root of aptos-core repository
./scripts/binary_release/build_binary_release.sh \
  <binary-name> \
  <crate-name> \
  <tool|performance> \
  <version> \
  [skip_checks]

# Example: Build aptos-node with performance profile
./scripts/binary_release/build_binary_release.sh \
  aptos-node \
  aptos-node \
  performance \
  1.2.3

# Example: Build a tool with the tool profile
./scripts/binary_release/build_binary_release.sh \
  aptos-debugger \
  aptos-debugger \
  tool \
  1.0.0
```

### Local Build (Windows)

```powershell
# From the root of aptos-core repository
.\scripts\binary_release\build_binary_release.ps1 `
  -BinaryName <binary-name> `
  -CrateName <crate-name> `
  -BuildProfile <tool|performance> `
  -Version <version> `
  [-SkipChecks $true]

# Example: Build aptos-node with performance profile
.\scripts\binary_release\build_binary_release.ps1 `
  -BinaryName "aptos-node" `
  -CrateName "aptos-node" `
  -BuildProfile "performance" `
  -Version "1.2.3"
```

## Installing Released Binaries

### Using the Install Script (Recommended)

**Unix/Linux/macOS:**
```bash
# Install latest version
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- aptos-node

# Install specific version
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- aptos-node --version 1.2.3

# Install to custom directory
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- aptos-node --bin-dir /usr/local/bin
```

**Windows (PowerShell):**
```powershell
# Install latest version
iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 -OutFile install.ps1
.\install.ps1 -BinaryName aptos-node

# Install specific version
.\install.ps1 -BinaryName aptos-node -Version 1.2.3

# Install to custom directory
.\install.ps1 -BinaryName aptos-node -BinDir "C:\Tools\bin"
```

### Using cargo-binstall

For crates published to crates.io:

```bash
# Install cargo-binstall if you haven't already
cargo install cargo-binstall

# Install the binary (downloads pre-built instead of compiling)
cargo binstall aptos-node
```

**Note**: Requires proper configuration in the crate's Cargo.toml. See [CARGO_BINSTALL.md](./CARGO_BINSTALL.md) for details.

### Manual Installation

1. Download the appropriate ZIP file for your platform from the [releases page](https://github.com/aptos-labs/aptos-core/releases)
2. Download the corresponding `.sha256` file
3. Verify the checksum:
   ```bash
   # Unix/Linux/macOS
   shasum -a 256 -c aptos-node-v1.2.3-x86_64-unknown-linux-gnu.zip.sha256

   # Windows (PowerShell)
   $expected = (Get-Content aptos-node-v1.2.3-x86_64-pc-windows-msvc.zip.sha256).Split()[0]
   $actual = (Get-FileHash aptos-node-v1.2.3-x86_64-pc-windows-msvc.zip -Algorithm SHA256).Hash.ToLower()
   if ($expected -eq $actual) { "OK" } else { "FAILED" }
   ```
4. Extract the ZIP file
5. Move the binary to a directory in your PATH

## Requirements

### For GitHub Actions
- The crate must exist in the aptos-core repository
- The crate must have a version field in its Cargo.toml
- The version in Cargo.toml must match the release_version (unless skip_checks is true)

### For Local Builds

**Unix/Linux/macOS:**
- Rust toolchain (matching rust-toolchain.toml)
- Standard build tools (gcc, clang, etc.)
- zip utility
- shasum or sha256sum (for checksums)

**Windows:**
- Rust toolchain (matching rust-toolchain.toml)
- Visual Studio Build Tools
- vcpkg (for OpenSSL)
- PowerShell 5.0 or later

### For cargo-binstall
- Crate must be published to crates.io
- Proper metadata configuration in Cargo.toml (see [CARGO_BINSTALL.md](./CARGO_BINSTALL.md))

## Crate Location Detection

The build scripts automatically search for the crate's Cargo.toml in the following locations:
1. `crates/<crate-name>/Cargo.toml`
2. `<crate-name>/Cargo.toml`
3. `aptos-move/<crate-name>/Cargo.toml`

## Examples

### Release aptos-node for production
```yaml
binary_name: aptos-node
crate_name: aptos-node
build_profile: performance
release_version: 1.2.3
dry_run: false
```

This creates:
- 5 platform-specific ZIP files
- 5 individual `.sha256` checksum files
- 1 combined `SHA256SUMS` file
- Release tagged as `aptos-node-v1.2.3`

### Release a debugging tool
```yaml
binary_name: aptos-debugger
crate_name: aptos-debugger
build_profile: tool
release_version: 1.0.0
dry_run: false
```

### Test build without releasing
```yaml
binary_name: aptos-node
crate_name: aptos-node
build_profile: performance
release_version: 1.2.3
dry_run: true
```

Artifacts are built and uploaded but no GitHub release is created.

## Verifying Downloads

Always verify checksums after downloading:

```bash
# Using individual checksum file
shasum -a 256 -c aptos-node-v1.2.3-x86_64-unknown-linux-gnu.zip.sha256

# Using combined SHA256SUMS file
shasum -a 256 -c SHA256SUMS
```

Our install scripts verify checksums automatically.

## Troubleshooting

### Version mismatch error
If you get a version mismatch error, either:
1. Update the version in the crate's Cargo.toml to match your desired release version
2. Use `skip_checks: true` to bypass version validation (not recommended for production)

### Binary not found
Ensure that:
1. The crate name is correct (check `cargo metadata` or the crate's Cargo.toml)
2. The crate produces a binary target (not just a library)
3. The binary target name matches the binary_name parameter (or the crate name)

### Build profile not found
If you get a profile error:
1. Verify the profile exists in the root Cargo.toml
2. Currently supported profiles: `tool`, `performance`

### cargo-binstall not finding the binary
See [CARGO_BINSTALL.md](./CARGO_BINSTALL.md) for configuration and troubleshooting.

### Checksum verification fails
1. Re-download the archive and checksum file
2. Ensure files weren't corrupted during download
3. Check that you're using the correct checksum file for the archive

## Files in This Directory

- `build_binary_release.sh` - Unix/Linux/macOS build script
- `build_binary_release.ps1` - Windows build script
- `install_binary.sh` - Unix/Linux/macOS installation script
- `install_binary.ps1` - Windows installation script
- `README.md` - This file
- `CARGO_BINSTALL.md` - cargo-binstall configuration guide

## Related Documentation

- [Install the Aptos CLI on Windows](https://aptos.dev/build/cli/install-cli/install-cli-windows)
- [Install the Aptos CLI on Linux](https://aptos.dev/build/cli/install-cli/install-cli-linux)
- [Install the Aptos CLI on Mac](https://aptos.dev/build/cli/install-cli/install-cli-mac)
- [cargo-binstall GitHub](https://github.com/cargo-bins/cargo-binstall)
