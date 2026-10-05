# Handoff

Status: 2026-10-05, branch `wrwg/lean3` (worktree `dev3`), clean at the commit that adds this file.

## Where the state is

- [`roadmap.md`](roadmap.md): project status and the test ledger.
- [`state-labels.md`](state-labels.md), "Milestones": S1, S2a, S3a, S3b done; S2b and S4 open.
- [`prover-test-problems.md`](prover-test-problems.md): the Prover-test registry. 112 of 414
  tests verify; the entries name the open problems and their tests.
- [`verification-benchmarks.md`](verification-benchmarks.md): the benchmark.

## Next, in order

1. **S2b, defined state labels** (registry V20, 5 tests): `publish`, `remove`, `update` as
   memory functions of the pre-state; a label an `ensures_of` or `result_of` defines as the
   invocation's post-state.
2. Then a registry entry by test count: V7 intrinsic maps (12), C8 `update` of a spec
   variable (9), V16 natives (7), the `update_field` operation (V1, 11 messages).
3. S4: attribute the remaining `state_labels/` failures.

## Open findings not yet in a design

- Benchmark, full run at e4fb2de4cc against 7457d37: 24/30 verified as before, heartbeats
  +1.3%; `comparator` +18.6% and `string` +11% heartbeats, not yet attributed to a commit.
- Benchmark problems `amm` and `calculator` fail before verification: `amm` reads a
  specification `let` (`effective_in`) in a proof block's `post assert`, which is not in
  scope there; `calculator`'s `inferred = sathard` attribute is rejected as "not a boolean
  flag".
- Prover test `generic_aliasing_all_partitions::never_alias` now exhausts its budget
  (was a clause failure) since S3b.
- From the S3b cleanup review, not done: `singleFieldRead?` (one integer field) and
  `boundWitnesses` (`<`, `≤`, `=` with the binder on one side) are the instances of "a binder
  read only through projections" and "the binder's interval from the bounds machinery".
  The `subst` and `projections` leaf stages run for every leaf of every target; whether
  they explain the `comparator`/`string` heartbeat growth needs a measurement.
- A quantifier over a vector's elements is not instantiated at a read position:
  `requires forall x in v: x < 10; ensures v[0] < 10` (with `len(v) > 0`) fails, by value
  and by reference alike, while `forall i in 0..len(v): v[i] < 10` verifies.
- `leaner_denote_witness` decides the conjuncts reading the binder first, in the order
  `Array.qsort` leaves them; a stable order puts `verify_vector::verify_contains` over its
  budget, so the search depends on an incidental order.

## How to work

- Prover tests, from `third_party/move/move-prover`: `MVP_TEST_FEATURE=lean cargo test`
  with one filter per run; `UPBL=1` regenerates the `.lean_exp` baselines. Read every
  changed baseline; an `-- error:` line is a regression unless intended.
- Suites: `lake build && lake test` in `leaner-ir`, `leaner-move`, `leaner-e2e-tests`.
  Never rebuild `leaner-ir` while a suite runs; iterate in a scratch copy.
- A deliberate cost change: `UB=1 lake env lean LeanerLang/Tests/DenotePerformance.lean`
  after `lake build`; the diff names only the targets meant to move.
- Benchmark: `python3 scripts/leaner-bench.py run`, then `compare @-2 @-1`.
- A new capability gets a Check fixture and a roadmap or design row. A Check fixture never
  records an unsupported positive proof as an expected failure.
- Debugging a target: `set_option leaner.denoteDebug true` and `verify f by skip` show the
  leaf obligations.
