# cargo-binstall Configuration

This guide explains how to configure your crate to work with [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), allowing users to install pre-built binaries instead of compiling from source.

## Overview

cargo-binstall allows users to quickly install Rust binaries without compilation:

```bash
cargo binstall aptos-node
```

Instead of compiling (which can take a long time), it downloads pre-built binaries from GitHub releases.

## Requirements

For a crate to work with cargo-binstall:

1. The crate must be published to crates.io
2. GitHub releases must follow a specific naming convention
3. (Optional) Add binstall metadata to Cargo.toml for custom naming

## Binary Release Naming

Our binary release workflow creates archives with this format:

```
<binary-name>-v<version>-<target-triple>.zip
```

For example:
- `aptos-node-v1.2.3-x86_64-unknown-linux-gnu.zip`
- `aptos-node-v1.2.3-aarch64-apple-darwin.zip`

The releases are tagged as:
```
<binary-name>-v<version>
```

For example: `aptos-node-v1.2.3`

## Configuring Cargo.toml

To make cargo-binstall work with our naming convention, add this to your crate's `Cargo.toml`:

```toml
[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/{ bin }-v{ version }/{ bin }-v{ version }-{ target }.zip"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "zip"

[package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
pkg-url = "{ repo }/releases/download/{ bin }-v{ version }/{ bin }-v{ version }-{ target }.zip"
```

### Template Variables

- `{repo}`: Repository URL from Cargo.toml (e.g., https://github.com/aptos-labs/aptos-core)
- `{bin}`: Binary name (e.g., aptos-node)
- `{version}`: Version from Cargo.toml (e.g., 1.2.3)
- `{target}`: Rust target triple (e.g., x86_64-unknown-linux-gnu)
- `{binary-ext}`: `.exe` on Windows, empty otherwise

## Complete Example

Here's a complete example for a hypothetical `aptos-node` crate:

```toml
[package]
name = "aptos-node"
version = "1.2.3"
edition = "2021"
repository = "https://github.com/aptos-labs/aptos-core"

[package.metadata.binstall]
pkg-url = "{ repo }/releases/download/{ bin }-v{ version }/{ bin }-v{ version }-{ target }.zip"
bin-dir = "{ bin }{ binary-ext }"
pkg-fmt = "zip"

[package.metadata.binstall.overrides.x86_64-pc-windows-msvc]
pkg-url = "{ repo }/releases/download/{ bin }-v{ version }/{ bin }-v{ version }-{ target }.zip"
```

## Testing cargo-binstall

After publishing your crate and creating a GitHub release:

1. Install cargo-binstall:
   ```bash
   cargo install cargo-binstall
   ```

2. Test the installation:
   ```bash
   cargo binstall your-crate-name
   ```

3. Verify it downloads the binary instead of compiling:
   ```bash
   cargo binstall your-crate-name --log-level debug
   ```

## Troubleshooting

### "Could not find a matching binary"

This usually means:
- The release doesn't exist for your platform
- The naming convention doesn't match the configured pattern
- The repository field in Cargo.toml is missing or incorrect

**Solution**: Check that:
- The release exists on GitHub
- The URL pattern matches your actual release names
- `repository` field in Cargo.toml points to the correct repo

### "Checksum verification failed"

cargo-binstall will look for checksum files with these names:
- `<archive-name>.sha256`
- `<archive-name>.sha512`
- `SHA256SUMS`
- `SHA512SUMS`

Our workflow generates both individual `.sha256` files and a combined `SHA256SUMS` file.

## Default cargo-binstall Behavior

If you don't add metadata, cargo-binstall uses this default:

```
{repo}/releases/download/v{version}/{name}-{target}-v{version}.{archive-format}
```

Note the differences from our format:
- Release tag: `v{version}` vs our `{bin}-v{version}`
- Archive name: `{name}-{target}-v{version}` vs our `{bin}-v{version}-{target}`

That's why explicit configuration is needed.

## References

- [cargo-binstall GitHub](https://github.com/cargo-bins/cargo-binstall)
- [cargo-binstall Configuration Documentation](https://github.com/cargo-bins/cargo-binstall/blob/main/SUPPORT.md)
- [Template Variables Reference](https://github.com/cargo-bins/cargo-binstall/blob/main/SUPPORT.md#package-configuration)

## Example Workflow

1. **Develop your crate**
   ```bash
   cargo build
   cargo test
   ```

2. **Add binstall metadata to Cargo.toml**
   ```toml
   [package.metadata.binstall]
   pkg-url = "{ repo }/releases/download/{ bin }-v{ version }/{ bin }-v{ version }-{ target }.zip"
   bin-dir = "{ bin }{ binary-ext }"
   pkg-fmt = "zip"
   ```

3. **Publish to crates.io**
   ```bash
   cargo publish
   ```

4. **Create GitHub release using the binary release workflow**
   - Go to Actions → Binary Release
   - Fill in: binary_name, crate_name, build_profile, release_version
   - Uncheck "dry_run"
   - Run workflow

5. **Users can now install with cargo-binstall**
   ```bash
   cargo binstall your-crate-name
   ```
