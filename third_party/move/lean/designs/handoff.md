# Handoff

Status 2026-10-13. #20746 (proof files as Lean files, `proof-file-editing.md`,
P1–P3; Table owners as owned values, `intrinsic-maps.md`, stage 2) and #20710
(Prover parity, authored proofs, the rename Leaner → Verex) are merged. Branch
`wrwg/lean-tables`: tables in the runtime semantics (`intrinsic-maps.md`,
"Tables in the runtime semantics"). The session notes up to 2026-10-11 are in
[`historical/handoff-2026-10.md`](historical/handoff-2026-10.md).

## Where the state is

- [`roadmap.md`](roadmap.md): project status and the test ledger.
- [`prover-test-problems.md`](prover-test-problems.md): the registry of the Move
  Prover's unit tests under Verex (437 files); `MVP_TEST_FEATURE=lean`, not in CI.
- [`verification-benchmarks.md`](verification-benchmarks.md): the benchmark; the last
  local run has 30 verified problems and AMM's expected rejection at 14.0G
  heartbeats. Not in CI before merge.
- [`perf-notes.md`](perf-notes.md): costs, open performance leads, rejected experiments.
- [`state-labels.md`](state-labels.md), "Milestones": S1–S3b implemented, S4 open.

## Outside the repository

- aptos-labs/internal-ops#9675 points the PIES nightly at `verex-bench.yaml`; merge
  it after #20710 lands. Then the report moves to the Pages path `verex-bench/`, and
  the benchmark's CI history restarts with the renamed workflow.
- Label `CICD:run-verex-bench` exists; delete `CICD:run-leaner-bench` once #20710
  lands.
- leanprover/lean4#15626: Lake passes no `moreServerOptions` to library modules, so
  `skipVerify` recognizes the language server by `internal.cmdlineSnapshots`; use
  `moreServerOptions` once it is fixed.

## Recent decisions

