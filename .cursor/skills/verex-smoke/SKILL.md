---
name: verex-smoke
description: Use between turns when iterating on the Verex Lean stack (third_party/move/lean): the core smoke suite after each edit batch, and the checkpoint gate every commit needs
---

# Verex smoke suite and checkpoint gate

The core smoke suite (`third_party/move/lean/scripts/smoke.sh`) is the
few-minute check between edit batches of the Verex Lean stack: the fixtures
that catch the regressions this stack produces. It does not replace the
checkpoint gate before a commit.

## Smoke suite

```bash
third_party/move/lean/scripts/smoke.sh            # ≈ 65 s warm; after a restack the Move CLI rebuilds too (cargo, minutes)
third_party/move/lean/scripts/smoke.sh --no-mvp   # without the Move Prover tests
third_party/move/lean/scripts/smoke.sh --no-exe   # Lean fixtures only: no executable, no cargo
third_party/move/lean/scripts/smoke.sh --jobs N   # Lean processes at once (or SMOKE_JOBS)
third_party/move/lean/scripts/smoke.sh --list     # the fixtures
```

It builds `verex-ir` and the e2e Check library in place, then elaborates the
Lean fixtures as parallel processes (by default half the hardware threads, at
most one per 3 GB of available memory), longest first by their times in the
previous run, and prints one line per fixture with pass/fail and its own
seconds, in the listed order. The `verex-move` executable (and the Move CLI,
unless `APTOS_MOVE_CLI` is set) builds beside the fixtures; `verex-move verify`
and the Move Prover tests run after them, beside each other. The fixtures:

- Check fixtures (`verex-e2e-tests/VerexE2ETests/Check/`): `Storage/TableReads`,
  `TableAllocation`, `TableGlobalWrites`, `TableStoredInvariants`,
  `WrappedTables`; `Specifications/TableSnapshots`, `InsertionMap`,
  `AbstractOwners` (a representation reader used through its specification),
  `OptionalMapResults` (enum-shaped optional map results, execution and proofs);
  `Closures/InvariantBehavior` (a stored function value with a `result_of`
  read).
- `verex-ir` test modules `VerexSL/Tests/StoredTableInvariantErrors` and
  `LiteralMaps`, `TypedMaps` (typed map preparation and mutation transport),
  `SharedPreparation` (authored preparation before clause splitting),
  and `CallRangeResults` (computed indices under the cheap solver budget).
- `verex-move verify` of `VerexE2ETests/SourceVerify/behavior_manual_facts.move`,
  whose `.exp` shows a typing theorem's hypothesis list (`NativesTyped`,
  `RunsAgree`, `OwnersRepresentable`, …).
- Move Prover tests with the lean feature, one process and one shared snapshot:
  `folds_of_map_intrinsic`, `fun_value_equality`,
  `intrinsic_map_insertion_order`, `simple_vector_client` (literal reads and
  generic vector conversions), `verify_table`, `data_invariant_in_map`.

Rules it encodes:

- Read-only with respect to baselines: a diff is a failure to report, never to
  absorb. The script refuses `UB`, `UPBL` and `UPDATE_BASELINE`.
- The Move Prover tests run a private copy of the verifier, taken when
  the run starts. The build and copy must observe unchanged source and artifact
  inventories; a concurrent modification discards the copy and retries. Only
  the verifier's import closure is copied, so tests can run beside later Lake
  builds and the Lean suites. A
  scratch replay (`lake env lean`) and the framework command-line run read
  the live `.lake` trees instead: run those beside test runs, never beside a
  build that rewrites `verex-ir` or `verex-move`.
- A Move Prover test the harness skips ("verifier is not built") is a
  failure, not a pass.

Run it after each edit batch, per the small-change validation rule. Green
smoke is not a commit gate.

Between smoke runs, build libraries, not executables: `lake build` in
`verex-ir`, `lake build VerexMove` in `verex-move`, `lake build VerexE2ETests`
in `verex-e2e-tests` (the default targets of the last two also compile and
link the executables). To verify a Move package or some of its modules,
`third_party/move/lean/scripts/verify-modules.py <package> [--filter <part>]`
runs one `verex-move verify` process per module, several at a time, and
prints the reports in module order; `verex-move verify <package>` runs the
targets one after another.

## Checkpoint gate (before every commit)

1. Full build and the four suites: `(cd verex-ir && lake build && lake test)`,
   and the same in `verex-move`, `verex-rust`, `verex-e2e-tests`.
2. The Move Prover gate alone, which builds and snapshots `verex-move` itself:
   `cd third_party/move/move-prover && MVP_TEST_FEATURE=lean UPBL=1 cargo test
   -p move-prover --test testsuite`; review `git diff --numstat
   third_party/move/move-prover/tests` and attribute each changed `.lean_exp`
   with `git log -1 -- <file>`. A run that reports 440 passed in a fraction
   of a second skipped every test.
3. For intrinsic-map work, the framework CLI compare against the previous
   stage log: `VEREX_OPTIONS=verex.stageLog=<file> target/ci/move prove --lean
   --package-dir aptos-move/framework/aptos-framework -f ordered_map`, per-module
   heartbeats compared.

## Where the build time goes

An edit rebuilds the edited module, every module importing it, and the C of
the edited module only. The solver, `Contract` and `Verify` are split into
parts so that most edits rebuild one part and its importers: an edit of the
solver's walker costs about 30 s to libraries and 15 s more to the
executable, against 90 s and 70 s before the split. Measurements and the
parts: `third_party/move/lean/designs/performance.md`, "Modules split for
edits", and `third_party/move/lean/CLAUDE.md`. After a restack, the first
smoke also rebuilds the Move CLI with cargo (8 min on 2026-10-22).
