# Handoff

Generic module axioms (2026-10-08):
- `axiom<T>` is assumed at each instantiation a verification applies, as the
  Prover monomorphizes it (`Contract.specInstantiations`, `axiomInstances`):
  the specification functions applied in the function's body and contract, in
  its callees' contracts and in the expansions of these, a callee's or an
  expansion's type parameter read as the application's argument; an axiom's
  own application `f<T>` binds `T`. Each instance is translated with the
  expansion fields `typeArguments`/`typeArgumentTypes`, so its binders range
  over the instance's values (`boundedMembership?` for a bounded integer);
  axioms go through the general quantifier translation, not the
  single-binder invariant path, which read binder types from the raw table.
- Surface: `axiom {T} e` (`Syntax`, `Ast.NamespaceInvariantDecl.generics`,
  `Elab`, `Lower`, both printers). Generic invariants stay rejected.
- `num` as a type argument: `opaqueSpec` takes `SpecTypeArgument`
  (`native τ | integer`), so `spec_id<num>` has a key (`performance_200511`
  reaches verification; two budget timeouts).
- A module's `aborts_if_is_strict`/`_partial` holds for its functions
  (`Contract`, the function's pragmas in effect); `opaque_native`'s negatives
  needed the callee not to abort. `TRACE(e)` encodes as `e` (`Encode`).
- Registry: `axiom_generic`, `axioms`, `mono_on_axiom_spec_type`,
  `opaque_native` verify; `performance_200511` changes from the type-argument
  rejection to two budget timeouts; no other baseline changed.
- Tests: `SourceVerify/generic_axioms{,_false}.move`,
  `SourceVerify/inherited_strictness{,_false}.move`.
- Benchmark against the previous checkpoint (15,375,903,448): same outcomes,
  15,376,387,648 (+0.003%); no problem moves by more than 0.03%.
- Open: generic module invariants (`invariant<T>`: instances from the
  resources a function reaches, and obligations at writes); `bitwise_table`
  and `verify_table` reach the natives behind `extensions::table`'s map roles
  (its role contracts are not used); mutable-reference behavioral predicates
  (V20); `bv_signed_generic` (awaiting the user's choice).

Registry follow-up: axioms, bitwise, module closure (2026-10-08):
- Module axioms over values are assumed by every verification of the unit
  (`Contract.invariantReached`: an axiom reading no memory needs no reach),
  as the Prover states its axioms globally; one reading memory still needs
  the function to reach it. The layout printer printed axioms as `invariant`
  (`Print/Layout.lean`); it prints `axiom` now. `bv_internal` verifies.
- Export: `select.rs` module closure walks module, function and struct
  specifications and spec-function bodies, so a module only an abort code
  names is exported (`bv_aborts` reaches verification and fails its clause,
  as in the Prover; exchange test in `aptos-move/cli/src/tests/exchange.rs`).
- An arbitrary value (`__leaner_arbitrary_…`) takes no type arguments
  (`Contract.arbitraryValue`), so `abort_in_fun` verifies. Membership lowers
  its element at the projected specification type (`Lower`).
- `int2bv` of a literal, also through specification casts, is the wrapped
  literal (`BitVectorConversion.constant?`), so `bitwise_operators`'
  recursive spec function decreases its measure and verifies.
- Equality's instantiation is its operands' type when they share one
  (`Encode`, beside the `old` normalization): an included schema kept `num`
  over substituted `u64` operands. `defines` verifies.
- Closer: bitwise identities with zero (`IntegerArithmetic`), the truncating
  remainder of a nonnegative shift is its remainder
  (`Types.shiftLeft_tmod_of_nonneg`), and `assertBounds` states `a & b = b & a`
  where a leaf mentions both orders. Commutativity as simp lemmas instead cost
  `features::change_feature_flags_for_next_epoch` 2x (rejected).
  `bitwise_features`: `contains`/`is_enabled` verify; `set` and
  `disable_feature_flags` still exceed the budget (bit clearing through a
  vector write needs bit-level reasoning).
- Tests: `SourceVerify/module_axioms{,_false}.move`,
  `SourceVerify/schema_equality.move`, leaner-move `Tests.BitVectorConversion`.
  `Check/Scalars/BitVectors`' `contains_without_bv` now verifies without
  `pragma bv` (valid; it was marked as needing bv); `toggled_without_bv`
  takes its place as the leaf integer arithmetic does not decide.
  Registry: `abort_in_fun`, `bitwise_operators`, `bv_internal`, `defines` now
  verify; `bv_aborts`, `bitwise_features` improved; no other baseline changed.
- Benchmark against the previous checkpoint's run (15,465,529,048 raw
  heartbeats): same outcomes (30 verified, one expected AMM rejection),
  15,375,903,448 (-0.58%). Moved: features -9.94% (contains -67.9%,
  change_feature_flags_for_next_epoch -36.1%, set -12.7%), option -0.88%,
  fixed_point64 -1.10%, fixed_point32 -0.53%, error +0.13%; every other
  problem within 0.1%.
- Open from this session: `bv_signed_generic` (awaiting the user's choice for
  `int2bv` at a type parameter); generic module axioms followed (above).

Partial specification functions by declared types (2026-10-08, option A):
- User decisions: an `int2bv` the compiler types at `num` wraps at `u64`
  (`BitVectorConversion.conversionWidth?`); one at a type parameter is still
  rejected (`bv_signed_generic`). A specification function is partial: defined
  where parameters declared with fixed-width integer types fit them,
  unspecified elsewhere (G15). The user rejected syntactic patches
  (contextual retyping, literals through casts) and chose deriving the domain
  from the declared types (option A) over a guard in the body (the earlier
  WIP commit e645c55f, superseded).
- Importer: spec-function parameters keep fixed-width declared types
  (`Encode.addSpecificationParameter`). LeanerLang reads such a parameter in
  the body as `Int`, as contracts read function parameters (`Lower`), and the
  printer treats parameters as logical locals, so `x + 18446744073709551616`
  and `x < 2^64` re-elaborate canonically. Intrinsic map `num` role
  parameters accept fixed-width ones (`Intrinsics.matchesParameter`).
- Verifier (`Contract.lean`): recursive definitions unfold inside the domain
  (existing `parameterBounds`); non-recursive expansions are guarded by
  `valueBounds`, minus conjuncts the arguments' types imply (`fitsByType`:
  literals, `SpecInt.val` of a fitting type, vector lengths); outside, both
  read the uninterpreted `f.spec.outside` of the bundle. Recursive definitions
  reading an uninterpreted value at type arguments take their instantiation
  (`definitionTakesTypes`); a non-generic one needs no family.
- Closer (`Close.lean`, `unfoldSpecsOnce`): a domain guard (`dite` binding
  `bounds`) is transparent: decide it, then the body's first condition, and
  drop the instance if that is open (as without a domain); guards are decided
  with the typed-value bounds `typedBound?` (shared with `assertBounds`);
  bundle projections of an instance are reduced. See perf-notes.md.
- Tests: `SourceVerify/spec_fun_domain{,_false}.move`,
  `Check/Specifications/SpecFunctionDomain{s,Errors}.lean`, leaner-move
  `Tests.BitVectorConversion`. Full registry (440 files) against the pre-change
  baselines (8c71365a): no function regresses; `folds_of_callee_ensures` loses
  its family error; three files new from main (`verify_only_list`,
  `regression/behavioral_predicate_cycle`, `regression/recursion_inline_bound`)
  get first baselines, not caused by this change (checked with an `Int`
  control for the one with typed spec parameters).
- Benchmark, measured against the pre-change commit 8c71365a built in a
  worktree on the same machine (15,084,073,019 raw heartbeats, reproducing the
  handoff's 15,083,357,067): same outcomes (30 verified, one expected AMM
  rejection), 15,465,529,048 (+2.53%). Moved: pool_u64 +10.93% (balance
  97M to 292M, buy_in and redeem_shares +8%), math128 +4.48% (sqrt), features
  +4.08% (change_feature_flags_for_next_epoch 42.9M to 76.4M), ordered_map
  +0.54%; every other problem within 0.1%. The cost is expansion guards whose
  arguments' range is not evident from their form (struct fields, vector
  elements, other specification results), which the closer then proves.
  Recovering it would mean bounding those forms statically (fields and
  elements of typed values) or changing how leaves decide inline guards.

Suspended 2026-10-07 at the user's request after committing this checkpoint.

Bit-vector representation retired (2026-10-07, user decision, validated):
- MVP's bv encoding (`pragma bv`/`bv_ret`/`bv_internal`, bitwise
  classification) is a backend hint, not semantics: spec arithmetic stays
  mathematical (G14 in `prover-test-problems.md`, `numeric-representation.md`).
  `pragma bv` still selects Leaner's BitLift decisions. The package guard and
  the scalar classification/rewriting modules (`Frontend/NumericRepresentation*`,
  `Tests.NumericRepresentation`, `Tests.NumericLowering`) are deleted.
- `int2bv(e)` is exact: `Frontend/BitVectorConversion.lean` lowers it to `e`
  when in range, else to a truncating-remainder residue (two's complement for
  signed); a parameter of the result type is kept. `bv2int` is the identity.
  `Tests.BitVectorConversion` checks all twelve widths against `BitVec`.
  The in-range guard matters: a bare residue made `features::set/contains` fail,
  because the closer has no division identity for literal divisors.
- Fixtures: the 14 earlier `SourceVerify/bv_*` files are replaced by
  `bv_conversion{,_false}` (both verifiers agree) and `bv_encoding{,_false}`
  (the registered difference, both directions); MVP verdicts were checked.
- Validation (`/tmp/bv-integer-logs/`): all four suites and the registry pass;
  benchmark 30 verified + one expected AMM rejection, 15,083,357,067 raw
  heartbeats (`/tmp/leaner-benchmark-bv-integer.json` = local JSON). Restored:
  features, cmp, math_fixed, ristretto255, ed25519, ordered_map; the other
  problems are unchanged (+0.000%). 55 baselines changed against the pre-change
  state (`all-changed-vs-pre-bv.json`): 26 MoveToLeanerLang translations no
  longer stop at the guard, ten registry files now verify cleanly, the rest
  reach verification.
