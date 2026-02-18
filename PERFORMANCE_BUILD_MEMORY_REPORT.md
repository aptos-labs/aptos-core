# Aptos Core: Performance, Build Times & Memory Report

**Date:** February 12, 2026
**Branch:** `cursor/performance-build-memory-bf8c`
**Scope:** Workspace-wide analysis of build time bottlenecks, runtime memory patterns, and low-hanging performance fruit.

---

## Executive Summary

This report examines the Aptos Core monorepo (264 workspace crates) for opportunities to improve build times, reduce memory usage, and improve runtime performance. The analysis covers dependency management, build profiles, hot-path code patterns, and compile-time bloat.

Key findings:
- **~157 duplicate dependency versions** inflate both build time and binary size
- `debug = true` in release profile generates full DWARF debug info, significantly increasing link times and binary size
- `syn v1` is still compiled alongside `syn v2` (45 vs 84 dependents) -- eliminating syn v1 would save meaningful compile time
- The `aptos-types` crate is a "god crate" (23,250 LOC, ~115 dependent crates) that pulls in heavy crypto deps like `aptos-dkg`, `ark-*`, and `jsonwebtoken` into nearly everything
- The block executor hot path has 60+ `RefCell::borrow/borrow_mut` calls in `view.rs` alone, along with heavy `HashMap`-based `CapturedReads` rebuilt per transaction
- `performance` profile uses `codegen-units = 1` with `lto = "thin"` -- this is extremely slow to build

---

## 1. Build Time Analysis

### 1.1 Build Profiles

| Profile | opt-level | debug | LTO | codegen-units | overflow-checks |
|---------|-----------|-------|-----|---------------|-----------------|
| release | (default=3) | **true** | none | (default=16) | true |
| performance | 3 | **true** | thin | **1** | true |
| cli | "z" | false | thin | 1 | (default) |
| ci | (default=3) | line-tables-only | none | (default=16) | true |
| bench | (default=3) | **true** | none | (default=16) | (default) |

**Findings:**

1. **`debug = true` in release and performance profiles**: Full DWARF debug info is extremely expensive. For `release`, this approximately doubles link time and can add 2-5x to binary size. Consider:
   - **LOW RISK**: Change `release` profile to `debug = "line-tables-only"` (like `ci`) -- this still enables `addr2line` backtraces but removes type/variable debug info. Saves ~30-50% link time.
   - If full debug info is needed for profiling, keep it only in `bench` or a dedicated `debug-release` profile.

2. **`performance` profile: `codegen-units = 1`**: This forces the entire crate to compile as a single codegen unit, which means **zero parallelism during codegen** for each crate. Combined with `lto = "thin"`, this makes the performance profile extremely slow to build.
   - **RECOMMENDATION**: Consider `codegen-units = 2` or `4` for the `performance` profile. The performance difference vs `1` is typically <2%, but build time can drop 30-50%.

3. **`build-override` opt-level 3**: The workspace already has `[profile.release.build-override] opt-level = 3` which is good -- it prevents double-compilation of the `aptos-cached-packages` build dependency tree.

### 1.2 Duplicate Dependencies (Build Time & Binary Bloat)

`cargo tree -d` for `aptos-node` reveals **~157 duplicated crate versions**. Key offenders:

