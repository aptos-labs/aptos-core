# aptos-rosetta — Test Coverage Baseline (Phase 1)

Measured with `cargo llvm-cov -p aptos-rosetta --summary-only` after Phase 1
(characterization suite). This is the baseline the Phase 2 rewrite must not
regress.

> **Note on staleness.** The per-file table below was measured against the
> *pre-split* layout, so it still lists `construction.rs` (~29%) and
> `types/objects.rs` (~26%) as single files. BC-7 split both into submodules; the
> covered lines moved with them but were not re-measured per submodule, so read
> those two rows as covering their whole submodule trees
> (`construction/*.rs`, `types/objects/*.rs`). Re-run the command above for a
> current per-file breakdown. The **overall** figure and the deferred-coverage
> analysis below are unaffected by the split.

## How to reproduce

```bash
cargo install cargo-llvm-cov --locked   # one-time
rustup component add llvm-tools-preview  # one-time
cargo llvm-cov -p aptos-rosetta --summary-only
```

## Baseline (line coverage)

| File | Lines cover | Notes |
|---|---|---|
| `network.rs` | 100% | fully characterized (routes + options/list/status) |
| `block.rs` | ~95% | route + block-building + retriever |
| `types/identifiers.rs` | ~80% | identifier construction/parsing |
| `error.rs` | ~76% | full error table pinned (`test/errors.rs`) |
| `common.rs` | ~72% | currency/block-index/BlockHash/Y2K helpers |
| `account.rs` | ~56% | base coin balance + secondary store + stake helpers |
| `types/misc.rs` | ~54% | op types, statuses, stake balance parsing |
| `construction.rs` | ~29% | offline round trips + submit; **staking/delegation parse/preprocess branches deferred** |
| `types/objects.rs` | ~26% | FA txn parsing + transfer/create-account InternalOperation; **staking/delegation txn parsing deferred** |
| `node_client.rs` | ~35% | prod impl exercised via wiremock; several delegators only hit in e2e |
| **overall** | **~50%** | includes non-unit-target files below |

Excluded from the meaningful denominator (not unit-test targets):

- `client.rs` (0%) — the outbound `RosettaClient` used by the CLI / smoke tests,
  not the server. Exercised by `testsuite/smoke-test/src/rosetta.rs`.
- `main.rs` (~2%) — the CLI binary; argument plumbing, exercised by `verify_tool`
  and manual runs.
- `types/requests.rs`, `types/move_types.rs` (0%) — pure data/serde structs,
  exercised transitively wherever they serialize.

## What is deliberately deferred (and why it's safe)

The two big gaps — the **staking and delegation** paths in `construction.rs`
(`parse_set_operator_operation`, `parse_*_delegation_*`, `fill_in_operator`,
`simulate_transaction`/`construction_metadata` online path) and
`types/objects.rs` (`from_transaction` for staking events, `InternalOperation`
extract/payload for the 11 staking op types) — are:

1. **Covered end-to-end** by `testsuite/smoke-test/src/rosetta.rs`
   (`test_delegation_pool_operations`, `test_transfer`, staking helpers) against a
   real `LocalSwarm` node.
2. **Still deferred after Phase 2 — carried forward.** The original plan was that
   each new submodule would land with its own unit tests when `objects.rs` and
   `construction.rs` were split. The split (BC-7) shipped; the per-submodule
   staking/delegation unit tests did **not**. Splitting the files was kept
   strictly verbatim to keep that change reviewable, and adding tests at the same
   time would have made it impossible to tell a move from a behavior change.

   So this gap is unchanged from the Phase 1 baseline: staking and delegation
   remain smoke-test-only. The `MockNodeClient` seam is now in place, and
   `construction/` and `types/objects/` are now small enough to test per file, so
   the follow-up is unblocked — it is simply not done here. Anyone picking it up
   should start with `construction/helpers.rs` (`fill_in_operator`,
   `simulate_transaction`) and `types/objects/internal_op.rs`
   (`extract`/`payload` for the 11 staking ops), which carry the most untested
   branching.

The Phase 1 suite already characterizes: every endpoint at least once (offline
via routes, online via `MockNodeClient`), the complete 36-entry error table, the
full offline construction round trip (preprocess→payloads→parse→combine→hash) for
APT transfer and create-account, and the real `RestNodeClient` HTTP path
(success + error mapping) via `wiremock`.

## Test inventory (`src/test/`)

- `mod.rs` — FA transaction parsing (mint/transfer/fee-payer/storage-refund).
- `errors.rs` — full error-table golden + HTTP-500 + uniqueness + `all()`-covers-
  every-variant guard (BC-3/BC-6).
- `handlers.rs` — offline network endpoints via routes; mock-driven online
  helpers; `OperationType::all()`-covers-every-variant guard (BC-6).
- `construction.rs` — offline construction round trips + rejections + derive.
- `online.rs` — network/status, block, account/balance, submit via `MockNodeClient`.
- `wire.rs` — real `RestNodeClient` over `wiremock` (parse + error propagation).
