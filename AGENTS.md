# AGENTS.md

This file provides guidance for AI coding agents working autonomously in this repository.

**Aptos CLI releases:** See `.cursor/skills/aptos-cli-release/SKILL.md` for version bumps and `crates/aptos/CHANGELOG.md` updates.

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

### Development Setup

```bash
./scripts/dev_setup.sh              # Install all build dependencies
./scripts/dev_setup.sh -y           # Include Move Prover tools (z3, boogie)
```

## Architecture Overview

### Core Transaction Flow

1. **API Layer** (`api/`) - REST endpoints receive transactions
2. **Mempool** (`mempool/`) - Transaction validation and ordering
3. **Consensus** (`consensus/`) - Byzantine fault-tolerant ordering
4. **Execution** (`execution/`) - Orchestrates VM execution
5. **Block Executor** (`aptos-move/block-executor/`) - Parallel execution via Block-STM
6. **Move** (`third_party/move/`) - Executes, compiles, and verifies Move bytecode
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

## Agent Operating Principles

### Safety-First Approach

1. **Never modify consensus-critical code without explicit user confirmation**
   - `consensus/safety-rules/`
   - `consensus/src/round_manager.rs`
   - Any code affecting Byzantine fault tolerance

2. **Never modify cryptographic code without explicit user confirmation**
   - `crates/aptos-crypto/`
   - Signature verification logic anywhere
   - Key generation or derivation

3. **Treat other security-critical modules with the same care**
   - `secure/`
   - `keyless/`

4. **Prefer reversible changes**: Make small, incremental commits that can be easily reverted

5. **When in doubt, ask**: If a change might affect security, consensus, or data integrity, pause and ask the user

### Codebase Exploration Strategy

1. **Start with semantic search** for unfamiliar areas
2. **Use grep for exact symbol lookups** (function names, types, constants)
3. **Review the architecture overview** before major changes
4. **Check existing tests** to understand expected behavior before modifying
5. **Review git history** for related changes: `git log -p --follow -- <file>`

## Standard Workflows

### Workflow: Bug Fix

```text
1. Reproduce: Understand the bug via tests or user description
2. Locate: Find relevant code using search tools
3. Analyze: Read surrounding code and tests
4. Fix: Make minimal change to fix the issue
5. Test: Run relevant tests
   cargo test -p <package> <test_name>
6. Verify: Check no regressions
   cargo test -p <package>
7. Lint: Ensure code quality
   cargo xclippy -p <package>
   cargo +nightly fmt --check
```

### Workflow: New Feature (Rust)

```text
1. Understand: Read related code and architecture
2. Design: Plan minimal implementation
3. Implement: Write code following existing patterns
4. Test: Add unit tests alongside implementation
5. Integrate: Update any affected modules
6. Verify:
   cargo build -p <package>
   cargo test -p <package>
   cargo xclippy -p <package>
```

### Workflow: Move Framework Changes

**MANDATORY RULE: Any change to a `.move` file under `aptos-move/framework/` (add/modify/delete) REQUIRES rebuilding cached packages before committing or testing. No exceptions.**

The cached `aptos-move/framework/cached-packages/src/head.mrb` is the binary bundle loaded at genesis and runtime. If it is out of sync with Move source, nodes and tests silently execute stale framework code.

```text
1. Locate: Find module in aptos-move/framework/<package>/sources/
2. Understand: Read module and its tests
3. Modify: Make changes to Move code
4. Test Move: cargo test -p <framework-package>
5. Rebuild cached packages (REQUIRED — do not skip):
   scripts/cargo_build_aptos_cached_packages.sh
6. Verify and commit the regenerated files:
   git status aptos-move/framework/cached-packages/
   # Expect: M aptos-move/framework/cached-packages/src/head.mrb
   # (plus generated *_sdk_builder.rs if entry functions changed)
```

### Workflow: API Changes

```text
1. Locate: Find endpoint in api/src/
2. Modify: Update implementation
3. Regenerate OpenAPI specs:
   cargo run -p aptos-openapi-spec-generator -- -f yaml -o api/doc/spec.yaml
   cargo run -p aptos-openapi-spec-generator -- -f json -o api/doc/spec.json
4. Test: cargo test -p aptos-api
```

## Package-Specific Guidelines

### Move VM (`third_party/move/move-vm/`)

- **Highly sensitive**: Changes affect all Move execution
- Platform-agnostic part of Move execution
- Always test with: `cargo test -p move-vm-runtime`
- Verify gas metering isn't affected unintentionally

### Aptos VM (`aptos-move/aptos-vm/`)

- **Highly sensitive**: Changes affect all Move execution
- Aptos-specific part of Move execution: parallel execution, gas, prologue, and epilogue
- Check parallel execution: `cargo test -p aptos-vm`

### Block Executor (`aptos-move/block-executor/`)

- **Parallel execution core**: Changes affect transaction throughput
- Test both sequential and parallel modes
- Check for MVCC conflicts and read/write set handling
- Key tests: `cargo test -p aptos-block-executor`

### Storage (`storage/`)

- **Data persistence**: Changes can affect all stored data
- Always verify backup/restore after changes
- Test pruning behavior
- Key tests: `cargo test -p aptos-db`

### Consensus (`consensus/`)

- **DO NOT modify without explicit approval**
- Changes require extensive testing with Forge
- Must maintain 3f+1 Byzantine fault tolerance