| Crate | Versions | Impact |
|-------|----------|--------|
| `rand` | 0.7.3, 0.8.5, 0.9.1 | 3 copies of a foundational crate |
| `rand_core` | 0.5.1, 0.6.4, 0.9.3, 0.10.0-rc-2 | 4 versions! |
| `digest` | 0.9.0, 0.10.7, 0.11.0-rc.4 | 3 copies, each pulling `block-buffer`, `crypto-common` |
| `sha2` | 0.9.9, 0.10.8, 0.11.0-rc.3 | 3 copies of a heavy crypto crate |
| `curve25519-dalek` | 3.2.0, 4.1.2 | 2 copies of a large crypto lib |
| `ed25519-dalek` | 1.0.1, 2.1.1 | 2 copies |
| `syn` | 1.0.109, 2.0.87 | 45+84 dependents -- both compiled |
| `http` | 0.2.11, 1.1.0 | 2 copies |
| `hyper` | 0.14.28, 1.4.1 | 2 copies of a large HTTP library |
| `rustls` | 0.21.10, 0.22.4, 0.23.7 | 3 copies of TLS stack |
| `ring` | 0.16.20, 0.17.7 | 2 copies of crypto (includes C code, slow build) |
| `dashmap` | 5.5.3, 7.0.0-rc2 | 2 copies |
| `indexmap` | 1.9.3, 2.7.0 | 2 copies |
| `hashbrown` | 0.12.3, 0.14.3, 0.15.3 | 3 copies |
| `itertools` | 0.10.5, 0.12.1, 0.13.0 | 3 copies |
| `clap` | 2.34.0, 4.5.21 | 2 copies |
| `base64` | 0.12.3, 0.13.1, 0.21.7, 0.22.1 | 4 versions! |
| `rsa` | 0.6.1, 0.9.6 | 2 copies (heavy) |
| `der` | 0.5.1, 0.7.8, 0.8.0 | 3 copies |
| `pkcs8` | 0.8.0, 0.10.2, 0.11.0 | 3 copies |

**RECOMMENDATION (HIGH IMPACT)**: Prioritize unifying:
- `rand` ecosystem (0.7 -> 0.8): The workspace pins `rand = "0.7.3"` -- upgrading to 0.8 would eliminate an entire parallel rand stack.
- `digest`/`sha2` (0.9 -> 0.10): The workspace pins `sha2 = "0.9.3"` and `digest = "0.9.0"` but also has `sha2_0_10_6`. Standardizing would remove duplicate crypto stacks.
- `ring` (0.16 -> 0.17): ring compiles C/ASM code and is one of the slowest deps to build.
- `base64` (0.13 -> 0.22): 4 versions is excessive.
- `rustls` (consolidate to one version): 3 TLS implementations is wasteful.
- `hyper`/`http` (0.14 -> 1.x): Would eliminate the old http/hyper stack.

**Estimated build time savings**: 10-20% for clean builds by reducing duplicate compilation.

### 1.3 Proc Macro / Compile-Time Cost

- **`syn` v1 + v2**: Both compiled (45 + 84 dependents). The workspace explicitly depends on `syn v1.0.92` for `aptos-crypto-derive`, `aptos-log-derive`, etc. Migrating proc-macro crates to `syn v2` would eliminate `syn v1` entirely.
- **`derivative` crate**: Used in 7 workspace crates. This is a proc-macro that generates `Clone`, `Debug`, `Default`, `PartialEq`, etc. with customization. Consider replacing with standard derives + manual impls where the customization is minimal. The `derivative` crate is no longer actively maintained and pulls in `syn v1`.
- **`shadow-rs`**: Used in `aptos` CLI and `aptos-build-info`. Runs `git` commands at build time which can cause unnecessary rebuilds. The `aptos-build-info` crate already has `rerun-if-changed` guards, which is good.

### 1.4 The `aptos-types` Monolith

`aptos-types` is 23,250 lines of Rust and is depended on by **~115 other workspace crates**. Any change to `aptos-types` triggers recompilation of nearly the entire workspace.

Heavy dependencies pulled into `aptos-types`:
- `aptos-dkg` (DKG cryptography)
- `ark-bn254`, `ark-ec`, `ark-ff`, `ark-groth16`, `ark-serialize`, `ark-std` (arkworks crypto suite)
- `jsonwebtoken` (JWT validation)
- `ring`, `rsa` (cryptographic verification)
- `poem-openapi`, `poem-openapi-derive` (OpenAPI codegen)
- `move-model` (Move language model, very large)

**RECOMMENDATION (HIGH IMPACT)**: Split `aptos-types` into smaller crates:
- `aptos-types-core`: Basic types (state keys, state values, transaction structures, chain ID, etc.)
- `aptos-types-crypto`: Keyless, DKG, JWK types that depend on heavy crypto
- `aptos-types-api`: Types with OpenAPI derives
- This would allow most crates to depend only on `aptos-types-core`, avoiding recompilation cascades when crypto or API types change.

