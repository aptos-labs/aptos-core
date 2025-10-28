# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Repository Overview

This is Aptos Core, a layer 1 blockchain platform built primarily in Rust with the Move programming language. The repository is organized as a large Cargo workspace with over 200 member crates, plus supporting Python, TypeScript, and infrastructure code.

### Key Architecture Components

- **aptos-move/** - Move language runtime, virtual machine, framework, and examples
- **consensus/** - Consensus protocol implementation (safety rules, consensus types)  
- **execution/** - Transaction execution engine and block processing
- **mempool/** - Transaction pool management
- **network/** - P2P networking layer and discovery
- **storage/** - Persistent storage (AptosDB, backup/restore, indexing)
- **state-sync/** - State synchronization between nodes
- **api/** - REST API server and types
- **crates/** - Core utilities (crypto, logging, telemetry, CLI tools)
- **testsuite/** - Integration testing framework (Forge) and test cases

The codebase follows a modular design where each major component is isolated with well-defined interfaces. The Move virtual machine (`aptos-move/aptos-vm/`) executes smart contracts written in Move, while the broader system handles consensus, networking, and storage.

## Build and Development Commands

### Primary Build Commands
```bash
# Build entire workspace (development)
cargo build

# Build with release optimizations
cargo build --release

# Build specific package
cargo build -p <package-name>

# Build with cached packages for Move framework
./scripts/cargo_build_aptos_cached_packages.sh
```

### Linting and Formatting
```bash
# Run complete linting suite (clippy, rustfmt, cargo-sort, machete)
./scripts/rust_lint.sh

# Check formatting without modifying files
./scripts/rust_lint.sh --check

# Format code manually
cargo +nightly fmt
```

### Testing Commands
```bash
# Run all Rust unit tests  
cargo test

# Run tests for specific package
cargo test -p <package-name>

# Run single test
cargo test <test-name>

# Run tests with output
cargo test -- --nocapture

# Run integration tests via Forge
cd testsuite && python3 forge.py
```

### Move-Specific Commands
```bash
# Test Move framework
cd aptos-move/framework && cargo test

# Run Move examples
cd aptos-move/move-examples && cargo test

# Build Move packages
cd aptos-move/move-examples/<example> && aptos move compile
```

### Development Setup
```bash
# Install all development dependencies
./scripts/dev_setup.sh

# Install specific toolsets:
./scripts/dev_setup.sh -t  # build tools only
./scripts/dev_setup.sh -o  # operations tools (helm, terraform, etc.)
./scripts/dev_setup.sh -y  # Move Prover tools (z3, cvc5, boogie)
```

## Code Standards and Practices

### Rust Guidelines
- Follow `RUST_CODING_STYLE.md` for detailed coding conventions
- Use `rustfmt.toml` configuration (requires nightly rustfmt)
- All public APIs must be documented with Rustdoc
- Use inclusive terminology (allowlist/blocklist, primary/secondary)
- Prefer `expect()` over `unwrap()` with descriptive error messages
- Use `#[cfg(test)]` for test-only code, `#[allow(dead_code)]` for future production code

### Architecture Patterns
- Components communicate via well-defined interfaces and channels
- Avoid exposing synchronization primitives (Mutex, RwLock) in public APIs  
- Use channels for ownership transfer and async communication
- Use concurrent types (like CHashMap) for shared state and caches
- Errors should be recoverable (Result) or properly documented panics

### Commit and PR Standards  
- Commits must be atomic and bisect-able
- Use conventional format: `[area] description` (e.g., `[consensus] fix safety rule`)
- All commits must build and pass tests independently
- Use `./scripts/rust_lint.sh` before committing
- Rebase rather than merge when updating PRs

## Dependencies and Tooling

### Required Tools
- Rust 1.89.0 (specified in `rust-toolchain.toml`)
- cargo-sort, cargo-machete, cargo-xclippy for linting
- protoc and protobuf plugins for gRPC services
- Node.js 20 for TypeScript components
- Python 3 with Poetry for testing infrastructure

### Key Cargo Features
- Uses workspace dependencies for version management
- Custom clippy configuration in `clippy.toml` 
- License checking via `deny.toml` (Apache-2.0, MIT, BSD approved)
- Pre-commit hooks for formatting and license headers

### Move Prover (Optional)
For formal verification of Move code:
- Z3 4.11.2, CVC5 0.0.3, Boogie 3.5.1, .NET 8.0
- Run tests: `cd aptos-move/framework && cargo test prover`

## Testing Strategy

The repository uses a multi-tier testing approach:
- **Unit tests**: Standard `cargo test` within each crate
- **Integration tests**: Cross-component tests in `testsuite/`
- **Forge**: Distributed testing framework for cluster scenarios
- **E2E tests**: Full blockchain scenarios in `aptos-move/e2e-tests`
- **Move tests**: Language-specific tests for smart contracts

Most development work should include unit tests. Integration testing via Forge is used for larger system changes affecting consensus, networking, or performance.