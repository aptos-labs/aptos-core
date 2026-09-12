# aptos-rosetta — Behavior Changes (Rewrite Changelog)

This file records every **intentional** behavior change made while rewriting
`aptos-rosetta` for legibility. The rewrite policy is *"fix quirks and bugs, but
document every change."*

Rules for this file:

- Wire-visible structure (routes, JSON field names, error `code`/`retriable`
  values) stays identical **unless an entry below says otherwise**.
- Every entry that changes wire output MUST have a corresponding updated golden
  test committed in the same change.
- Deviations from the Rosetta spec that are *preserved* (not changed) live in
  [`SPEC_DEVIATIONS.md`](./SPEC_DEVIATIONS.md), not here.

Status legend: **PLANNED** (agreed, not yet done) · **DONE** (implemented +
golden updated) · **WONTFIX** (documented, left as-is).

---

## Error module (`src/error.rs`)

### BC-1 — Fix typo'd error messages · DONE
The Rosetta spec requires the `message` for a given error `code` to be stable.
These messages contain typos/grammar issues. The codes are unchanged; only the
human-readable strings change.

| code | variant | before | after |
|---|---|---|---|
| 14 | `NodeIsOffline` | `This API is unavailable for the node because he's offline` | `This API is unavailable because the node is offline` |
| 25 | `BlockNotFound` | `Block is missing events` | `Block not found` |
| 33 | `CoinTypeFailedToBeFetched` | `Faileed to retrieve the coin type information, please retry` | `Failed to retrieve the coin type information, please retry` |

Risk: an integrator string-matching on `message` (rather than `code`) would
break. Rosetta explicitly ties semantics to `code`, so this is considered safe.

### BC-2 — Fix `InvalidTransactionUpdate` mis-mapping · DONE
`From<RestError>` maps `AptosErrorCode::InvalidTransactionUpdate` to
`ApiError::InvalidInput` (code 28) instead of `ApiError::InvalidTransactionUpdate`
(code 29) (`src/error.rs:305-307`). After the fix it maps to
`InvalidTransactionUpdate` (code 29).

Risk: wire-visible — the returned `code` changes 28 → 29 for this node error.
This is a genuine bug fix; documented here because it is technically a wire
change.

### BC-3 — Single source of truth for error tables · DONE
`code()`, `message()`, `details()`, `retriable()`, and `all()` are currently four
parallel `match` blocks whose ordering disagrees (e.g. `GasEstimationFailed` is
declared 10th but has code 16). Consolidate into one table/definition so the
number, message, retriable flag, and details accessor for each variant live
together. **No wire change** — the emitted `code`/`message`/`retriable` values
stay exactly the same (verified by the error golden tests); only the internal
structure changes.

---

## Block cache naming (`src/lib.rs`, `src/block.rs`)

### BC-4 — Rename `BlockRetriever`/`block_cache` · DONE
Renamed the `RosettaContext.block_cache` field and `block_cache()` accessor to
`block_retriever`, and corrected the misleading "cache" doc comments. No caching
was added (the retriever still hits the node every call) and there is no wire
change; this is internal-only. Decision: rename (not add a cache) — a real cache
can come later if profiling shows it's needed.

`BlockRetriever` is described as a cache but performs no caching
(`src/lib.rs:143`, `src/block.rs:167-172`). Rename to reflect reality (e.g.
`BlockRetriever` stays but the `block_cache` field/accessor become
`block_retriever`), or add a real cache. Internal-only; **no wire change**.
Decision on "rename vs. add caching" to be recorded here when made.

---

## Operation types (`src/types/misc.rs`)

### BC-6 — `update_commission` missing from `OperationType::all()` · DONE
`OperationType` has 15 variants but `OperationType::all()`
(`src/types/misc.rs:150-168`) lists only 14 — `UpdateCommission` is omitted. As a
result `/network/options` advertises 14 operation types and never lists
`update_commission`, even though the type is parsed and produced elsewhere. Add
`UpdateCommission` to `all()`.

Risk: wire-visible — `network/options.allow.operation_types` grows from 14 to 15
entries. Pinned at 14 today by
`test::handlers::network_options_matches_documented_deviations`; that assertion
flips to 15 in the same change that fixes `all()`.

---

## On-chain typo (`src/types/move_types.rs`)

### BC-5 — `update_commision` misspelling · WONTFIX
`UPDATE_COMMISSION_FUNCTION = "update_commision"` matches the on-chain entry
function name and cannot change without a framework change
(`src/types/move_types.rs:54-56`). Left as-is; kept in `SPEC_DEVIATIONS.md §13`.
If the on-chain name is ever corrected, handle both spellings.

---

## Module reorganization (`src/types/objects.rs`, `src/construction.rs`)

### BC-7 — Split large modules into submodules · DONE (internal-only, no wire change)
The two largest files were split into focused submodules behind thin re-export
parents, so every `crate::types::*` / `crate::construction::*` path resolves
exactly as before:

- `src/types/objects.rs` (3034 lines) → `objects/{currency,operation,transaction,internal_op}.rs`.
- `src/construction.rs` (1561 lines) → `construction/{routes,combine,derive,hash,metadata,parse,payloads,preprocess,submit,helpers}.rs`.

All items moved verbatim; the only source changes were per-submodule `use` blocks
and widening a handful of cross-submodule helpers to `pub(crate)`. **No wire
change** — routes, JSON, and error codes are byte-identical, verified by the full
characterization suite (41 tests) staying green. Also included: `common.rs`
legibility doc comments citing `SPEC_DEVIATIONS.md` (no code/behavior change).

Citations in `SPEC_DEVIATIONS.md` were repointed to the new submodules.

---

## Phase 2 outcome

Beyond the entries above, Phase 2 was **behavior-preserving**. The wire-visible
changes are exactly BC-1 (error message typos), BC-2 (`InvalidTransactionUpdate`
code 28→29), and BC-6 (`update_commission` added to `network/options`); BC-3/4/7
are internal-only, BC-5 is WONTFIX. No operation-ordering, fee-attribution,
FA-resolution, construction-flow, or error-path changes were introduced by the
rewrite.