---

## 2. Runtime Memory & Performance Analysis

### 2.1 Block Executor Hot Path (`aptos-move/block-executor/`)

The block executor is the performance-critical parallel execution engine (Block-STM). Key observations:

**`CapturedReads` struct** (`captured_reads.rs:558`):
```rust
pub(crate) struct CapturedReads<T, K, DC, VC, S> {
    data_reads: HashMap<T::Key, DataRead<T::Value>>,
    group_reads: HashMap<T::Key, GroupRead<T>>,
    delayed_field_reads: HashMap<DelayedFieldID, DelayedFieldRead>,
    aggregator_v1_reads: HashSet<T::Key>,
    module_reads: hashbrown::HashMap<K, ModuleRead<DC, VC, S>>,
    // ... flags ...
}
```

- This struct is **created fresh for every transaction execution** and wraps 5 hash maps/sets.
- The `HashMap::new()` calls allocate on creation. Consider using `with_capacity()` hints based on typical transaction read counts, or reusing/clearing these maps across re-executions.
- The `data_reads` HashMap has **62 `.clone()` calls** across the captured_reads module.

**`RefCell` in view.rs**:
- **60 `borrow()`/`borrow_mut()` calls** in `view.rs` for `CapturedReads` wrapped in `RefCell`. While `RefCell` is correct for interior mutability, each borrow/borrow_mut involves a runtime check. In the parallel execution path, this is called per-read, per-transaction.
- Consider whether some of these can be restructured to hold a mutable reference directly (reducing borrow overhead).

**`TriompheArc` cloning**:
- The codebase uses `triomphe::Arc` (lighter than `std::Arc`) for `DataRead::Versioned` values. However, `versioned_convert_to` clones both the Arc and the layout on every read conversion. For high-read transactions, this creates many atomic increment/decrements.

**Lock contention** (`scheduler_v2.rs`):
- 35 `.lock()` calls in `scheduler_v2.rs`, 29 in `scheduler_status.rs`, 9 in `scheduler.rs`. The scheduler is central to Block-STM coordination. Profiling lock contention under load would be valuable.

### 2.2 `aptos-types` Runtime Patterns

**`StateKey` design** (`types/src/state_store/state_key/mod.rs`):
```rust
pub struct StateKey(Arc<Entry>);
```
StateKey uses `Arc<Entry>` with a global registry, which is a good interning pattern for reducing duplicates. This is well-designed for memory efficiency.

**`StateValue` design** (`types/src/state_store/state_value.rs`):
```rust
pub struct StateValue {
    data: Bytes,              // ref-counted, cheap clone
    metadata: StateValueMetadata,
    maybe_rapid_hash: Option<(u64, usize)>,
}
```
- Uses `Bytes` (ref-counted) for data -- good for clone efficiency.
- Has `rapid_hash` for fast equality checks -- good optimization.
- `StateValueMetadata` has `Option<StateValueMetadataInner>` where inner is 24 bytes. Total `StateValue` is ~56 bytes + the Bytes allocation. Reasonable.

**`Transaction` enum** (`types/src/transaction/mod.rs:3038`):
```rust
#[allow(clippy::large_enum_variant)]
pub enum Transaction {
    UserTransaction(SignedTransaction),
    GenesisTransaction(WriteSetPayload),
    BlockMetadata(BlockMetadata),
    // ... more variants
}
```
- The `large_enum_variant` lint is suppressed. This means all `Transaction` values are sized to the largest variant. If `SignedTransaction` is much larger than other variants, all other variants waste that space. Consider `Box`-ing the largest variant(s).

### 2.3 Consensus Layer

**Clone intensity**: 1,350+ `.clone()` calls across consensus source. While not all are in hot paths, the `epoch_manager.rs` (81 clones), `round_manager.rs` (45 clones), `pipeline/buffer_item.rs` (48 clones), and `pipeline/pipeline_builder.rs` (63 clones) warrant review.

