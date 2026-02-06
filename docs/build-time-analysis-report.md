# Build Time Analysis Report

This report investigates remaining build time issues after PR #18625's optimization to eliminate the cached-packages custom build step.

## Executive Summary

PR #18625 eliminates the `build.rs` in `aptos-cached-packages` by checking in pre-compiled Move framework artifacts directly. This removes a significant build time bottleneck. However, several other factors continue to impact build times significantly.

**Key findings:**
- The codebase has **269 workspace members** with **457 dependencies**
- **~575,000 lines of Rust code** across the main directories
- Several "hub" crates cause cascading rebuilds when modified
- Heavy use of procedural macros and generic code creates compilation overhead

---

## Impact of PR #18625

### What It Solves
PR #18625 removes the expensive `build.rs` that:
1. Walked all Move source files in 7 framework directories
2. Computed SHA256 hashes of all `.move` and `Move.toml` files
3. Called `ReleaseTarget::Head.create_release()` to compile the entire Move framework
4. This process could take several minutes on each clean build

### New Workflow
- `src/head.mrb` (1.3MB) is now checked into the repository
- Developers modifying Move code must run `scripts/cargo_build_aptos_cached_packages.sh`
- CI validates artifacts are fresh with `--check` flag

### Remaining Overhead from PR #18625
The `move-package` crate still has a `build.rs` that writes a marker file for cache invalidation. This is now unnecessary and could be removed as a follow-up optimization.

---

## Remaining Build Time Bottlenecks

### 1. Hub Dependency Crates (CRITICAL)

These crates are dependencies for many other crates. Changes to them trigger cascading rebuilds.

| Crate | Dependencies | Dependents | Impact |
|-------|-------------|------------|--------|
| `aptos-types` | 76 | 100+ | Very High |
| `aptos-crypto` | 61 | 30+ | High |
| `aptos-framework` | 65+ | 20+ | High |
| `move-core-types` | ~30 | 50+ | High |

**Specific issues with `aptos-types`:**
- 226 `Serialize/Deserialize` derives in `types/src` alone
- Heavy use of `serde`, `serde_with`, `serde_json`
- Depends on arkworks cryptographic libraries (19 crates)
- Core to virtually every subsystem

**Recommendation:** Consider splitting `aptos-types` into smaller, more focused crates to reduce cascading rebuild scope.

### 2. Move Compiler Stack (HIGH)

The Move compiler ecosystem is large and complex:

| Component | Lines of Code | Notes |
|-----------|--------------|-------|
| `move-compiler-v2` | 49,192 | Main compiler |
| `legacy-move-compiler` | Significant | Fallback compiler |
| `move-prover` | 23,493 | Proof generation |
| Total Move tooling | 254,703 | Full `third_party/move/` |

Both V1 and V2 compilers may be compiled in dev builds, doubling compilation time.

**Recommendation:** Feature-gate the legacy compiler and move prover for development builds.

### 3. Block-STM Execution Engine (MEDIUM-HIGH)

The parallel execution engine in `aptos-move/block-executor/` (25,933 lines) uses heavy generics:

```rust
TxnLastInputOutput<T: Transaction, O: TransactionOutput<Txn = T>>
```

These multi-level generic trait bounds cause significant monomorphization bloat at compile time.

**Recommendation:** Consider reducing generic parameters or using trait objects where performance isn't critical.

### 4. Serde Derives (MEDIUM)

The codebase has extensive use of serde:
- **226 `Serialize/Deserialize` derives** in `aptos-types` alone
- **17 crates** explicitly depend on `serde` with derive feature

Each derive generates serialization/deserialization code that must be compiled.

**Recommendation:**
- Use `#[serde(skip)]` for fields that don't need serialization
- Consider `serde_bytes` for byte arrays to reduce generated code

### 5. Clap CLI Derives (MEDIUM)

The `aptos` CLI crate uses `clap` with derives extensively:
- **49 uses** in the aptos CLI crate alone
- **16 crates** depend on clap with derive

**Recommendation:** Consider lazy initialization for rarely-used subcommands.

### 6. Async-Trait Usage (MEDIUM)

**118 occurrences** of `async_trait` across the workspace. Each usage generates boxed future wrapper code.

High-usage areas:
- `state-sync/`
- `executor/`
- `network/`
- `indexer-grpc/`

**Recommendation:** Consider using native async traits (stabilized in Rust 1.75+) where possible.

