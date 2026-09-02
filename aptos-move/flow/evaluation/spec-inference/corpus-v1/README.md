# Corpus V1 — Aptos framework and experimental

> **Status: superseded as a benchmark; retained as infrastructure.**
> Benchmark rounds run on [`corpus-v3`](../corpus-v3/README.md). V1 was retired
> because its targets are public and so are their specifications — 16 of 24
> checked targets already had a published upstream specification for the very
> function the agent was asked to specify, so a success there may be recall
> rather than inference.
>
> It is kept for three reasons. It is the only **publishable** corpus, since the
> V2/V3 sources cannot be redistributed. It is the only source of
> higher-order/iterator and global-state coverage, which the dependency-light
> private pool structurally lacks. And its authored opaque dependency contracts
> — 8,000-plus specification lines and 357 `modifies` frames — are the proof
> infrastructure behind [`metadata/prover-repairs.md`](metadata/prover-repairs.md).
> See [`DESIGN.md`](../DESIGN.md) for how the three corpora relate.

This is the human-inspectable source catalog for the corpus prepared from Aptos
Core commit `6d836beedc56fc70c54f3b3046d1d248d850c64b`. Every experimental arm receives the same source
hash for a sample; treatment-specific skills and tools are stored separately.

## Metadata

- [`manifest.json`](manifest.json): the 30 prepared sample records and hashes;
  its `corpus_status` is authoritative for round readiness.
- [`metadata/candidate-inventory.json`](metadata/candidate-inventory.json): the
  complete compiler-AST source frame.
- [`metadata/selection.json`](metadata/selection.json): inclusion, exclusion,
  reserve, and replacement decisions.
- [`screening/manifest.json`](screening/manifest.json) and
  [`screening/results/`](screening/results/): compatibility evidence, valid for
  the current corpus only when its identity is recorded by `manifest.json`.
- [`metadata/dependency-contract-verification.json`](metadata/dependency-contract-verification.json):
  bottom-up body proofs for every ordinary authored opaque dependency contract.
  The target remains opaque; the pragma controls callers and does not disable
  verification of the target body. Prover intrinsics have
  built-in semantics, so they require neither an authored opaque contract nor
  a Move body proof.
- [`metadata/trusted-verification-boundaries.json`](metadata/trusted-verification-boundaries.json):
  explicit assumptions used instead of routine body verification. Each
  entry records either a successful larger-timeout proof and artifact or an
  expert-assumption rationale; the corresponding source has an adjacent
  comment at `pragma verify = false`.
- [`metadata/dependency-contract-audit.json`](metadata/dependency-contract-audit.json):
  the current gate for the complete dependency-proof closure. In addition to
  result and abort conditions, it checks every opaque global-state mutator has
  a `modifies` frame whose resource types cover its executable closure. A
  mutable argument or a read-only global access does not require such a frame.
  It rejects undocumented `pragma verify = false` and admits only complete,
  framed contracts listed in the trusted-boundary metadata. The report retains
  missing frame types even when disabled verification is also present.
- [`metadata/prover-repairs.md`](metadata/prover-repairs.md): behavior-preserving
  shared-framework source repairs and their proof evidence.

## Shared editable framework

[`framework/`](framework/) is the only Move package stored by the corpus. It
contains 152 modules and
275 Move source/specification files: the union
of all targets and their source-level transitive dependencies. Named addresses,
original paths, and the exact module-to-file mapping are in
[`framework/corpus-modules.json`](framework/corpus-modules.json).

Every sample is a small overlay recipe. At run time the controller copies the
shared package and applies the sample's preparation patch, which removes only
that target's reference specification and adds its task descriptor. There are
no per-sample framework snapshots.

## Samples

Each sample README records provenance, dependency closure, address aliases,
preparation edits, allowed edit paths, required contract categories, and hashes.