**Observation**: `consensus_observer/observer/subscription_utils.rs` has 40 clones and `subscription_manager.rs` has 63 clones -- these may be creating excessive copies of subscription state.

### 2.4 Storage Layer (`storage/aptosdb/`)

- `state_store/mod.rs` has 33 `.clone()` calls and 8 `.lock()` calls.
- The speculative state workflow test shows 44 clones -- confirming that the state store path involves significant data copying.
- `with_capacity` is used in only 12 places across the storage layer -- there may be opportunities to pre-size allocations in batch operations.

### 2.5 Mempool

- `shared_mempool/use_case_history.rs` has 24 clones -- the use-case history tracking may be doing excessive copying.
- `shared_mempool/priority.rs` has 20 clones and 4 `collect::<Vec>` calls -- the priority ordering logic may benefit from in-place operations.

---

## 3. Low-Hanging Fruit Recommendations

### 3.1 Build Time (Estimated Impact: HIGH)

| # | Action | Est. Build Time Saving | Effort |
|---|--------|----------------------|--------|
| 1 | Change `[profile.release] debug = "line-tables-only"` | 15-30% link time | Trivial |
| 2 | Upgrade `rand` from 0.7 to 0.8 workspace-wide | 3-5% (eliminates rand 0.7 tree) | Medium |
| 3 | Upgrade `digest`/`sha2` from 0.9 to 0.10 | 3-5% (eliminates old crypto tree) | Medium |
| 4 | Unify `base64` to single version (0.22) | 1-2% | Low |
| 5 | Migrate proc-macro crates from `syn v1` to `syn v2` | 2-5% (eliminates syn 1.x) | Medium |
| 6 | Replace `derivative` with standard derives | 1-2% + eliminates syn v1 dep | Medium |
| 7 | Set `performance` profile `codegen-units = 2` or `4` | 30-50% for performance builds | Trivial |
| 8 | Split `aptos-types` into core/crypto/api | Large incremental build savings | High |
| 9 | Consolidate `ring` to single version (0.17) | 2-3% (ring is slow to compile) | Medium |
| 10 | Consolidate `hyper`/`http` to v1 | 2-3% | Medium-High |

### 3.2 Runtime Memory (Estimated Impact: MEDIUM)

| # | Action | Est. Memory Saving | Effort |
|---|--------|-------------------|--------|
| 1 | Pre-size `CapturedReads` HashMaps with `with_capacity` | Fewer reallocations per txn | Low |
| 2 | Pool/reuse `CapturedReads` across re-executions | Eliminates alloc/dealloc churn | Medium |
| 3 | Box large `Transaction` enum variants | Reduces stack size of all Transaction values | Low |
| 4 | Audit consensus cloning hotspots (epoch_manager, pipeline) | Reduces heap pressure | Medium |
| 5 | Review `RefCell` borrow patterns in block executor `view.rs` | Reduces runtime checks | Medium |

### 3.3 Runtime Performance (Estimated Impact: MEDIUM)

| # | Action | Est. Perf Improvement | Effort |
|---|--------|----------------------|--------|
| 1 | Profile lock contention in `scheduler_v2.rs` (35 locks) | Potential parallel speedup | Medium |
| 2 | Reduce `TriompheArc` cloning in `DataRead::convert_to` | Fewer atomic ops per read | Low |
| 3 | Use `Cow` instead of clone for `DataRead` conversions | Avoids unnecessary copies | Medium |
| 4 | Pre-size Vecs in storage batch operations | Fewer reallocations | Low |

---

## 4. Configuration Observations

### 4.1 Good Practices Already in Place
- `triomphe::Arc` used instead of `std::Arc` where appropriate (lighter, no weak refs)
- `StateKey` uses an interning registry (Arc<Entry>) to deduplicate
- `StateValue` uses `Bytes` (ref-counted) for cheap clones
- `rapidhash` used for fast equality pre-checks on StateValue
- `lld` linker configured for Linux x86_64 builds
- `target-cpu=x86-64-v3` for x86_64 Linux (AVX2 instructions)
- Build-override `opt-level = 3` for build-deps (avoids double-compilation)
- `dashmap` 7.0 with `inline-more` feature for hot concurrent maps

