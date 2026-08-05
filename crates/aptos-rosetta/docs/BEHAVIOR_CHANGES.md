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

### BC-3 — Single source of truth for the error table · DONE
`code()`, `message()`, and `retriable()` were three parallel `match` blocks whose
ordering disagreed (e.g. `GasEstimationFailed` is declared 10th but has code 16).
They are now thin accessors over one private `ApiError::info() -> ErrorInfo`
table, so the code, message, and retriable flag for a variant live together and
cannot drift apart. `From<ApiError> for types::Error` destructures `info()` once
rather than calling all three accessors.

**No wire change** — the emitted `code`/`message`/`retriable` values are
unchanged (pinned by `test::errors::error_table_matches_golden`); only the
internal structure changed.

Two siblings are deliberately *not* folded into `info()`:

- `details()` returns a per-variant payload (`Option<String>`), not static
  metadata, so it cannot live in a `&'static str` table. It is now an
  **exhaustive** match (no `_ => None` arm) per the repo's match convention, so a
  new variant must state whether it surfaces details. Expanding that wildcard
  revealed that `StateValueNotFound`, `RejectedByFilter`, and `RateLimited` each
  carry an `Option<String>` that has never reached the wire — see BC-8.
- `all()` is a hand-maintained `Vec` because its *order* is wire-visible. It is
  now guarded by `test::errors::all_lists_every_api_error_variant`, which
  compares it against a `cfg(test)`-only `strum` `EnumIter` derive, so a variant
  can no longer be added without appearing in `all()`.

---

## Block cache naming (`src/lib.rs`, `src/block.rs`)

### BC-4 — Rename `BlockRetriever`/`block_cache` · DONE
Renamed the `RosettaContext.block_cache` field and `block_cache()` accessor to
`block_retriever`, and corrected the misleading "cache" doc comments. No caching
was added (the retriever still hits the node every call) and there is no wire
change; this is internal-only. Decision: rename (not add a cache) — a real cache
can come later if profiling shows it's needed.

---

## Operation types (`src/types/misc.rs`)

### BC-6 — `update_commission` missing from `OperationType::all()` · DONE
`OperationType` has 15 variants but `OperationType::all()` listed only 14 —
`UpdateCommission` was omitted, so `/network/options` never advertised
`update_commission` even though the type is fully parsed and produced elsewhere
(`InternalOperation::UpdateCommission`, `parse_update_commission_operation`).
`UpdateCommission` was added to `all()`.

Risk: wire-visible — `network/options.allow.operation_types` grew from 14 to 15
entries. Pinned at 15 by
`test::handlers::network_options_matches_documented_deviations`.

Recurrence guard: `all()` stays hand-maintained (its order is wire-visible, and
the enum's declaration order differs — `Fee` is last in the enum but 4th in
`all()`), so `test::handlers::all_lists_every_operation_type_variant` compares it
against a `cfg(test)`-only `strum` `EnumIter` derive. A `len()` assertion alone
could not catch the next omission; this can.

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

## Dropped error details (`src/error.rs`)

### BC-8 — Three errors silently discard their details · DEFERRED
Making `details()` exhaustive (BC-3) surfaced that three variants carry an
`Option<String>` which never reaches the wire, because the old `_ => None` arm
swallowed them:

| code | variant | populated by |
|---|---|---|
| 34 | `StateValueNotFound` | `From<RestError>`, from the node's message |
| 35 | `RejectedByFilter` | `From<RestError>`, from the node's message |
| 36 | `RateLimited` | `From<RestError>`, from the node's message |

So a caller hitting a rate limit or a filter rejection gets the generic message
with no `details`, even though the node explained why. This is a genuine (if
minor) information loss.

**Deferred, not fixed here.** Surfacing them adds a `details` field to those
three error responses — a wire change, which this changelog's own rules say needs
its own entry plus a golden update, and which is out of scope for a restructuring
change. The arms are written out explicitly with a comment so the omission is now
visible in the source rather than hidden behind a wildcard. Fixing it is a
one-line change per variant once someone decides the wire change is acceptable.

---

## Phase 2 outcome

Beyond the entries above, Phase 2 was **behavior-preserving**. The wire-visible
changes are exactly BC-1 (error message typos), BC-2 (`InvalidTransactionUpdate`
code 28→29), and BC-6 (`update_commission` added to `network/options`); BC-3/4/7
are internal-only, BC-5 is WONTFIX, BC-8 is deferred. No operation-ordering,
fee-attribution, FA-resolution, construction-flow, or error-path changes were
introduced by the rewrite.
