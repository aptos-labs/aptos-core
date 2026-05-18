# Binary Release Quick Start

## For Users: Installing a Released Binary

### One-Line Install

**Unix/Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.sh | sh -s -- <binary-name>
```

**Windows (PowerShell):**
```powershell
iwr https://raw.githubusercontent.com/aptos-labs/aptos-core/main/scripts/binary_release/install_binary.ps1 -OutFile install.ps1; .\install.ps1 -BinaryName <binary-name>
```

**Using cargo-binstall:**
```bash
cargo binstall <crate-name>
```

## For Maintainers: Creating a Release

### Step 1: Prepare Your Crate

Ensure your crate's `Cargo.toml` has the correct version:

```toml
[package]
name = "my-tool"
version = "1.2.3"  # This should match your release version
```

### Step 2: Run the GitHub Actions Workflow

1. Go to [Actions → Binary Release](../../actions/workflows/binary-release.yaml)
2. Click "Run workflow"
3. Fill in:
   - **binary_name**: `my-tool`
   - **crate_name**: `my-tool`
   - **build_profile**: `tool` or `performance`
   - **release_version**: `1.2.3`
   - **dry_run**: Uncheck for real release

### Step 3: (Optional) Enable cargo-binstall

If your crate is on crates.io, add to `Cargo.toml`:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/{ bin }-v{ version }/{ bin }-v{ version }-{ target }.zip"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "zip"
```

See [CARGO_BINSTALL.md](./CARGO_BINSTALL.md) for details.

## What Gets Created

For a release of `my-tool` version `1.2.3`:

### Release Tag
```
my-tool-v1.2.3
```

### Artifacts (5 platforms)
```
my-tool-v1.2.3-x86_64-unknown-linux-gnu.zip
my-tool-v1.2.3-aarch64-unknown-linux-gnu.zip
my-tool-v1.2.3-x86_64-apple-darwin.zip
my-tool-v1.2.3-aarch64-apple-darwin.zip
my-tool-v1.2.3-x86_64-pc-windows-msvc.zip
```

### Checksums
```
my-tool-v1.2.3-x86_64-unknown-linux-gnu.zip.sha256
my-tool-v1.2.3-aarch64-unknown-linux-gnu.zip.sha256
my-tool-v1.2.3-x86_64-apple-darwin.zip.sha256
my-tool-v1.2.3-aarch64-apple-darwin.zip.sha256
my-tool-v1.2.3-x86_64-pc-windows-msvc.zip.sha256
SHA256SUMS (combined)
```

## Build Profiles

### `tool` - For CLI tools and utilities
- Small binary size
- Fast startup
- Good for distribution

### `performance` - For services and daemons
- Maximum runtime speed
- Optimized for long-running processes
- Best for aptos-node, indexers, etc.

## Examples

### Release a CLI tool
```yaml
binary_name: aptos-debugger
crate_name: aptos-debugger
build_profile: tool
release_version: 1.0.0
dry_run: false
```

### Release a performance-critical service
```yaml
binary_name: aptos-node
crate_name: aptos-node
build_profile: performance
release_version: 1.2.3
dry_run: false
```

## Testing Before Release

Set `dry_run: true` to build and test without creating a release:

```yaml
binary_name: my-tool
crate_name: my-tool
build_profile: tool
release_version: 1.2.3
dry_run: true  # ← Test mode
```

This builds all artifacts but doesn't create a GitHub release.

## More Information

- Full documentation: [README.md](./README.md)
- cargo-binstall setup: [CARGO_BINSTALL.md](./CARGO_BINSTALL.md)
- Build scripts: `build_binary_release.sh`, `build_binary_release.ps1`
- Install scripts: `install_binary.sh`, `install_binary.ps1`