## Verification Commands

### Quick Verification (use frequently)

```bash
# Check compilation
cargo check -p <package>

# Run package tests
cargo test -p <package>

# Check lints for package
cargo xclippy -p <package>
```

### Pre-Commit Verification

```bash
# Full lint check
./scripts/rust_lint.sh --check

# Or individual checks
cargo xclippy
cargo +nightly fmt --check
cargo sort --grouped --workspace --check
```

### Full Verification (before major changes)

```bash
# Core packages
cargo test -p aptos-vm -p aptos-types -p aptos-crypto

# Framework
cargo test -p aptos-framework -p aptos-stdlib

# Smoke tests (slower but comprehensive)
cargo test -p smoke-test
```

## Common Patterns

### Finding Code Patterns

```bash
# Find function definitions
grep -r "fn function_name" --type rust

# Find trait implementations
grep -r "impl.*TraitName" --type rust

# Find type usages
grep -r "TypeName" --type rust --glob "*.rs"

# Find Move module
grep -r "module.*::module_name" --glob "*.move"
```

### Understanding Dependencies

```bash
# Show what a package depends on
cargo tree -p <package>

# Show what depends on a package
cargo tree -p <package> --invert

# Find duplicate dependencies
cargo tree -d
```

### Test Organization

- **Unit tests**: Same file as code, in `#[cfg(test)] mod tests { ... }`
- **Integration tests**: `<crate>/tests/` directory
- **E2E tests**: `aptos-move/e2e-tests/`
- **Move unit tests**: `#[test]` attributes in `.move` files
- **Move test directories**: `aptos-move/framework/*/tests/`
- **Smoke tests**: `testsuite/smoke-test/`

### Error Handling

- Prefer `thiserror` / `anyhow::Result` for error handling
- Use `expect()` over `unwrap()` with descriptive messages
- Use checked arithmetic (`checked_add`, `saturating_sub`, etc.)
- Use infallible locks via the `aptos-infallible` crate

### Pattern Matching

Always use exhaustive `match` — never use a wildcard `_` arm to silence new enum variants.

### Conditional Test Code

```rust
#[cfg(any(test, feature = "fuzzing"))]
fn test_helper() { ... }
```

## Move Coding Conventions

- Struct names: CamelCase (`OrderedMap`)
- Module names: snake_case (`ordered_map`)
- Function names: snake_case (`register_currency`)
- Constants: UPPER_SNAKE_CASE (`TREASURY_ADDRESS`)
- Import types at top-level; use functions qualified by module

## Error Recovery

### Build Failures

```bash
# Clean and rebuild
cargo clean -p <package>
cargo build -p <package>

# Full clean (last resort)
cargo clean
cargo build
```

### Framework Not Updating

```bash
# Rebuild cached packages
./scripts/cargo_build_aptos_cached_packages.sh
```

### Protobuf Issues

```bash
cd protos && ./scripts/build_protos.sh
```

### Lint Failures

```bash
# Auto-fix formatting
cargo +nightly fmt

# Sort dependencies
cargo sort --grouped --workspace

# Check for unused deps
cargo machete --fix
```

## Autonomous Decision Guidelines

### ALWAYS DO

- Run `cargo check` after any Rust code change
- Run tests for modified packages
- Check existing tests before modifying behavior
- Follow existing code style in the file
- Use the type system for safety (prefer `Result` over `panic!`)

### NEVER DO (without user confirmation)

- Modify `consensus/safety-rules/`
- Change signature verification logic
- Alter gas metering significantly
- Delete or rename public API endpoints
- Modify database schema
- Change network protocol messages

### ASK USER WHEN

- Unsure about backwards compatibility
- Change affects multiple packages
- Modifying critical path (consensus, execution, storage)
- Adding new dependencies
- Performance implications unclear

## Performance Considerations

- Avoid unnecessary allocations in hot paths
- Use `Arc` for shared data across threads
- Prefer iterators over collecting to `Vec`
- Check Block-STM compatibility for VM changes
- Profile before optimizing: `cargo run -p aptos-vm-profiling`

## Integration Points Reference

When modifying one component, verify these integration points:

| Component | Affects | Verify With |
|-----------|---------|-------------|
| VM | Execution, Gas | `cargo test -p aptos-vm -p aptos-block-executor` |
| Framework | VM, API | `cargo test -p aptos-framework && scripts/cargo_build_aptos_cached_packages.sh` |
| Storage | Everything | `cargo test -p aptos-db -p aptos-executor` |
| API | Clients | `cargo test -p aptos-api` + check OpenAPI spec |
| Types | Everything | `cargo test -p aptos-types` |
| Network | Consensus, Sync | `cargo test -p aptos-network` |

## File Naming Conventions

- Rust: `snake_case.rs`
- Move: `module_name.move`
- Tests: `*_test.rs` or in `tests/` directory
- Configs: `*.yaml` or `*.toml`

## Commit Message Format

```text
[area] Brief description (50 char max)

Detailed explanation of why, not what.

- Detailed change 1
- Detailed change 2

Example:
[vm] Fix gas metering for vector operations

- Corrected gas calculation for vector push
- Added unit tests for edge cases
```

Areas: `consensus`, `mempool`, `network`, `storage`, `execution`, `vm`, `framework`, `api`, `cli`, `crypto`, `types`, `docs`, `test`