### 4.2 Potential Risks
- The `[patch.crates-io]` section patches `futures`, `ark-*`, `merlin`, and `jemalloc-sys` to forked repos. These forks may drift from upstream and block dependency upgrades.
- Using `dashmap = "7.0.0-rc2"` (release candidate) in production code.
- `reqwest` has `blocking` feature enabled workspace-wide, which pulls in an additional blocking HTTP runtime even for async-only crates.

---

## 5. Quick Wins Summary

If you want to start with the highest-impact, lowest-effort changes:

1. **`debug = "line-tables-only"` in release profile** -- one line change, major link time savings
2. **`codegen-units = 2` in performance profile** -- one line change, major build time savings for perf builds
3. **Pre-size `CapturedReads` HashMaps** -- small code change, reduces per-transaction allocation churn
4. **Box the largest `Transaction` enum variants** -- reduces stack usage for all transaction handling
5. **Upgrade `base64` to single version** -- simple dependency cleanup

---

## 6. Dependency Deduplication Plan & Progress

### 6.1 Completed

| Dependency | Old Version | New Version | Impact |
|-----------|-------------|-------------|--------|
| `base64` | 0.13.0 | 0.22.1 | Eliminates one of 4 versions; workspace code now on modern API |

### 6.2 Attempted and Reverted

| Dependency | Attempted | Reason for Revert |
|-----------|-----------|-------------------|
| `criterion` | 0.3.5 → 0.5.1 | `criterion-cpu-time 0.1.0` implements `Measurement` trait for criterion 0.3 only; no updated version available |

### 6.3 Remaining Duplicates -- Classification & Plan

The following table classifies all remaining duplicate dependencies by whether they're **directly controlled** (workspace Cargo.toml declares them) or **transitively pulled** (from external crates we depend on).

#### Crypto Stack (Deeply Interconnected -- Requires Dedicated Project)

These form a single interconnected graph through `aptos-crypto`:

| Dependency | Versions | Root Cause | Fix Approach |
|-----------|----------|------------|-------------|
| `rand` | 0.7, 0.8, 0.9 | Workspace pins 0.7; `ed25519-dalek` 1.0 requires 0.7 | Upgrade `ed25519-dalek` to 2.x + `rand` to 0.8 |
| `rand_core` | 0.5, 0.6, 0.9, 0.10-rc | Follows rand versions | Fixed by rand upgrade |
| `rand_chacha` | 0.2, 0.3, 0.9 | Follows rand versions | Fixed by rand upgrade |
| `digest` | 0.9, 0.10, 0.11-rc | Workspace pins 0.9; `sha2 0.9` needs it | Upgrade `sha2`/`digest` to 0.10 |
| `sha2` | 0.9, 0.10, 0.11-rc | Workspace pins 0.9 | Upgrade to 0.10 |
| `sha3` | 0.9, 0.11-rc | Workspace pins 0.9 | Upgrade to 0.10 |
| `hmac` | 0.8, 0.10, 0.12, 0.13-rc | Various crypto crates | Fixed by digest upgrade |
| `hkdf` | 0.10, 0.12 | Workspace pins 0.10 | Fixed by digest upgrade |
| `ed25519-dalek` | 1.0, 2.1 | Workspace pins 1.0 | Major upgrade (signature API change) |
| `curve25519-dalek` | 3.2, 4.1 | `ed25519-dalek` 1.0 needs v3 | Fixed by ed25519-dalek upgrade |
| `signature` | 1.6, 2.2, 3.0-rc | Crypto crate version splits | Fixed by crypto stack upgrade |
| `ring` | 0.16, 0.17 | Workspace pins 0.16; rustls uses 0.17 | Upgrade ring to 0.17 (part of crypto upgrade) |
| `rsa` | 0.6, 0.9 | 0.6 from `google-cloud-storage` | Update `google-cloud-storage` dependency |
| `getrandom` | 0.1, 0.2, 0.3 | Follows rand | Fixed by rand upgrade |
| `block-buffer` | 0.9, 0.10, 0.11 | Follows digest | Fixed by digest upgrade |
| `crypto-common` | 0.1, 0.2-rc | Follows digest | Fixed by digest upgrade |