- 2026-10-13: a table owner's type has a hidden last field holding its contents;
  the runtime boundary erases it (`intrinsic-maps.md`, "Owned Table values").
  Tables behind type parameters are not carried. Decision 2026-10-13, revised
  the same day: the verifier represents tables as values; the runtime and the
  big-step semantics keep native storage behind handles, since a value
  representation is not adequate there; the agreement reconciles the two, so
  the erasing boundary stays (`intrinsic-maps.md`, "Tables in the runtime
  semantics"). The earlier decision to make tables runtime values and remove
  the boundary is withdrawn.
- 2026-10-12: the default budget is 1000 thousand `maxHeartbeats`, about thirty
  seconds; the registry runs with it (was 25 thousand).
- 2026-10-12: an authored proof is kept only where the automatic run fails, or
  where the proof costs at most 66% of it; `--ignore-proofs` measures this
  (`scripts/proof-census.py`, `perf-notes.md`, "Proof census").
- 2026-10-12: the names Verex, VIR, VerexSL (VSL for short), applied everywhere,
  `v0/` and `historical/` included.
- 2026-10-12: a function with a body that declares no contract and plays no map or
  table role claims no frame (`source-verification.md`, "Move frames").
- 2026-10-12: authored proofs and a module's theorems see `VerexIR` opened, and
  `open N …` items open further namespaces for them (`source-verification.md`,
  "Proofs").
- 2026-10-11: an authored `verify f by …` owns `f`'s whole goal, one goal per
  clause (decision D; `source-verification.md`, "Proofs"; `perf-notes.md`).
  In the editor, `verify` items run only when marked `verify!`.
- 2026-10-10: V20 loop cost is reduced by local levers only, each validated by the
  benchmark.
- 2026-10-09: callers assume the contracts of failed or unattempted callees; a
  callee still in progress is refused (`denotation.md`).
- 2026-10-09: priority order: behavioral predicates and H4d, then BCS, then the
  rest of the registry. The first two are done.
- 2026-10-08: `int2bv` at `num` wraps at `u64`; specification functions are partial,
  their domain derived from the declared types (option A; G14, G15). Rejected:
  syntactic patches, and the in-body guard of WIP e645c55f.
- 2026-10-07: the Prover's bit-vector representation is not modeled (G14).
- 2026-10-07: a Table is an owned value of identity and contents; snapshots with one
  identity and different contents are observations across a mutation; caller
  specifications need no program points (`intrinsic-maps.md`).
- 2026-10-06: Table specification equality is allocation identity
  (`intrinsic-maps.md`).

## Next, in order

The goal: close the Move Prover registry gaps toward parity, at measured proof
costs.

1. Clause markers for the leaves of authored proofs still named `leaf`: `modifies`
   frames, stored invariants and state typing (`leafName` in `Solve.lean` has no
   case for them). Then per-clause scripts for `process`, `buy_in` and
   `deduct_shares` (about 0.3G).
2. Partial specification functions (G15) cost +2.53% on the benchmark: `pool_u64`
   +10.93% (`balance` 97M → 292M, `buy_in` and `redeem_shares` +8%), `math128` +4.48%
   (`sqrt`), `features` +4.08% (`change_feature_flags_for_next_epoch` 42.9M → 76.4M),
   from expansion guards over struct fields, vector elements and other
   specification results. Recovery: bound the fields and elements of typed values
   statically, or change how leaves decide inline guards.
3. Registry gaps by size: V7 intrinsic maps (empty abstract map owners;
   tables behind type parameters, `intrinsic-maps.md`), C8 `update`
   of a specification variable (9), V16 natives, V1 `update_field` rendering (11
   messages). Vector reverse preparation is the next proof lead
   (`perf-notes.md`).
4. S4: `unmodified_memory_at_label::swap` needs a decision on reads of removed
   resource contents (`state-labels.md`).
5. Unbounded loops under `pragma unroll` time out at 25k before the exhausted-bound
   message, since each iteration's return path is taken first:
   `verify_vector::*_with_unroll`, `bug_15044::compare_u8_vector`.
6. Proof leads:
   - `bp_forwarding::all_values_bounded`: a current-index witness for the
     early-break equivalence proves it at 100k (30.25M), not at 25k; the
     remaining cost is preparation, loop and assertion.
   - `vector_hofs_fold::count_even_concrete`: `vir_denote_norm` has no ground
     `Int.tmod`/`Int.tdiv` simprocs; adding them removes the last concrete-result
     residual (scratch proof 52.9M at 100k, 41.06M before the authored proof).
   - Skipping repeated failed `omega` refutations (one V20 leaf, about 0.15M).
   - `verex_witness` decides the conjuncts reading the binder in the order
     `Array.qsort` leaves them; a stable order puts
     `verify_vector::verify_contains` over its budget.
   - `macro_verification::foreach` times out at 25k; the lead is the loop's
     unchanged-tail invariant (`perf-notes.md`).
7. The 40 authored proofs the census still needs (`perf-notes.md`, "Proof
   census"): fee formulas with nested quotients by literals, `sqrt`, chains of
   products, array sizes after updates, `Option.map` encodings, witnesses over
   literal vectors. Rerun the census after each family.
8. Awaiting a user decision: `int2bv` at a type parameter.

## Open findings not yet in a design

- Tables (2026-10-13, in `intrinsic-maps.md`, "Tables in the runtime
  semantics"): the runtime has no semantics of the table natives
  (`nativeCall` is opaque; `TableOperations.lean` and `TableLoans.lean` are
  unused), and `TablesEncode.stored` admits no state enclosing a table only a
  frame value holds, so the runtime form of a contract has a false
  precondition at such calls and `GlobalsPreserved` fails for a run that
  allocates. Read from the definitions; no Lean build was run.
- The InfoView works on Move proof files since 2026-10-12
  (`proof-file-editing.md`, P1–P3). Rust `.spec.lean` files (P4: contracts
  attached after registration, or an explicit deferral) and renderings as
  importable Lake modules (P5) are open.
- With its `verify` items skipped, the editor still re-elaborates a module's
  registration on every edit (`OrderedMap.lean`: 7–8 s, measured 2026-10-11).
- `bug_15880::test2` fails unattributed: its failing asserts (`v == 4`,
  `global<R>(@0x1).v == 3`) are made while the `_v`/`x` loans live, G12's behavior,
  but G12 lists only `bug-17117` (from the source and the baseline, not a run).
- `SourceVerify/field_update_labels::caller_wrong`, an intended negative, rejects
  only at the suite's default budget and times out at a true 25k cap (status
  uncertain).
- The Prover's default run verifies no root of `math_fixed8`: its
  `verify_duration_estimate = 60` exceeds the default timeout; without the pragma it
  verifies all three. Parity evidence for `pow_raw` and `exp_raw`.
- `generic_aliasing_all_partitions::never_alias` exhausts its budget since S3b (was a
  clause failure).
- Unattributed cost changes: `comparator` +18.6% and `string` +11% (e4fb2de4cc against
  7457d37), `simple_map::add_all` 159M → 183M and `features::apply_diff` 206M → 223M,
  +0.31% at the `&mut`-predicates checkpoint (calculator +0.67%, ordered_map +0.51%,
  Quicksort +0.42%). The `subst` and `projections` leaf stages run for every leaf of
  every target; whether they explain `comparator`/`string` needs a measurement.
- From the S3b review, not done: `singleFieldRead?` and `boundWitnesses` are the
  instances of "a binder read only through projections" and "the binder's interval
  from the bounds machinery".
- Uncertain: plain-index upper bounds written as closed arithmetic (`1 + 1`) are not
  recognized (about 10M in fallback pipelines); scratch variants of a false
  one-increment labeled claim time out instead of being rejected.
- The `open` items commit of #20710 was spot-checked only on the registry and not
  benchmarked.

## How to work

- Small changes: build and run the affected fixture, then push; CI (`lean.yaml`)
  runs the four Lean suites and compiler-v2's Lean and transactional tests. The
  registry and the benchmark run outside CI: run the affected tests where a change
  can move verdicts or costs.
- The proof census: `python3 scripts/proof-census.py [--only <path part>]
  [--renderings <dir>]`; with `--renderings`, the renderings re-verify against a
  scratch copy of `verex-ir` (`lake env lean -Dweak.verex.ignoreProofs=true`)
  for iterating on the solver without rebuilding the tree.
- Before pushing a change to a PR: `gt sync --no-interactive`, `gt submit --no-stack
  --no-interactive`, then update the PR description.
- During performance work, run the affected benchmark and regenerate its data and
  HTML before broad suites. Heartbeats are the measure.
- Prover tests, from `third_party/move/move-prover`: `MVP_TEST_FEATURE=lean cargo test`
  with one filter per run; `UPBL=1` regenerates the `.lean_exp` baselines. Read every
  changed baseline; an `-- error:` line is a regression unless intended. The
  "verification diagnostics" category includes six preexisting elaboration errors:
  inspect the replacement diagnostic of a removed proof name before counting it.
- One registry file by hand, from `move-prover`: `target/ci/move-prover
  --dependency=../move-stdlib/sources --dependency=../move-stdlib/nursery/sources
  --dependency=../extensions/move-table-extension/sources --named-addresses std=0x1
  extensions=0x2 --lean --heartbeats=25 -o <dir>/out.bpl <file>`; the rendering lands
  at `<dir>/out.lean`, run it with `lake env lean` from `verex-move`.
- Suites: `lake build && lake test` in `verex-ir`, `verex-move`, `verex-rust`,
  `verex-e2e-tests` (`VEREX_E2E_SUITE=verify|check|move|rust` runs one).
  Never rebuild `verex-ir` while a suite or the benchmark runs; iterate in a scratch
  copy.
- A deliberate cost change: `UB=1 lake env lean VerexSL/Tests/DenotePerformance.lean`
  after `lake build`; the diff names only the targets meant to move.
- Benchmark: `python3 scripts/verex-bench.py run --out <fresh.json>`, then
  `python3 scripts/verex-bench.py report --local <fresh.json> --branch main --html local_benchmark.html`.
  Compare against main, not intermediate local runs. Inputs are read at `HEAD`:
  commit before measuring. A standalone `verex-bench` can be stale or exit 0 with a
  failed target; read the JSON status. Compare samples target by target.
- A new capability gets a Check fixture and a roadmap or design row. A Check fixture never
  records an unsupported positive proof as an expected failure, and a stricter scratch
  budget is not a fixture's baseline.
- Debugging a target: `set_option verex.solverDebug true` and
  `verify f by verex_obligations` show the leaf obligations. That proof hands over
  every leaf undecided: test solver changes on the automatic verification.
- Traps:
  - `isolate-target.py` disables the module's other targets, so callee theorems
    vanish; run the whole module when a proof uses them.
  - In a solver step that restores the state on failure, debug output must use
    `IO.println` (`logInfo` is rolled back), and a failed attempt must restore the
    whole state (a leftover `replaceTargetDefEq` left an unassigned placeholder).
  - A failing scratch `macro_rules` alternative reruns the older expansion: wrap it
    in `try`, and in production add it as an ordinary first alternative.
  - A single-file `SourceVerify` fixture exports only its own module (`std::bcs` does
    not resolve); use a Check fixture.
  - Unattributed solver cost is the total minus the stages' sum.
