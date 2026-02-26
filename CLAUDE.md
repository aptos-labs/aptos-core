# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Aptos Core is a layer 1 blockchain written primarily in Rust with Move smart contracts. It's a production-grade system with 200+ workspace crates organized into major subsystems: consensus, execution, storage, network, mempool, API, and Move VM.

## Essential Commands

### Build & Check
```bash
cargo build -p <package>           # Build a single package
cargo check -p <package>           # Quick compile check (faster than build)
cargo build --profile performance  # Optimized build with LTO
```

### Testing
```bash
cargo test -p <package>                    # Test a single package
cargo test -p <package> -- <test_name>     # Run a specific test
cargo test -p aptos-framework              # Framework tests
cargo test -p smoke-test                   # E2E smoke tests
cargo test -p e2e-move-tests               # Move e2e tests
```

### Linting & Formatting
```bash
./scripts/rust_lint.sh              # Full lint (clippy + fmt + sort + machete)
./scripts/rust_lint.sh --check      # Check-only mode for CI
cargo xclippy                       # Just clippy
cargo +nightly fmt                  # Just formatting
```

### Move Framework Changes
After modifying Move code in `aptos-move/framework/`:
```bash
cargo build -p aptos-cached-packages   # REQUIRED: rebuild cached packages
```

### Development Setup
```bash
./scripts/setup_build.sh            # Install all build dependencies
./scripts/setup_build.sh -y         # Include Move Prover tools (z3, boogie)
./scripts/setup_build.sh --help     # Show all flags and options
```

## Architecture Overview

### Core Transaction Flow
1. **API Layer** (`api/`) - REST endpoints receive transactions
2. **Mempool** (`mempool/`) - Transaction validation and ordering
3. **Consensus** (`consensus/`) - Byzantine fault-tolerant ordering
4. **Execution** (`execution/`) - Orchestrates VM execution
5. **Block Executor** (`aptos-move/block-executor/`) - Parallel execution via Block-STM
6. **Move** (`third_party/move/`) - Executes Move bytecode, Compiles Move, Verifies Move
7. **Storage** (`storage/`) - Persistent state (JellyfishMerkleTree)
8. **State Sync** (`state-sync/`) - Blockchain synchronization

### Move Framework Stack
- `aptos-move/framework/move-stdlib/` - Core Move stdlib
- `aptos-move/framework/aptos-stdlib/` - Aptos-specific stdlib
- `aptos-move/framework/aptos-framework/` - Core chain modules (coin, account, staking)
- `aptos-move/framework/aptos-token-objects/` - NFT standards

### Key Crates
- `aptos-types` - Core type definitions used everywhere
- `aptos-vm` - VM integration and transaction execution
- `aptos-crypto` - Cryptographic primitives (security-critical)
- `aptos-api-types` - API request/response types

## Safety-Critical Code

These directories require extra care and should not be modified without explicit approval:
- `consensus/safety-rules/` - Byzantine fault tolerance
- `crates/aptos-crypto/` - Cryptographic implementations
- `secure/` - Security-critical modules
- `keyless/` - Keyless authentication

## Commit Message Format

```
[area] Brief description (50 char max)

Detailed explanation of why, not what.

Areas: consensus, mempool, network, storage, execution, vm, framework, api, cli, crypto, types
```

## Common Patterns

### Test Organization
- Unit tests: `#[cfg(test)] mod tests { ... }` in source files
- Integration tests: `<crate>/tests/` directories
- Move tests: `#[test]` attributes in `.move` files

### Error Handling
- Prefer thiserror / anyhow `Result` for error handling
- Use `expect()` over `unwrap()` with descriptive messages
- Use checked arithmetic (`checked_add`, `saturating_sub`, etc.)
- Infallible locks via `aptos-infallible` crate

### Conditional Test Code
```rust
#[cfg(any(test, feature = "fuzzing"))]
fn test_helper() { ... }
```

## Move Coding Conventions

- Struct names: CamlCase (`OrderedMap`)
- Module names: snake_case (`ordered_map`)
- Function names: snake_case (`register_currency`)
- Constants: UPPER_SNAKE_CASE (`TREASURY_ADDRESS`)
- Import types at top-level, use functions qualified by module