**Estimated effort**: 2-4 weeks of dedicated work by a developer familiar with `aptos-crypto`.
**Risk**: HIGH -- security-critical cryptographic code.
**Recommendation**: Plan as a dedicated project with thorough review and testing.

#### HTTP/TLS Stack (External Crate Upgrades)

| Dependency | Versions | Root Cause | Fix Approach |
|-----------|----------|------------|-------------|
| `http` | 0.2, 1.1 | `reqwest` 0.11 uses http 0.2; `hyper` 1.x uses http 1.x | Upgrade `reqwest` to 0.12+ |
| `hyper` | 0.14, 1.4 | Workspace pins 0.14 | Upgrade to 1.x |
| `rustls` | 0.21, 0.22, 0.23 | Various TLS consumers | Consolidate to 0.23 |
| `tokio-rustls` | 0.24, 0.25, 0.26 | Follows rustls | Fixed by rustls consolidation |
| `h2` | 0.3, 0.4 | From old/new hyper | Fixed by hyper upgrade |
| `cookie` | 0.16, 0.18 | From warp 0.3 vs poem | Update warp or drop it |

**Estimated effort**: 1-2 weeks -- `reqwest` 0.12 and `hyper` 1.x have API changes.
**Risk**: MEDIUM -- networking code changes but no cryptographic risk.

#### Serialization (Fork Dependencies)

| Dependency | Versions | Root Cause | Fix Approach |
|-----------|----------|------------|-------------|
| `serde_yaml` | 0.8, 0.9 | Workspace uses 0.8; `poem` uses 0.9; `serde-generate` fork pins 0.8 | Update Aptos `serde-generate` fork |
| `indexmap` | 1.9, 2.7 | `serde_yaml 0.8` and `tower 0.4` use 1.x | Fixed by serde_yaml + tower upgrade |
| `bcs` | 0.1.4, 0.1.6 | Aptos fork vs processor SDK version | Align versions |

**Estimated effort**: 1 week -- `serde_yaml::Value` behavior changed between 0.8 and 0.9.
**Risk**: MEDIUM -- config parsing might break subtly.

#### Misc Crates (Mostly Transitive, Low Priority)

| Dependency | Versions | Root Cause | Can Fix? |
|-----------|----------|------------|---------|
| `syn` | 1.0, 2.0 | `derivative` and old proc macros use syn 1 | Replace `derivative` with manual impls |
| `num-bigint` | 0.2, 0.3, 0.4 | 0.3 from workspace (tied to rand 0.7); 0.2 from `simple_asn1`; 0.4 transitive | Fixed by rand upgrade |
| `hashbrown` | 0.12, 0.14, 0.15 | Various transitive deps | Partially -- upgrade workspace to 0.15 |
| `itertools` | 0.10, 0.12, 0.13 | 0.10 from `criterion 0.3`; 0.12 from `yup-oauth2` | Fixed by criterion upgrade |
| `base64` | 0.12, 0.21, 0.22 | 0.12 from `cloud-storage`; 0.21 from `reqwest`/`rustls-pemfile` | Fixed by reqwest/cloud-storage upgrade |
| `clap` | 2.34, 4.5 | `criterion 0.3` and `dudect-bencher` use clap 2 | Drop or fork criterion-cpu-time for criterion 0.5 |
| `bitflags` | 1.3, 2.9 | Old crates use 1.x | Most migration to 2.x is trivial |
| `strum` | 0.25, 0.27 | `passkey-types` uses 0.25 | Wait for passkey-types update |
| `derive_more` | 0.99, 1.0 | Transitive from various crates | Wait for ecosystem migration |

---

*Report generated by automated analysis of the Aptos Core repository.*
