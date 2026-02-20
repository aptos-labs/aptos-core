# Aptos Rosetta: Audit, Evaluation & Improvement Plan

**Date:** 2026-02-13
**Scope:** `crates/aptos-rosetta` and `crates/aptos-rosetta-cli`
**Author:** Automated Audit

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Performance Evaluation](#performance-evaluation)
4. [Usability Evaluation](#usability-evaluation)
5. [Code Quality & Maintainability](#code-quality--maintainability)
6. [Security Considerations](#security-considerations)
7. [Correctness Issues](#correctness-issues)
8. [Documentation Cleanup](#documentation-cleanup)
9. [Improvement Recommendations](#improvement-recommendations)
10. [Priority Roadmap](#priority-roadmap)

---

## 1. Executive Summary

The Aptos Rosetta implementation provides a [Rosetta API](https://www.rosetta-api.org/) sidecar proxy that translates Rosetta-standard requests into Aptos REST API calls. It supports three operational modes (online, online-remote, offline) and covers the Data API (Network, Block, Account) and Construction API (Preprocess through Submit).

**Overall Assessment:** The implementation is functional and covers the core Rosetta spec, but has accumulated significant technical debt, several dead/commented-out code paths, naming mismatches, missing caching, and areas where performance and maintainability can be significantly improved.

### Key Findings Summary

| Category | Severity | Count |
|----------|----------|-------|
| Performance Bottlenecks | Medium-High | 6 |
| Code Quality Issues | Medium | 12 |
| Correctness Concerns | Medium | 5 |
| Documentation Gaps | Low-Medium | 8 |
| Dead Code / TODOs | Low | 15+ |

---

## 2. Architecture Overview

### Crate Structure

```
crates/aptos-rosetta/         # Main Rosetta server library + binary
├── src/
│   ├── lib.rs                 # Server bootstrap, routes, RosettaContext
│   ├── main.rs                # CLI entry point (online/offline/online-remote modes)
│   ├── account.rs             # /account/balance endpoint
│   ├── block.rs               # /block endpoint, BlockRetriever
│   ├── construction.rs        # /construction/* endpoints (8 sub-routes)
│   ├── network.rs             # /network/* endpoints (list, options, status)
│   ├── client.rs              # RosettaClient for testing/integration
│   ├── common.rs              # Shared utilities (BlockHash, currency helpers)
│   ├── error.rs               # ApiError enum and conversions
│   ├── types/                 # All Rosetta data types
│   │   ├── mod.rs
│   │   ├── identifiers.rs     # AccountIdentifier, BlockIdentifier, etc.
│   │   ├── objects.rs         # Transaction, Operation, InternalOperation (~3000 lines)
│   │   ├── misc.rs            # OperationType, OperationStatusType, balance helpers
│   │   ├── move_types.rs      # Move struct/module constants and BCS types
│   │   └── requests.rs        # All request/response types
│   └── test/mod.rs            # Unit tests for FA operations
├── Cargo.toml
├── README.md
├── rosetta_cli.json           # Official Rosetta CLI config
└── aptos.ros                  # Rosetta DSL test script

crates/aptos-rosetta-cli/      # Developer CLI for testing
├── src/
│   ├── main.rs
│   ├── common.rs
│   ├── account.rs
│   ├── block.rs
│   ├── construction.rs
│   └── network.rs
├── Cargo.toml
└── README.md
```

### Data Flow

```
Client → Rosetta HTTP (warp) → Route Handler → REST Client → Aptos Fullnode
                                     ↓
                              RosettaContext (chain_id, currencies, block_cache)
```

---

## 3. Performance Evaluation

### 3.1 No Block Caching Despite Variable Named `block_cache`

**Severity: High**

The `BlockRetriever` (referred to as `block_cache` throughout) has **no actual caching**. Every request to fetch block info, even for the same block, results in a full REST API call to the fullnode. There is even a TODO comment acknowledging this:

```rust
// TODO: The BlockRetriever has no cache, and should probably be renamed from block_cache
```

**Impact:**
- `/block` requests always hit the fullnode twice (once for the current block, once for the parent block)
- `/account/balance` also fetches block info every time
- `/network/status` fetches 3 blocks (genesis, oldest, latest) on every call
- Under load, this creates unnecessary pressure on the fullnode REST API

**Recommendation:** Implement an LRU cache (e.g., `lru` or `moka` crate) for `BlockInfo` keyed by height. Genesis block (height 0) should always be cached since it never changes.

### 3.2 Sequential Transaction Processing in Blocks

**Severity: Medium**

In `block.rs`, transactions within a block are processed sequentially:

```rust
// TODO: Parallelize these and then sort at end
if let Some(txns) = block.transactions {
    for txn in txns {
        let transaction = Transaction::from_transaction(server_context, txn).await?;
        ...
    }
}
```

For large blocks, this is a bottleneck since each transaction's operations are parsed independently.

**Recommendation:** Use `futures::stream::FuturesOrdered` or `tokio::spawn` to process transactions concurrently, then collect and sort results.

### 3.3 Sorting After Every Block Parse

**Severity: Low-Medium**

Transactions are sorted by version after processing, and operations within each transaction are also sorted. While necessary for correctness, the comment notes:

```rust
// NOTE: sorting may be pretty expensive, depending on the size of the block
```

The current sort is `O(n log n)` on operations, which is fine for typical blocks but could be optimized if operations are built in-order.

### 3.4 Currency Lookup is Linear Scan

**Severity: Low-Medium**

`find_coin_currency` and `find_fa_currency` perform linear scans over `HashSet<Currency>` on every operation. With many configured currencies, this becomes noticeable:

```rust
// TODO: Probably want to cache this
AccountAddress::from_str(fa_address)
    .map(|addr| addr == metadata_address)
    .unwrap_or(false)
```

**Recommendation:** Build a `HashMap<String, Currency>` (keyed by canonical move type) and a `HashMap<AccountAddress, Currency>` (keyed by FA address) at startup for O(1) lookups.

### 3.5 Double Block Fetch for Non-Genesis Blocks

**Severity: Medium**

`get_block_by_index` fetches both the current block (with transactions) and the previous block (without transactions) for every block request. The previous block fetch could be avoided with caching.

### 3.6 Redundant Write Set Parsing

**Severity: Low-Medium**

In `Transaction::from_transaction`, the write set is iterated twice: once in `preprocess_write_set` (to build owner/currency maps) and again in `parse_operations_from_write_set`. These could be combined into a single pass.

---

## 4. Usability Evaluation

### 4.1 Block Hash is Not a Real Hash

The "block hash" is `chain_id-block_height` (e.g., `mainnet-12345`), not a cryptographic hash. This is documented in the README but could confuse integrators expecting standard behavior. The README correctly notes:

> Block hash is `<chain_id>:<block_height>` and not actually a hash.

(Note: the README says colon but the code uses hyphen — this is an inconsistency.)

### 4.2 Only Ed25519 Single Signer Supported

The construction API only supports Ed25519 single signer authentication. Multi-sig, multi-agent, and key rotation scenarios are not supported. This limits usability for institutional integrators who may use multi-sig wallets.

### 4.3 Error Messages Could Be More Helpful

Some error messages are generic:
- `"This API is unavailable for the node because he's offline"` — casual language
- `"Faileed to retrieve the coin type information, please retry"` — typo ("Faileed")
- `"Block is missing events"` — misleading for `BlockNotFound`

### 4.4 `RejectedByFilter` Missing from `all()` List

The `ApiError::RejectedByFilter` variant is defined and has a code (35) but is **missing from the `ApiError::all()` method**, which means it won't appear in `/network/options` error listing — a Rosetta spec violation.

### 4.5 Hardcoded USDC Addresses

USDC addresses for mainnet and testnet are hardcoded:
```rust
const USDC_ADDRESS: &str = "0xbae207659db88bea0cbead6da0ed00aac12edcdda169e591cd41c94180b46f3b";
const USDC_TESTNET_ADDRESS: &str = "0x69091fbab5f7d635ee7ac5098cf0c1efbe31d68fec0f2cd565e8d168daf52832";
```

These should be configurable or loaded from the currency config file rather than hardcoded.

### 4.6 CLI is Missing Several Operations

The `aptos-rosetta-cli` only supports a subset of construction operations:
- `CreateAccount`, `Transfer`, `SetOperator`, `SetVoter`, `CreateStakePool`

Missing from CLI:
- `ResetLockup`, `UnlockStake`, `UpdateCommission`, `DistributeStakingRewards`
- All delegation pool operations (`AddDelegatedStake`, `UnlockDelegatedStake`, `WithdrawUndelegated`)

### 4.7 Thread Parking for Shutdown

The main binary uses a busy-park pattern for staying alive:
```rust
let term = Arc::new(AtomicBool::new(false));
while !term.load(Ordering::Acquire) {
    std::thread::park();
}
```

This never actually terminates gracefully. Signal handling (SIGTERM/SIGINT) should be implemented via `tokio::signal`.

---

## 5. Code Quality & Maintainability

### 5.1 `objects.rs` is ~3000 Lines

The `types/objects.rs` file is extremely large and handles:
- Type definitions (Operation, Transaction, InternalOperation, etc.)
- Transaction parsing from on-chain data
- Write set interpretation
- Event filtering
- Failed transaction parsing
- Currency matching

This should be broken into multiple files.

### 5.2 Massive `construction_payloads` Match Block

The `construction_payloads` function contains a 200+ line match block validating that each `InternalOperation` variant matches its metadata counterpart. Most arms do essentially the same thing with slight differences. This is a refactoring opportunity.

### 5.3 Duplicated Parsing Logic

Transaction parsing exists in three separate places:
1. `construction::parse` — for parsing unsigned/signed construction transactions
2. `objects::parse_operations_from_write_set` — for parsing committed transactions
3. `objects::parse_failed_operations_from_txn_payload` — for parsing failed transactions

These share significant overlap but are maintained independently, creating a risk of divergence.

### 5.4 Dead/Commented-Out Code

There are significant blocks of commented-out code:
- `parse_stake_pool_resource_changes` is an empty function with ~200 lines of commented-out code for balance changes
- `operator_stake` support has TODO comments indicating it's not implemented
- Multiple `// TODO: Right now operator stake is not supported` comments

### 5.5 Excessive `unwrap()` Usage

Several places use `unwrap()` without descriptive messages:
- `File::open(filepath).unwrap()` in currency config loading
- `serde_json::from_reader(file).unwrap()` for currency parsing
- `AccountAddress::from_str(pool).unwrap()` in sub-account constructors

Per CLAUDE.md guidelines, these should use `expect()` with descriptive messages or proper error handling.

### 5.6 Inconsistent Error Handling Patterns

Some functions return `ApiResult<T>`, others return `anyhow::Result<T>`, and the client uses a mix. The boundary between these is not well-defined.

### 5.7 TODO Comments Count

There are 15+ TODO comments scattered throughout the codebase, indicating unfinished work:
- Cache-related TODOs
- Refactoring TODOs
- Feature TODOs (multi-signer, key rotation, mempool)
- Fix TODOs (commission typo, balance calculations)

---

## 6. Security Considerations

### 6.1 No Rate Limiting

The Rosetta server has no built-in rate limiting. Since it proxies to a fullnode, a malicious client could overwhelm the fullnode through the Rosetta sidecar.

### 6.2 CORS is Allow-Any-Origin

```rust
warp::cors()
    .allow_any_origin()
    .allow_methods(vec![Method::GET, Method::POST])
```

This is appropriate for a backend API but should be documented as a conscious decision.

### 6.3 Debug Logging of Full Responses

```rust
debug!("Response: {:?}", serde_json::to_string_pretty(&response));
```

Full responses are logged at debug level, which could include sensitive balance information in production logs.

### 6.4 No Input Size Validation on BCS Decoding

BCS-encoded inputs from construction requests are decoded without size validation. A malicious actor could send extremely large payloads.

---

## 7. Correctness Issues

### 7.1 Staking Balance Calculation Concerns

The code itself has a TODO acknowledging potential issues:
```rust
// TODO: I think all of these are off, probably need to recalculate all of them
```

Specifically:
- Active stake calculation subtracts commission, which may not be correct in all cases
- Pending inactive uses a `pending_attribution_snapshot` view function that may not reflect the actual pending inactive balance
- Total stake calculation also subtracts commission

### 7.2 Lockup Expiration Only Works for Single Staking Contract

```rust
// TODO: This seems like it only works if there's only one staking contract (hopefully it stays that way)
lockup_expiration = balance_result.lockup_expiration;
```

If an owner has multiple staking contracts, only the last one's lockup expiration is reported.

### 7.3 Block Hash Separator Inconsistency

The README says `<chain_id>:<block_height>` (colon) but the code uses `<chain_id>-<block_height>` (hyphen). The `BlockHash` implementation correctly uses hyphen, but the documentation is wrong.

### 7.4 `update_commision` Typo in On-Chain Function

```rust
// TODO fix the typo in function name. commision -> commission (this has to be done on-chain first)
pub const UPDATE_COMMISSION_FUNCTION: &str = "update_commision";
```

The on-chain function has a typo (`commision`), and the Rosetta code matches it. If/when the on-chain function is fixed, this needs to be updated — ideally supporting both spellings during transition.

### 7.5 `ConstructinoPreProcessRequest` Typo in README

The README references `ConstructinoPreProcessRequest` (should be `ConstructionPreProcessRequest`).

---

## 8. Documentation Cleanup

### 8.1 README Issues

1. **Block hash separator**: Says colon (`:`), should say hyphen (`-`)
2. **Typo**: `ConstructinoPreProcessRequest` → `ConstructionPreProcessRequest`
3. **Typo**: `chage` → `change` in Set Voter section
4. **Missing operations**: The README lists only 6 operations (create_account, withdraw, deposit, fee, set_operator, set_voter) but the implementation supports 15 operation types
5. **No mention of FA/USDC support**: The README only mentions APT for balances, but the code now supports fungible assets and USDC
6. **No mention of delegation pool operations**: These are fully implemented but undocumented
7. **Stale link**: Rosetta CLI link may be outdated
8. **Missing architecture diagram**: The architecture section is text-only

### 8.2 In-Code Documentation

1. Many functions lack doc comments (especially in `objects.rs`)
2. The `InternalOperation` enum variants are undocumented
3. Event parsing functions have no documentation about which events they expect
4. The `Operation` constructors don't document their field semantics

### 8.3 CLI README

The CLI README is minimal (7 lines) and doesn't list available commands or provide usage examples.

### 8.4 Configuration Documentation

The `rosetta_cli.json` and `aptos.ros` files have no inline documentation about what they test or how to customize them. The currency config file format is only documented in a CLI `--help` docstring.

---

## 9. Improvement Recommendations

### 9.1 High Priority

| # | Improvement | Effort | Impact |
|---|-------------|--------|--------|
| 1 | Add actual caching to `BlockRetriever` | Medium | High (performance) |
| 2 | Fix `RejectedByFilter` missing from `all()` | Trivial | High (spec compliance) |
| 3 | Fix README inaccuracies (separator, typos) | Trivial | Medium (correctness) |
| 4 | Fix `"Faileed"` typo in error message | Trivial | Low (polish) |
| 5 | Add graceful shutdown signal handling | Low | Medium (operations) |
| 6 | Audit staking balance calculations | High | High (correctness) |

### 9.2 Medium Priority

| # | Improvement | Effort | Impact |
|---|-------------|--------|--------|
| 7 | Break up `objects.rs` into multiple files | Medium | High (maintainability) |
| 8 | Parallelize transaction processing in blocks | Medium | Medium (performance) |
| 9 | Build indexed currency lookups at startup | Low | Medium (performance) |
| 10 | Consolidate operation parsing logic | High | High (maintainability) |
| 11 | Remove or implement all commented-out code | Medium | Medium (code quality) |
| 12 | Replace `unwrap()` with `expect()` or proper errors | Low | Medium (robustness) |
| 13 | Make USDC addresses configurable | Low | Medium (flexibility) |

### 9.3 Low Priority / Future

| # | Improvement | Effort | Impact |
|---|-------------|--------|--------|
| 14 | Add multi-signer support | High | Medium (feature) |
| 15 | Implement mempool APIs | High | Low (feature) |
| 16 | Add rate limiting | Medium | Medium (security) |
| 17 | Add CLI commands for all operations | Medium | Medium (usability) |
| 18 | Add integration test suite | High | High (confidence) |
| 19 | Support key rotation in derive | Medium | Low (feature) |
| 20 | Add metrics/prometheus endpoint | Medium | Medium (observability) |

---

## 10. Priority Roadmap

### Phase 1: Quick Wins (1-2 days)
- Fix all typos (README, error messages)
- Add `RejectedByFilter` to `all()` list
- Replace `unwrap()` with `expect()` in currency loading
- Add signal handling for graceful shutdown
- Update README with all supported operations and correct documentation

### Phase 2: Performance (1 week)
- Implement LRU block cache in `BlockRetriever`
- Build indexed currency lookups (HashMap by type tag and FA address)
- Parallelize transaction processing
- Combine write set preprocessing and parsing into single pass

### Phase 3: Code Quality (1-2 weeks)
- Split `objects.rs` into `transaction.rs`, `operation.rs`, `internal_operation.rs`, `write_set_parser.rs`
- Consolidate operation parsing between construction parse, write set parse, and failed txn parse
- Remove all dead/commented-out code or replace with proper feature flags
- Refactor `construction_payloads` validation into a trait method on `InternalOperation`
- Add comprehensive doc comments

### Phase 4: Correctness (1 week)
- Audit and fix staking balance calculations with tests against real chain data
- Handle multiple staking contracts for lockup expiration
- Add validation for block hash when both index and hash are provided

### Phase 5: Features (2-4 weeks)
- Add remaining CLI commands for all operations
- Implement mempool APIs (at least basic lookup)
- Explore multi-signer support
- Add metrics endpoint for monitoring
- Consider WebSocket support for block streaming

---

## Appendix: File-by-File Notes

### `lib.rs`
- Clean bootstrap logic
- `block_cache` naming is misleading (it's a retriever, not a cache)
- Route setup is clean but could use a macro to reduce boilerplate

### `main.rs`
- Three modes are well-structured
- Thread parking for keepalive needs replacement
- `owner_address_file` is deprecated but still in the struct

### `error.rs`
- Comprehensive error mapping from REST errors
- Missing `RejectedByFilter` in `all()`
- `"Faileed"` typo in `CoinTypeFailedToBeFetched` message

### `common.rs`
- `BlockHash` is a good abstraction for the fake hash
- Currency helpers are clean but need indexing for performance
- Good unit tests for `BlockHash` parsing

### `account.rs`
- Balance retrieval logic is split into 3 paths (base, delegation, staking) — good separation
- `view` function is a clean wrapper
- Sequence number fallback to 0 for non-existent accounts is correct per Rosetta spec

### `block.rs`
- `BlockRetriever` needs caching
- Genesis block special-casing is correct per Rosetta spec
- `keep_empty_transactions` metadata flag is a nice optimization

### `construction.rs`
- 8 route definitions are clean
- `fill_in_operator` has duplicated logic between SetOperator and SetVoter
- Gas estimation via simulation is well-implemented
- `construction_payloads` validation is extremely verbose and repetitive
- Transaction parse supports many entry function types — good coverage

### `network.rs`
- Clean implementation
- Version numbers are hardcoded (`NODE_VERSION = "0.1"`) — should come from node
- `timestamp_start_index: 2` needs better documentation about why

### `client.rs`
- Full E2E client for testing — very useful
- Contains assertions (`assert_eq!`, `assert!`) that panic on failure — appropriate for test client but should be documented
- `parse_not_same` flag is a workaround that should be better explained

### `types/objects.rs`
- By far the largest and most complex file (~3000 lines)
- `InternalOperation` is the central enum for all supported operations
- Write set parsing is comprehensive but complex
- Event filtering supports both V1 and V2 events — good forward compatibility
- `Operation::Ord` implementation ensures stable ordering — important for Rosetta compatibility

### `types/misc.rs`
- Clean separation of helper types
- `OperationType` ordering is critical and well-documented
- Balance retrieval for delegation pools uses JSON view (not BCS) — inconsistent with other view calls

### `types/move_types.rs`
- Good centralization of Move constants
- Several unused or stale types (e.g., event types for V1 events that may no longer be emitted)
- `UPDATE_COMMISSION_FUNCTION` has the on-chain typo documented

### `types/requests.rs`
- Comprehensive request/response types matching Rosetta spec
- `GasPricePriority` is a nice addition for gas price flexibility
- `MempoolRequest`/`MempoolResponse` types exist but are unused

### `types/identifiers.rs`
- Complex sub-account system for staking (total, active, pending_active, inactive, pending_inactive, commission, rewards, operator, delegation variants)
- Good unit tests for account identifier types
- `is_operator_stake` uses negative logic (everything that isn't something else) — fragile
