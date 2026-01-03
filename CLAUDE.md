# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Aptos Core Overview

Aptos is a Layer 1 blockchain built with the Move programming language. This repository contains the full node implementation, consensus protocol (AptosBFT), execution engine, storage layer, networking stack, and the Aptos Move framework.

## Build, Lint, and Test Commands

### Linting and Formatting

```bash
# Run all linting checks (clippy, formatting, cargo-sort, unused dependencies)
./scripts/rust_lint.sh

# Check without fixing issues
./scripts/rust_lint.sh --check

# Run clippy only (with project-specific configuration)
cargo xclippy

# Format code (requires nightly Rust)
cargo +nightly fmt
```

### Building

```bash
# Build the entire workspace
cargo build

# Build in release mode
cargo build --release

# Build specific package (e.g., aptos-node)
cargo build -p aptos-node

# Build with CLI profile (optimized for size)
cargo build --profile cli
```

### Testing

```bash
# Run all tests using nextest (preferred)
cargo x nextest run

# Run tests for specific package
cargo x nextest run -p <package-name>

# Run traditional cargo test
cargo test

# Run tests for a specific crate
cargo test -p <package-name>

# Run single test by name
cargo x nextest run <test-name>

# Skip slow tests (e.g., prover tests in framework)
cargo test -- --skip prover
```

### Move Framework Testing

```bash
# Test Move framework from framework directory
cd aptos-move/framework
cargo test

# Skip Move prover tests
cargo test -- --skip prover

# Run tests for specific Move package (e.g., aptos_stdlib)
cargo test -- aptos_stdlib --skip prover

# Filter by test name or module
TEST_FILTER="bulletproofs" cargo test -- aptos_stdlib --skip prover

# Show time and gas stats for Move tests
REPORT_STATS=1 cargo test -- aptos_stdlib --skip prover

# If running out of stack memory, adjust stack size or use release mode
export RUST_MIN_STACK=4297152
# or
cargo test --release -- --skip prover
```

### Useful Cargo X Commands

The repository includes a custom cargo CLI (`cargo x`) with these commands:

- `cargo x check` - Run cargo check on workspace
- `cargo x xclippy` - Run clippy with project configuration
- `cargo x fmt` - Format code
- `cargo x nextest` - Run tests with nextest
- `cargo x test` - Run standard cargo tests
- `cargo x affected-packages` - Show packages affected by changes
- `cargo x changed-files` - Show changed files
- `cargo x targeted-unit-tests` - Run targeted tests based on changes

## High-Level Architecture

### Major Components

**Consensus (AptosBFT)**
- Location: `consensus/`
- BFT consensus based on Jolteon, providing safety and liveness in partial synchrony
- Uses 3-chain commit rule for block finalization
- Key sub-components:
  - **BlockStore**: Manages blocks and execution state
  - **RoundManager**: Processes proposals and votes
  - **SafetyRules**: Ensures consensus safety (persisted across restarts)
  - **PayloadClient**: Interface to mempool for transaction pulling

**Mempool**
- Location: `mempool/`
- Shared memory pool for pending transactions across validators
- Maintains two indexes:
  - **PriorityIndex**: Gas-price sorted queue of consensus-ready transactions
  - **ParkingLotIndex**: Transactions waiting for dependencies
- Transactions must have sequential sequence numbers to be consensus-ready

**Execution**
- Location: `execution/`
- Executes transactions via Aptos VM and applies state updates
- Maintains execution tree for speculative execution on different consensus branches
- Key operations:
  - `execute_block`: Speculative execution of proposed blocks
  - `commit_block`: Persist committed blocks to storage

**Storage (AptosDB)**
- Location: `storage/`
- Authenticated blockchain storage using RocksDB
- Sub-databases:
  - **ledger_db**: Transactions, events, write sets
  - **state_merkle_db**: Jellyfish Merkle tree for state authentication
  - **indexer_db**: Account-based indexing (optional)
- Implements pruning for ledger history and state snapshots

**State Sync**
- Location: `state-sync/`
- Synchronizes nodes to latest blockchain state
- Four components:
  - **Driver**: Verifies and persists data
  - **Data Streaming Service**: Creates data streams for clients
  - **Aptos Data Client**: Handles data requests and peer discovery
  - **Storage Service**: Peer-to-peer data fetching API

**Network (AptosNet)**
- Location: `network/`
- TCP + NoiseIK for authenticated and encrypted communication
- Two interfaces:
  - **DirectSend**: Fire-and-forget messages
  - **RPC**: Unary remote procedure calls
- Validators maintain full-mesh connectivity

**Aptos Move VM**
- Location: `aptos-move/aptos-vm/`
- Wraps Move VM with Aptos-specific logic
- Transaction execution flow: Prologue → Execution → Epilogue
- Supports both sequential and parallel (Block-STM) execution

**Aptos Move Framework**
- Location: `aptos-move/framework/`
- Four packages:
  - **move-stdlib**: Base Move standard library
  - **aptos-stdlib**: Aptos standard library extensions
  - **aptos-framework**: Core Aptos modules (accounts, coins, governance, gas)
  - **aptos-token**: Token standards