| Sample | Target | Granularity | Target source in shared package |
| --- | --- | --- | --- |
| [`AF-account-018`](samples/AF-account-018/) | `0x1::account::get_guid_next_creation_num` | `function` | `sources/AptosFramework/account/account.move` |
| [`AF-account-025`](samples/AF-account-025/) | `0x1::account::increment_sequence_number` | `function` | `sources/AptosFramework/account/account.move` |
| [`AF-aggregator-v2-017`](samples/AF-aggregator-v2-017/) | `0x1::aggregator_v2::string_concat` | `function` | `sources/AptosFramework/aggregator_v2/aggregator_v2.move` |
| [`AF-big-ordered-map-038`](samples/AF-big-ordered-map-038/) | `0x1::big_ordered_map::internal_leaf_iter_borrow_entries_and_next_leaf_index` | `function` | `sources/AptosFramework/datastructures/big_ordered_map.move` |
| [`AF-big-ordered-map-051`](samples/AF-big-ordered-map-051/) | `0x1::big_ordered_map::iter_modify` | `function` | `sources/AptosFramework/datastructures/big_ordered_map.move` |
| [`AF-block-014`](samples/AF-block-014/) | `0x1::block::update_epoch_interval_microsecs` | `function` | `sources/AptosFramework/block.move` |
| [`AF-code-017`](samples/AF-code-017/) | `0x1::code` | `module` | `sources/AptosFramework/code.move` |
| [`AF-config-buffer-006`](samples/AF-config-buffer-006/) | `0x1::config_buffer` | `module` | `sources/AptosFramework/configs/config_buffer.move` |
| [`AF-event-009`](samples/AF-event-009/) | `0x1::event` | `module` | `sources/AptosFramework/event.move` |
| [`AF-fungible-asset-080`](samples/AF-fungible-asset-080/) | `0x1::fungible_asset::symbol` | `function` | `sources/AptosFramework/fungible_asset.move` |
| [`AF-multisig-account-015`](samples/AF-multisig-account-015/) | `0x1::multisig_account::can_execute` | `function` | `sources/AptosFramework/multisig_account.move` |
| [`AF-multisig-account-067`](samples/AF-multisig-account-067/) | `0x1::multisig_account::validate_owners` | `function` | `sources/AptosFramework/multisig_account.move` |
| [`AF-ordered-map-031`](samples/AF-ordered-map-031/) | `0x1::ordered_map::iter_is_end` | `function` | `sources/AptosFramework/datastructures/ordered_map.move` |
| [`AF-primary-fungible-store-018`](samples/AF-primary-fungible-store-018/) | `0x1::primary_fungible_store::primary_store_exists` | `function` | `sources/AptosFramework/primary_fungible_store.move` |
| [`AF-resource-account-006`](samples/AF-resource-account-006/) | `0x1::resource_account` | `module` | `sources/AptosFramework/resource_account.move` |
| [`AF-storage-gas-008`](samples/AF-storage-gas-008/) | `0x1::storage_gas::new_gas_curve` | `function` | `sources/AptosFramework/storage_gas.move` |
| [`AF-transaction-fee-001`](samples/AF-transaction-fee-001/) | `0x1::transaction_fee::burn_fee` | `function` | `sources/AptosFramework/transaction_fee.move` |
| [`AF-version-001`](samples/AF-version-001/) | `0x1::version::initialize` | `function` | `sources/AptosFramework/configs/version.move` |
| [`AF-vesting-032`](samples/AF-vesting-032/) | `0x1::vesting::unlock_rewards_many` | `function` | `sources/AptosFramework/vesting.move` |
| [`AF-vesting-042`](samples/AF-vesting-042/) | `0x1::vesting::vesting_schedule` | `function` | `sources/AptosFramework/vesting.move` |
| [`AX-bulk-order-book-011`](samples/AX-bulk-order-book-011/) | `0x7::bulk_order_book::get_sizes` | `function` | `sources/AptosExperimental/trading/order_book/bulk_order_book.move` |
| [`AX-bulk-order-utils-010`](samples/AX-bulk-order-utils-010/) | `0x7::bulk_order_utils` | `module` | `sources/AptosExperimental/trading/order_book/bulk_order_utils.move` |
| [`AX-dead-mans-switch-tracker-003`](samples/AX-dead-mans-switch-tracker-003/) | `0x7::dead_mans_switch_tracker::keep_alive` | `function` | `sources/AptosExperimental/trading/market/dead_mans_switch_tracker.move` |
| [`AX-market-bulk-order-004`](samples/AX-market-bulk-order-004/) | `0x7::market_bulk_order` | `module` | `sources/AptosExperimental/trading/market/market_bulk_order.move` |
| [`AX-market-types-013`](samples/AX-market-types-013/) | `0x7::market_types::get_bulk_order_remaining_size` | `function` | `sources/AptosExperimental/trading/market/market_types.move` |
| [`AX-market-types-019`](samples/AX-market-types-019/) | `0x7::market_types::get_market_address` | `function` | `sources/AptosExperimental/trading/market/market_types.move` |
| [`AX-order-book-007`](samples/AX-order-book-007/) | `0x7::order_book::decrease_single_order_size` | `function` | `sources/AptosExperimental/trading/order_book/order_book.move` |
| [`AX-order-book-008`](samples/AX-order-book-008/) | `0x7::order_book::get_bulk_order` | `function` | `sources/AptosExperimental/trading/order_book/order_book.move` |
| [`AX-single-order-book-014`](samples/AX-single-order-book-014/) | `0x7::single_order_book::new_single_order_book` | `function` | `sources/AptosExperimental/trading/order_book/single_order_book.move` |
| [`AX-single-order-book-018`](samples/AX-single-order-book-018/) | `0x7::single_order_book::reinsert_order` | `function` | `sources/AptosExperimental/trading/order_book/single_order_book.move` |

