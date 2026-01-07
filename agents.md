# Agents Guide for Aptos Core

This document provides guidance for AI coding agents working with the Aptos Core codebase.

## Project Overview

Aptos Core is a Layer 1 blockchain implementation built primarily in Rust, with smart contracts written in Move. The codebase is large and modular, consisting of:

- **Core blockchain infrastructure** (consensus, execution, storage, networking)
- **Move virtual machine and compiler** (under `third_party/move/` and `aptos-move/`)
- **Developer tooling** (CLI, SDK, indexers)
- **Testing infrastructure** (unit tests, integration tests, fuzzing, forge test framework)

## Repository Structure

Key directories:

| Directory | Purpose |
|-----------|---------|
| `aptos-node/` | Main node binary entry point |
| `consensus/` | Consensus protocol implementation |
| `execution/` | Transaction execution engine |
| `storage/` | Persistent storage layer (RocksDB-based) |
| `mempool/` | Transaction mempool |
| `network/` | P2P networking layer |
| `state-sync/` | State synchronization between nodes |
| `aptos-move/` | Aptos-specific Move framework and VM integration |
| `third_party/move/` | Move language compiler, VM, and tooling |
| `types/` | Core type definitions |
| `crates/` | Shared utility crates (crypto, logging, etc.) |
| `api/` | REST API implementation |
| `testsuite/` | Integration tests and forge test framework |
| `ecosystem/` | Indexer and related ecosystem tooling |

## Build System

### Cargo Workspace

The project uses a Cargo workspace with ~200+ crates. Key commands:

```bash
# Build the main node
cargo build -p aptos-node

# Build the CLI
cargo build -p aptos

# Build all default members (production binaries)
cargo build

# Run tests for a specific crate
cargo test -p <crate-name>
```

### Custom Tooling

The project provides custom cargo extensions via `cargo x`:

```bash
# Run lints (rustfmt + clippy with project config)
./scripts/rust_lint.sh

# Or use cargo x commands
cargo x lint
cargo x test
cargo x clippy
```

## Coding Standards

### Rust Guidelines

Follow the project's coding guidelines documented in:
- `RUST_CODING_STYLE.md` - General Rust coding conventions
- `RUST_SECURE_CODING.md` - Security-focused coding practices

Key points:
- Use `checked_*`, `saturating_*`, or `overflowing_*` for integer arithmetic
- Prefer `Result<T, E>` over `unwrap()` in production code
- Use `expect()` with descriptive messages when panicking is acceptable
- Document public APIs with Rustdoc
- Use inclusive terminology (allowlist/blocklist, primary/secondary)

### Error Handling

```rust
// Prefer this:
fn do_something() -> Result<Value, Error> {
    let result = operation()?;
    Ok(result)
}

// Over this:
fn do_something() -> Value {
    operation().unwrap()  // Avoid in production code
}
```

### Logging Levels

- `error!` - Unexpected errors requiring attention
- `warn!` - Automatically handled issues
- `info!` - One-time or infrequent events
- `debug!` - Frequent events (>1/sec), disabled in production
- `trace!` - Function entry/exit only

## Testing

### Unit Tests

Unit tests are colocated with source code:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_something() {
        // ...
    }
}
```

### Running Tests

```bash
# Run all tests in a crate
cargo test -p <crate-name>

# Run a specific test
cargo test -p <crate-name> -- <test_name>

# Run with output
cargo test -p <crate-name> -- --nocapture
```

### Test Features

For test-only code in production crates:

```rust
#[cfg(any(test, feature = "fuzzing"))]
fn test_helper() { ... }
```

## Move Language

### Move Code Location

- `aptos-move/framework/` - Aptos framework modules (stdlib, account, coin, etc.)
- `third_party/move/` - Move compiler and VM implementation

### Move Testing

```bash
# Run Move unit tests
cargo test -p aptos-framework

# Run Move prover
cargo test -p move-prover
```

## Common Tasks

### Adding a New Dependency

1. Add to `[workspace.dependencies]` in root `Cargo.toml` with version
2. Reference in individual crate's `Cargo.toml` without version

### Modifying Existing Code

1. Find the relevant crate using directory structure or search
2. Make changes following existing patterns
3. Run tests: `cargo test -p <crate-name>`
4. Run lints: `./scripts/rust_lint.sh`

### Creating New Crates

1. Create directory under appropriate location
2. Add to `[workspace.members]` in root `Cargo.toml`
3. Follow existing crate structure patterns

## CI/CD

The project uses GitHub Actions for CI. Key workflows:
- `lint-test.yaml` - Main lint and test workflow
- `forge-*.yaml` - End-to-end testing with forge framework

## Tips for Agents

1. **Explore before editing**: Use the codebase search to understand patterns before making changes
2. **Follow existing conventions**: Match the style of surrounding code
3. **Check dependencies**: Use workspace dependencies from root `Cargo.toml`
4. **Run targeted tests**: Test only affected crates to save time
5. **Lint before committing**: Run `./scripts/rust_lint.sh` to catch issues early
6. **Read module docs**: Many modules have inline documentation explaining their purpose

## Useful Commands Reference

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy -p <crate-name>

# Check compilation without building
cargo check -p <crate-name>

# Generate documentation
cargo doc -p <crate-name> --open

# Search for text in codebase
rg "pattern" --type rust

# Find files
fd "filename"
```

## Getting Help

- Check existing code for patterns and examples
- Read inline documentation and comments
- Refer to `README.md` files in major directories
- Review related test files for usage examples