- Open (exposed, not caused, by the removal): `MoveToLeanerLang/constants.exp.lean`
  is an error baseline because a width-less `int2bv(value + 1)` (XAST type `num`,
  width from the spec function's result) is rejected; it needs contextual width
  inference, as does generic `bv_signed_generic`. Registry gaps now visible:
  `bv_aborts` ("specification function call namespace is out of range"),
  `bv_internal` (rendering type/call-name errors), `abort_in_fun` (arbitrary
  value type argument), `bitwise_features` timeouts, `bitwise_error_2` storage
  clause, `bitwise_operators` termination measure, `folds_of_callee_ensures`.
- Unused: XAST v10's `defaulted_num` marker is still decoded and tested.

Unrolling resumed and validated (2026-10-07, evening):
- Literal results are no longer SSA-named. The `lir_denote` simproc
  `namedLiteral` (`Denote/Types.lean`, with `forall_named_fits`) substitutes a
  checked-arithmetic result that normalizes to a fitting Int literal, so literal
  loop conditions decide during normalization. The ground 2×2 nested unroll
  falls from 64.5M to 7.7M closer heartbeats and verifies at 25k; it is now a
  positive root of `LeanerLang/Tests/LoopUnroll.lean`, and `designs/repros/` is
  removed. `bug_15880` test1/test4 now fail as MVP expects instead of timing out.
- The unroll exhaustion goal carries `Provenance.unrollBound`: "the bound
  `pragma unroll = N` of a loop is not established". Its first failure admits
  the remaining goals; in unroll mode split subgoals are taken first-case first.
  `LoopUnroll.lean` guards the message with an insufficient bound
  (`loop_unroll_bound::outrun`). `math8::floor_log2` (always three iterations,
  `unroll = 2`) now fails with that message instead of timing out.
- `closeTactic` is built `withRef reference`: closer messages without a clause
  range (residual obligations, callee preconditions/continuations, loop
  invariants by provenance) are reported at the function, not module line 1.
- Validation: two full runs (`/tmp/unroll-literals-logs/`). All four suites and
  the registry pass; 21 reviewed baseline diffs are only those anchor moves and
  the bound message, plus the earlier `bug_15044`/`bug_15880`/`math8`/
  `verify_vector`/`loop_unroll_false` updates. Benchmark: same outcomes (24
  verified, one expected rejection, six C17 import failures), 9,470,179,029 raw
  heartbeats (−0.482% vs 9,516,016,778; VectorOperations −16.2%), measured
  JSON `/tmp/leaner-benchmark-unroll-literals.json` = local JSON, HTML refreshed.
- Cost gates were NOT refreshed. Their `.exp` drift (+3–8% on small targets,
  −42% `classify_primitive`) predates this work; the simproc measures 0% on
  every gate target, so recording it would name targets this change does not move.
- Open, not fixed: `verify_vector` `*_with_unroll` and `bug_15044` (unbounded
  loops) still exceed 25k before reaching the exhausted bound; the return path
  of each iteration is a first case. `math_fixed8::pow_raw` remains infeasible at
  25k (needs mask-to-remainder facts for `n & 1`, omega-derived pinning of `n`,
  and ~26 nonlinear overflow leaves; MVP's default run skips it). Lean's `split`
  (default `backward.split`) is exponential in nested `ite` depth (2^k
  discharges), which inlined spec if-chains such as `spec_pow_raw` hit.
- The six C17 benchmark modules were then restored (see the bit-vector
  checkpoint above).

Suspension checkpoint — bounded loop unrolling (2026-10-07):
The user requested suspension and a handoff for another agent. The overarching
parity goal is incomplete. No commit was requested or made at this checkpoint;
preserve the large existing worktree.

Correction to the preceding checkpoint: Leaner preserved `pragma unroll = 4`
for `math_fixed8::pow_raw` but never consumed it. The old failure was under the
ordinary invariant rule, not an already-unrolled proof. There is no authored
invariant sufficient to prove the intended exponentiation relationship.

Current implementation:
- `leaner-ir/LeanerIR/Proofs/Denote/LoopUnroll.lean` proves three rules over the
  actual `Spec.fix` semantics. `loopUnroll` wraps `fixApprox` with a separate
  proof allowance; its allowance does not change semantics. Entry quantifies
  all semantic fuel, each step consumes one proof allowance, and exhaustion
  requires `False`. All normal, abort, and undefined outcomes are preserved.
  `bound + 1` allows the final while-condition check.
- `LeanerIR/Proofs/Denote/Close.lean` exposes one unrolled iteration at a time.
  `LeanerLang/Verify.lean` passes the owning declaration's nonnegative integer
  `unroll` pragma and its loop sites to the closer. Nested loops each receive
  that allowance. Ordinary invariant verification remains the default.
- `LeanerLang/Tests/LoopUnroll.lean` covers bounded symbolic counting, zero
  iterations, early return, and three kernel controls against losing normal,
  abort, or undefined outcomes at allowance zero. It is now registered in the
  IR test roots. Source counterparts are `SourceVerify/loop_unroll.move` and
  `loop_unroll_false.move` in the E2E package.

Open performance problem at this checkpoint: both `math_fixed8::pow_raw`
and a ground 2-by-2 nested loop exhausted 25,000 maxHeartbeats. The nested loop
is resolved by the literal checkpoint above (now in `LoopUnroll.lean`);
`pow_raw` remains open. Do not increase the budget or weaken the contract.

A generic `splitOnce` fast path that skipped marked `Obligation` goals was
tried and removed: it changed the search path without resolving either timeout.
No such global leaf-splitting change remains. Earlier profiler evidence pointed
to structural `split` traversing expanded conditional specifications; with the
experiment, `math_fixed8` instead spent ~13M raw heartbeats in a failing leaf
pipeline. The ground nested loop still progressed through many bounds leaves
before timing out. Relevant scratch logs:
`/tmp/loop-unroll-math-profiler.log`, `/tmp/loop-unroll-leaves-math-auto.log`,
`/tmp/loop-unroll-nested-leaves-debug.log`. The latter two describe the removed
experiment, not final performance measurements.

MVP differential evidence: the positive source fixtures verify
(`/tmp/loop_unroll-mvp.log`). MVP accepts the false `insufficient` postcondition
when `unroll = 0` truncates its loop, while rejecting `incorrect` with a sufficient
bound (`/tmp/loop_unroll_false-mvp.log`). Leaner's exhaustion rule deliberately
requires unreachability instead. Check both negative roots individually, not
just the source command's overall exit status.

Focused validation at suspension:
- `lake build LeanerLang.Tests.LoopUnroll`: PASS, 117 jobs, including all three
  positive roots and all three kernel soundness controls
  (`/tmp/loop-unroll-suspend-focused.log`).
- Source-verification dependency build: PASS, 150 jobs
  (`/tmp/loop-unroll-source-build.log`). The two positive source roots verify;
  both `insufficient` and `incorrect` are rejected independently. The new
  `loop_unroll_false.exp` was generated with the owning `Baseline.checkOutput`
  API through `/tmp/loop-unroll-source-check.lean`, restricted to these two
  fixtures; no other baseline was touched. Generation log:
  `/tmp/loop-unroll-source-check.log`.
- Axiom audit of all three new unrolling rules: only `propext`,
  `Classical.choice`, and `Quot.sound`; no admissions
  (`/tmp/loop-unroll-axioms.log`).
- Normal source-baseline recheck without updates: PASS, exit 0
  (`/tmp/loop-unroll-source-recheck.log`); expected rejection diagnostics match.
  `git diff --check` passes. All build/test handles are terminal and the process
  scan shows no remaining Lean, Lake, benchmark, or registry jobs.
  The goal is being paused at the user's explicit request.

The existing benchmark JSON/HTML and full-suite/437-registry pass belong to the
previous scalar-spec checkpoint, not these unroll changes. No full benchmark or
broad suite was started for this suspension. On resumption, finish focused
performance/source checks, rebuild the CLI and benchmark executables, regenerate
measured JSON/main-relative HTML first, then run all four suites and the normal
registry. `math_fixed8` is still unproved; do not report its timeout baseline as
successful verification. No admissions, budget increases, or source-spec changes.

Reproduce the real registry export from `../move-prover`:
```sh
/home/wrw/aptos-core/.worktrees/dev3/target/debug/move-prover \
  tests/sources/functional/math_fixed8.move --language-version 2.5 \
  --dependency=../move-stdlib/sources \
  --dependency=../move-stdlib/nursery/sources \
  --dependency=../extensions/move-table-extension/sources \
  --named-addresses std=0x1 extensions=0x2 \
  --lean --heartbeats=25 --output /tmp/bv-spec-math-fixed8.bpl
```
The current CLI executable may predate unrolling. The existing exported
`/tmp/bv-spec-math-fixed8.lean` can be checked against fresh library oleans from
`leaner-ir` using `lake env lean -Dweak.leaner.verifyHeartbeats=25000 FILE`.
Use `ELAN_TOOLCHAIN=leanprover/lean4:v4.32.2` throughout; never build shared
production oleans while their readers are running.

Table-model clarification from the user conversation: snapshots with the same
identity and different contents mean observations before/after mutation (or
functional specification updates), not duplicate live ownership. This does not
require a heap-based prover representation. An owned `{identity, contents}`
model supports it, with identity comparison separate from full Lean snapshot
equality. State-label observations at inserted caller specs must not require
program points. The table integration remains unfinished; this continuation
worked on registry unrolling, not on completing native table integration.

Specification-call numeric analysis and concrete arithmetic (2026-10-07):
Scalar ordinary/specification declarations now share one representation graph.
Nonrecursive scalar spec calls support unsigned wrapping and `num` sinks, with
physical return widths retained at generalized call nodes. An additional check
rejects signatures whose BV classification depends on a later traversal, mixed
caller representations, recursive/two-state/labeled calls, and BV spec returns
in ordinary `bv_internal` contracts. The limitation reflects MVP's first-call
signature initialization; joining all callers without checking it changes
specification arithmetic. Aggregate/generic support remains open.

`Tests.NumericRepresentation` adds ten checks for these boundaries. New source
fixtures `bv_spec_calls.move` (three roots) and `bv_spec_calls_false.move` (two
false clauses) agree with MVP: the three positives verify and both negatives
are rejected. The nested positive initially exhausted 1,500,000 maxHeartbeats
(about 1.5 billion raw heartbeats). Its leaf was concrete modular arithmetic,
but omega expanded repeated remainder constraints before substituting `x = 255`.
`leaner_denote_ground_arithmetic` now rewrites only literal Int/Nat equalities
in the target and invokes the existing closed-goal decision step before omega.
Five focused positive/negative arithmetic tests pass, including the nested
shape under 10,000 maxHeartbeats; no budget was raised.

Fresh native source measurement `/tmp/bv-spec-calls-cost.json`: all three roots
verified, 2,241,018 verification heartbeats total; nested target 763,888 raw
heartbeats. The first cost attempt used a stale benchmark executable and must
not be treated as evidence. The generator subsequently rebuilt that executable;
the fresh rerun is `/tmp/bv-spec-calls-source-cost-fresh.log`. The saved validation
recipe now explicitly builds it before standalone measurements and checks the
JSON status, since the executable can exit zero while reporting a failed target.

All four full Lean suites pass (IR, Move, Rust, E2E), including the performance
gates. `/tmp/bv-spec-calls-validation.py` returned 1 solely for the normal registry
baseline mismatch: `math_fixed8` now reaches verification instead of the
representation guard, but `pow_raw` exhausts 25,000 maxHeartbeats before a manual
`skip` can expose its obligations (the unroll pragma was ignored at that checkpoint). Its callers remain blocked. The
other 436 registry checks passed. The owning harness refreshed that one baseline;
normal full registry recheck passes all 437 tests in 82.48s with no baseline
updates (`/tmp/bv-spec-calls-registry-final.log`, exit 0).
This is a remaining proof-performance issue, not a newly proved module.

MVP's default run actually selects zero roots for `math_fixed8`, because its
`verify_duration_estimate = 60` exceeds the default timeout. A temporary source
copy removing only that scheduling pragma verifies all three roots at the
unchanged solver budget (`/tmp/bv-spec-math-fixed8-probe-mvp.log`). The original
Move file is unchanged. Generated Lean and the low-budget manual-skip probe are
`/tmp/bv-spec-math-fixed8.lean` and `/tmp/bv-spec-math-fixed8-debug.lean`; this is a
concrete next automation target. A direct source CLI probe imports a broader
stdlib dependency set and still hits the package guard; reproduce the registry
path through move-prover with its test dependency flags instead.

The benchmark/main HTML was regenerated before broad checks: 24 verified,
one expected rejection, six import failures, 9,516,016,778 raw heartbeats
(+0.069% from the previous local run). `local_benchmark.json` matches
`/tmp/leaner-benchmark-bv-spec-calls-reports.json`; type_info remains absent.
Baseline changes are the new two-clause negative source fixture and math_fixed8's
remaining timeout; the pre-run snapshot is `/tmp/bv-spec-calls-baselines-before.json`.
Representation-guard registry files decrease from 27 to 26. The two baseline
deltas are audited in `/tmp/bv-spec-calls-baseline-audit.json`. All validation
handles are terminal. No commits were made.

The low-budget debug trace `/tmp/bv-spec-math-fixed8-profile.log` confirms the
remaining timeout during loop splitting/continuation normalization, before
authored leaf tactics. It repeatedly exposes the large spec_pow_raw conditional
inside continuations. The later investigation above supersedes the original
claim that these were already unrolled: the pragma had no implementation. Preserve the 25,000 maxHeartbeats budget when
addressing this; the next step is reducing obligation preparation cost, not
raising the budget or changing the mathematical contract.

Closed scalar-package integration (2026-10-07, validated):
`ScalarPackage.preparePackage` is now called by `Encode.package`. It preserves
the original integer/internal-body route, and admits remaining seeds only when
all owned/dependency declarations can be collected and rewritten. Source bodies
retain checked operations. Derived Move spec functions are separately collected
in specification mode and rewritten too; unsupported general spec-function calls,
fields, containers, generics, inline specs/proofs and dynamic narrowing keep the
original guard. Representation pragmas are stripped only after successful
rewriting of the complete supported package.

`ScalarExpressions` now records declaration/binder provenance and declaration
roots. `ScalarWidths` resolves physical binding types independently of node
types; generalized num operands can physically render at a concrete width.
The rewriter rejects incompatible physical widths rather than inventing casts.
Literal conversions fold and already-normalized modular expressions are reused.
Statically in-range narrowing literals need no arbitrary overflow declaration.
The 70 representation checks and 25 lowering guards plus its universal kernel
proof pass (`/tmp/bv-scalar-integration-build.log`).

New `SourceVerify/bv_scalar_arithmetic.move`: all seven roots pass both MVP and
Leaner (conversion, add/subtract/multiply wrapping, division/remainder by zero,
and checked executable multiplication that aborts). Native cost is 22,775,665
verification heartbeats across seven roots (max 5,755,523), 30,269,461 total raw
heartbeats (`/tmp/bv-scalar-arithmetic-cost.json`). All six earlier false
overflow/arithmetic/signed-cast/implicit/body/pragma source regressions reach
verification and reject their postconditions. The focused registry run for
`bv_mul_overflow` now rejects `aborts_if false`, an expected baseline difference
(`/tmp/bv-scalar-registry-focused.log`).

`/tmp/bv-scalar-validation.py` finished with exit 1 solely because the normal
registry run found the restored `test_bitvector` success differed from its guard
baseline (436/437 passed). Full Move/E2E suites pass. The seven source baseline
diffs all replace import guards with rejection of false postconditions, including
`bv_internal_concrete`; the new positive source has no error baseline.
`bv_mul_overflow`'s expected failure baseline and `test_bitvector`'s now-empty
baseline were refreshed by the owning registry harness. MVP independently agrees
on both roots (`/tmp/bv-scalar-{checked-overflow,test-bitvector}-mvp.log`).
There are now 27 representation-guard registry files, down from 29.

The final normal registry recheck passes all 437 tests without updates (77.45s),
log `/tmp/bv-scalar-registry-final.log`, exit 0. No validation processes remain live.
The full benchmark and main HTML were regenerated before broad Lean checks:
24 verified, one expected AMM rejection, six unchanged import failures,
9,509,422,287 raw heartbeats. `local_benchmark.json` matches
`/tmp/leaner-benchmark-bv-scalar-reports.json`; `type_info` remains absent.
The checkpoint's nine baseline changes are audited in
`/tmp/bv-scalar-baseline-audit.json`, against the saved pre-run contents in
`/tmp/bv-scalar-baselines-before.json`. All validation handles are terminal.

Scalar numeric lowering checkpoint (2026-10-07, focused validation):
`NumericRepresentation/ScalarLowering.lean` now builds explicit mathematical
expressions for unsigned conversion, wrapping arithmetic, comparisons, bitwise
operations, shifts, and casts. It handles negative conversion inputs, SMT
division/remainder by zero, independent shift-count widths, and oversized shifts.
Narrowing requires a caller-supplied occurrence-specific arbitrary expression;
there is no default witness. Allocating independent symbolic declarations and
connecting resolved node/physical operand representations remain open. These
builders are not wired into production import, so no source guards were removed.

`Tests.NumericLowering` is registered and passes 24 guards, including 4,116
generated-expression/BitVec comparisons across all six Move widths, plus a
kernel proof of the conversion identity for every integer. The earlier 66
representation checks also pass (`/tmp/bv-lowering-tests.log`). A diagnostic
generator `/tmp/RenderNumericLowering.lean` sends constructed lowered formulas
through the real XAST→validated LIR→Leaner printer. Six resulting verification
controls pass (wrapping, division/remainder by zero, narrowing bound and in-range
cast, oversized shift); false wrapping and zero-on-narrowing-overflow controls
fail their ensures clauses. Logs: `/tmp/bv-lowering-{good,false,DivisionZero,
RemainderZero,NarrowingBound,NarrowingFalseWrap,NarrowingInRange,OversizedShift}.log`.
These controls test the builders through the backend; they do not establish
that the original guarded Move sources now import or verify. No benchmark or
registry outcome changed. No processes remain live; broad suites were not rerun
for analysis/builders outside the production path.

Scalar numeric analysis checkpoint (2026-10-07, focused validation): added
`NumericRepresentation/{Constraints,ScalarExpressions,ScalarWidths}.lean` and
registered `Tests.NumericRepresentation`. The analysis reserves shared function
slots, distinguishes executable/specification propagation, and separates ordinary
`bv_internal` contracts from bodies. The width pass then infers concrete types
through nested bitwise specification arithmetic and adapts only marked literals
that fit. It retains independent occurrence identities, shift-count widths,
boolean result types, and explicit cast targets. Explicit suffix/range mismatches
are diagnosed; integer arithmetic does not acquire those width restrictions.

All 66 focused guards/kernel examples pass (`/tmp/bv-width-tests.log`). The
assembled analysis also succeeds on seven exported source files (15 functions,
168 expressions; `/tmp/bv-width-source.log`). A fresh export of the two-module
`bv_spec_literal_default` registry source rejects the explicit `u8`/`u256`
mismatch at the same expression as MVP. An isolated analysis of its two positive
functions adapts both defaulted spec literals to `u8`, including the caller's
clause (`/tmp/bv-literal-width-{analysis,mvp}.log`). This is analysis evidence,
not source verification. `git diff --check` passes.

The new analysis remains outside production import. Do not remove guards until
whole-relevant-source classification and representation-aware lowering exist.
Open work includes binder/physical-local widths, fields/containers, generic and
specification-function slots, inline specs/proofs, consumer conversions, wrapping
spec arithmetic, and arbitrary narrowing results. No registry baseline or
benchmark outcome changed at this checkpoint, and broad suites were not rerun
for unused analysis infrastructure. The last full benchmark/suite checkpoint is
the literal-metadata run below. No checkpoint processes remain live.

Numeric literal metadata checkpoint (2026-10-07, validated): XAST version 10
now preserves `defaulted_num` on value nodes. The exporter copies the compiler's
`SpecDefaultedNumLocs` marker; the Lean mirror/decoder retains it until source
representation analysis. This distinguishes `3` from `3u256` when both export
at type `u256`, including schema-expanded expressions. Existing exchange
baseline diffs contain only the version and literal markers. New owning
fixtures are `move-model/exchange/tests/ast_sources/defaulted_num.move` and
`leaner-move/Tests/Xast.lean`. All 59 exchange tests pass without updates;
Cargo check, package formatting and package Clippy with `--no-deps` pass.
The repository `xclippy` alias is not package-scoped, and a dependency-inclusive
Clippy attempt encountered existing `aptos-metrics-core` warnings; no unrelated
lint changes were made. Logs: `/tmp/bv-literals-{cargo-check,exchange-check,
rustfmt,clippy-no-deps}.log`.

The remaining classification/lowering requirements are recorded in
[`numeric-representation.md`](numeric-representation.md). In particular,
bitvector-to-bitvector narrowing spec casts produce a target-typed arbitrary
value on overflow, unlike modular `int2bv`. The new
`SourceVerify/bv_representation_downcast.move` explicitly seeds bitvector
parameters: MVP proves the in-range round trip, result bounds and wrapping
conversion, and rejects both false narrowing claims. Generated Boogie contains
distinct arbitrary symbols for the two cast sites (`/tmp/bv-downcast.vc_0003.bpl`,
`/tmp/bv-downcast-mvp.log`). The Leaner guard still rejects this unimplemented
family; no new registry verification success is claimed for the metadata work.

`/tmp/bv-literals-validation.py` completed with exit 0. It rebuilt the ci Move
CLI and registry executable against v10, built Move/E2E, regenerated the full
benchmark/main HTML first, refreshed the source baselines, then passed full
Move/E2E suites and all 437 registry checks without updates (152.89s).
`local_benchmark.json` matches `/tmp/leaner-benchmark-bv-literals-reports.json`:
24 verified, one expected AMM rejection, six unchanged import failures,
9,509,559,974 raw heartbeats. HTML still ranks expensive targets first and
excludes `type_info`. No processes from this checkpoint remain live. General
bitvector representation analysis/lowering remains the next implementation
step; keep existing soundness guards until affected flows are modeled.

Scalar `bv_internal` checkpoint (2026-10-07, validated): opaque scalar bodies
now retain executable Move bitwise semantics while their ordinary contracts and
callers stay in integer representation. `bv_internal_wrapping` verifies `g`
and rejects the deliberately false contract of `f`, matching MVP. Four new
positive roots cover explicit `bv`/`bv_ret`, a caller crossing the callee's
width, mutable-reference updates and their caller. All four pass both backends.
Inline specifications, concrete clauses and outgoing calls have independent
negative fixtures: MVP rejects their false claims and Leaner rejects their
unsupported representation flow. Aggregates/generics and proof blocks also
remain guarded; general representation analysis is still open.

The reference test exposed a separate importer bug: specification casts did
not implicitly read reference operands. `Encode.addExpr` now inserts that read
only in logical contexts. `reference_spec_cast.move` verifies current/old and
shared-reference cases while rejecting a false unchanged-value postcondition
in both backends. The explicit `num` cast also avoids an MVP code-generation
type mismatch in the mutable `bv_internal` postcondition.

`/tmp/bv-internal-validation.py` completed with exit 0. It rebuilt Move/E2E,
regenerated the full benchmark and main-relative HTML first, refreshed owning
baselines, then passed full Move/E2E suites and all 437 registry checks without
baseline updates (registry 159.93s). Exactly two registry baselines changed:
`bv_internal_wrapping` now reaches verification, and `bv_internal` now reports
the remaining concrete-clause boundary. There are 29 representation rejections,
down from 30; baseline agreement is not general MVP parity. Logs:
`/tmp/bv-internal-{validation,move-tests,e2e-tests,registry-check}.log`.
No processes from this checkpoint remain live. Core/Rust were unchanged.

The refreshed `local_benchmark.json` matches
`/tmp/leaner-benchmark-bv-internal-reports.json`: 24 verified, one expected
AMM rejection, six unchanged import failures, 9,509,276,380 raw heartbeats.
The generated HTML retains the most expensive targets first, compares with
main, and excludes `type_info`. Coverage is unchanged from the seed-guard
checkpoint; its lower coverage than earlier runs is not a speed improvement.

Representation-seed follow-up (2026-10-07, validated): the explicit
wrapper guard was insufficient. MVP rejects `(x & 255) + 1 > 255` at `x = 255`,
but Leaner accepted it; `pragma bv = b"0"` also made plain `x + 1 > 255` an
incorrect acceptance. A bitwise operation in executable code can classify
the returned value and change arithmetic in its postcondition as well.
Evidence: `/tmp/bv-{implicit,pragma}-arithmetic-{mvp,leaner}.log` and
`/tmp/bv-body-mvp.log`. All three have independent SourceVerify fixtures.

`Frontend/NumericRepresentation.lean` now checks complete XAST declarations
before encoding, including dependency interfaces, retained inline bodies,
specifications, proof steps and frames. It rejects numeric AND/OR/XOR,
`bv`/`bv_ret` pragmas, and explicit conversion wrappers. Merely rejecting
operations inside spec expressions would miss executable-to-contract flow.
The four direct implicit/pragma/body/signed-cast probes reject normally after
the build (`/tmp/bv-seeds-*-check.log`). No integer-or-bit-vector classification
analysis or wrapping rewrite has been implemented yet; restoring that support
remains the next priority. `/tmp/bv-seeds-validation.py` refreshes the complete
benchmark/main HTML first, then owns both E2E baseline refreshes, the registry
refresh and ordinary full Move/E2E/registry checks. It completed with exit 0;
no validation processes from this checkpoint remain live. The prior checkpoint
below is historical.
The refreshed JSON/HTML now reports 24 verified, one expected AMM rejection,
and six import failures (`features`, `cmp`, `math_fixed`, `ristretto255`,
`ed25519`, framework `ordered_map`), at 9,509,557,743 raw heartbeats. This is
reduced coverage, not a speed improvement. The authored LeanerLang `OrderedMap`
still verifies. The registry refresh passes 437 files; 30 now stop at the
representation boundary, including 10 with previously empty Leaner baselines
(`/tmp/bv-seeds-registry-audit.json`). Full Move and E2E suites pass, as does
the ordinary 437-file registry recheck (169.68s). No baseline updates were
enabled for those final checks. Logs: `/tmp/bv-seeds-{validation,move-tests,
e2e-tests,registry-check}.log`. The active parity goal is not complete.

Bit-vector soundness checkpoint (2026-10-07): `/tmp/bv-guard-validation.py`
completed with exit 0. Move/E2E builds, source baseline regeneration, the
full Move and E2E suites, and all 437 registry checks pass; the final registry
check ran without baseline updates (206.15s). The full measured benchmark
and generated main-relative HTML were refreshed before the broad suites:
27 verified, one expected AMM rejection, and three import failures
(`features`, `ristretto255`, `ed25519`), totaling 13,664,061,946 raw heartbeats.
The total excludes their former verification work and is not a performance
improvement. `local_benchmark.json` matches
`/tmp/leaner-benchmark-bv-guard-reports.json`; top expensive targets remain
first and `type_info` is absent. Logs: `/tmp/bv-guard-{validation,benchmark,
move-tests,e2e-tests,registry-check}.log`. Core/Rust are unchanged since the
preceding full checkpoint. Restoring the affected samples with sound
representation semantics remains open; baseline agreement is not MVP parity.

LIR import failures now return source error reports instead of throwing,
so the benchmark records their frontend heartbeats and the source suite can
continue. This also gives both independent overflow regressions their own
generated `.exp` baseline. Errors currently anchor at input line 1; precise
location recovery remains G1 work. The CLI only claims to have generated a
rendering after success, since an import failure may leave an older output
file in place. Its rebuilt executable passes positive and stale-output negative
probes (`/tmp/bv-guard-cli-{build,positive,negative}.log`).

Conditional-range follow-up (2026-10-07): the range solver now specializes
conditional quantified facts at a goal's integer positions after its original
matching path fails. It preserves range premises and uses known guards at the
new endpoint. `ArithmeticContext` passes both branch cases and rejects missing
branch/endpoint evidence (`/tmp/conditional-range-build.log`, 116 jobs).
The even-count fold remains a native-25k timeout: the installed change reduces
its diagnostic closer work from 39.076M to 34.551M raw heartbeats, but does not
finish its authored proof. A scratch nonnegative-remainder normalizer plus the
range prototype passes at 48.803M; a prefix-count lemma passes at 78.885M and
is rejected as slower. Neither scratch arithmetic change is installed and no
acceptance budget or count-even companion has changed. See perf-notes.md.

The checkpoint driver `/tmp/conditional-range-checkpoint.py` completed successfully:
the refreshed full benchmark and main-relative HTML retain 30 verified samples
plus one expected AMM rejection at 15,066,137,470 raw heartbeats (+0.0328% from
the preceding checkpoint). Both cost gates and the fold/for-each/pure-callee
registry baselines pass without refresh. The subsequent full checkpoint passes
all four builds and suites plus all 437 registry baseline checks without
refresh (`/tmp/conditional-range-full-tests.log`, exit 0;
`/tmp/conditional-range-full-registry-check.log`, 145.92s). This also validates
the registered Table boundary checks above the previous pool checkpoint.

Next frontend lead: `bv_signed_generic` is rejected because source `bv2int`
and `int2bv` wrappers have the polymorphic signature `T -> T`, while the
adapter maps them to numeric LIR conversion operations. Inspect the source
builtin signatures and the Boogie translator's instantiated representation
rules before changing this boundary; numeric conversion and representation
wrappers must not be conflated. The importer now rejects both source wrappers
explicitly until their representation semantics are supported.
The executable MVP probe `/tmp/bv_wrapper_semantics.move` confirms that these
wrappers cannot simply be erased: with `x = 255`, it proves
`bv2int(int2bv(((x as num) + 1) as u8)) == 0` and rejects equality to 256,
while proving the signed generic `int2bv(bv2int(x))` round trip. The log is
`/tmp/bv-wrapper-mvp.log`. The builtins' `T -> T` signature is a representation
interface, not a theorem that the conversion preserves every mathematical
integer. Explicit unsigned truncation, signed clamping, and generic
instantiation need separate treatment; do not relax the numeric validator or
erase source wrappers merely to get the generic fixture past validation.

The stronger negative probe `/tmp/bv_wrapper_range.move` exposed an actual
false acceptance: at `x = 255`, Leaner erased `int2bv` and proved
`int2bv(((x as num) + 1) as u8) > 255`, which MVP rejects. The import guard
now rejects that source (`/tmp/bv-wrapper-range-guard.log`, exit 1), and
`SourceVerify/bv_representation_overflow.move` records the rejection.
Move and E2E builds pass (276/320 jobs). This is a soundness correction,
not completed bit-vector support: it also rejects previously accepted
`features` and the package-wide MoveStdlib translation fixture. Their old
successes passed through the same conversion-erasing path. The guard is
newer than the full conditional-range checkpoint above; its own benchmark
and registry validation must be reported separately.

A second executable MVP probe, `/tmp/bv_wrapper_arithmetic.move`, proves
`int2bv(x) + int2bv((1 as u8)) == 0` at `x = 255` and rejects the same sum
being greater than 255 (`/tmp/bv-wrapper-arithmetic-mvp.log`). Both operands
already lie in range. Thus adding a modulo at each wrapper alone is also
incorrect: the surrounding addition must carry the representation. A separate
E2E `SourceVerify/bv_representation_arithmetic.move` retains this case, so one
unsupported import cannot mask the other regression. Inspect the MVP number-operation
analysis before implementing the representation flow across operations,
locals, function results and instantiated generic types.

The compiler export `/tmp/bv_wrapper_arithmetic.xast.json` gives that addition
the source type `num`, even though MVP evaluates it as an 8-bit sum. The
export carries operand types and wrappers but no representation classification.
Therefore the result's source type alone cannot determine the modulus. The
representation analysis must retain the originating width across expressions
and distinguish an explicit cast to `num`, which severs that propagation.

Signed-wrapper propagation probe (2026-10-07): even a wrapper on a statically
signed operand cannot simply be erased. At `x : i8 = 127`, MVP proves
`(int2bv(((x as num) + 129) as i8) as u8) == 0` and rejects its being greater
than 255. Without the wrapper, it proves the cast expression equals 256.
Evidence: `/tmp/bv_signed_cast.move`, `/tmp/bv-signed-cast-mvp.log`.
The signed wrapper has integer rendering but retains a `Bitwise` classification
that propagates into the unsigned cast. Thus classification and rendering must
be separate in the implementation; signed clamping is not permission to remove
the classification. `SourceVerify/bv_representation_signed_cast.move` retains
the negative case independently. No signed-wrapper erasure was installed.
The owning source suite passes with its generated baseline
(`/tmp/bv-signed-cast-source-baselines.log`, exit 0). MVP rejects the fixture's
postcondition, Leaner rejects the unsupported wrapper, and an unwrapped
control verifies in Leaner with value 256. Logs:
`/tmp/bv-signed-cast-fixture-{mvp,leaner}.log` and
`/tmp/bv-signed-cast-control-leaner.log`. The measured benchmark and preceding
full checkpoint are unchanged; this follow-up adds regression coverage only.

Owned-value carrier follow-up (2026-10-07): `TableValueBoundary.Observed`
retains a typed physical value and a logical snapshot with a representability
witness. `Observed.view` checks live-state consistency; generic transport preserves
the snapshot with inverse proofs. `ArgumentRow` maps the existing physical HList
to these enriched values; top-level mutable arguments retain current/prophetic
observations at two supplied memories. Tuple components and `ResultShape` also
retain their separate reference observations. `StateView.prod/atState/prophecy`
compose these boundaries. Nineteen registered checks pass, including same-handle/different-
contents current and prophetic values, rejecting both at one state, retained old/new
Table values, mixed Table argument/result rows, void results and generic transport. Six more theorem
audits use only standard axioms. Logs: `/tmp/table-observed-carrier-final-tests.log`,
`/tmp/table-observed-carrier-audit.log`.

Next integration must connect enriched values to the existing loan-resolution
relation and compiler/contract route. `StateView.function` checks **ordinary**
inputs at one state: do not apply it to mutable rows by demanding that a future
snapshot match entry storage. For escaping loans the supplied prophetic memory
must resolve the returned row; physical function-exit memory alone is insufficient.
Nested references/other profiles remain separate obligations. Table native
allocation/mutation roles are still disabled. No source/registry/benchmark
improvement is claimed from these test-only adapters; the full checkpoint below
remains authoritative.

Table frame dependencies (2026-10-07): new `Denote/TableFootprint.lean`
collects the typed slots read by a snapshot, including nested stored Tables,
aggregates, reference observations, absent slots and instantiated generics.
It proves observation equality from agreement on the pre-state footprint,
and from a disjoint write frame. `PreservesOutside.set/trans` frames typed
updates, including allocation history only when explicitly listed.
`footprintRuntime?` checks encoded contract inputs; its encode and generic
instantiation laws preserve malformed-input rejection. The eight-theorem
standard-axiom audit passes (`/tmp/table-footprint-audit.log`). The registered
`Tests/TableFootprint.lean` has eight checks: absent and nested dependencies,
a negative outer-only frame, unrelated-handle preservation, unregistered shape,
allocator-history preservation, distinct phantom instances, malformed input.
The fresh 45-job focused build passes (`/tmp/table-footprint-final-build.log`).
This is a test-imported leaf, not yet in the public verifier import closure;
the pool checkpoint below remains the last full benchmark/four-suite result.
No Table native role was enabled and no registry baseline was changed here.

Next: follow **Owned Table values in the denotation** in `intrinsic-maps.md`.
The user questioned exposing native storage in ordinary Table proofs; the selected
approach is an owned identity+contents value. The experimental `ownedTableSlots`
source-frame extension was removed in full. Do not resume it as the client model.
`StateView` and `Denote/TableValueBoundary` now provide a checked representation
boundary without requiring handle-only erasure to be injective across states.
Native mutation/allocation dispatch and the logical carrier are still unconnected;
no new registry pass is claimed. See the design's dependency-ordered integration
steps and the existing full-checkpoint evidence below.

Boundary validation: registered `LeanerIR.Tests.StateView` covers identity erasure
across states, impossibility of a handle-only codec, recovery at the final state,
rejection of an old result at the new state, inconsistent inputs even for a bottom
computation, undecodable successes, abort and undefined preservation, actual Table
snapshot recovery, a typed-storage update returning its new value, rejection of
two distinct live snapshots at one state, and generic instantiation. The 120-job
core/frontend plus boundary build passes (`/tmp/table-value-boundary-checkpoint.log`),
and the final 46-job focused build passes (`/tmp/table-value-boundary-final-tests.log`).
Seven boundary theorem audits use only standard axioms (five use none), recorded in
`/tmp/table-value-boundary-audit.log`. The restored public verifier does not import
these new leaves; no changed benchmark result or broad-suite run is claimed.


Pool performance checkpoint (2026-10-07), COMPLETE:
The speculative residual-search ceiling is 2M raw heartbeats, still at most
five percent of the target budget; native 25k retains its original 1.25M
allowance. The pool companion tries map coverage before general automation.
The AMM companion handles abort branches before deriving successful-return
fee bounds. No acceptance budget, source specification or baseline was weakened.
The complete benchmark preserves 30 verified + one expected AMM rejection
at 15,061,197,787 total raw heartbeats (4.03% below 15,694,278,677). Pool is
2,940,369,701 (-7.68%); native buy_in is 729,515,623 versus 847,944,044 and
deduct_shares 445,113,352 versus 571,115,483. Ordered_map is 4,031,232,214 and
AMM 1,293,879,404. Measured JSON and main-relative HTML are refreshed, with the
global ranking first and type_info absent. All four builds and full suites pass,
and all 437 registry baselines match without refresh (146.59s). Both cost gates
pass. Session 14303 is terminal (0); `/tmp/pool-final-checkpoint.log` and
`/tmp/pool-final-*` hold the evidence. This also validates the product endpoint
guard after its earlier two registry regressions. No checkpoint readers remain.

A separate redeem_shares manual-preparation experiment is rejected: it verifies
but costs 1,087,346,931 versus 642,092,999 isolated typed raw heartbeats.
No redeem_shares companion is installed. Scratch comparisons are
`/tmp/PoolRedeem{Current,Prepared}.lean` and `/tmp/pool-redeem-*.log`.

Read-only next-registry probe during the pool checkpoint:
`bp_forwarding::all_values_bounded` proves automatically at diagnostic 100k
(29.014M in the closer) but still exceeds native 25k. The only residual in a
manual probe is the early-break equivalence: the current element exceeds 100,
so the universal bound is false at the current index. A direct current-index
witness proves it at diagnostic 100k (30,252,694 whole typed raw heartbeats),
but the 25k probe times out before the authored work can finish. No proof file,
acceptance limit or baseline was changed. Scratch sources/logs are
`/tmp/BpAllValues{Current,Witness,Witness25k}.lean` and
`/tmp/bp-all-values-{current,witness,witness-25k}.log`. Preparation/loop/assertion
cost, rather than the final quantified inference, is the next lead for this
specific target. Do not count the higher-budget probe as a parity fix.

Benchmark presentation (2026-10-07): the generator now opens the page with
the latest run’s 20 most expensive targets, ranked by heartbeats, with problem
links and explicit outcomes (including expected rejections). The generated
local HTML is refreshed against main. All eight Python report tests pass;
the rendered ranking matches the latest measured JSON. No Lean rebuild or
full-suite run was needed for this rendering change.

Product-fold range-bound follow-up (2026-10-07):
`vector_hofs_fold::product_concrete` now verifies at native 25k. The range
instance solver tries congruence (`grind only`) at the new endpoint, so an
accumulator equality also transports a multiplication bound. Previous
normalization fallbacks remain. ArithmeticContext includes a positive case
and rejects the same inference without either the current bound or the
accumulator equality. The original solver fails the positive regression.
The installed whole typed proof costs **23,960,945 raw heartbeats**, 22,596
objects, transport 282,210. Native refresh removes only the product timeout;
`count_even_concrete` is the sole remaining failure in this fold module.
Core/Move builds, both cost gates, ArithmeticContext, LiteralOptionalReads,
CallRangeResults and StateLabelMemoryEquality pass. Benchmark/report refresh
finished, preserving 30 verified + one expected rejection. The registry
check then found two regressions (435/437 matched): count_all in bp_pure_callee
no longer proves, and vector_hofs_for_each::find_value changes from a clause
failure to a timeout. These baselines were not refreshed. Session 97209 is
terminal (101); no readers remain. The endpoint congruence attempt is now
restricted to goals with symbolic products, preserving the original path
for linear goals. Core/Move builds, ArithmeticContext and both cost gates pass.
Native bp_pure_callee, vector_hofs_for_each and vector_hofs_fold all match
their existing baselines without refresh; the product improvement is retained.
Logs: /tmp/range-congruence-guard-*. The completed pool checkpoint above
subsequently validates this guard in all four full suites and all 437 registry
checks without refresh.

Remaining even-count investigation (scratch only):
The quantified overflow invariant contains a conditional, but its goal has
already selected a branch. Specializing that invariant at the actual Int
position, normalizing only the new fact, then applying the range solver
closes the four range-extension leaves. `simpByContext` requires `setGoals`
after adding facts; omitting it produces a misleading "No goals to be solved".
Nonnegative literal-list reads also connect spec `Int.tmod` to runtime `%`.
Ground `Int.tmod`/`Int.tdiv` reductions are absent from lir_denote_norm;
adding their standard simprocs eliminates the last concrete-result arithmetic
residual. The combined scratch proof succeeds at a diagnostic 100k but costs
52,910,850 typed raw heartbeats; it still times out at native 25k. No count
helper, literal-read nonnegativity rule, extra reduction rule, or search-policy
change is installed. `/tmp/FoldCountGroundRemainderQuickProfile.lean` and
`/tmp/fold-count-ground-remainder-quick-profile.log` record the successful
higher-budget diagnostic, not a native acceptance. It spends 41.061M before
the authored proof (25.141M on residual leaves); merely reducing early search
adds extra normalization and does not solve the native cost problem.

Optional integer reads and sum folds (2026-10-07):
The literal certified-read normalizer now also handles optional reads,
preserving signed values and the actual fallback. Its certificate uses
`val_getD_getElem?_map_val`; `Option.bind_fun_some` is normalized as well.
Registered `LiteralOptionalReads` covers unsigned/signed nonzero fallbacks
and a symbolic stored value. The native `vector_hofs_fold` runner now verifies
`sum_concrete`, `sum_inferred`, and `sum_scaled` at the unchanged 25k budget.
Their companion unfolds recursive specifications, rewrites the goal using
context equations, then decides finite index cases. Whole typed artifacts
cost 27,897,980 / 27,332,778 / 27,979,895 raw heartbeats respectively; those
include work outside the native verification-budget scope. No residual-search
or acceptance-budget change is installed. The owning runner removed exactly
these three obsolete timeout sections; product and even-count still time out.
Core/Move builds, both cost gates, optional-read, call-range, and labeled-memory
fixtures pass. The full benchmark preserves 30 verified + one expected rejection at
15,688,482,526 raw heartbeats (-0.0493% versus aliasing). Measured JSON and
main-relative HTML are refreshed, without type_info. All 437 registry
baselines match without refresh (`/tmp/fold-optional-checkpoint.log`, exit 0). All four full suites passed
at the preceding aliasing checkpoint; they have not been rerun for this increment.

Aliasing and conditional-frame follow-up (2026-10-07):
`state_labels/aliasing.move` now fully verifies at the unchanged native 25k
limit. The owning runner removed its obsolete failure baseline after the
ordinary run showed empty output. `different_addr_global` costs **19,832,667
typed raw heartbeats**, 13,887 proof objects, transport 216,817. Two updated
memories are compared in a bounded 3M-heartbeat attempt; reflexivity handles
opaque empty row tails before congruence resolves the remaining reads.
The generic simplifier's self-reference check now also inspects conditional
and quantified equations. Such equations remain available facts but are not
rewrite rules. A decoded-read branch consumes the resulting present-value
observations without expanding the cyclic frame. It uses the existing decoded
read budget; no native acceptance setting or specification changes.
`StateLabelMemoryEquality.lean` is registered: writer and opaque caller verify,
and a wrong labeled update is rejected. No callee program point is required
by the caller. The caller costs **11,577,355 typed raw heartbeats**, 8,649 objects,
transport 298,173. ArithmeticContext includes two conditional-cycle regressions.
Core (118 jobs) and Move (276 jobs) builds pass; both cost gates, all five focused
state-label fixtures, and ArithmeticContext pass. Logs use
`/tmp/aliasing-observations-`, including `native`, `native-refresh`, `writer-cost`,
`caller-cost`, and fixture names. The full checkpoint is **complete**, wrapper session 48800 exited 0.
Benchmark: **30 verified + one expected rejection**, no timeouts,
**15,696,214,192 raw heartbeats** (+19,472,285, +0.1242% versus the Boolean
checkpoint). Framework ordered_map costs 4,135,557,520; LeanerLang OrderedMap
costs 1,433,966,047. `/tmp/leaner-benchmark-aliasing-observations.json` matches
local JSON, and generated main-relative HTML excludes type_info. **All four
builds and full suites pass**, including the registered MemoryEquality and
ArithmeticContext regressions. The full registry matches **437/437**, without
further refresh. Logs `/tmp/aliasing-observations-{checkpoint,benchmark,
full-tests,registry-check}.log`. No checkpoint reader remains.

Boolean-observation and eligibility follow-up (2026-10-07, checkpoint complete):
Only direct ResultOf equalities and positive/negative AbortsOf facts now enable
the early call-observation proof. Mentions inside quantified invariants no
longer make constructor proofs pay for that speculative attempt. The isolated
AMM create_pool proof verifies again at 25k (`/tmp/boolean-observation-create-pool.log`).
The bounded Boolean-observation proof is also installed. It requires multiple
distinct conditional equalities for each Boolean polarity, prepares the leaf
to identify unchanged memories, then cases on a shared guard and simp_all.
The attempt has a 3M raw-heartbeat cap and restores on failure.
`test_config_preserved` verifies at 25k, **8,793,754 typed raw heartbeats**,
6,552 objects, transport 366,506 (`/tmp/boolean-observation-target.log`).
The earlier aborts_if_at_state_label proof still passes at 20,251,543 typed
heartbeats. The new registered StateLabelBooleanObservations fixture has three
verified functions and one intended clause rejection; both cost gates and all
three prior state-label fixtures pass. The core build passes (118 jobs).
The frontend build passes (276 jobs); the native AMM baseline matches without
refresh. Intermediate_states' reviewed diff removes only test_config_preserved's
obsolete failure, preserving the two intended clause rejections. The owning
runner refreshed it; the ordinary 30-file state-label recheck passes.
Logs `/tmp/boolean-observation-{amm-native,intermediate-native,
intermediate-refresh,state-labels}.log`. The regular benchmark and full registry recheck are complete (wrapper exit 0,
session 97473). Benchmark: **30 verified + one expected rejection**, no timeouts,
**15,676,741,907 raw heartbeats** (-1,200,444 against call-observation).
`/tmp/leaner-benchmark-boolean-observation.json` matches local JSON; main-relative
HTML is refreshed with no type_info. Full registry recheck: **437/437**, without
refresh. Logs `/tmp/boolean-observation-{checkpoint,benchmark,registry-check}.log`.
All four full suites passed immediately before these two small follow-ups;
focused label and cost gates cover the final changes. No checkpoint reader remains.


Call-observation follow-up (2026-10-07):
`aborts_if_at_state_label::caller` now verifies at the unchanged native 25k
budget. The owning runner confirmed empty output and removed its obsolete
timeout baseline. Before rebuilding behavioral contracts, try the facts already
supplied by the call rule on leaves mentioning ResultOf or AbortsOf. The
attempt is capped at 3M raw heartbeats and restores the whole goal/context on
failure. Its arithmetic bounds omit generated magnitude facts; the ordinary
fallback keeps the original full bounds. No premises or specifications change.
All 16 target leaves close in this attempt. Native-budget diagnostic cost:
**20,251,470 typed raw heartbeats**, 14,523 proof objects, transport 396,238
(`/tmp/call-observation-target.log`). The registered StateLabelCallObservations
fixture verifies both opaque callees, their labeled caller, and an opaque
caller of that contract; it rejects a false postcondition. Both cost gates,
StateLabelDecoding and StateLabelWitnesses pass. Core and Move builds pass
(118 and 276 jobs). Logs: `/tmp/call-observation-{build,move-build,regression,
native,native-refresh}.log` and `/tmp/call-observation-{DenotePerformance,
CompositionPerformance,StateLabelDecoding,StateLabelWitnesses}.log`.
The ordinary 30-file labeled registry recheck passes (no refresh), log
`/tmp/call-observation-state-labels-recheck.log`. The initial run also found
`aliasing::remove_then_try_read` newly verifies; intermediate_states no longer
times out on test_config_preserved, but its result equality remains unproved.
Those reviewed diagnostics were refreshed by their owning runners.
The regular benchmark is complete: **30 verified + one expected rejection**,
no timeouts, **15,677,942,351 raw heartbeats** (+33,195,672, +0.2122% against
invocation-reuse). `/tmp/leaner-benchmark-call-observation.json` matches local
JSON; main-relative HTML is regenerated and excludes type_info. The periodic
full-suite/437-registry checkpoint is running (session 49131) via
`/tmp/call-observation-checkpoint.py`, log `/tmp/call-observation-checkpoint.log`.
All four builds and all four full suites pass. The registry finished 436/437:
amm_example::create_pool gained a timeout because the invocation fast-path
eligibility scanned inside quantified invariants. Its baseline was **not**
refreshed. The checkpoint exited 1; no readers from it remain. The direct-fact
eligibility correction below is under validation.

Pre-install scratch evidence (superseded by the Boolean follow-up above): intermediate_states'
`test_config_preserved` has four implications equating two results to the
same counter value when Config.active is true and to zero when it is false.
The residual only needs cases on that shared Bool, followed by simp_all.
`/tmp/NextIntermediateBoolBudget.lean` manual diagnostic verifies at 25k,
11,011,614 typed heartbeats. The generic automatic scratch attempt is now
`/tmp/NextIntermediateBoolScopedSingle.lean`: **9,230,291 typed heartbeats**,
6,552 objects, transport 582,905, both budgets 25k. It requires at least two
distinct guarded equalities for each polarity, prepares the leaf (identifying
the unchanged memories), then splits a Bool antecedent and simp_all. The whole
attempt has a 3M raw-heartbeat cap. It changes no premises.
`/tmp/NextIntermediateBoolScopedAllSingle.lean` tests the complete module:
only the two intended negatives fail, with clause rejections and no timeouts.
Logs `/tmp/next-intermediate-bool-{ScopedSingle,ScopedAllSingle}.log`.
**Scratch override caveat:** a failing new macro_rules alternative can trigger
the old macro expansion again. The scratch macro wraps its whole alternative
in `try` so a failed proof leaves the goal open (reported by the main closer)
instead of running the original solver twice. The earlier negative timeout
was this artifact; the narrowed guard skips that negative entirely. Native
production insertion should be an ordinary first-alternative, not a second
macro_rules declaration. The native insertion is now implemented as described above.
Generated source `/tmp/next-intermediate-states.lean`; residual trace
`/tmp/next-intermediate-residual.log`. These are diagnostic artifacts; native
acceptance and the checked-in regression are recorded at the top of this file.

Previous invocation-reuse guards remain installed: hasKnownInvocation requires
matching EnsuresOf, ResultOf and StateOf facts; abortCases? recognizes direct
positive/negative abort facts by reducible conversion (encodeFor abbreviates
encode). Their regular benchmark completed with **30 verified + one expected
AMM rejection**, no timeouts, **15,644,746,679 raw heartbeats** (-2,547,240
against the witness checkpoint). `/tmp/leaner-benchmark-known-invocation.json`
matches local JSON; main-relative HTML has no type_info entry. Logs:
`/tmp/known-invocation-{checkpoint,benchmark}.log`. This benchmark predates the
new call-observation attempt; the latest full suites also predate those guards.

Scratch findings: simplifying call-result equations only after deriving every
contract cost 26.541M even with magnitude bounds omitted; moving all bounds
after simplification lost useful certificates and triggered fallback. Reusing
call observations before contract derivation resolves the target instead.

Latest witness-search follow-up (2026-10-07):
`spec_fun_old_param_labeled_with_memory::inc_under_cap_twice` now verifies at
the unchanged native 25k limit. Construct witnesses from the predicate and
context before enumerating program points. Typed proof cost falls from
30,912,290 to **24,132,039 raw heartbeats**, with the same 15,523 proof objects
(transport: 208,976). The new registered `StateLabelWitnesses` fixture passes
at 25k; `StateLabelDecoding`, both cost gates, and the existing StateLabels
fixture pass (the latter retains byte-identical rejection diagnostics).
The owning runner removed the obsolete timeout baseline; the ordinary
state-label-directory recheck passes **30/30**. Logs:
`/tmp/state-label-witness-{build,fixture,decoding,total,registry-recheck}.log`
and `/tmp/state-label-witness-{DenotePerformance,CompositionPerformance}.log`.
The regular benchmark passed: **30 verified + one expected AMM rejection**,
no timeouts, **15,647,293,919 raw heartbeats** (+164,359, +0.00105% against
the decoded-state checkpoint). `/tmp/leaner-benchmark-state-label-witness.json`
matches local JSON; generated HTML compares against main and excludes type_info.
After that refresh, **all four builds and full suites pass**, and the full
registry matches **437/437** baselines without refresh. The checkpoint wrapper
exited 0 (session 49647): `/tmp/state-label-witness-checkpoint.log`,
`/tmp/state-label-witness-full-tests.log`, and
`/tmp/state-label-witness-registry-check.log`. No checkpoint process remains.

Next target scratch investigation (no production changes):
`aborts_if_at_state_label::caller` has a valid diagnostic proof in
`/tmp/NextAbortLabelSingle.lean` (exit 0; `/tmp/next-abort-label-single.log`).
It disables leaf/residual closers, then proves contradictions with omega and
uses `simp_all [LeanerIR.packResults_single, Int.tdiv_eq_ediv_of_nonneg]` on
remaining leaves. Nine residuals reduce to none. This uses a 500k diagnostic
budget; the pre-script closer alone costs about 210M heartbeats, so it is not
an accepted native fix or installed companion. Pure equality simplification
leaves the quotient-overflow branch and the nested conv ResultOf equality.
The latter needs the known fee ResultOf rewritten before singleton packing
and nonnegative tdiv/ediv normalization. Expanding `packResults` itself instead
of its singleton lemma causes expensive simplification. The initial early-leaf
version still fails (`/tmp/NextAbortLabelFastLeaf.lean`); adding bounds and
Obligation normalization also fails (192.131M typed diagnostic heartbeats;
`/tmp/NextAbortLabelFastLeafBounds.lean`). No scratch implementation is installed.

Latest follow-up (2026-10-07): decoded state-label updates now normalize known
memory values before proving decoder range checks. Known decoder results are
used before rewriting their inputs, and present-value witnesses are extracted
from both non-absence and isSome facts. This preserves the synthesized memory
labels used when an opaque contract replaces a call; no callee ProgramPoint is
required. `two_state_labels::two_increments` now verifies at the unchanged
native 25k limit. The runner removed its obsolete timeout baseline; the final
state-label-directory check matches **30/30** baselines
(`/tmp/state-label-decoded-registry-directory.log`). Its whole typed proof costs
**18,269,904 raw heartbeats / 11,428 objects**, plus 367,301 transport heartbeats
(`/tmp/state-label-decoded-total.log`); the previous high-budget diagnostic still
failed its clauses, so this is a proof-automation fix, not just a cost reduction.
`LeanerLang/Tests/StateLabelDecoding.lean` is registered and passes three positive
proofs (including an opaque caller) plus two expected rejections (missing
resource and overflowing update), all at 25k. Both cost gates and the existing
DefinedStateLabels/InvocationStateLabels fixtures pass. Native build and logs:
`/tmp/state-label-decoded-final-build.log`,
`/tmp/state-label-decoding-registered.log`, `/tmp/state-label-decoded-final-*Performance.log`,
`/tmp/state-label-decoded-{defined,invocation}.log`.
The regular benchmark completed successfully: **30 verified + one expected AMM
rejection**, no timeouts, **15,647,129,560 raw heartbeats** (+199,877, +0.0013%
against the restored-guard checkpoint). `/tmp/leaner-benchmark-state-label-decoded.json`
matches local JSON; main-relative HTML is regenerated with type_info absent.
Logs: `/tmp/state-label-decoded-{checkpoint,benchmark}.log`. The native
table_option recheck also passes. No new full-suite run was scheduled for this
increment; the preceding all-suite/437 registry checkpoint below remains the
latest broad validation. Scratch variants of a false
one-increment labeled claim still time out; they are not installed regressions
or newly verified targets.

Status: 2026-10-06, branch `wrwg/lean4` (worktree `dev3`); resumed the MVP-parity goal after checkpoint `fff7651976`. Vector normalization, bounded residual search, and vector companions pass all four full suites, including both cost gates and the 123 Check fixtures. The subsequent opaque-inline frontend fix passes the Move package suite and all 437 registry baseline checks, with no regression in previously verified targets.

Current checkpoint (2026-10-07): vector element quantifiers pass five positive
proofs and two intended rejections at 25k. The runtime-value specialization
preserves every benchmark target outcome: 30 verified problems and one expected
rejection, at 15,744,282,692 raw heartbeats (+0.1096% over certified reads).
`/tmp/leaner-benchmark-element-quantifiers-final.json` is byte-identical to
local JSON; generated HTML compares with main and omits retired type_info.
All four full suites pass, including both cost gates and 130 Check fixtures
(`/tmp/element-final-full-tests.log`). The full registry check matches 436/437
baselines; its only difference is that `moved_local_in_loop` now verifies at
25k. The owning runner removed its obsolete timeout baseline, and the ordinary
focused recheck passes (`/tmp/element-final-moved-local-recheck.log`). This is
one full registry run plus one focused recheck, not a second full run.
The earlier literal-vector companion regression was repaired before these
checks. `verify_vector` matches its baseline in the final full run. Changes
remain uncommitted; continue with remaining parity gaps.

Current follow-up (2026-10-07): generated contracts now carry external Table
entry invariants alongside the existing global-resource predicate. Declaration
callbacks retain resolved native arguments, including phantom parameters;
Table operations in bodies and specifications extend memory reach even for a
Table parameter with no global read. The closer derives stored invariants from
a successful raw-key lookup, without reconstructing a certified numeric key.
Snapshot bridges cover both handle layouts and dependent type arguments.
`TableStoredInvariants.lean` passes nine positive proofs and two intended
rejections at 25k (`/tmp/table-stored-invariants-source-final.log`). A separate
frontend regression rejects a Table-identity invariant that the initial broad
physical-input shortcut could have strengthened incorrectly
(`/tmp/table-stored-invariant-errors.log`). Only a bare vector local's length
bypasses observation now; content-sensitive stored invariants remain explicit
unsupported cases. The eight-theorem audit has only standard Lean axioms
(`/tmp/table-invariant-integration-audit.log`).

The fresh native CLI build passes. The ordinary `table_option` registry check
still matches its timeout baseline (`/tmp/table-invariant-native-registry-final.log`):
this is **not** a successful verification. A diagnostic source run before the
final length-only restriction closes the proof at about 25.99M raw heartbeats
(`/tmp/table-option-Production-profile.log`), above the unchanged 25k budget.
Do not transplant its larger diagnostic budget into acceptance settings.
The full benchmark refresh preserves all outcomes: 30 verified and one expected
AMM rejection, at 15,725,534,459 raw heartbeats (−18,748,233, or −0.1191%,
from the element-quantifier checkpoint). The measured artifact
`/tmp/leaner-benchmark-table-invariants.json` is byte-identical to local JSON;
the driver regenerated main-relative HTML, with `type_info` absent. Capability
falls from 283,915,114 to 237,395,590 heartbeats; framework ordered_map rises
from 4,145,558,609 to 4,157,454,627, still fully verified. All four full suites pass after that refresh, including both cost gates and
131 Check fixtures (`/tmp/table-invariants-full-tests.log`). The full registry
run matches 435/437 baselines (`/tmp/table-invariants-registry-check.log`):
`different_addr_global` now finishes with an ordinary verification failure
rather than a timeout; `verify_remove_with_unroll` still times out with one fewer
clause diagnostic. Neither is a newly verified target. The owning runner
refreshed these two diagnostic baselines; both ordinary focused rechecks pass
(`/tmp/table-invariants-aliasing-recheck.log`,
`/tmp/table-invariants-vector-recheck.log`). This is one full run plus two
focused rechecks, not a second full registry run.
Next finish the `table_option` timeout reduction. Table identity/content
invariants, carrier-dependent generic callbacks, mutation/allocation contracts,
and ownership frames remain open.

Current performance follow-up: snapshot equality uses only the normalizer
rules and context equalities. The closer-only `table_option` diagnostic improves
25,986k → 24,867k raw heartbeats, but the **whole theorem costs 29,139,877 raw heartbeats**
(`/tmp/table-option-measured-total.log`). The registry still times out at
25k; its ordinary baseline check matches. Nine positive/two negative invariant
checks and the identity guard pass with this change. The full benchmark is refreshed: 30 verified plus one expected AMM rejection,
15,725,421,251 raw heartbeats (−113,208, or −0.00072%, locally).
`/tmp/leaner-benchmark-snapshot-equality.json` is byte-identical to local JSON;
the generated main-relative HTML excludes type_info. Both focused cost gates
pass (`/tmp/snapshot-equality-DenotePerformance.log`,
`/tmp/snapshot-equality-CompositionPerformance.log`); no post-change full-suite
result is claimed. The detailed heartbeat
trace isolates about 1.26M in initial tactic work and 2.58M in kernel checking
(tracing adds overhead). Reduce proof construction/checking as well as leaf
search; do not treat the closer profile as the whole acceptance budget.

Current call-range checkpoint (2026-10-07): speculative call normalization
first checks for a possible small literal range, retaining transported or
unresolved views conservatively. Ineligible continuations remain untouched.
`CallRangeResults.lean` covers eight verified targets at the unchanged 25k
limit: bounded opaque results, strict large bounds, mutable results, and generic
instantiation (`/tmp/call-range-results-generic.log`). `table_option` improves
to **28,686,998 raw heartbeats for the whole theorem**, with unchanged 22,076
proof objects; it still times out at the registry limit. The ordinary registry
baseline check matches (`/tmp/call-range-guard-table-option-registry.log`).
The full benchmark preserves 30 verified targets and one expected AMM rejection
at **15,624,166,021** raw heartbeats, down 101,255,230 (0.6439%) from the snapshot
checkpoint. `/tmp/leaner-benchmark-call-range-guard.json` is byte-identical to
local JSON; HTML is regenerated against main and excludes type_info.
All four package builds and full suites pass, and all 437 registry baseline
checks match (`/tmp/call-range-checkpoint-full-tests.log`). No baseline refresh
was needed at this checkpoint.
A separate scratch fixture exposes an existing computed-index gap: bounded
`result` is connected to the actual vector index only by `named.val = result.val
- 10`. Direct-position range detection misses this equality. The failure also
reproduces with the pre-guard implementation. A scratch scalar-equality rewrite
before descendant-position detection proves all eight targets at 25k and rejects
four deliberately false postconditions. Guarding the rewrite on nonempty literal
bounds avoids about 1.1M extra Table proof work in comparable scratch runs.
The computed-index fix is installed after the full-suite checkpoint. Native
build and both focused cost gates pass. `CallRangeResults.lean` now passes 16
positive targets and two guarded rejections (false value and out-of-bounds
computed index) at 25k (`/tmp/computed-index-regression.log`). The installed
Table theorem costs 28,689,764 heartbeats, just 2,766 above the preceding
checkpoint; proof objects remain 22,076. Its native registry timeout baseline
still matches. The `verify_vector` registry check also matches unchanged
(`/tmp/computed-index-vector-registry.log`). The subsequent regular benchmark preserves every outcome at
15,646,803,818 raw heartbeats (+22,637,797, +0.1449%). The measured artifact
`/tmp/leaner-benchmark-computed-index.json` matches local JSON; main-relative
HTML is refreshed and excludes type_info. The all-suite result above predates
this computed-index change.

Table-option normalization checkpoint (2026-10-07): `table_option` now verifies
with the unchanged native 25k setting. The runner removed its obsolete failure
baseline; the ordinary recheck passes (`/tmp/table-option-fixed-registry-recheck.log`).
Whole typed-theorem cost is **25,254,204 raw heartbeats**, down 3,435,560 (11.9749%),
with **21,500 proof objects** (was 22,076); transport costs 465,850. The full
sample includes processing outside the local verification-budget scope; the
actual native check establishes acceptance.
The implementation transports a stored invariant through the read's exact
lookup equality before normalization, selects known aggregate fields before
physical encoding, and removes
trivial integer-vector invariant traversal. Unknown snapshot projections retain
the original normal form. The initial broad reversal regressed generic counts
and phantom-tagged entries and was replaced before baseline regeneration.
Native build, all nine positive/two negative Table invariant cases, the identity
guard, both cost gates, 16 positive/two negative call-range cases, TableReads,
TableSnapshots and the kernel-level snapshot/invariant tests pass. New lemmas
use only standard Lean axioms (`/tmp/table-normalization-final-table-normalization-audit.log`).
The first normalization benchmark completed but exposed a regression:
framework ordered_map upsert/remove-or-none/iter-collect become much more
expensive, and LeanerLang OrderedMap times out. The checkpoint script stopped
at its outcome assertion, so **no full suites ran for this batch**. Its honest
measured data/report are `/tmp/leaner-benchmark-table-option-fixed.json` and
local JSON/HTML. The culprit is restricting call-range eligibility to context
facts. In an isolated identical upsert source, restoring the original guard
reduces typed work from 432,274,533 to 120,693,726 heartbeats (scratch overrides
add overhead). Merely looking at the context misses necessary call simplification.
The original guard, including target transports, is restored in production;
the corrected native build passes (`/tmp/table-option-restored-guard-build.log`).
The remaining Table normalization is unchanged. A scratch table_option proof
still verifies at the actual 25k setting with the restored guard. The native
ordinary registry check also passes (`/tmp/table-restored-registry-recheck.log`),
as do both focused cost gates (`/tmp/table-restored-{DenotePerformance,CompositionPerformance}.log`).
The corrected regular benchmark passes: **30 verified and one expected AMM
rejection, no timeouts**, at **15,646,929,683 raw heartbeats** (+125,865,
+0.0008044% against the previous good computed-index checkpoint). Framework
ordered_map costs 4,123,667,100; LeanerLang OrderedMap costs 1,433,690,178.
`/tmp/leaner-benchmark-table-restored.json` matches local JSON; the generated
HTML compares against main and omits type_info. All four builds and **all four full suites pass**, and the full registry
**matches 437/437 baselines without refresh** (`/tmp/table-restored-full-tests.log`,
`/tmp/table-restored-registry-check.log`). The checkpoint process is terminal
zero. With the conservative guard restored, the final whole Table theorem
cost is **26,045,418 raw heartbeats, 21,500 proof objects**, plus 465,826
transport heartbeats (`/tmp/table-restored-total.log`). This supersedes the
25,254,204 measurement from the context-only guard; native 25k acceptance
remains independently confirmed. Logs use `/tmp/table-restored-`. The failed checkpoint
process is terminal, not active. Do not reuse its full-suite status as a pass.

A separate read-only investigation captures vector_hofs_fold and two_state_labels
under `/tmp/next-proof-*/generated.lean`. sum_concrete verifies at a diagnostic
200k with 45.414M whole-theorem heartbeats; two_increments still has real residual
clauses at that diagnostic budget. The fold misses plain-index upper bounds
written as closed arithmetic (`1 + 1`), and spends about 10M in fallback pipelines.
A scratch add/sub literal recognizer finds those bounds but does not yet get
the fold under 25k. No closed-arithmetic bound change is installed.
Further scratch work finds inconsistent literal reads: certified optional reads
and integer reads are separate omega atoms. A proof-backed literal-only
normalizer plus Option.bind_fun_some reduces sum_concrete to 29,379,460 raw
heartbeats (`/tmp/NextFoldOptionalBind.lean`), but the actual 25k check still
fails (`/tmp/next-fold-optional-budget.log`). It is not installed. A List lookup
bridge plus splitting normalized range premises similarly reduces the proof
to 29,143,345 (`/tmp/NextFoldSplitRange.lean`), still only a diagnostic result.
For two_increments, decoded-read preparation uses Option.ne_none_iff_exists,
which produces reversed `some value = memory resource key` equations; the
memory-read tactics recognize only the other direction. Explicitly rewriting
these reads and discharging the integer decoder's fit condition closes a
manual scratch proof, including the synthesized state-label witness. The version
using Option.ne_none_iff_exists' and existing read tactics verifies at a
diagnostic budget with 34,792,885 typed heartbeats / 12,431 objects
(`/tmp/NextTwoIncrementsPrime.lean`, `/tmp/next-two-increments-prime.log`),
but still times out at 25k (`/tmp/next-two-increments-budget.log`). This is
an automation/performance lead, not an installed fix or registry success.
All experiments are scratch only; the full-suite checkpoint above validates
unchanged production sources.

The preceding full benchmark is `/tmp/leaner-benchmark-certified-reads-final.json`: 30
verified problems, one expected AMM rejection, no unexpected failures/timeouts,
and 15,727,043,391 raw heartbeats (−81,383,741, or −0.5148%, from vector ranges).
Local JSON is byte-identical to the measured artifact. The driver regenerated
HTML against main CI 37250691414 (`ea4ecc43e7`) before broad checks. Retired
problems no longer reappear in the HTML index or sections from historical runs;
`type_info` is absent. All four suites now pass, including both cost gates
and 129 E2E Check fixtures (`/tmp/certified-read-full-tests.log`). The full
437-file registry check matched 432 baselines and exposed five differences.
The stale `count_all` companion was repaired and passes its unchanged baseline;
the four reviewed diagnostic baselines were refreshed and pass normal rechecks
(`/tmp/certified-read-pure-callee-recheck.log`,
`/tmp/certified-read-registry-refresh.log`). `folds_of_callee_ensures::count_small`
now verifies at 25k. No previously verified target remains regressed; the
diagnostic audit is `/tmp/certified-read-registry-audit.json`. The preceding
full-run registry checkpoint follows for historical comparison.
The shared-Table-read full validation passes: all four suites (127 E2E Check
fixtures and both cost gates), and all 437 registry baseline checks
(`/tmp/table-reads-full-tests.log`, `/tmp/table-reads-prover-check.log`).
The full audit (`/tmp/table-reads-registry-audit.json`) has 122 files without
diagnostics, 5 with warnings only, 47 rejected before verification, 231 with
verification diagnostics, and 32 compiler/other errors. Intentional negatives remain rejected;
these categories are not parity counts. Changes are uncommitted.

The subsequent canonical Table-slot increment passes its focused proofs and
source checks; all four scoped registry checks pass with a freshly built CLI.
The following full-storage/returned-loan agreement increment passes five focused
test roots and a 31-theorem standard-axiom audit. Shared Table read roles are now
enabled through explicit intrinsic-contract assumptions, with five positive
source proofs and a negative regression. The remaining `table_option` obligations
need aggregate snapshot normalization and deep invariants of stored values.
Allocating/mutating roles remain disabled. These follow-ups close no additional
whole registry fixture. The subsequent aggregate-projection normalization has
its own refreshed benchmark and focused regressions; the shared-read run remains
the latest full-suite checkpoint.

The literal-swap-read increment is validated: the ground benchmark target
falls from 64,554,891 to 47,189,204 heartbeats (−26.9%), and `VectorOperations`
falls 7.4%. All four full suites pass, including both cost gates and the 123
Check fixtures. All 437 registry checks pass with no baseline changes. The
validated residual-search budget is retained; reverse preparation still
exceeds its 25M acceptance limit.

## Where the state is

- [`roadmap.md`](roadmap.md): project status and the test ledger.
- [`state-labels.md`](state-labels.md), "Milestones": S1, S2, S3a, S3b implemented; S4 open.
- [`prover-test-problems.md`](prover-test-problems.md): the Prover-test registry.
  The latest full run discovers 437 files: 122 have no diagnostics and 5
  only warnings. All 437 full-run baselines refresh successfully, with no
  regression in previously verified targets; intentional negatives
  mean baseline matches are not proof/parity counts.
- [`verification-benchmarks.md`](verification-benchmarks.md): the benchmark.

## Next, in order

Current user goal: address most Move Prover registry gaps and approach MVP
parity. The vector and opaque-inline batches below are validated. Continue with
the remaining registry gaps and measured proof costs. Keep benchmark data and
its generated main-relative HTML ahead of broad tests; run full suites at batch
boundaries, and preserve the existing acceptance budgets.

1. AMM/calculator valid targets now verify and all four full suites pass.
   AMM's intentionally non-compliant constructor now rejects normally at
   185,036,096 heartbeats, down 87.7% from its 1.5G timeout. The benchmark labels
   it **expected rejection**, and counts rejected attempts in suite totals.
   Remaining performance work includes calculator `process`
   (989M), and ordered-map performance: all
   29 targets verify with zero errors, at 4,157,416,781 total heartbeats. The
   largest remaining target is `test_verify_iter_walk_mut_symbolic` (955M),
   followed by drain (439M). A restricted `grind only` companion experiment
   did not close the walk proof and was discarded. The pool now verifies all 22 targets with zero errors or timeouts;
   `buy_in` costs 854M heartbeats instead of its 1.5G timeout.
   `behavior` now verifies all 20 targets after the nested-invocation fix below.
   `capability` verifies completely. The resumed vector batch reduces pool
   `deduct_shares` to 560M. `type_info` was removed from the manifest at the
   checkpoint; the current full benchmark has 31 problems.
2. The C12 opaque-inline positive gap is fixed and validated. Larger entries
   by test count remain V7 intrinsic maps (5), C8 `update` of a
   spec variable (9), V16 natives (7), and `update_field` rendering (V1, 11
   messages). The next proof lead is vector reverse preparation;
   its diagnostic profile is recorded in `perf-notes.md`.
   V7 investigation: Table specification equality must use allocation identity
   (user decision, 2026-10-06), including distinct executions at the same `new`
   call site. Contents and identity are separate observations. Move's function
   call borrow rules tie returned entry references to the whole Table argument;
   five focused compiler probes confirm shared/successive/distinct-Table borrows
   compile and conflicting same-Table borrows fail even at different keys.
   See `intrinsic-maps.md`, "Ownership of table contents". This permits reuse of
   owned-map reasoning, but Table view/native storage agreement, fresh allocation,
   old/labelled snapshots, and the registry's empty abstract map owners remain
   unimplemented. Probe packages and logs: `/tmp/leaner-table-borrow-probe-*`.
3. S4: attribute the remaining `state_labels/` failures. Invocation labels now
   translate in `aborts_if_at_state_label::caller`, but its proof exceeds the
   runner's 25k heartbeat budget. Reduce preparation cost while retaining that limit.
   `unmodified_memory_at_label::swap` still needs a decision about reads of
   removed resource contents (see the state-label design).

## This increment

- Certified integer reads retain bounds without runtime encoding/projection;
  computed `.val` reads supply their certified bounds. Unused continuation
  alias chains are cleared without clearing definitions still required by a
  goal or hypothesis. `CertifiedReads` has four positives and an intended
  signed-value rejection at 25k; `ContinuationCleanup` checks both removal
  and preservation. Both cost gates pass unchanged. Framework `ordered_map`
  verifies all 29 targets after its duplicate-key companion normalizes its
  helper fact to the new read form. The full benchmark retains every target
  outcome and cuts comparator heartbeats 21.0%, bit_vector 3.2%, and features
  1.6%. `foreach` is not fixed: its instrumented positive-only diagnostic
  drops 48.483M → 36.190M, but the ordinary 25k probe still times out.
  Logs: `/tmp/certified-read-{focused,validation}.log` and
  `/tmp/certified-read-foreach-profile.log`.

- Element-wise vector preconditions now establish indexed-read properties.
  `forall_runtime_mem_map_iff` exposes the native element predicate before the existing
  membership-to-position normalization. `leaner_denote_instantiate_positions`
  keeps non-position value binders universally quantified through
  `bindOpenPremises` instead of rejecting them. No guessed value or additional
  assumption is introduced. `ElementQuantifiers` proves reads from value and
  reference vectors, signed integers at arbitrary in-range positions, Booleans,
  and a generic equality predicate. It rejects both a stronger bound and a
  missing-entry claim at 25k.
  Core build, all six focused checks, and both cost gates pass
  (`/tmp/element-quantifiers-narrow-build.log`, `/tmp/element-final-focused.log`).
  Final benchmark and broad checks pass as described above. The earlier scratch files
  `/tmp/ElementQuantifierBinder{Probe,Negative}.lean` preserve the isolated
  diagnosis; their private-helper aliases are not part of production code.
  Installed cost refinement: restrict the mapped-list universal
  rewrite to predicates over `RuntimeValue` instead of registering generic
  `List.forall_mem_map`. All six source checks still pass with that local rule
  (`/tmp/ElementQuantifiersNarrow.lean`), as does a generic-vector equality
  read at 25k (`/tmp/ElementQuantifierGeneric.lean`). Isolated native comparisons
  reduce `bit_vector::shift_left_for_verification_only` 252,685,657 →
  248,810,251 heartbeats and `features::change_feature_flags_for_next_epoch`
  40,974,012 → 40,297,016. See `/tmp/element-{shift,features}-compare.log`.
  The proved specialization sits beside the mapped-list lemmas in `Types`,
  replaces the generic attribute in `Close`, and has the generic source check.
  The native CLI build and repaired literal-vector registry check pass. The ordinary
  `foreach` positive probe still times out at 25k with the wider rule
  (`/tmp/element-quantifiers-foreach.log`).

- Table integration is in progress. `Proofs/Maps/Table.lean` separates
  allocation identity from content snapshots, reuses entry-level map laws,
  proves mutable-entry reconciliation frames/stable key positions, and
  constructs fresh logical allocations. `Proofs/Denote/TableMemory.lean`
  carries unbounded typed entries in logical slots scoped by owner/types and
  handle. Reads take memory explicitly, so old/labeled states need no program
  point. Physical Table fields remain handles/metadata.
  `RuntimeState.tables` holds native contents and persistent allocation
  history. History uses a fixed runtime-family carrier, with no callee
  substitution or artificial Move-vector bound.
  `StorageEncodes` connects both stores to prophetic calls, runtime contracts,
  and behavioral predicates. `AgreeUnnamed` preserves resources outside
  globals, Table contents, and allocation history. Loan independence shifts
  native contents and preserves allocation identities; `state_of` uniqueness
  covers both stores. `LeanerIR`/`LeanerLang` build and focused Behavior tests
  pass (`/tmp/table-call-language-build.log`, `/tmp/table-call-behavior.log`).
  The strengthened uniqueness, loan-mirroring, and runtime-contract proofs
  use only Lean's standard logical axioms
  (`/tmp/table-call-agreement-audit.{lean,log}`).
  Full benchmark `/tmp/leaner-benchmark-table-call.json` completed and the
  driver regenerated local JSON/HTML against main: 30 verified, one expected
  AMM rejection, no unexpected failures/timeouts; 15,824,599,521 raw heartbeats
  (−136,422 from the preceding run, −0.00086%). All four suites passed the
  preceding storage-field increment (`/tmp/table-storage-full-tests.log`).
  For the call-agreement increment, all four full suites and all 437 MVP
  registry baseline checks pass (`/tmp/table-call-full-tests.log`,
  `/tmp/table-call-prover-check.log`, `/tmp/table-call-e2e-recheck.log`). E2E
  initially found a stale theorem in `Check/Generics/Generics.lean`; it now
  requires `StorageEncodes`, its focused check passes, and the full rerun
  reaches and passes the MonoVM/differential stages. Those readers are done.
  The following returned-storage increment is now installed and under
  validation. `Denote/ReturnedStorage.lean` observes heap holes through an
  explicit returned row, preserves allocation history, and proves lookup,
  sortedness, plain-row identity, and loan-renaming laws. `propheticRun` and
  `closureMeaning` encode post-storage at the prophecy row; runtime contracts,
  `ensures_of`, and labeled post-state uniqueness use the current row.
  `Contract.frame` now takes the result between the states. Source `modifies`
  frames keep their original predicates; the runtime adapter ties its storage
  observation to the actual returned row, rather than an unconstrained witness.
  Typed/skolem adapters, closure-frame automation, and the LooseFrame fixture
  carry the added result. No source specification or heartbeat budget changed.
  Core proofs build (`/tmp/returned-storage-core-build.log`) and new
  `LeanerIR/Tests/ReturnedStorage.lean` checks pass
  (`/tmp/returned-storage-test.log`). Language and E2E builds pass
  (`/tmp/returned-storage-language-build.log`, `/tmp/returned-storage-e2e-build.log`).
  Nine focused checks pass, including closure frames, generics, returned refs,
  global borrows, and the new result-frame regression
  (`/tmp/returned-storage-focused.log`). The axiom audit uses only standard
  Lean logical axioms (`/tmp/returned-storage-audit.log`). Its full benchmark
  `/tmp/leaner-benchmark-returned-storage.json` finished successfully: 30 verified,
  one expected rejection, no unexpected failures/timeouts, 15,826,904,482 raw
  heartbeats (+0.0146% from the preceding checkpoint). Local JSON matches the
  measured artifact, and generator-produced HTML still compares against main
  CI 37250691414 (`ea4ecc43e7`). All four package builds, IR/Move/Rust suites,
  and all 437 registry baseline checks pass (`/tmp/returned-storage-full-tests.log`,
  `/tmp/returned-storage-prover-check.log`). E2E exposed a real automation regression in
  `StoredWriteFrames.concrete_call`, plus extra diagnostics in two related
  negatives. The result-frame migration accidentally added a fifth binder to
  the four-binder `FramedAt.mono` premise. Restoring its original arity fixes
  all three fixtures; the distinct five-binder `Contract.frame` implication
  keeps the result. The owning generator confirms all original baselines are
  restored and the spurious positive `.exp` is removed
  (`/tmp/returned-storage-refresh-frames.log`). Language rebuild passes
  (`/tmp/returned-storage-frame-fix-build.log`); ordinary capped rechecks follow
  in `/tmp/returned-storage-frame-recheck.log`. Earlier source/render/Check
  stages and remaining MonoVM/differential stages completed, but no full
  E2E rerun is claimed after this fix. No specification or budget was changed.
  Backups before result-aware frames: `/tmp/result-frames-before/`.
  **Next:** finish routing validation/benchmark refresh, then native operation
  contracts, registration, and ownership frames. Table
  frontend roles still reject unsupported owners; no V7 target is claimed
  fixed. Routing must distinguish native slots from globals even at identical
  `GlobalKey`s. The observed post-state does not itself perform a write-back.
  A concrete interpreter probe returning two field references from a borrowed
  global owner writes both fields correctly (`/tmp/returned-global-pair.lean`,
  empty success log `/tmp/returned-global-pair.log`). The first-hole transfer
  helper alone is not evidence of a reachable multiple-field bug; keep the
  enclosing owner lifetime in reproductions. Tagged storage-loan routing was
  prototyped in `/tmp/table-routing-checkpoint/leaner-ir` and installed after
  all preceding readers finished. It replaces `globalLoans` by `storageLoans`
  with `.global`/`.table` targets; no artificial key encoding or heap scan.
  The isolated core/language build passes (`/tmp/table-routing-core3.log`),
  as do reference tests and three kernel-checked write-back regressions
  (`/tmp/table-routing-references2.log`, `/tmp/table-routing-returned-storage4.log`).
  General read-after-write, unrelated-slot, and allocation-history laws also
  compile (`/tmp/loan-target-laws.log`). Shared IR/language build now passes
  (`/tmp/table-routing-shared-build.log`, 112 jobs). The actual shared axiom
  audit contains only standard Lean axioms (`/tmp/table-routing-audit.log`).
  `/tmp/table-routing-validate.py` finished: Move/E2E executable builds and
  all 14 focused checks pass. Its full benchmark is
  `/tmp/leaner-benchmark-table-routing.json`: 30 verified, one expected AMM
  rejection, zero unexpected failures/timeouts, 15,826,709,038 raw heartbeats
  (−195,444 from returned-storage, −0.0012%). JSON exactly matches the measured
  artifact; generated HTML is still relative to main CI 37250691414
  (`ea4ecc43e7`). The driver refreshed both before broad testing.
  All four full suites and all 437 registry baseline checks then passed,
  including the new Table-loan test root and the complete E2E/MonoVM stages
  (`/tmp/table-routing-full-tests.log`, `/tmp/table-routing-prover-check.log`).
  No baseline changes. The former closure-frame regression is covered by this
  full passing run. All benchmark and suite readers are terminal.
  Backup of changed shared sources: `/tmp/table-routing-before/`.

  `Semantics/TableLoans.lean` now implements shared lookup and mutable entry
  registration by key/index. `borrowStoredAt?_discipline` proves fresh
  registration/frontier preservation; `borrowStoredAt?_mirror` proves loan
  independence. `TableEntryFocus` proves selection, holing, and reconciliation
  while retaining the key, other entries, globals, and allocation history.
  Nine executable checks cover real key lookup, missing keys/slots, malformed
  rows, colliding store keys, and successive borrow/write cycles
  (`/tmp/table-loans-tests.log`). These primitives are a directly imported
  leaf module; `nativeCall` dispatch and Table frontend roles are not enabled.
  After the full passing run, an equivalent entry decoder was factored out to
  avoid a Lean equality-theorem failure on array literal patterns. Key lookup
  is now proved to select exactly a `(queried key, value)` pair. The changed
  leaf rebuilds and its focused tests pass (`/tmp/table-loans-key-build.log`,
  `/tmp/table-loans-key-tests.log`); the audit has only standard Lean axioms
  (`/tmp/table-loans-key-audit.log`). No other core import or executable changed
  in that follow-up, and no full rerun is claimed for it.
  The next carrier audit found a size mismatch: handle-only Table natives
  impose no total-entry-count guard, but `Contents` inherited the Move vector
  bound. `ResourceKind.collection` now stores unbounded finite arrays of typed
  elements; ordinary `.value` resources retain their native carrier. Table
  contents and allocation history use collection resources; history is a fixed
  address collection instead of a `.param 0` slot. Encoding injectivity,
  loan-freedom, memory uniqueness, global typing, and call agreement compile.
  `ResourceType.subst` preserves the domain. Its regression exposed reversed
  `NRow.subst` arguments in `Contract.quoteResource`; the helper now instantiates
  the resource's row with the specification call's arguments, including phantom
  parameters. `Storage/GenericSpecResourceArguments.lean` covers two-parameter
  specifications reading a one-parameter resource, in both parameter positions
  and from generic callers. `Tests/TableMemory.lean` covers arbitrary finite
  carrier size, ordinary vector bounds, instantiation, domain separation, and
  distinct snapshots at one allocation identity.
  All 18 focused checks pass (`/tmp/table-contents-focused.log`), including the
  existing generic-resource negative. The proof audit uses only standard Lean
  axioms (`/tmp/table-contents-audit.log`). The capability companion's four
  resource constructors were migrated to the explicit `.value` kind.
  The final full benchmark (`/tmp/leaner-benchmark-table-contents-final.json`)
  is 30 verified plus the expected AMM rejection, no unexpected failures or
  timeouts, and 15,807,436,992 raw heartbeats (−19,272,046 / −0.122% from the
  previous Table-routing measurement). `local_benchmark.json` matches the
  measured artifact byte-for-byte, and the generator refreshed HTML relative
  to main CI 37250691414 (`ea4ecc43e7`) before broad testing. Framework
  ordered_map is 29/29 at 4,157,756,570 heartbeats; capability 6/6 at 283,352,792;
  pool_u64 22/22 at 3,189,740,143; calculator 8/8 at 1,066,861,880.
  All four full suites and all 437 registry checks pass through
  `/tmp/table-contents-full-tests.py` (log `/tmp/table-contents-full-tests.log`).
  This includes both cost gates, all 124 E2E Check fixtures, and MonoVM/differential
  checks. The new positive fixture has no diagnostic baseline. All 317 existing
  registry baselines are byte-identical to the pre-change snapshot; no baselines
  were updated. All benchmark and suite readers are terminal.
  **Next integration:** typed native contracts/registration tied to owner,
  type and handle; whole-memory preservation and ownership frames;
  logical Table snapshots in the specification carrier, including nested/old
  and labeled observations, functional `spec_set`, and identity equality.
  Directly interpreting `old(t)` as an old physical handle and then reading
  current memory is wrong. Labels must remain usable without program points.
  Table roles and the empty abstract map owners remain unsupported; no V7
  registry fixture is claimed fixed yet.

  The subsequent operation leaves now implement fresh-handle allocation,
  absent-key insertion, removal and checked empty retirement. Their proofs
  establish exact effects, freshness/no reuse across owner/type instances,
  unrelated-storage and loan-bookkeeping frames, sortedness and loan renaming.
  Typed shared lookup/add/remove agree with execution, including absence and
  duplication. Typed mutable borrowing now follows the real key lookup and
  retains a typed focus whose write-back preserves keys, distinctness and
  surrounding entries. The native adapter still owes owner/type resolution,
  whole-memory preservation, ownership and production-native agreement.
  The focused build passes 48 jobs, including eleven executable lifecycle/error
  checks and three generic typed boundary examples
  (`/tmp/table-operations-focused-final.log`). All seventeen audited theorems
  depend only on standard Lean axioms (`/tmp/table-operations-audit.log`). These
  leaves are imported only by their tests, so no new benchmark or broad suite
  run is claimed; the full checkpoint above remains authoritative.
  **Next:** connect the specification aggregate's physical-only representation
  to the view that retains nested content snapshots per occurrence. In
  `LeanerLang/Contract.lean`, inspect `Domain.aggregate`, `runtimeAggregateValue`
  and `mapSpecCall`. Preserve projections, lets, specification calls, quantifiers,
  old/labeled memories and functional updates; two snapshots of the same nested
  identity can coexist in one aggregate. Do not implement direct-handle reads
  as a shortcut for `table_contais_to_length`. Pure `spec_new` needs a logical
  identity policy; it cannot perform executable fresh allocation. Native models
  are registered in `LeanerLang/Profile.lean`.

  `Denote/SnapshotValue.lean` now implements the separate logical carrier.
  Table nodes carry physical metadata and optional, recursively observed
  contents; missing storage is distinct from empty storage. Ordinary aggregate
  projection and Table lookup retain child snapshots. Functional set/remove
  update contents and any cached length while preserving allocation identity;
  `SameIdentity` compares Table identities recursively inside aggregates and
  never substitutes their content snapshots. Typed observation preserves the
  physical codec encoding and is injective at a fixed memory. `observeInFrame`
  resolves types before observation, with a generic caller/callee transport
  theorem. `observeRuntime?` is the checked bridge for codec-encoded inputs;
  already-logical locals must bypass it to retain their original observation.
  A physical projection alone cannot license a behavioral/native invocation of
  a functional snapshot; that boundary still needs agreement with its selected
  execution memory. Pure `spec_new`'s identity policy also remains open.
  Eleven new kernel-checked regressions pass, including nested Tables at two
  explicit memories, two different functional snapshots of the same nested
  identity in one parent, generic instantiation, cached lengths, missing
  storage, and an unregistered struct with the same physical layout.
  Focused build: `/tmp/snapshot-value-focused.log` (44 jobs). Twelve audited
  theorems use only standard Lean axioms (`/tmp/snapshot-value-audit.log`).
  Contract translation now imports this carrier. It observes physical inputs at
  the selected clause memory and retains logical values through aggregate
  expressions, lets, expanded spec calls, branches and vector operations.
  Length/membership/get/set/remove Table roles now use these observations.
  `table_contais_to_length` verifies at its unchanged budget; its obsolete
  baseline was regenerated and a normal filtered recheck passes
  (`/tmp/snapshot-routing-registry-{refresh,check}.log`).
  Five new source proofs pass, including caller-side labels with no program
  points and two functional snapshots with equal identity but different contents
  in one vector. Six focused compatibility checks pass, including the carrier,
  entries-layout maps, encoded vectors, spec functions, invocation labels and
  generic resource arguments (`/tmp/snapshot-routing-check-*.log`). The fifteen
  audited carrier theorems use only standard Lean axioms
  (`/tmp/snapshot-routing-audit.log`). The full benchmark completed before
  broad suites: 30 verified, one expected rejection, no unexpected failures,
  15,807,887,568 raw heartbeats (+450,576, +0.0029%). The generator refreshed
  local JSON/HTML relative to main. All four full suites pass, including 125
  Check fixtures and both cost gates. The registry also verifies
  `map_equality_encoding`. Six other Table fixtures get past specification
  translation and now report missing native models/reference denotation.
  Their seven baselines were regenerated and reviewed, and all 437 normal
  registry checks pass (`/tmp/snapshot-routing-prover-recheck.log`). No
  previously verified target regressed; the diagnostic categories are
  122 clean, 5 warnings, 47 preverification, 231 verification diagnostics,
  32 compiler/other (`/tmp/snapshot-routing-registry-audit.{json,log}`).
  A native-adapter audit found that shared-reference erasure and caller/callee
  namespaces gave one Table contents resource multiple runtime keys. A real
  two-module source probe reproduced the aliases. `TableMemory.resourceOf`
  now accepts only nominal owners; `runtimeResourceOf` selects the declaring
  namespace and earliest matching type entry. Logical templates remain usable
  from any namespace. Canonical uniqueness is kernel-proved without assuming
  type interning is injective. `slotOf?` resolves a caller's closed type and
  handle to that unique key, with soundness and resource-transport proofs.
  `Denote/TableStorage.lean` now proves whole-heap encoding preservation for
  insertion/replacement and retirement; it is imported by the TableMemory test
  root. This closes the alias obstacle to the native adapter's single-slot
  updates. The following increment now supplies whole-storage laws for
  allocation, insertion, removal and checked retirement, including ordinary
  globals and persistent allocation history. Their returned-storage versions
  allow other owners to retain active loans. Raw typed contents at the selected
  runtime slot remain an explicit premise for insertion/removal/borrowing;
  resolving an outstanding borrow is not permission to execute through it.
  `StorageEncodesReturned.borrow_table_reconcile` connects actual key lookup,
  registered mutable borrowing and write-back to the updated typed memory.
  Returned observation commutes with heap insertion/erasure and preserves plain
  stored values. Two new regressions retain a colliding global loan while a stale
  returned Table loan cannot overwrite a replacement or resurrect a removed slot.
  The five focused test roots pass (58 jobs,
  `/tmp/table-storage-returned-focused-build.log`), and all 31 audited theorems use
  only standard axioms (`/tmp/table-storage-returned-audit.log`).
  New `Check/Storage/TableStorageKeys.lean` checks caller/callee resolution,
  shared-reference exclusion, open templates, distinct handles and exactly one
  physical name. It and all five Table snapshot proofs pass. The 53-job focused
  build passes (`/tmp/table-slot-focused-build.log`), and six audited theorems
  use only standard axioms (`/tmp/table-slot-audit.log`). The current Check tree
  has 126 fixtures; the last full run above covered 125 before this new file.
  Fresh CLI build and all four scoped registry checks pass
  (`/tmp/table-slot-native-build.log`, `/tmp/table-slot-registry-check.log`).
  No new full benchmark/suite run is claimed for these two focused follow-ups.

  The read-only increment now enables `map_has_key` and `map_borrow` for Table
  owners through explicit intrinsic-contract hypotheses. Five new source proofs
  and the false-lookup rejection pass (`/tmp/table-read-source.log`); the existing
  snapshot fixture also passes. Caller/callee observation conversion is proved
  for all runtime inputs, and snapshot lookup agrees with typed contents lookup.
  Five audited theorems use standard axioms only (`/tmp/table-read-proof-audit.log`).
  Auditing the five source theorems reports the existing `compileFunction_agrees`
  assumption and standard axioms, with native contracts retained as explicit
  parameters (`/tmp/table-read-source-audit.log`); no new axiom was added.
  The CLI is freshly built (`/tmp/table-read-native-build.log`). The registry's
  `table_option` now reaches verification but still times out at its original
  25,000 maxHeartbeats. Prepared goals show aggregate observation projections
  need normalization and the nested Option's length-at-most-one invariant is
  absent: physical Table fields do not expose external values to deep invariant
  instrumentation. The native result's generic type does not supply it either.
  Do not add the bound as an unproved premise or raise the budget. Source/export/
  goals: `/tmp/TableOption.generated.lean`, `/tmp/table-read-option-export`,
  `/tmp/table-option-goals.log`. `bitwise_table` and `verify_table` now pass the
  shared-read barrier and reach their mutation natives. No additional whole
  registry fixture verifies yet. The Check tree now contains 127 fixtures.
  Scratch normalization `/tmp/TableOptionNormalize3.lean` reduces two residual
  goals exactly to the stored Option vector's `size ≤ 1`; the bound is absent
  (`/tmp/table-option-normalize3.log`). This scratch proof is not a companion.

  **Aggregate projection follow-up:** `SnapshotValue` now normalizes successful
  runtime-frame observations of known nominal/vector shapes, and reconciles the
  list traversal of observations with array-based executable codecs. Unknown
  Table observations/projections stay opaque. The expanded `TableReads` fixture
  has six positive proofs (including nested-vector payload equality and length)
  and two intended rejections (including an unjustified payload length bound).
  It, `TableSnapshots`, and the core `SnapshotValue` tests pass. The six-source-
  theorem audit retains only the existing compiler-agreement assumption in
  addition to standard axioms (`/tmp/table-reads-projection-audit.log`).
  A diagnostic source probe leaves three obligations: one remaining postcondition
  normalization goal, and the two missing stored Option length invariants
  (`/tmp/table-option-projection-goals.log`). That probe uses the source verifier's
  default budget, not the registry budget. Pinning **both** `maxHeartbeats` and
  `leaner.verifyHeartbeats` to 25000 reproduces the official `whnf` timeout
  (`/tmp/TableOptionRegistryBudget.lean`, `/tmp/table-option-registry-budget.log`).
  No additional whole registry fixture
  is claimed verified. The full benchmark/report refresh passes 30 problems plus
  the expected AMM rejection at 15,807,844,181 heartbeats (−0.0013% locally),
  recorded in `/tmp/leaner-benchmark-table-projections.json` with generator-produced
  main-relative HTML. Both cost gates and five scoped registry checks pass after
  that refresh (`/tmp/table-projections-check.log`). The sixth, `table_option`,
  needed its changed diagnostic baseline regenerated by the owning runner; its
  normal recheck is `/tmp/table-projections-option-recheck.log`. No new broad
  suite run was started for this increment.

  A proof-only checkpoint adds `Denote/StoredValueInvariants.lean`: traversal
  follows resolved native types through external collection entries, and three
  theorems transfer those invariants to members, Table lookups and the selected
  snapshot. Its focused tests prove an Option-like stored value has length at
  most one and reject a typed heap containing length two. The 52-job build and
  three-theorem standard-axiom audit pass (`/tmp/stored-value-invariants-{checkpoint,audit}.log`).
  This leaf is not connected to generated contracts; it does not resolve
  `table_option`. Table frontend integration has resumed with the generic
  callback/write-preservation increment described at the top of this handoff.

  **Next Table implementation:** deep Table value invariants and remaining snapshot projection
  normalization for `table_option`; then typed mutation/allocation contracts,
  native adapters and ownership frames, plus
  the remaining specification boundaries: Table type-domain quantification,
  future `final` contents, pure `spec_new` identity, opaque/recursive spec
  functions and entries-layout maps with Table values. Executable snapshot
  operands explicitly reject until memory agreement is established.


- Literal swap reads now use the standard lookup theorem through a guarded
  simplification procedure. Arrays backed by explicit list constructors are
  normalized; symbolic arrays retain their existing proof form. Unconditional
  expansion pushed the generic model-call proof beyond 25M and was discarded.
  The production isolated ground proof costs 45.087M versus 62.296M (−27.6%),
  and official vector/closure ordinary baseline checks pass. Both native tools
  rebuilt before the full benchmark, whose generated report preceded broad
  tests. All four full suites and the 437-file registry refresh pass; the
  registry audit has no baseline changes. Logs:
  `/tmp/literal-swap-ground-production.log`, `/tmp/literal-swap-{vector,bp}-check.log`,
  `/tmp/literal-swap-<package>-{build,test}.log`, and
  `/tmp/literal-swap-registry-audit.{json,log}`.

- Opaque-inline declarations were dropped even when opaque calls and behavioral
  predicates still named them. The frontend now retains opaque declarations
  and their comments in modules and interfaces. `behavioral_predicate_inline_fun`
  verifies completely. `opaque_inline_body_fail` rejects its bad body and the
  dependent caller; the explicitly trusted case verifies. The loop-sum
  companion proves the body (13.196M) and double call (5.093M) at unchanged
  25M limits; all positive targets verify and `test_sum_wrong` still fails.
  All three ordinary scoped checks pass. After the final benchmark/report,
  Move-package tests pass and the 437-file registry refresh changes only these
  three baselines, with no regression in previously verified targets. Logs:
  `/tmp/opaque-inline-*.move-check.log`, `/tmp/opaque-inline-leaner-move-test.log`,
  `/tmp/opaque-inline-prover-refresh.log`, and
  `/tmp/opaque-inline-registry-audit.{json,log}`.

- Completed vector-companion follow-up: the new
  `verify_vector.proof.lean` proves the update/truncation equation for swapping
  with the last array element and erasing it. The direct `verify_swap_remove`
  passes the official scoped update and ordinary baseline check at 25k
  (`/tmp/verify-vector-companion-{update,check}.log`). The library-call variant
  needs mapped bounded reads normalized and spent 28.4M before the script.
  A proportional speculative budget (5% of the target budget, at most the
  existing 20M cap) reduces its scratch proof to 21.351M at the unchanged
  25k limit. A fixed small cap regressed larger authored examples and was
  discarded. Quicksort and OrderedMap pass the proportional-cap scratch
  checks with the E2E driver limits. Production now includes that cap and
  `Array.getElem_map` normalization. Both native executables rebuilt successfully
  (`/tmp/vector-residual-{native,move}-build.log`), and the production library-call
  proof costs 21.313M. Both swap-remove targets pass the official ordinary
  baseline check (`/tmp/vector-residual-verify-vector-check.log`). The smaller
  residual allowance exposed an extra `count_all` loop-invariant obligation;
  its companion now handles the earlier/current entry cases explicitly, and
  its ordinary baseline check passes (`/tmp/vector-residual-bp-check.log`).
  Full benchmark `/tmp/leaner-benchmark-vector-residual.json` is complete:
  30 verified, one expected rejection, no unexpected failures/timeouts,
  15,846,440,343 raw heartbeats (+0.0055% versus the first vector run).
  The driver regenerated the full local JSON/HTML against main before
  starting `/tmp/vector-residual-full-tests.py`. All four full suites pass, including both cost gates and the 123 Check fixtures. The full 437-file registry refresh passes, with
  only the intended additional model-swap-remove and index-of proof changes
  since its snapshot, and no newly failed targets
  (`/tmp/vector-residual-registry-audit.{json,log}`).
  The same companion now also proves `verify_index_of` at 22.520M, using the
  no-match prefix invariant explicitly; its scoped official refresh passes
  (`/tmp/vector-index-mvp-update.log`). The next `verify_reverse` target still
  times out before its authored script at 25M; profile preparation next.

- Resumed vector follow-up: kernel-proved swap/remove normalization preserves
  the original denotation and reuses vector representability certificates.
  Map/operation commutation preserves generic carrier transport; the default
  search index is below the length exactly when the vector is nonempty.
  The remove rule explicitly retains its canonical result carrier. Dependent
  wp branches introduce their assumptions before normalization, and a wp which
  preparation changes returns to structural processing instead of remaining
  a leaf with an unprocessed continuation.
- Focused validation passes: official `bp_pure_callee` update at the unchanged
  25k budget removes only `remove_all_found`'s timeout; existing VectorOperations
  and new GenericSwapRemove (generic callers, values and bounds aborts, with
  interpreter checks) pass. The isolated production proof costs 22.751M versus 52.221M
  diagnostic baseline, with no new companion. Logs:
  `/tmp/resume-vector-{build,mvp-update,generic-check,existing-check}.log`,
  `/tmp/resume-fallback-mvp.log`.
- The first full benchmark `/tmp/leaner-benchmark-vector-resumed.json` found
  authored proof regressions in Quicksort and LeanerLang OrderedMap. Their
  proofs now use the certified swap/read form. Both verify in the native
  follow-up `/tmp/leaner-benchmark-vector-examples.json`, whose generated
  report refresh preceded the broad tests. Quicksort costs 245M versus the
  checkpoint's 265M; OrderedMap remains about 1.44G.
- All four full suites pass, including 123 Check fixtures and both cost gates:
  `/tmp/vector-resumed-<package>-{build,test}.log`. The full registry refresh
  passes 437/437, with no new failed targets. Only `verify_vector` changes
  beyond the scoped fix: the same failed targets, more specific residuals,
  two ordinary rejections instead of timeouts. Logs:
  `/tmp/vector-resumed-prover-refresh.log`,
  `/tmp/vector-resumed-registry-audit.{json,log}`.
- Final full benchmark `/tmp/leaner-benchmark-vector-final.json` (same-stem
  log) is complete: 30 verified problems and the expected AMM rejection,
  no unexpected failures/timeouts. Total 15,845,571,410 heartbeats, down 1.18%
  against the checkpoint's same 31 problems (16,035,112,646). The script-generated
  local JSON and HTML show the complete suite against main, without intermediate
  runs. Pool is 22/22 at 3,188,968,055; Framework ordered_map is 29/29 at
  4,157,613,974; Quicksort is 3/3 at 245,199,348 and LeanerLang OrderedMap
  17/17 at 1,441,120,989.
- Performance follow-ups: simple-map `add_all` rises 159M→183M, features
  `apply_diff` 206M→223M, and ground `swap_remove_value` 36.6M→64.5M.
  Scratch read-normalization attributes reduce the isolated last proof
  62.3M→45.1M but remain uninstalled; see the final entry in `perf-notes.md`.
  The registry's swap-remove equations are now proved by the follow-up above.

- Requested checkpoint: commit the validated AMM/report, pool and nested-behavior
  follow-ups and suspend. `type_info` is removed from `bench/problems.toml`:
  the next full benchmark has 31 problems. The existing generated HTML/JSON
  intentionally retains the last complete 32-problem measurement until then.
- Checkpoint validation after removing the experiment: restored native build
  passes (249 jobs); existing VectorOperations, Behavior and ArithmeticContext
  checks pass; the ordinary scoped MVP baseline check passes without updating
  expectations; eight Python report tests pass. Logs:
  `/tmp/checkpoint-restored-native-build.log`, `/tmp/checkpoint-vector-check.log`,
  `/tmp/checkpoint-behavior-check.log`, `/tmp/checkpoint-arithmetic-check.log`,
  `/tmp/checkpoint-bp-baseline.log` and `/tmp/checkpoint-report-tests.log`.
  The restored closer exactly matches the previously full-suite-validated
  behavior implementation; no further full benchmark or broad suite was started
  for the requested suspension.
- Resumption experiment for `bp_pure_callee::remove_all_found`: the current
  implementation reproduces a 25M timeout; diagnostic automatic proof at 100M
  takes 52.221M. Kernel-proved swap/remove normalization, array-map transport
  laws and a search-index bound reduce it to 22.698M and pass the official
  scoped runner. However, the removal rewrite regresses existing vector checks
  (`remove_middle`, `swap_remove_value`) and generic swap/remove callers. All
  experimental production changes and their scoped baseline update were removed;
  this checkpoint retains the prior verified implementation and baseline.
  Scratch sources are `/tmp/registry-vector-operations-checkpoint-experiment.lean`
  and `/tmp/registry-vector-generic-checkpoint-experiment.lean`; logs include
  `/tmp/registry-remove-official-25k.log`, `/tmp/registry-vector-existing-check.log`
  and `/tmp/registry-vector-generic-residual.log`. Investigate the residual
  `wp (Spec.pure ...)` and simplifier transparency before installing the rewrite.

- Nested-behavior fix: the closer splits marked literal-closure abort
  alternatives before analyzing individual invocations. Shared argument
  decoding now recognizes the certified integer inside a packed single-result
  projection; both termination and contract dispatch use it. `abortCases?`'s
  existing suppression rule remains unchanged. The valid two-increment
  regression passes at a 50M target budget, and a guarded negative still
  rejects a clause that omits second-invocation overflow.
- Focused native benchmark `/tmp/leaner-benchmark-behavior-fixed.json` verifies
  20/20 behavior targets, zero errors. `add_two` costs 18,943,903 heartbeats;
  module total 125,588,922. The benchmark script regenerated the local JSON
  and HTML before broad testing. The fresh full run
  `/tmp/leaner-benchmark-behavior-final.json` records 30 verified modules,
  one expected rejection (AMM), one existing failure (`type_info`), and zero
  timeouts. All measured work is 16,168,784,777 heartbeats (+0.017% versus
  the previous pool full run). Behavior verifies all 20 targets at 125,589,180
  total; `add_two` costs 18,944,093. The complete local JSON and script-generated
  HTML now show this run against main. Pool, ordered-map and calculator retain
  their complete verification, and AMM's declared negative rejects normally.
- Behavior validation: the positive and guarded negative core checks pass in
  scratch. Native Move tools were rebuilt before measurements. The full core
  suite passes (135 jobs, including both performance gates and the new
  positive/negative behavior checks) after publishing the full benchmark
  data/HTML. Eight Python report tests pass. Core log:
  `/tmp/behavior-final-core-tests.log`. The broad MVP-parity goal remains
  paused.

- Scoped pool follow-up: `pool_u64.proof.lean` rewrites carrier scalar call
  summaries by hypothesis identity for `buy_in`, and uses proved map/vector
  coverage lemmas for the two `deduct_shares` search branches. All 22 targets
  verify under the regular budget. Existing Move sources/specifications are
  unchanged. The new helpers are invoked explicitly by the companion; adding
  them to automatic search regressed option/simple-map and was discarded.
- Previous pool full native benchmark: `/tmp/leaner-benchmark-pool-final.json`;
  the new behavior runs replace it in the default generated report against main. Results: 29 verified, 1 expected
  rejection, 2 existing failures (`type_info`, `behavior`), zero timeouts.
  All measured work totals 16,166,103,625 heartbeats. Pool totals 3,393,032,305,
  including two previously skipped downstream proofs; its total rose from
  3,050,102,018 despite `buy_in` falling to 854,318,280. `deduct_shares` costs
  763,749,746, `redeem_shares` 632,868,911 and `transfer_shares` 307,052,384.
  Option/simple-map verify, bit-vector cost is restored, ordered-map verifies
  all 29 targets, calculator all eight, and AMM's six positives verify while
  its declared negative rejects normally at 185,035,863 heartbeats.
- Pool validation: focused scalar/coverage checks and eight Python report tests
  pass. Full core suite passes (135 jobs, including both performance gates) after
  the full benchmark data and HTML were regenerated. Log:
  `/tmp/pool-final-core-tests.log`. The broad MVP-parity goal remains paused.

- Scoped AMM/report follow-up (the broad goal stays paused): function-valued
  automatic leaves bound speculative cheap/prepared/pipeline/case solvers by
  the existing 20M attempt budget. Repeated leaves of an already rejected
  clause reuse its rejection, keyed by both clause and provenance. Logged
  errors still prevent certifying the target. Ordinary vector/reflection
  solvers retain their original budgets; an unconditional cap regressed ACL
  and was discarded. The six valid AMM targets still verify.
- Previous AMM baseline benchmark: `/tmp/leaner-benchmark-amm-final.json`;
  the new pool run above replaces it in the default local report, which omits
  intermediate local runs.
  Results: 28 verified, 1 expected rejection, 2 unresolved rejections
  (`type_info`, `behavior`), and 1 timeout (`pool_u64`). No newly failed problems
  or missing targets relative to the committed checkpoint. All measured work
  totals 15,822,941,895 heartbeats; the 28 verified problems total 10,968,397,511
  (+0.22%). AMM totals 1,547,801,426; its negative constructor costs 185,036,096
  and reports all four invariants normally, including the no-abort invariant.
- Validation after publishing benchmark data/HTML: `leaner-ir` full `lake test`
  PASS (both cost gates included), Behavior and InvariantBehavior checks PASS,
  eight Python report tests PASS, and four native classification checks PASS
  (expected rejection, unexpected acceptance, timeout, unrelated frontend error).
  Logs: `/tmp/amm-final-{core-tests,native-outcome-tests,invariant-behavior-check}.log`
  and `/tmp/amm-function-budget-behavior-check.log`. Native Move/benchmark tools
  are rebuilt. The broad Move/Rust/e2e/registry sweep was not resumed.

- Checkpoint requested by user: suspend after the current guard validation and
  report refresh; do not begin another fix. This is the validated commit checkpoint.
- Prepared-behavior optimization: manual residual proofs try existing call facts
  before behavioral contract derivation, restoring the goal and using the original
  fallback on failure. Each attempt has a 2M raw-heartbeat cap inside the unchanged
  target budget. The final guard requires a literal closure and permits at most
  two failed attempts per target; successful attempts do not consume this allowance.
  One failed attempt was too restrictive for the intended negative fixture.
- Core and native builds, both cost gates, cross-resource 6/6 at the official 25k
  budget, and behavior_manual_facts at 25k/default all pass. The fixture has three
  positives and one wrong-value negative; the latter retains its normal rejection.
  Logs: `/tmp/behavior-guard-two-{ir-build,move-build,composition,denote,cross-check,
  fixture-25k,fixture-default}.log`.
- Installed `bp_pure_callee.proof.lean`: count_all instantiates the established
  fold equation at 0, i, and i+1, then uses omega. Official scoped baseline update
  and ordinary check pass; only remove_all_found's existing timeout and warnings
  remain. Evidence: `/tmp/behavior-guard-bp-pure-update.log` and
  `/tmp/behavior-guard-two-pure-check.log`.
- Cross-resource preparation drops from 28.437M to 20.974M raw heartbeats using
  existing call facts. Its short companion closes all six targets at 25k;
  higher-order callers retain the original fallback.
- All four full Leaner suites passed immediately before the small final guard:
  `/tmp/registry-prepared-behavior-<package>-{build,test}.log`. That full registry
  refresh passed all 437 baselines, with 119 clean, 4 warning-only, 50 preverification,
  232 verification, and 32 other diagnostic files. Baseline matches include
  intentional negatives and are not proof-success counts.
- Final guard validation completed successfully; no validation processes remain.
  Fresh data: `/tmp/leaner-benchmark-behavior-guard.json`; generated main-relative
  report: `../local_benchmark.html`. Benchmark is 28/32, with no newly failed or
  missing targets relative to update-normalization. Framework ordered_map is
  29/29 at 4,145,490,348 total heartbeats; calculator 8/8 at 1,072,283,339;
  AMM’s six positives pass and its intentional negative remains rejected,
  2,862,902,197 total. Calculator process is 988,943,602 versus 988,919,167
  before this batch; the unguarded regression is removed. AMM constructors
  retain about 1.4M extra heartbeats each, bounded instead of repeated per leaf.
- Full 437-file registry refresh passed; audit has no diagnostic differences
  from the pre-refresh snapshot (which includes the scoped count_all fix).
  Counts remain 119 clean / 4 warning-only / 50 preverification / 232 verification /
  32 other. Logs: `/tmp/behavior-guard-{validation,prover-refresh,registry-audit}.log`.
  Full suites were run before the final small guard; afterward the focused
  fixture checks, both cost gates, benchmark, and full registry refresh passed.
  No additional full-suite run was needed. `git diff --check` passed.
- Next target after resuming: remove_all_found. Diagnostic 100k manual simp_all
  closes, but preparation costs 50.437M raw heartbeats (24 residual leaves 23.096M,
  binds 5.914M, exits 5.210M). Its ordinary std::vector.swap_remove body expands
  bounds/swap/pop cases. Use `/tmp/bp-pure-callee-official-export` captured from
  MVP; standalone CLI export omitted the dependency body and is not comparable.
  Evidence: `/tmp/bp-pure-callee-official-profile.log`.
- Discarded trials: profiling found behavior derivation 7.849M of 28.437M,
  versus state/result rewriting0.171M (`/tmp/behavior-dispatch-profile.log`).
  Sibling proof cache saved only0.178M; pre-split derivation worsened cost and
  disturbed proof contexts; 500k bare simp timed out; skipping behavior for all
  manual proofs helped the direct caller but broke higher-order callers.
  Current bounded PREPARED attempt fixes that distinction; 1.5M sub-budget
  narrowly missed acceptance, 2M succeeds without raising the25k target limit.
- `simple7` remains a semantic observation gap, now V27 in the registry. No
  alias rewrite installed: current-owner tracking must account for reborrows,
  reassignment, branch joins, and old/saved-label observations.

- Read-only diagnosis during the live validation: `specs_in_fun_ref::simple7`
  is not a timeout. The true-25k residual is `False` because `let a := x`
  compiles to `Term.take x`, emptying x's slot; `assertionConditions` then
  requires that original slot to be defined for `assert x == y`. Reproduction:
  `/tmp/specs-in-fun-ref-current.{lean,log}` and
  `/tmp/specs-in-fun-ref-residual.{lean,log}`. A sound fix needs current reference
  ownership/alias tracking at the observation site. Substituting the entry
  value or the reference's final prophecy would be wrong when later writes occur.
  No alias-semantics changes installed.
- Discarded performance experiment after full validation: batching consecutive
  syntactic forall introductions in `Close.splitOnce` reduces cross-resource
  behavior splits 79 → 12 but preparation only 28.437M → 28.383M raw heartbeats;
  it still times out at true 25k. Both cost gates pass, but this is not the
  bottleneck. Source restored from `/tmp/batched-binders-before-Close.lean`.
  Restore build `/tmp/batched-binders-restore-build.log` PASS (session 9046,
  exit 0); source matches the pre-experiment snapshot. No native Move rebuild
  is needed because the original core is restored. Experiment and baseline logs:
  `/tmp/batched-binders-cross-resource-{25k,profile}.log`,
  `/tmp/cross-resource-binder-baseline.{lean,log}`, cost logs
  `/tmp/batched-binders-cost-{composition,denote}.log`.
  One premature scoped anchor check ran while the experimental build had
  temporarily removed Close.olean; its missing-object error is infrastructure,
  not a baseline regression, and its baseline was not changed. Repeated after
  restoration: ordinary anchor check PASS (`/tmp/update-normalization-anchor-check.log`).
  Full suites and full registry above completed BEFORE the discarded experiment.

- Latest installed follow-up: Contract.updateNominalField is now irreducible
  during definitional reduction, with kernel-proved lir_denote_norm equations
  for optional reads, nominal inputs, and unit. This prevents irrelevant
  runtime-constructor splits on typed resource reads. Snapshot:
  `/tmp/update-normalization-before-Contract.lean`. Core and native builds pass:
  `/tmp/update-normalization-{ir,move}-build.log`. New companion
  `state_labels/spec_fun_cross_resource.proof.lean` proves both update-labelled
  callers at true 25k using a codec round trip and explicit resource lookup
  cases. Official registry update and ordinary check PASS:
  `/tmp/update-normalization-registry-{update,check}.log`; baseline now only
  test_behavior_cross_resource's existing 25k timeout (5/6 positives prove).
  Both cost gates pass (`/tmp/update-normalization-cost-*.log`). Existing
  field_update_labels default baseline and both new SourceVerify fixtures pass
  (`/tmp/update-normalization-fixtures-default.log`). New fixtures also pass
  25k (`/tmp/update-normalization-fixtures-25k.log`). field_update_labels' negative
  timed out at 25k both BEFORE and AFTER this change; do not treat that stricter
  scratch cap as its established baseline. Its positive caller's closer drops
  21.5M → 12.8M raw heartbeats (`/tmp/field-update-{before-normalization,current}.log`).
- COMPLETED `/tmp/update-normalization-validation.py`, session 52911, exit 0:
  fresh benchmark/main HTML (28/32, no newly failed/missing targets), all four
  full suites, and 437-file registry refresh PASS. Framework ordered_map 29/29,
  4,145,409,644 total heartbeats; calculator 8/8, 1,072,258,030; AMM six valid
  plus intended negative, 2,860,116,991. Logs `/tmp/update-normalization-*`,
  package logs `/tmp/registry-update-normalization-<package>-{build,test}.log`.
  Audit: 118 clean/4 warnings/50 preverify/233 verification/32 other.
  Newly proves anchor_mutation_pre_state::valid_derived_claim; its negative
  still fails. Aliasing adds one clause diagnostic before the same timeout;
  no newly failed targets. Audit `/tmp/update-normalization-registry-audit.{json,log}`.
  Production freeze is over. No commits made.
- The previous closure-frames controller COMPLETED successfully: fresh benchmark
  28/32, no new failed/missing targets, main HTML, full437 refresh. Audit
  `/tmp/closure-frames-registry-audit.{json,log}` has no baseline changes vs its
  pre-refresh snapshot:118 clean/4 warnings/50 preverify/233 verify/32 other.
  It deliberately did not repeat full suites; the current controller does.
- Remaining cross-resource caller: manual residual proof simp_all closes at a
  diagnostic100k budget, but closer itself costs28.5M raw HB (cap25M), then the
  script1.6M. Diagnostic profile `/tmp/cross-resource-both-native.log`.
  Making StateOf or RuntimeValue.asBool locally irreducible and adding a bool-read
  normalization lemma did not fix it; scratch-only, not installed. Optional-read
  update lemmas only helped once updateNominalField was irreducible, now installed.
  Scratch proof development `/tmp/cross-resource-{entry-native,both-proofs}.lean`;
  installed companion has no diagnostic budget changes or local attributes.
  Next independent gap worth diagnosing: functional/specs_in_fun_ref::simple7
  still fails assertion x == y after moving x's reference to a and writing a;
  no other clause failures are reported. Inspect moved-reference specification reads.


- Current follow-up: closure-frame widening fixed in Close. The verified-target
  frame implication did not unfold `Weave.compose`, although its precondition
  proof did. An extracted implication fails without that normalization and
  passes with it using the existing solver. Final Close diff versus
  `/tmp/cross-resource-frame-before-Close.lean` is only adding Weave.compose to
  that simp list. The extra grind and diagnostic logging experiments are removed.
  Core/native builds: `/tmp/cross-resource-frame-compose-ir-build.log`,
  `/tmp/cross-resource-compose-move-build.log`. Both cost gates pass unchanged:
  `/tmp/cross-resource-compose-cost-{composition,denote}.log`.
  SourceVerify/closure_frame_widening proves six positives at true 25k, including
  a captured argument, and rejects the callback writing an unauthorized captured
  address. Baseline update/check pass (`/tmp/closure-frame-fixtures-*.log`).
  Larger cross-resource residuals lose both FramedAt obligations, but their memory
  and label obligations remain. The optional-read/updateField lemma experiment
  did not change those residuals and is scratch-only, not installed.
- Completed controller `/tmp/closure-frames-validation.py`, session 27220, PID 676365:
  fresh benchmark `/tmp/leaner-benchmark-closure-frames.json`, main-relative HTML,
  target comparison against returned-freeze-final, then full 437-file baseline
  refresh. Logs `/tmp/closure-frames-{validation,benchmark,report,prover-refresh}.log`.
  No four-package full-suite repeat for this small follow-up; those passed just
  before it. Production frozen until controller completes. Audit full registry
  after completion, especially previously failed sample target lists and any
  replacement frontend errors. Then continue resource/label proof automation.


- Latest verified checkpoint: `/tmp/returned-freeze-final-validation.py` exited
  successfully. All four full suites and the 437-file refresh PASS. Fresh full
  benchmark `/tmp/leaner-benchmark-returned-freeze-final.json` has 28/32 with no
  newly failed or missing targets; main-relative HTML generated before suites.
  Framework ordered_map: 29/29, 4,145,487,311 heartbeats; calculator: 8/8,
  1,072,225,189; AMM: 2,860,062,743, six positives plus intended negative.
  Audit `/tmp/returned-freeze-final-registry-audit.{json,log}`: 118 clean,
  4 warnings, 51 preverification, 232 verification diagnostics, 32 other.
  No previously proved target regressed. Earlier E2E/type regressions are fixed.
- Subsequent C18 frontend follow-up is now installed: logical value-typed
  borrowGlobal normalizes to global; reference-typed runtime borrows retain
  semantics. Snapshot `/tmp/projected-global-before-Encode.lean`; native build
  `/tmp/projected-global-move-build.log` passes. SourceVerify/projected_global_reads
  covers four positives at true 25k, opaque caller labels without callee points,
  and one intended negative. Updated and ordinary checks pass:
  `/tmp/projected-global-fixtures-{update,check}.log`.
  Scoped spec_fun_cross_resource registry update/check passes; increment,
  flip_flag, apply prove, all three larger callers time out at 25k.
  `/tmp/projected-global-registry-{update,check}.log`. Counts after scoped update:
  118/4/50/233/32. The full benchmark/suites precede this small follow-up;
  refresh them at the next batch boundary, not after every small change.
  Profile test `/tmp/projected-global-profile-test.log` passes.
  Next: improve larger caller automation (diagnostic residuals below), then other
  registry gaps. Mutable-selector C18 still requires V20, not borrow peeling alone.


- Current batch: C16's implicit reference freezing is normalized in the Move
  frontend. Call results keep the declared instantiated type, then freeze the
  required tuple components; computed freeze operands are evaluated once into
  locals. A subsequent `bp_forwarding` rendering error exposed quantifier-domain
  predeclaration seeing nested locals outside their lexical scope. That is fixed
  in Lower. Print and Close experiments are reverted; this batch changes Encode
  and Lower, plus two SourceVerify fixtures. Seven positive regression targets
  prove at 25k; exactly two intended negatives remain. New fixture baseline
  update and non-update checks pass. Frontend tests and both cost gates pass.
  Both C16 registry files reach verification; two bp_forwarding positives prove,
  and four other positive loop targets still exceed 25k. See C16 in the registry.
  Snapshots: `/tmp/returned-freeze-before-{Encode,Lower}.lean`.
  Native build: `/tmp/returned-freeze-move-build6.log`.
  First validation finished: fresh benchmark `/tmp/leaner-benchmark-returned-freeze.json`
  retains 28/32 with no target regressions; main HTML generated before suites.
  IR, Move, and Rust full suites passed. E2E exposed a real smart_table rendering
  fixed-point regression: runtime freeze materialization had also affected logical
  operands. The encoder now keeps logical freezes unchanged. This also restores
  two registry vector_hofs files whose type errors had masked their proof targets.
  Native correction build: `/tmp/returned-freeze-move-build8.log`. Seven positive
  SourceVerify targets now pass; two intended negatives remain, with updated and
  checked baselines (`/tmp/returned-freeze-fixtures-{update2,check3}.log`).
  Full 437 registry refresh passed; scoped corrections for vector_hofs_for_each
  and vector_hofs_mut_receiver are refreshed. Audit:
  `/tmp/returned-freeze-registry-audit.{json,log}` (JSON exists; regenerate log).
  Counts using the established diagnostic categories: 118 clean, 4 warnings,
  51 preverification rejections, 232 verification diagnostics, 32 other errors.
  These broad categories include six preexisting Lean elaboration errors under
  verification diagnostics; removed proof names require inspecting replacement
  diagnostics, not treating them automatically as progress.
  All focused Move rendering baselines PASS (session 93339 exited zero):
  `/tmp/returned-freeze-corrected-format2.log` (first attempt lacked CLI env).
  Completed final controller `/tmp/returned-freeze-final-validation.py`, session 79804,
  PID 653603: corrected fresh benchmark/main HTML, all four full suites, then
  437-file registry refresh. Logs `/tmp/returned-freeze-final-*`; package logs
  `/tmp/registry-returned-freeze-final-<package>-{build,test}.log`.
  Controller completed; results are summarized at the top.

  The arithmetic-only normalization discharger experiment had no useful gain;
  it was reverted (`/tmp/context-discharge-before-Close.lean`).
  Budget-unit correction: CLI `--heartbeats` / SourceVerify `heartbeats` are
  thousands of maxHeartbeats units. Use 25, not 25000, to match the registry.
  Raw `-Dleaner.verifyHeartbeats=25000` is correct. The focused fixture driver
  was corrected and passes at the true 25k cap:
  `/tmp/returned-freeze-fixtures-true25k.log` (seven positives, two negatives).
  Earlier cross-resource scratch claims used the oversized CLI argument;
  the combined test_behavior_cross_resource times out at true 25k, though the
  primitive functions prove. Its frontend normalization remains promising.
  Ready C18 regression: `/tmp/projected_global_regression.move`. Unmodified
  frontend reproduces `$increment: borrow result is not a reference type`
  (`/tmp/projected-global-before.log`). Scratch export changes only value-typed
  borrow_global to global, preserving reference-typed runtime borrows; verification
  at CLI --heartbeats 25 leaves exactly wrong's two intended diagnostics
  (`/tmp/projected-global-regression-25k.log`). Install after current validation:
  addExpr in logical mode can normalize a non-reference borrowGlobal node to
  `.global none` with the same type, location, instantiations, operands and surface.
  Add focused SourceVerify fixture and regenerate its expectation using 25.
  Larger caller residuals at diagnostic-only raw budget 100000 are in
  `/tmp/cross-resource-residual-diagnostic.log`: frame widening for literal
  increment/flip closures and impossible missing-resource/updateField branches.
  No larger acceptance budget was installed. V20 needs a genuine mutable-input/
  writeback invocation model; peeling projected borrows alone is insufficient.
  Next-gap scratch probes (not installed): C18's spec-function value-typed
  `borrow_global` should become a logical global read; replacing it in the
  cross-resource export lets increment/flip_flag/apply/test_behavior_cross_resource
  prove, leaving two labelled wrapper proofs unproved without timeouts
  (`/tmp/spec-fun-cross-resource-probe2.log`). Mutable selector BPs need both
  peeling a value-typed borrow and expanding implicit pre/post arguments; even
  then they hit V20's unsupported mutable-parameter behavioral predicates
  (`/tmp/bp-mut-ref-selector-probe2.log`). Preserve caller-side label semantics.

- Current performance batch: kernel-proved `Flow.pure_shortCircuit_and/or`
  combine pure boolean guards only when both branches preserve the same locals.
  This avoids duplicate continuations. A bounded arithmetic-only omega attempt
  precedes the full-context solver in the cheap bound decider. The new
  `ArithmeticContext` test checks the fast path and fallback; `ControlForms`
  covers effectful short-circuiting. Registry `constant_product` now proves at
  25k with its installed companion; its closer is 15.1M heartbeats / 17 leaves.
  Logs: `/tmp/short-circuit-ir-build2.log`, `/tmp/short-circuit-move-build2.log`,
  `/tmp/short-circuit-arithmetic-test.log`, `/tmp/short-circuit-control-test.log`,
  `/tmp/short-circuit-registry-amm.log`. The remaining fee/swap/constructor
  obligations still need work at 25k.
  The first benchmark/report completed with 28/32 and no target regressions:
  `/tmp/leaner-benchmark-short-circuit.json`. AMM is 2.55G (previous 2.87G), with
  six valid targets and the intended negative retained. Move/Rust suites passed;
  IR's cost gate caught proof-term growth on simple arithmetic, and E2E found
  `Scalars/Order::ordered3`'s authored proof depended on the old branch split.
  The fast path now requires memory-dependent hypotheses; its prototype passes
  the cost gate. `ordered3` now introduces both comparison premises explicitly
  and passes. The 437-file refresh passed, with no new failed functions:
  `/tmp/short-circuit-registry-audit.log` (118 clean / 3 warnings / 53 preverify /
  231 verification / 32 other). Two changed diagnostics: shorter aborts_if
  residuals and bug_15044 now fails normally instead of timing out.
  The corrected final benchmark completed: 28/32, no newly failed targets,
  `/tmp/leaner-benchmark-short-circuit-final.json`. HTML was generated against
  main before the full suites. AMM is 2.86G versus 2.87G; constant_product is
  16.1M versus 26.8M (-39.8%). The earlier non-compliant-pricing improvement
  did not survive the memory-context restriction needed by the cost gate.
  Both corrected cost gates pass. `/tmp/short-circuit-final-validation.py`
  completed successfully: all four full suites pass, as does the 437-file
  registry refresh. The final audit found no changed diagnostics (118 clean,
  3 warnings, 53 preverify, 231 verification, 32 other). Logs use
  `/tmp/short-circuit-final-*` and `/tmp/registry-short-circuit-final-*`.
  Vector-fold probes show
  base/exit cases close after context normalization and recursive-spec
  unfolding, but loop-step preparation still exceeds 25k. Even an unconditional
  kernel-proved concrete prefix formula times out; repeated normalization and
  conditional arithmetic remain costly (`/tmp/fold-unconditional-debug.log`).
  No fold experiment is installed. The subsequent `specialize_generic_caller`
  companion fixes `use_concrete` at 25k: inlining omits the callee's proof block,
  so instantiate its proved `count_is_len` lemma at u64 for entry/exit/back-edge
  obligations. Official scoped refresh and non-update baseline check pass
  (`/tmp/specialize-generic-caller-registry-{update,check}.log`). Only the
  existing compiler warning remains. Current counts after that follow-up:
  118 clean / 4 warnings / 53 preverify / 230 verification / 32 other.
  The full suites above precede this proof-only change. No broad rerun is needed
  after each companion; keep the user's benchmark-first batch workflow.
  Sound concrete-loop unfolding would need
  to prove actual exit; MVP's bounded-cut `pragma unroll` is not acceptable.


- Current batch: explicit stored-field write frames are implemented across
  export, validation, printing, contracts, and kernel-checked encoding lemmas.
  Addressed/wildcard frames, generic resources, opaque returns, and captured
  owners pass focused checks at 25k. The new SourceVerify fixture has exactly
  three intended failures; the Check error fixture has four. An invalid
  frame conversion now fails normally instead of exhausting recursion.
  Resource-invariant dispatch is computed on newly derived facts and written
  values, avoiding a whole-context simplification. `stored_fun_values` now
  proves 33/42 targets (33/35 MVP-positive) with no heartbeat timeouts. A
  kernel-checked companion transports invariants through addressed frames.
  Seven failures are intended; one wildcard call lacks an invariant-preservation
  guarantee, and `create_modifier_valid` violates its implicit empty write
  frame. See V26 in the registry. Dependency interfaces reject explicit field
  frames until they can carry the owning contracts; they cannot silently
  substitute the default empty frame.
  Snapshots before the batch: `/tmp/stored-frames-before`.
  Core/native builds: `/tmp/stored-frames-ir-build7.log`,
  `/tmp/stored-frames-move-build5.log`. Focused validation:
  `/tmp/stored-frames-positive7.log`, `/tmp/stored-frames-fixtures-check.log`,
  `/tmp/stored-frames-prover-followup.log`. All four full suites passed,
  including 122 Check fixtures and both cost gates: logs
  `/tmp/registry-stored-frames-<package>-{build,test}.log`. The 437-file registry
  refresh passed (`/tmp/stored-frames-prover-refresh.log`): 118 clean, 3 warnings,
  54 pre-verification rejections, 230 verification diagnostics, 32 other errors.
  Move's full suite passed again after the interface guard. The first full
  benchmark exposed two AMM proof-script regressions hidden by unchanged sample
  status; both scripts now tolerate earlier invariant normalization. The final
  full rerun `/tmp/leaner-benchmark-stored-frames-final.json` passed the target-level
  regression comparison (`/tmp/stored-frames-final-status.log`), retaining 28/32
  passing samples and all six valid AMM targets. `local_benchmark.html` is
  regenerated against main, without intermediate runs. Framework ordered_map
  is 29/29 at 4.142G heartbeats; calculator is 8/8 at 1.071G.
  Scoped follow-up: implicit function-wrapper conversions (C20) now emit ordinary
  packs/projections with instantiated field types; both printer stages quote
  positional frame targets. `SourceVerify/function_wrappers.move` proves all
  eight valid targets at 25k and rejects its invalid pack. Frontend formatting
  and Move profile checks pass. Logs: `/tmp/function-wrappers-fixture-check.log`,
  `/tmp/function-wrappers-frontend-test3.log`, `/tmp/function-wrappers-profile-test.log`.
  Core/native builds: `/tmp/function-wrappers-ir-build2.log`,
  `/tmp/function-wrappers-move-build6.log`. Registry AMM now reaches verification
  (`/tmp/function-wrappers-registry-amm-final.log`), with `create_pool` proved,
  four pricing/swap timeouts and three dependent constructors. The attempted
  arithmetic companion also timed out at 25k; it is only in
  `/tmp/registry-amm-arithmetic-candidate.proof.lean`. This scoped follow-up is
  not covered by the preceding full-suite/benchmark runs. Next: profile AMM
  preparation/proof cost at 25k, then continue the registry; refresh benchmark
  data/HTML before the next broad suites.

- Validated milestone (2026-10-06): fresh full benchmark
  `/tmp/leaner-benchmark-registry-final.json`, HTML against main CI
  `ea4ecc43e7`/run 37250691414, then all four full Leaner suites passed.
  Logs: `/tmp/registry-final-<package>-{build,test}.log`. All 120 Check
  fixtures and both cost gates passed. The 437-file registry refresh passed
  (`/tmp/registry-final-prover-refresh.log`), with counts in the registry.
  The benchmark keeps all 28 previous passing samples; passing aggregate
  heartbeats change +0.16% from the prior abort-code batch. Calculator 8/8
  at 1.07G; framework ordered_map 29/29 at 4.14G. AMM's six valid targets pass.
- Runtime resource lookup now excludes open generic templates; frame
  coherence retains the template lookup. Memory typing and encoding share
  the closed-key lookup. `GenericResources` covers both classes. Constant
  vector indexing resolves the constant as a value rather than a resource.
  Encoded stored-field behavioral facts normalize in a bounded speculative
  attempt that restores context on failure. No global `NTy.encode_function`
  normalization attribute remains.
- Nominal quantifiers containing function fields remain explicitly rejected
  until the MVP field-validity domain is modeled. Unrestricted field-row
  quantification was shown to prove the invalid `other_instantiation` in a
  scratch experiment; that expansion is not applied. Four direct-field
  positives now verify, and generic-aliasing negatives stay rejected.
- Proof companions now prove `nonlinear_arithm::mul5`, all of
  `invariants_with_quant`, and all of `simple_vector_client`, preserving
  the 25k registry budget. Concrete vectors compute their list spines.
  The follow-up `/tmp/registry-for-loops-final.log` confirms all eleven
  `for_loop_invariants` positives and rejects both incorrect cases.
  Its triangular-division rewrite avoids unnecessary absolute-value cases;
  all three sum proofs now fit 25k. The regular benchmark inputs are unchanged.
- In progress after the full suites: generic enum invariants now lower their
  `this` type using the declared type arguments, as struct invariants do.
  Core and Move driver builds pass. `enum_19575` verifies, and the new source
  fixture `generic_enum_invariants.move` proves generic constructors/readers
  and an opaque caller while rejecting invariant/postcondition violations,
  also checked at the registry's 25k budget.
  This scoped follow-up raises the registry's no-diagnostic count to 116;
  the four full-suite logs above precede this lowering change.
  Next diagnosed issue: in-body assertions over live mutable borrows read
  the parent’s prophecy, not the holder’s current value (`bug-17117`, and
  likely `bug_15880`). `/tmp/live-borrow-probe.lean` isolates it; its leaf
  asks `value.val = r_x.val` without a connecting fact. Inspect
  `Verify.assertionConditions`/`overDefined` and the compile loan facts; a
  current-value view must reconstruct parents from active holders without
  changing caller-side state labels or ending loans that are still used.

- Next registry batch is in progress: `aborts_with` is carried through the
  frontend, printer, lowering, and contract generator. Abort-code coverage is
  checked independently of partial abort conditions, matching MVP. Partial
  conditions without codes now have a trivial failure-permission predicate.
  The remaining positive `conditional_abort` exposed arithmetic traps modeled
  as user aborts carrying their operands/results. Move arithmetic now uses a
  `runtime.arithmetic_error` profile throw; explicit aborts retain their codes.
  Both arithmetic and vector runtime errors map to `EXECUTION_FAILURE` in
  contract code clauses. The two MVP abort fixtures now prove all 12 valid
  functions and reject all 15 intentionally invalid ones. The differential
  arithmetic-overflow fixture now matches MonoVM instead of reporting abort 300.
  Fresh benchmark `/tmp/leaner-benchmark-registry-aborts.json` preserves all
  28 passing samples; their combined heartbeats decrease 1.5%, with calculator
  down 13.9%. HTML compares this run against main. All four full suites pass after report generation, including MonoVM and
  differential checks (`/tmp/registry-aborts-leaner-e2e-tests-test5.log`).
  The checks now distinguish arithmetic errors from explicit user aborts;
  three partial-code fixtures now cover their previously omitted abort
  boundaries. Existing negative proofs remain rejected. The full MVP
  refresh matches all 437 baselines, with aggregate diagnostic categories
  unchanged (`/tmp/registry-aborts-prover-refresh.log`).

- Registry follow-up in progress: functional `update_field` now renders,
  parses, lowers, and translates into contracts, including generic and nested
  updates and fields at different enum positions. Partial-variant and intrinsic
  map updates remain explicitly unsupported. Lowering `old` now selects the
  pre-state before descending into its operand; this removes false cycles in
  old-only state-label definitions. A bounded decoded-memory closing attempt
  handles the opaque caller case without ProgramPoint witnesses. The new
  `FieldUpdate` Check fixture passes four proofs; the source fixture
  `field_update_labels` verifies the correct caller and rejects `caller_wrong`.
  Fresh full benchmark `/tmp/leaner-benchmark-registry-field.json`: 28/32
  verified, preserving every previously passing problem; their combined
  heartbeat cost changes by +0.1%. Framework ordered_map keeps all 29 proofs
  at 4,128,189,059 heartbeats; calculator keeps all eight. The HTML was
  regenerated against main before starting the four full Lake suites.
  All four full Lake suites pass, including 119 Check fixtures and both
  performance gates. The full registry refresh matches all 437 baselines;
  aggregate diagnostic counts are unchanged, with V1/V2 updated to the more
  specific remaining failures. Logs: `/tmp/registry-field-*-test.log` and
  `/tmp/registry-field-prover-refresh.log`.

- AMM/calculator follow-up: calculator now verifies in the regular native
  benchmark, including its callers. Its specification captures the old
  continuation before selecting the post-removal label; reading the removed
  resource inside that label selected an absent value in Leaner's partial memory.
  Stored function fields now carry the Move default read-only frame through
  construction, mutation, storage, removal, and invocation. The compiler and
  contract generator share the predicate deciding which declarations require
  these checks. Explicit field `modifies_of` is rejected until supported.
  Behavioral data invariants request the typing/determinism and termination
  assumptions their clauses need. The closer retains arithmetic names used by
  saved continuations, clears unused continuation lets before authored proofs,
  and uses the frame equation when identifying an invocation's final state.
  New positive and negative stored-field fixtures cover these boundaries.
  AMM's companion proves arithmetic pricing laws and valid construction/swap
  obligations; the intentionally non-compliant constructor must remain rejected.
  Full benchmark `/tmp/leaner-benchmark-amm-calculator-complete.json`: **28/32
  verified**; calculator has zero errors, all six valid AMM targets pass, and
  ordered_map retains all 29 passing targets. The other failures remain
  `type_info`, `pool_u64`, and `behavior`. Generated HTML compares this one
  fresh full run against `main`, without intermediate local runs. All four
  package builds and full Lake suites pass, including 118 Check fixtures,
  source verification, MonoVM/differential checks, and both performance gates.
  Existing baselines were not regenerated; the new negative field-frame
  fixture has its own reviewed baseline. Logs: `/tmp/amm-calculator-*-test.log`.

- Latest ordered-map result: **29/29 targets verified, zero errors**. Total
  heartbeats fall from **6,900,704,697 to 4,125,866,249** (40.2%). The driver
  regenerated benchmark data and HTML before the final broad test run.
  All four Lake suites pass, including end-to-end, MonoVM, differential tests,
  and both performance gates, without baseline changes for this increment.
  The first full run caught lost loop-invariant diagnostic markers; matching
  now normalizes only abort-continuation markers and preserves source clauses.
  See the latest entry in `perf-notes.md` for fixes and remaining costs.

- Full benchmark after these fixes: **24/32 verified**, up from 23/32 in
  the fresh baseline; neither map problem crashes. `ordered_map` uses 7.8%
  fewer heartbeats (13.37G → 12.34G), with 16 failing targets instead of 17.
  Other previously verified problems have effectively unchanged heartbeat
  costs. All four package builds/suites and the Python isolation test pass.
- Benchmark fixes: the registry carries the two insertion-position roles now
  used by `simple_map`, preserving its sequence discipline; `InsertionMap`
  checks descending keys and ranks. This unblocks `simple_map` and `pool_u64`
  before verification. The invocation projection scan visits shared terms
  once. Both performance gates pass without updating their baselines.
- `scripts/isolate-target.py` now disables assertion-only functions and omits
  unrelated proof commands. Earlier isolated map runs also verified these
  extra targets; inspect the JSON `targets` array before attributing timings.
  See [`perf-notes.md`](perf-notes.md), "Failed benchmark targets".

- Invocation-defined labels use `Proofs.StateOf`, sharing the successful
  invocation selected by `ResultOf`. A kernel-checked determinism proof
  identifies its post-state, including unnamed resource types. No new axioms.
  Naming a label adds no assumption that the invocation succeeds.
- These state expressions survive substitution of an opaque callee's contract
  at its caller; they do not depend on internal `ProgramPoint` witnesses.
  Call and terminating-run rules expose post-state equations to the closer.
  Literal callable encodings retain a reducible native-type hint so invocation
  projections can recover their typed arguments and results.
- `InvocationStateLabels`: forward definitions, chained invocations, opaque
  callers of `result_of` and `ensures_of`, and function-valued parameters.
  `InvocationStateLabelErrors`: an always-aborting call fails its intended
  `aborts_if false` clause. The Prover's `aborting_result_definition` likewise
  reaches its intended failures; invocation-label unsupported errors are gone.
- `Contract.lean` translates the state-change predicates and resolves their free
  labels recursively, across clause order and chains. A definition does not assume
  its predicate, even under negation or an implication antecedent.
- `Check/Specifications/DefinedStateLabels.lean`: direct changes, generic resources,
  chained labels, local spec bindings, and an opaque caller proving fields and frames.
  `DefinedStateLabelErrors.lean`: false clauses and a removal that can abort.
- `mixed_callee_postcondition` and `bp_inline_derive` now verify. All 30 state-label
  Prover tests and the additional inline test match their reviewed baselines.
- The design records the remaining `unmodified_memory_at_label::swap` gap: Boogie
  retains a removed resource's contents, whereas Leaner's partial memory discards
  them. Its negative `swap_wrong` fails as intended.
- Validation: builds and full tests passed in all four packages, including
  all 113 Check fixtures, source verification, MonoVM, and differential tests.
  Both verification cost gates passed without baseline changes. The full
  end-to-end suite and 31 Prover tests passed without baseline regeneration.

## Open findings not yet in a design

- Benchmark, full run at e4fb2de4cc against 7457d37: 24/32 verified, heartbeats
  +1.3%; `comparator` +18.6% and `string` +11% heartbeats, not yet attributed to a commit.
- The frontend blockers for benchmark problems `amm` and `calculator` are fixed:
  condition attributes preserve values such as `inferred = sathard`, proof steps
  substitute phase-appropriate contract lets, post-only labels retain grouping, and
  free label definitions retain lexical let bindings. Captured function literals
  now translate in specifications and compare with executable closure values.
  Both samples reach proof obligations; their remaining failures are no longer
  unsupported syntax or unknown locals. The AMM companion proves the three pricing
  implementations without changing their Move specifications. The deliberately
  non-compliant pool constructor must still be rejected.
- The three failing `ristretto255` targets now verify using a companion proof:
  normalize option payloads and apply byte-vector codec tightness. No cryptographic
  code, native model, or specification changed.
- The entire `capability` module now verifies. The companion normalizes address
  and parameter transport, proves that writes to one generic resource declaration
  preserve reads of another, and reduces stored-value transport round trips.
  This closes `delegate` and `revoke` as well as both acquisition targets,
  without changing Move code or specifications.
- `ordered_map` investigation found expensive `omega` calls before authored
  proofs: 508M heartbeats on one prepared enumeration leaf, then a timeout on
  the next. Residual deciders now use the existing 20M speculative-attempt
  bound. The companion adds proofs for `ground_enum_123`, duplicate-key
  construction, and front/back key borrowing. Further enumeration, lower-bound,
  and loop failures remain. The native benchmark drops from 12.336G to
  11.043G heartbeats (10.5%); `ground_enum_123` drops from a 1.5G timeout
  to 264M. See `perf-notes.md` for the remaining expensive targets.
- Follow-up: `test_verify_lower_bound_gap_symbolic` now verifies automatically
  at 37.8M native target heartbeats versus 1.078G failed. Normalize enum payloads
  and canonicalize `NTy.encode`/`HList.encode` across generic caller/callee
  families so the contract's encoded index and caller's integer are one term.
  No Move/specification changes or manual gap proof. `GenericEnumPayload`
  covers the generic opaque enum-return case. The whole module rerun drops
  from 11.043G to 8.379G heartbeats (24.1%); other targets still fail (43 errors).
  Benchmark data and HTML are regenerated; see `perf-notes.md`.
- Next most expensive target, `test_verify_remove_or_none`, now verifies with
  its companion proof at 124.9M native target heartbeats, down from the 1.501G
  timeout. Proven constructor/size/membership reductions keep its literal
  three-entry map computational before residual proof search. No core verifier,
  Move source, or specification changes; no broad suites run for this increment.
  The module benchmark confirms 124.0M target heartbeats and 8.379G → 6.901G
  total (17.6% fewer); `test_verify_upsert` also stops failing, with no new
  failures. Six module targets remain failed; data and HTML are updated.
- Prover test `generic_aliasing_all_partitions::never_alias` now exhausts its budget
  (was a clause failure) since S3b.
- From the S3b cleanup review, not done: `singleFieldRead?` (one integer field) and
  `boundWitnesses` (`<`, `≤`, `=` with the binder on one side) are the instances of "a binder
  read only through projections" and "the binder's interval from the bounds machinery".
  The `subst` and `projections` leaf stages run for every leaf of every target; whether
  they explain the `comparator`/`string` heartbeat growth needs a measurement.
- The element-quantifier/read-position gap now passes focused positive and
  negative source checks; the current increment above records the outstanding
  benchmark and registry validation.
- `leaner_denote_witness` decides the conjuncts reading the binder first, in the order
  `Array.qsort` leaves them; a stable order puts `verify_vector::verify_contains` over its
  budget, so the search depends on an incidental order.

- Vector index ranges now translate in quantifiers and specification slices,
  including generic vectors and old/current values. `VectorRanges.lean` proves
  five positives and rejects the upper endpoint at 25k; `TableSnapshots.lean`
  also passes with a range over logical snapshot values. The scoped
  `macro_verification` runner update and ordinary check pass. Its `foreach`
  now reaches proof search but times out even in a positive-only probe;
  normalization of the loop's unchanged-tail invariant is the next concrete
  performance lead (`/tmp/vector-ranges-foreach-debug.log`). No bit-vector
  conversion behavior was changed: the source's representation wrappers and
  LIR's numeric conversion operations need a separate semantic treatment.
  The full benchmark/report refresh retains 30 verified problems and one
  expected AMM rejection at 15,808,427,132 heartbeats (+0.0037% locally),
  followed by both passing cost gates (`/tmp/vector-ranges-postcheck.log`).
  Generated HTML compares against main. No new broad-suite run is claimed.

## How to work

- User preference: during benchmark/performance work, run the affected benchmark
  and regenerate its data and HTML **before** broad test suites. Use focused
  checks for small iterations; run the full suites periodically or after a
  substantial batch, not after every small change. Heartbeats are the primary
  performance metric.

- Prover tests, from `third_party/move/move-prover`: `MVP_TEST_FEATURE=lean cargo test`
  with one filter per run; `UPBL=1` regenerates the `.lean_exp` baselines. Read every
  changed baseline; an `-- error:` line is a regression unless intended.
- Suites: `lake build && lake test` in `leaner-ir`, `leaner-move`, `leaner-e2e-tests`.
  Never rebuild `leaner-ir` while a suite runs; iterate in a scratch copy.
- A deliberate cost change: `UB=1 lake env lean LeanerLang/Tests/DenotePerformance.lean`
  after `lake build`; the diff names only the targets meant to move.
- Benchmark: `python3 scripts/leaner-bench.py run --out <fresh.json>`, then
  `python3 scripts/leaner-bench.py report --local <fresh.json> --branch main --html local_benchmark.html`.
  Compare against main, not intermediate local runs.
- A new capability gets a Check fixture and a roadmap or design row. A Check fixture never
  records an unsupported positive proof as an expected failure.
- Debugging a target: `set_option leaner.denoteDebug true` and `verify f by skip` show the
  leaf obligations.