This corpus was built by the selection pipeline below rather than by a
generator script, which is why its construction is recorded here rather than in
a `build.py`. The commands are retained so the corpus can be rebuilt or
extended; run them from the `spec-inference` directory.

## Reproducing this corpus

Run from the pinned `aptos-core` checkout:

```text
move-flow experiment inventory \
  --repo-root . \
  --source-commit 6d836beedc56fc70c54f3b3046d1d248d850c64b \
  --output evaluation-artifacts/work/raw-inventory.json

move-inference-corpus \
  --inventory evaluation-artifacts/work/raw-inventory.json \
  --config aptos-move/flow/evaluation/spec-inference/config/corpus.json \
  --output evaluation-artifacts/work/provenance.provisional.json
```

The current source frame is `corpus-v1/metadata/candidate-inventory.json`; the
current selected-task manifest is `corpus-v1/manifest.json`.

The inventory publishes both eligible and excluded function/module candidates,
with stable reason codes and AST-derived features. Selection uses SHA-256 of
the source commit plus the `selection_seed_suffix` from `config/corpus.json`,
applies the 20/10 source and 24/6 granularity quotas, caps functions per
module, and emits a deterministic reserve order. It never runs an experimental
arm.

`manifest.json` is the frozen record of the selection that was actually run;
it was produced under an earlier seed suffix, so re-running selection now
yields a different draw. Treat the manifest, not a re-run, as authoritative
for this corpus.

If a selected target exceeds the compatibility timeout or fails package
preparation, replace it using the deterministic reserve order.
The deterministic hierarchy is same cell, same semantic stratum, same size
stratum, then any reserve with the same source root and granularity:

```text
move-inference-replace-task \
  --manifest evaluation-artifacts/work/provenance.provisional.json \
  --task-id TASK \
  --reason compatibility_timeout \
  --output evaluation-artifacts/work/provenance.replaced.json
```

The replacement command records the input hash, both task IDs, both cells, and
the fallback tier. Corpus changes after seeing results are allowed only as a
new, explicitly versioned corpus and experiment round; prior artifacts remain
unchanged and reportable.

Materialize all selected packages and screen them before reference work:

```text
move-inference-prepare-corpus \
  --provenance evaluation-artifacts/work/provenance.provisional.json \
  --repo-root ../../../.. \
  --artifacts-root evaluation-artifacts/work/corpus-preparation \
  --patches-dir evaluation-artifacts/work/preparation-patches \
  --output evaluation-artifacts/work/provenance.prepared.json

move-inference-screen-corpus \
  --manifest evaluation-artifacts/work/provenance.prepared.json \
  --experiment-config config/default.json \
  --corpus-config config/corpus.json \
  --results-dir evaluation-artifacts/work/compatibility \
  --output evaluation-artifacts/work/provenance.screened.json
```

Preparation creates this inspectable layout:

```text
evaluation-artifacts/work/corpus-preparation/
├── README.md                 # corpus index
├── packages/                 # one union copy per Move package
├── pristine/                 # localized root closures
├── samples/<task-id>/
│   ├── README.md             # target, provenance, deps, aliases, hashes
│   ├── source -> ../../snapshots/<task-id>
│   └── preparation.patch -> ...
└── snapshots/<task-id>/     # exact self-contained run source
```