### Component Interactions

Transaction flow through the system:
1. Client submits transaction → REST API → Mempool
2. Mempool broadcasts to validators via Network
3. Consensus leader proposes block with transactions
4. Execution runs transactions via Aptos VM
5. Validators vote on block + execution results
6. After 3-chain commit, Consensus commits block
7. Storage persists transactions and state
8. State Sync notifies Mempool and synchronizes lagging nodes

### Parallel Execution (Block-STM)

- Location: `aptos-move/block-executor/`
- Optimistic concurrency control for parallel transaction execution
- Uses **MVHashMap** (multi-version hashmap) to track read/write versions
- Transactions execute speculatively with validation
- Failed validation triggers re-execution with updated dependencies

### Key Architectural Patterns

- **Actor Model**: Components use message-passing with tokio runtime
- **Speculative Execution**: Consensus executes blocks before commitment
- **Authenticated State**: All state changes authenticated via Jellyfish Merkle tree
- **On-Chain Configuration**: System parameters updated via governance without hard forks
- **Gas-Based Prioritization**: Transactions prioritized by gas price (within sequence constraints)

## Repository Structure

```
aptos-core/
├── aptos-node/              # Main validator/fullnode binary
├── consensus/               # AptosBFT consensus implementation
├── mempool/                # Transaction mempool
├── execution/              # Transaction execution and block processing
├── storage/                # AptosDB and persistent storage
├── state-sync/             # State synchronization
├── network/                # Networking layer (AptosNet)
├── api/                    # REST API for clients
├── types/                  # Core data types
├── aptos-move/
│   ├── aptos-vm/           # Aptos VM wrapper
│   ├── framework/          # Move framework (stdlib, aptos-framework, tokens)
│   ├── block-executor/     # Parallel execution (Block-STM)
│   ├── mvhashmap/          # Multi-version data structure
│   └── aptos-gas-*/        # Gas metering and scheduling
├── config/                 # Node configuration
├── crates/                 # Shared utility crates
│   ├── aptos-crypto/       # Cryptographic primitives
│   ├── aptos-logger/       # Logging infrastructure
│   └── aptos-*/            # Various utilities
├── ecosystem/              # Ecosystem tools
│   └── indexer-grpc/       # gRPC indexer
├── third_party/move/       # Move language and VM
├── sdk/                    # SDKs
└── scripts/                # Development scripts
```

## Rust Coding Guidelines

### Code Quality

- All code must pass `./scripts/rust_lint.sh` before submission
- Use checked arithmetic (`checked_add`, `saturating_sub`) instead of direct operators
- Document all public functions with Rustdoc (single-line summary + detailed description)
- Use `expect()` instead of `unwrap()` with detailed error messages
- Use infallible types from `aptos-infallible` for `RwLock` and `Mutex`

### Testing

- Unit tests in the same file as code under test, in `#[cfg(test)]` module
- For test-only code used across crates, use `#[cfg(any(test, feature = "fuzzing"))]`
- Integration tests in separate test-only crates (see `workspace.test-only` in Cargo.toml)
- Property-based tests using `proptest` framework

### Commit Message Format

```
[area] Short summary (50 chars max)

Detailed explanation of the problem, the solution,
and any alternatives considered.
```

Common areas: `consensus`, `mempool`, `network`, `storage`, `execution`, `vm`, `framework`, `api`, `ci`

### Important Practices

- Every commit must build and pass all tests (bisect-able history)
- Use `expect()` to document invariants that should never fail
- Avoid exposing synchronization primitives (Mutex, RwLock) in public APIs
- Prefer `//` and `///` comments over block comments
- Keep generics minimal; consider trait objects for complex generic types
- Use terminology: allowlist/blocklist, primary/secondary, leader/follower

## Move Development

### Compiling Framework

```bash
# Build cached packages (generates Move bytecode and artifacts)
./scripts/cargo_build_aptos_cached_packages.sh

# Or via cargo
cargo run -p aptos-framework
```

### Move Documentation

```bash
# Generate Move documentation
aptos move document --help

# Documentation is auto-generated during framework builds
```

### Testing Move Code

- Move tests run via Rust test harness in `aptos-move/framework/tests/`
- Use `TEST_FILTER` environment variable to filter tests
- Set `REPORT_STATS=1` to show gas and time stats
- Framework tests can be memory-intensive; use release mode or increase stack size if needed

## Development Setup

```bash
# Install required tools (cargo-sort, cargo-machete, etc.)
./scripts/dev_setup.sh

# For Windows
./scripts/windows_dev_setup.ps1
```

## Additional Resources

- [Aptos Developer Documentation](https://aptos.dev)
- [Move Book](https://aptos.dev/move/book/SUMMARY)
- [Move Coding Conventions](https://aptos.dev/move/book/coding-conventions/)
- [Rust Coding Style](./RUST_CODING_STYLE.md)
- [Rust Secure Coding Guidelines](./RUST_SECURE_CODING.md)
- [Contributing Guide](./CONTRIBUTING.md)
