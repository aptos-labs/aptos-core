# agents.md

This file provides guidance for AI coding agents working autonomously in this repository. It extends `CLAUDE.md` with agent-specific workflows and safety guidelines.

> **Prerequisites**: Read and understand `CLAUDE.md` before using this file. That document contains essential build commands, architecture overview, and project structure.

## Agent Operating Principles

### Safety-First Approach

1. **Never modify consensus-critical code without explicit user confirmation**
   - `consensus/safety_rules/`
   - `consensus/src/round_manager.rs`
   - Any code affecting Byzantine fault tolerance

2. **Never modify cryptographic code without explicit user confirmation**
   - `crates/aptos-crypto/`
   - Signature verification logic anywhere
   - Key generation or derivation

3. **Prefer reversible changes**: Make small, incremental commits that can be easily reverted

4. **When in doubt, ask**: If a change might affect security, consensus, or data integrity, pause and ask the user

### Codebase Exploration Strategy

1. **Start with semantic search** for unfamiliar areas
2. **Use grep for exact symbol lookups** (function names, types, constants)
3. **Read CLAUDE.md** for architecture context before major changes
4. **Check existing tests** to understand expected behavior before modifying
5. **Review git history** for related changes: `git log -p --follow -- <file>`

## Standard Workflows

### Workflow: Bug Fix

```
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

```
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

```
1. Locate: Find module in aptos-move/framework/<package>/sources/
2. Understand: Read module and its tests
3. Modify: Make changes to Move code
4. Test Move: cargo test -p <framework-package>
5. Rebuild cached packages (REQUIRED):
   cargo build -p aptos-cached-packages
6. Verify: Check for uncommitted generated files
   git status aptos-move/framework/cached-packages/
```

### Workflow: API Changes

```
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
- Platform agnostic part of Move execution
- Always test with: `cargo test -p move-vm-runtime`
- Verify gas metering isn't affected unintentionally

### Aptos VM (`aptos-move/aptos-vm/`)

- **Highly sensitive**: Changes affect all Move execution
- Aptos specific part of Move execution: parallel, gas, prologue, epilogue
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

### Navigating Test Structure

- **Unit tests**: Same file as code, in `#[cfg(test)] mod tests { ... }`
- **Integration tests**: `<crate>/tests/` directory
- **E2E tests**: `aptos-move/e2e-tests/`
- **Move tests**: `aptos-move/framework/*/tests/`
- **Smoke tests**: `testsuite/smoke-test/`

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
# Force rebuild cached packages
cargo clean -p aptos-cached-packages
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

- Modify `consensus/safety_rules/`
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
| Framework | VM, API | `cargo test -p aptos-framework && cargo build -p aptos-cached-packages` |
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

```
[area] Brief description

- Detailed change 1
- Detailed change 2

Example:
[vm] Fix gas metering for vector operations

- Corrected gas calculation for vector push
- Added unit tests for edge cases
```

Areas: `vm`, `framework`, `consensus`, `storage`, `api`, `network`, `types`, `cli`, `docs`, `test`

## Cursor Cloud specific instructions

### System Dependencies

The VM update script runs `./scripts/dev_setup.sh -b -t -k` (batch, build tools, skip pre-commit) which installs clang-21, lld, protoc 3.21.4, cargo-sort, cargo-machete, and the Rust 1.93.1 toolchain.

One extra apt package is needed beyond what `dev_setup.sh` installs: `libstdc++-14-dev`. Without it, clang-21 cannot find C++ standard library headers (`<limits>`, `<memory>`) and RocksDB compilation fails. The update script installs this automatically.

### Building

- `cargo check -p <package>` for quick compilation checks (no RocksDB dependency).
- `cargo build -p aptos-node` builds the main node binary (~4 min from cold, ~1 min incremental). Requires RocksDB (C++ compilation via clang).
- `cargo build -p aptos` builds the CLI tool (~8 min from cold).
- See `CLAUDE.md` for the full set of build/test/lint commands.

### Running a Local Testnet

Start a local testnet with a faucet using the CLI:

```bash
cargo build -p aptos-node -p aptos
./target/debug/aptos node run-local-testnet --with-faucet --force-restart --assume-yes
```

This starts:
- REST API on `http://127.0.0.1:8080`
- Faucet on `http://127.0.0.1:8081`
- Indexer gRPC on `127.0.0.1:50051`
- Metrics on `http://127.0.0.1:9101`

The faucet takes ~15-20 seconds to become ready after the API is up.

### Linting

- `cargo xclippy` runs workspace-wide clippy (the `xclippy` alias in `.cargo/config.toml` includes all needed flags).
- Per-package clippy: replicate the flags from the xclippy alias but replace `--workspace` with `-p <package>`.
- `cargo +nightly fmt --check` for formatting.
- `cargo sort --grouped --workspace --check` for Cargo.toml dependency ordering.
- `./scripts/rust_lint.sh --check` runs all lint checks together.

### Testing

- `cargo test -p <package>` for package-level tests.
- The `accounts/resources` REST endpoint requires an indexer; use specific resource lookups (`/v1/accounts/{addr}/resource/{type}`) instead.
- The `--test-threads=4` flag is recommended to avoid overloading the 4-core VM.