### 7. Arkworks Cryptographic Libraries (MEDIUM)

The workspace depends on **19 arkworks crates**:
```
ark-bls12-381, ark-bn254, ark-ec, ark-ff, ark-ff-asm, ark-ff-macros,
ark-groth16, ark-poly, ark-relations, ark-serialize, ark-snark, ark-std...
```

These have heavy generics for elliptic curve operations and polynomial arithmetic.

**Recommendation:** Ensure these are compiled with release optimizations even in dev builds to avoid debug-mode slowdowns.

### 8. Shadow-RS Build Info (LOW)

Two crates use `shadow-rs` for build information:
- `crates/aptos-build-info/build.rs`
- `crates/aptos/build.rs`

This inspects git history on every build, adding overhead.

**Recommendation:** Cache git info or make it optional for development.

---

## CI/Docker Build Analysis

### Current CI Build Configuration

The Docker build uses a 64-CPU runner with:
- Separate builder stages for node, tools, and indexer
- BuildKit cache mounts for cargo registry, git, and target directories
- Parallel builds via `docker buildx bake`

### Build Targets

| Builder | Packages | Profile |
|---------|----------|---------|
| `aptos-node-builder` | aptos-node, aptos-forge-cli | release/performance |
| `tools-builder` | aptos CLI, faucet, telemetry, etc. | cli |
| `indexer-builder` | 8 indexer packages | release/performance |

### Observations

1. **Separate target directories per feature set:**
   ```bash
   CARGO_TARGET_DIR="target/${FEATURES:-"default"}"
   ```
   This prevents cache reuse between feature-enabled and default builds.

2. **Framework release built during tools build:**
   ```bash
   cargo run --package aptos-framework -- release
   ```
   This is still being done in `build-tools.sh`, though with PR #18625 it may be redundant for cached-packages.

3. **Multiple profile builds:** CI builds release, performance, and failpoints variants, tripling build time on pushes to main.

---

## Recommendations Summary

### Immediate Wins (Low Effort)

1. **Remove `move-package` build.rs marker file** - No longer needed after PR #18625
2. **Remove framework release from build-tools.sh** - Already cached in head.mrb
3. **Disable shadow-rs in dev builds** - Use feature flag

### Medium-Term Improvements

4. **Split `aptos-types`** into domain-specific crates (consensus-types, network-types, etc.)
5. **Feature-gate Move prover** - Only compile when explicitly needed
6. **Feature-gate legacy Move compiler (V1)** - Default to V2 only
7. **Upgrade to native async traits** where async-trait is used
8. **Share target directory** between feature variants where possible

### Long-Term Architectural Changes

9. **Reduce generic complexity** in block-executor and MVHashMap
10. **Consider workspace inheritance** for common dependencies to reduce duplication
11. **Implement incremental compilation caching** across CI runs (sccache or similar)
12. **Profile actual build times** with `cargo build --timings` to identify specific bottlenecks

---

## Estimated Impact

| Optimization | Estimated Build Time Reduction |
|-------------|-------------------------------|
| PR #18625 (cached-packages) | 30-50% (already implemented) |
| Remove redundant framework builds | 5-10% |
| Split aptos-types | 10-20% (cascading rebuild reduction) |
| Feature-gate prover/V1 compiler | 10-15% |
| Async trait migration | 3-5% |
| Sccache integration | 20-40% (incremental builds) |

---

## Conclusion

PR #18625 addresses the single largest build time bottleneck by eliminating the Move framework compilation during builds. The remaining optimizations require more architectural changes but could cumulatively reduce build times by an additional 30-50%.

The most impactful next steps are:
1. Clean up the now-unnecessary build.rs in move-package
2. Remove redundant framework release in build-tools.sh
3. Plan for splitting aptos-types to reduce cascading rebuilds
4. Investigate sccache or similar for CI caching

---

## Appendix: Build Time Profiling Commands

```bash
# Generate build timing report
cargo build --timings -p <package>

# Profile with cargo-bloat
cargo bloat --release -p <package>

# Check dependency tree
cargo tree -p <package> --depth 2

# Find unused dependencies
cargo machete

# Check feature unification issues
cargo tree -e features
```

---

*Report generated: 2026-02-06*
*Context: Investigation of build times after PR #18625*
*Slack thread: https://aptos-org.slack.com/archives/C03N9HNSUB1/p1770363912230209*