Unchanged files in `pristine/` and `snapshots/` are hard-linked from the union
package store, avoiding physical duplication while preserving a self-contained
tree for each sandboxed run. The links inside `samples/` are only navigation
aids and are never exposed to the agent.

To add this catalog to an older prepared corpus without changing its task
snapshots:

```text
move-inference-catalog-corpus \
  --corpus evaluation-artifacts/work/provenance.screened.json \
  --output evaluation-artifacts/work/provenance.cataloged.json
```

Preparation recursively follows every local normal and development dependency
declared in `Move.toml`, rewrites each local edge inside the standalone tree,
and records the full package graph. Every package record includes its source
and snapshot manifest hashes, declared address aliases, development aliases,
and the resolved alias map. Screening compiles the stripped package, runs WP on
a disposable package copy, and proves that exact enriched copy. A condition
marked `[inferred = vacuous]` or `[inferred = sathard]` is recorded as an
untrusted repair obligation; it is never removed to make screening pass. It
must be replaced by an invariant-backed contract that is proved against its body
before it may become an opaque dependency boundary. A stage that
exceeds the configured 40-second threshold excludes the target and permits the next
reserve under the deterministic fallback hierarchy. A compiler, WP, Flow, or prover implementation failure does
not exclude a target: fix the defect and re-screen every affected target.
Missing executables and other host failures are infrastructure failures and
must also be corrected and rerun. Set `BOOGIE_EXE` and `Z3_EXE` to the pinned
artifact binaries before screening.

Before screening a corpus after adding or strengthening a dependency contract,
verify that contract bottom-up or record it as a trusted boundary. The verifier
targets each ordinary opaque function in the unchanged shared package. The
prover checks that function's body against its contract; `pragma opaque` stays
in place and makes callers consume its contract instead of its implementation.
For an explicit `pragma verify = false` boundary, a separate diagnostic attempt
may copy the package and change only `verify` to true; it must not remove
`opaque`:

```text
move-inference-verify-dependency-contracts \
  --package corpus-v1/framework \
  --manifest corpus-v1/manifest.json \
  --trusted-boundaries corpus-v1/metadata/trusted-verification-boundaries.json \
  --move-flow /absolute/path/to/move-flow \
  --output corpus-v1/metadata/dependency-contract-verification.json
```

The checked-in report separates intrinsic and native bindings, body-proved
opaque contracts, and explicit trusted assumptions. It records the exact
leaf-to-caller order and per-target prover results for ordinary opaque targets
only.

### Reference and mutant staging

For richer semantic scoring, create a recipe conforming to
`schemas/task-recipe.schema.json`. A recipe names pristine and prepared
snapshots, a replayable preparation patch, the hidden reference package,
reviewer approvals, the mutant manifest, and its validation result.

Reference packages and mutants must remain outside every prepared snapshot.
Stage the hidden review material from a clean 30/30 screen, then audit its
machine-verifiable and human-approved state without manufacturing approvals:

```text
move-inference-stage-reviews \
  --provenance corpus-v1/manifest.json \
  --output-dir evaluation-artifacts/work/review

move-inference-audit-reviews \
  --provenance corpus-v1/manifest.json \
  --review-dir evaluation-artifacts/work/review \
  --output evaluation-artifacts/work/review/review-audit.json
```

The staging command restores exact pinned-commit Framework specifications and
marks Experimental references as requiring study authorship. The audit counts
only explicit independent approvals and validated essential mutants; it never
infers or creates expert approval.

First record treatment-blind compiler, WP, and prover compatibility on the
reviewed reference package:

```text
move-inference-check-compatibility \
  --config config/default.json \
  --package HIDDEN_REFERENCE_PACKAGE \
  --target TARGET \
  --output HIDDEN_TASK_DIR/compatibility.json
```

Run mutant validation against the handwritten reference:

```text
move-inference-mutants \
  --config config/default.json \
  --package HIDDEN_REFERENCE_PACKAGE \
  --target TARGET \
  --mutants HIDDEN_TASK_DIR/mutants.json \
  --output HIDDEN_TASK_DIR/reference-mutant-validation.json \
  --timeout 40
```

The review audit reports how many tasks have compatible references, independent
reviews, and validated essential mutants. Incomplete coverage simply means a
round uses core compile/prove/contract scoring for those tasks. It does not
block scheduling or execution.
