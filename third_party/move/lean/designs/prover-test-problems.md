# Move Prover tests under Leaner: problem registry

Status: 2026-10-07, with the Move Prover's `lean` test feature. C1, C3,
C4, C5, C7, C9, C10, C13, C14, C15, and C19 are fixed, and V9, V17, and V19
no longer occur. G6 (in-body assertions not checked) and G7 (in-body assumptions not
used) are fixed: a function's theorem takes that its assumptions hold
([`denotation.md`](denotation.md), "Assumption ledger"). Since the
static memory ([`static-memory.md`](static-memory.md)), V5 and V13 no
longer occur, and V4, V8, and V12 are fixed. G9 (signers not assumed
transaction signers) is fixed. G5 is fixed: a data invariant is owed where
a value is constructed and where a mutation of a local ends, at the death of
its loan or after a write into it, as the Prover checks it. A module
invariant is owed after each write of memory it reads and after each call
of a callee that leaves it to its callers, and the invariants of every
module of the file (the package) concern a function, not only those of the
modules it uses. A unit's type table holds
every type its generic frames require (V14's second cause, fixed
2026-10-03). A specification reads a field at its physical type where the
consumer asks for it, as a vector element does, so V11, which followed from
such a mismatch in `std::ascii`, no longer occurs. Intrinsic `vector` functions are read as LIR
operations ([`source-verification.md`](source-verification.md), "Move
vector intrinsics"). Matches and destructuring bindings over values are
carried, and a value no pattern fits aborts with Move's incomplete-match
code ([G13](#g13-a-destructuring-that-does-not-fit-aborts-with-the-match-code)).
V24 is fixed: a loan no later operand uses dies before the next operand of
the same operation. V14 is fixed: a pattern through a reference is rewritten
into the variant tests and field borrows Move's compiler emits for it, so the
enum tests fail the functions the Prover fails.

The Move Prover's unit tests (`third_party/move/move-prover/tests/sources`,
437 files) run with the Leaner verifier through the test feature `lean`
([`source-verification.md`](source-verification.md), "From the Move
Prover"). This registry lists every problem that run shows, read from the
`.lean_exp` baselines beside the tests. Entries record symptoms. A cause is
named only where it is known.

## Updating the registry

After a fix, run from `third_party/move/move-prover`:

```bash
MVP_TEST_FEATURE=lean UPBL=1 cargo test -p move-prover --test testsuite
```

Review the `.lean_exp` diffs. Remove the tests that no longer show an
entry's message, and the entry once no test does. Identifiers stay stable:
a new problem takes the next free number of its section. Test paths are
relative to `tests/sources`.

## Summary

Callee preconditions (2026-10-09): a caller owes the `requires` of a callee
whose body it inlines, where the callee starts, as the Prover asserts a
callee's preconditions at every call; before, only a callee used through its
contract had them checked. A function without a specification is verified
when it calls a function with a precondition. `schema_apply` fails as the
Prover does: `the precondition `requires false` of `f` does not hold at this
call`.

Inline specifications (2026-10-08): a non-opaque inline function with a
specification of its own is verified against it, as the Prover verifies it;
before, the importer dropped it with the calls it had expanded, and its
specification was never checked. `inline_spec_no_opaque` fails `bad_inc` as
the Prover does.

Loop invariant placement (2026-10-08): a loop takes the leading run of
`invariant`s of the specification blocks its header begins with, as the
Prover's loop analysis does (`fat_loop.rs`); any other loop invariant is an
error at its location and nothing is verified (`Frontend.LoopInvariants`).
`loop_invariant_invalid` reports the Prover's four errors. Consecutive
specification blocks of a header are joined into one by the importer, and
all annotations of one loop form its specification
(`Contract.loopSpecifications`): before, a loop's second annotation
silently replaced its first, whose invariants were neither checked nor
assumed. The Prover's exemption of declarations in unreachable code is not
modeled; no test has one.

Generic axioms (2026-10-08): `axiom<T>` is assumed at the instantiations a
verification applies (V1), `num` is a type argument of specification
functions (V2), a module's abort strictness holds for its functions, and
`TRACE(e)` reads as `e`. `axiom_generic`, `axioms`, `mono_on_axiom_spec_type`
and `opaque_native` verify; `performance_200511` reaches verification.

Axioms and bitwise follow-up (2026-10-08): a module axiom over values is
assumed by every verification, as the Prover assumes its axioms globally, and
the rendering prints it as an axiom; the export includes the modules
specifications read; an arbitrary value takes no type arguments; an `int2bv`
of a literal through casts is folded; an equality compares at its operands'
type when a schema's inclusion gave them another; the closer relates a
conjunction's two operand orders and reads a nonnegative shift's truncating
remainder as its remainder. `abort_in_fun`, `bitwise_operators`,
`bv_internal` and `defines` now verify,
`bv_aborts` fails its clause as the Prover does, and `bitwise_features` keeps
two budget failures of four (G14, V2).

The Prover's bit-vector representation is no longer modeled (G14, decided
2026-10-07): the representation guard and the scalar wrapping lowering described
in the older paragraphs below are retired, `int2bv`/`bv2int` are imported
exactly, and the former representation-guard files reach verification.

Closed scalar-package lowering now restores `regression/test_bitvector` to
verification success and makes `regression/bv_mul_overflow` reject the required
arithmetic abort, matching MVP. Seven new scalar arithmetic controls pass both
backends; seven existing source negatives (including a concrete internal clause)
now fail during verification rather than import. Representation rejections fall
from 29 to 27. Full Move/E2E suites and all 437 normal registry checks pass after the two
intended baseline updates (`/tmp/bv-scalar-registry-final.log`).
The benchmark/main HTML was regenerated first: 24 verified, one expected AMM
rejection, six unchanged import failures, 9,509,422,287 raw heartbeats.

The XAST v10 literal-metadata checkpoint passes 59 exchange tests, full
Move/E2E suites and all 437 registry checks without baseline updates
(`/tmp/bv-literals-validation.log`, exit 0). It supplies a missing source fact
for C17's future width inference; it does not change registry coverage. The
regenerated main-relative benchmark has 24 verified problems, one expected
AMM rejection, six unchanged import failures and 9,509,559,974 raw heartbeats.

The scalar `bv_internal` checkpoint passes full Move/E2E suites and all 437
registry checks without baseline updates (`/tmp/bv-internal-validation.log`,
exit 0). `bv_internal_wrapping` now verifies its valid function and rejects
its deliberately false contract. The other changed registry baseline is
`bv_internal`, whose remaining concrete clause is diagnosed explicitly.
There are now 29 representation rejections. The full benchmark/main-relative
HTML was regenerated before the suites: 24 verified, one expected AMM rejection,
six unchanged import failures, 9,509,276,380 raw heartbeats. Reference casts
also now read their operands correctly; positive and negative source tests
agree with MVP. General bit-vector propagation remains open.

The expanded representation-seed guard has refreshed measured benchmark data
and main-relative HTML: 24 verified, one expected AMM rejection, six import
failures, 9,509,557,743 raw heartbeats. The reduced coverage must not be counted
as a speed improvement. Its registry refresh passes all 437 files; C17 now
covers 30 rejected files, including 10 with previously empty baselines. The
full Move/E2E suites and normal 437-file registry recheck pass without baseline
updates. `/tmp/bv-seeds-validation.py` completed with exit 0 (2026-10-07).

The bit-vector soundness checkpoint (2026-10-07) passes the full Move/E2E
suites and all 437 registry checks without baseline updates after regeneration
(`/tmp/bv-guard-validation.log`). The source importer now rejects conversions
whose previous erasure allowed false overflow proofs (C17), and reports
import failures without an uncaught exception (G1). Five registry files hit
this boundary. The measured benchmark/main-relative HTML was refreshed first:
27 verified, one expected AMM rejection, three import failures, 13,664,061,946
raw heartbeats. The smaller total reflects missing verification work in
`features`, `ristretto255`, and `ed25519`; it is not a speed improvement.
Restoring their representation semantics remains open.

The conditional-range checkpoint (2026-10-07) passes all four full suites and
all 437 registry baseline checks without refresh
(`/tmp/conditional-range-full-tests.log`, exit 0). The new fallback handles
split conditionals in range invariants; its positive and rejection tests pass.
`count_even_concrete` still exceeds native 25k, so no registry success is
claimed for it. The full benchmark and generated main-relative HTML were
refreshed before broad testing: 30 verified + one expected AMM rejection,
15,066,137,470 raw heartbeats (+0.0328% from the pool checkpoint).

The pool performance checkpoint (2026-10-07) passes all four full suites and
all 437 registry baseline checks without refresh. It includes the product-fold
endpoint guard below. The benchmark preserves 30 verified + one expected AMM
rejection at 15,061,197,787 raw heartbeats (4.03% below the preceding full run);
measured JSON and main-relative HTML are refreshed before the suites. See
`/tmp/pool-final-checkpoint.log` (exit 0) and handoff.md. Matching registry
baselines does not mean all registry specifications are verified: the remaining
unsupported constructs, intended rejections and timeouts below remain open.

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
Core/Move builds and the focused checks pass. The first broad registry check
exposed two regressions; restricting endpoint congruence to symbolic products
restores their original behavior. The subsequent pool checkpoint passes all four
full suites and all 437 registry checks without refresh, retaining this product
improvement (`/tmp/pool-final-checkpoint.log`).

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

`state_labels/aliasing` now fully verifies at native 25k (2026-10-07).
The remaining `different_addr_global` proof costs 19.833M typed raw heartbeats.
Updated-memory equality uses reflexivity for opaque empty row tails, and the
context simplifier excludes conditional self-referential equations from its
rewrite rules while retaining their facts. A registered opaque-caller test
verifies using only the labeled contract (11.577M typed heartbeats); a wrong
update remains rejected. The owning runner removed the obsolete baseline;
both cost gates and all focused label checks pass. The refreshed benchmark
retains 30 verified + one expected rejection at 15,696,214,192 raw heartbeats.
All four full suites and all 437 registry checks subsequently pass (handoff.md).

`intermediate_states::test_config_preserved` now verifies at native 25k
(2026-10-07), with 8.794M typed raw heartbeats. Conditional equalities for
both Boolean outcomes are compared after identifying unchanged memories.
The bounded attempt introduces no branch assumptions without case analysis.
The two negative siblings retain their original clause diagnostics; the
owning runner removes only this positive's obsolete failure. Both cost gates
and four focused label fixtures pass. AMM create_pool also matches its
original baseline again after restricting call-observation eligibility to
direct facts. All four full suites passed just before these follow-ups;
the subsequent benchmark preserves 30 verified + one expected rejection at
15,676,741,907 raw heartbeats, and the full registry matches 437/437 baselines
(handoff.md).

`aborts_if_at_state_label::caller` and `aliasing::remove_then_try_read` now
verify at the native 25k limit (2026-10-07). A bounded attempt consumes existing
call observations before reconstructing runs or re-deriving contracts. The
first target uses 20.251M typed raw heartbeats; the registered fixture covers
an opaque caller and an intended false-postcondition rejection. Both cost
gates and decoding/witness fixtures pass. At that checkpoint test_config_preserved
stopped timing out but still left its result equality; the follow-up above
resolves it. Owning-runner baselines are refreshed (handoff.md).

`spec_fun_old_param_labeled_with_memory` now verifies at the native 25k limit
(2026-10-07). Constructive/context witnesses precede the program-point search,
reducing the typed proof from 30.912M to 24.132M heartbeats with unchanged
proof size. The owning runner removes its timeout baseline; the ordinary
30-file state-label check, registered 25k fixture, and both cost gates pass.
The refreshed benchmark retains 30 verified + one expected rejection at
15,647,293,919 heartbeats (+0.00105%). Main-relative JSON/HTML are current;
all four builds/full suites and 437/437 registry checks subsequently pass.

`two_state_labels` now fully verifies at the unchanged native 25k limit
(2026-10-07). Its timeout baseline was removed by the owning runner; all 30
state-label registry baseline checks pass afterward. The closer exposes known
memory/decoder results before reducing integer range checks; it also consumes
isSome facts from an opaque callee's state-change contract. The new registered
IR test verifies the opaque caller and rejects missing-resource/overflowing
state updates. Both cost gates pass. Whole typed proof cost is 18,269,904 raw
heartbeats. The regular benchmark passes with 30 verified + one expected
rejection at 15,647,129,560 heartbeats (+0.0013%); main-relative JSON/HTML are
refreshed. No newer full-suite result is claimed than the preceding Table
checkpoint below.

`table_option` is now fully verified at the unchanged native 25k setting
(2026-10-07). The owning runner removed its obsolete failure baseline and its
ordinary recheck passes (`/tmp/table-option-fixed-registry-recheck.log`). Exact
lookup transport and selective aggregate normalization reduce its whole typed
theorem from 28.690M to 25.254M raw heartbeats. All Table invariant tests,
identity rejection guards and both cost gates pass. The first benchmark exposed
an ordered-map regression, so broad validation did not start. The original
call-range eligibility guard is restored. Native table_option and both cost
gates pass; the corrected benchmark has 30 verified plus one expected AMM
rejection, no timeouts, and 15,646,929,683 heartbeats. Measured JSON and
main-relative HTML were refreshed before the full suites. All four builds
and full suites pass; the complete registry matches 437/437 baselines without
refresh (`/tmp/table-restored-registry-check.log`). The final Table theorem
cost with the guard restored is 26,045,418 raw heartbeats; native 25k acceptance
is independently confirmed. Earlier checkpoint results below remain historical.

The call-range eligibility checkpoint (2026-10-07) passes all four full suites
and all 437 ordinary registry baseline checks without any baseline changes
(`/tmp/call-range-checkpoint-full-tests.log`,
`/tmp/call-range-checkpoint-registry-check.log`). This validates the preceding
snapshot optimization too. `table_option` remains a timeout: its full typed
theorem costs 28,686,998 raw heartbeats, although closer-only work is 24,405k.
The benchmark was refreshed before the suites: 30 verified, one expected AMM
rejection and 15,624,166,021 heartbeats (−0.6439% from the snapshot checkpoint).
A subsequent computed-index range-detection fix passes 16 positive fixture
proofs, two rejection guards, both cost gates and unchanged table_option and
verify_vector registry checks. Its fresh benchmark preserves all outcomes at
15,646,803,818 heartbeats (+0.1449%); the full-suite result above predates that
increment. A further Table normalization batch is building. No new whole-file registry success is
claimed.

The Table stored-invariant batch (2026-10-07) passes all four full suites,
131 Check fixtures and both cost gates. The full registry matches 435/437
baselines (`/tmp/table-invariants-registry-check.log`). The two differences
reduce diagnostics: `different_addr_global` finishes with an ordinary failure
instead of timing out, while `verify_remove_with_unroll` still times out with
one fewer clause message. Both reviewed baselines were refreshed by the owning
runner and pass ordinary focused rechecks (`/tmp/table-invariants-aliasing-recheck.log`,
`/tmp/table-invariants-vector-recheck.log`). Neither is a new verified target. `table_option`
remains over the unchanged 25k limit. The benchmark and generated main-relative
HTML were refreshed before broad checks: 30 verified, one expected AMM
rejection, 15,725,534,459 raw heartbeats. The subsequent snapshot-equality
optimization preserves these outcomes at 15,725,421,251 heartbeats. It reduces
`table_option` closer work to 24.87M, but the entire typed theorem remains
29.14M and still times out at 25k; no new registry pass is claimed.

The element-quantifier batch (2026-10-07) passes all four full suites, 130 Check
fixtures and both cost gates. The full registry run matches 436/437 baselines;
its only difference is that `moved_local_in_loop` now verifies at the unchanged
25k budget. Its obsolete timeout baseline was regenerated with the owning runner
and the ordinary focused recheck passes. Evidence:
`/tmp/element-final-full-tests.log`, `/tmp/element-final-registry-check.log`,
and `/tmp/element-final-moved-local-recheck.log`. The benchmark and generated
main-relative HTML were refreshed first: 30 verified problems, one expected AMM
rejection, and 15,744,282,692 raw heartbeats (+0.1096% over certified reads).
Baseline matches include intended negatives and are not parity counts.

The certified-read/continuation-alias batch passes all four full suites,
including 129 Check fixtures and both cost gates. The subsequent full registry
check matches 432 of 437 baselines. A stale `count_all` companion was repaired
and passes unchanged; four reviewed diagnostic baselines pass ordinary checks
after regeneration. `folds_of_callee_ensures::count_small` now verifies at its
unchanged 25k budget. No previously verified target remains regressed. Evidence:
`/tmp/certified-read-full-tests.log`, `/tmp/certified-read-registry-check.log`,
`/tmp/certified-read-pure-callee-recheck.log`,
`/tmp/certified-read-registry-refresh.log`, and
`/tmp/certified-read-registry-audit.json`. Benchmark data and main-relative HTML
were refreshed first: 30 verified problems plus the expected AMM rejection,
15,727,043,391 raw heartbeats (0.5148% lower than the preceding local run).

The full 2026-10-06 resumed run discovers 437 files. All four Leaner packages build
and pass their full suites, including the 123 Check fixtures, both cost
gates, source verification, MonoVM, and differential tests. Logs:
`/tmp/vector-resumed-<package>-{build,test}.log`.

The owning Move Prover test runner refreshed all 437 baselines successfully
(`/tmp/vector-resumed-prover-refresh.log`). The diagnostic audit finds no
new failed targets. Besides the scoped `remove_all_found` improvement,
`verify_vector` has more specific residuals and two normal rejections instead
of timeouts; its failed target names are unchanged
(`/tmp/vector-resumed-registry-audit.{json,log}`). `bp_pure_callee::count_all` now proves
at 25k with its fold-equation companion. The resumed vector follow-up now
also proves `remove_all_found` automatically at the unchanged 25k budget;
its scoped baseline retains only the two expected fold-derivation warnings.
The subsequent focused checks also prove `verify_swap_remove`,
`verify_model_swap_remove`, and `verify_index_of` with an authored companion
at 25k. The model-call swap-remove proof costs 21.313M raw heartbeats, and
the index search proof costs 22.520M; neither raises the acceptance budget.
Their core residual-attempt cap now passes all four full suites and the
437-file registry audit, with no new failed targets
(`/tmp/vector-residual-<package>-{build,test}.log`,
`/tmp/vector-residual-registry-audit.{json,log}`).
Returned-reference freezing
and quantifier lexical scope now let both C16 files reach verification; two
previously blocked `bp_forwarding` targets prove. The generic-caller companion
closes `specialize_generic_caller::use_concrete` at 25k, leaving only its existing
compiler warning. The subsequent opaque-inline frontend fix passes its Move package suite and
all 437 registry baseline checks. Only its three intended baselines change:
one now has no diagnostics and two reach their deliberate negative cases. No
previously verified target regresses (`/tmp/opaque-inline-registry-audit.{json,log}`).
The literal-swap-read optimization also passes all four full suites and all
437 registry checks, with no baseline changes
(`/tmp/literal-swap-registry-audit.{json,log}`).
The Table call-agreement extension also passes all 437 ordinary baseline
checks with no diagnostic changes (`/tmp/table-call-prover-check.log`). It
extends storage encoding, loan independence, and labeled post-state uniqueness;
The following result-aware frame/returned-storage extension also passes all
437 ordinary checks (`/tmp/returned-storage-prover-check.log`). Table native
contracts remain disabled, so V7 outcomes are unchanged.
The native-collection carrier follow-up also passes all four full suites,
including 124 E2E Check fixtures and both cost gates, plus all 437 registry
checks with no baseline changes (`/tmp/table-contents-full-tests.log`,
`/tmp/table-contents-prover-check.log`). It removes the unintended Move-vector
size bound from Table contents and fixes reversed resource-argument substitution
in expanded generic specifications. Its new generic-resource regression covers
both argument positions and generic callers. The generated benchmark remains
30 verified plus the expected AMM rejection at 15,807,436,992 raw heartbeats,
relative to main; native Table frontend roles remain disabled, so V7 is unchanged.
The following Table snapshot integration now proves `table_contais_to_length`
at its original budget. Its obsolete baseline was removed by the owning runner,
and a normal filtered recheck passes. The new source checks preserve old/nested
observations, caller-side state labels and different functional contents for the
same identity. The full benchmark passes 30 samples plus one expected AMM rejection at
15,807,887,568 raw heartbeats (+0.0029% from the preceding local checkpoint).
All four full suites pass, including both cost gates and 125 E2E Check fixtures.
The full registry check exposes a second newly verified fixture,
`map_equality_encoding`; six other Table fixtures now reach native-operation
or mutable-reference denotation gaps. Their seven baselines were regenerated
and reviewed; all 437 normal checks pass on recheck. No previously verified
target regressed. Logs: `/tmp/snapshot-routing-full-tests.log`,
`/tmp/snapshot-routing-prover-recheck.log`,
`/tmp/snapshot-routing-registry-audit.{json,log}`.

The subsequent shared-Table-read increment enables membership and shared lookup
through explicit intrinsic-contract assumptions. Its five new source proofs pass,
and an incorrect lookup claim is rejected. `table_option` now reaches its assertion
but still exceeds its unchanged 25,000 maxHeartbeats: snapshot projection
normalization and the nested Option's data invariant remain missing. The bitwise
and verify_table fixtures advance to mutation natives. Three changed baselines
were regenerated by the owning runner and all five scoped normal rechecks pass
(`/tmp/table-read-registry-refresh.log`, `/tmp/table-read-registry-recheck.log`).
Those baseline matches are not new verified fixtures. The subsequent full run
passes all 437 registry baseline checks and all four suites (127 Check fixtures
and both cost gates). The complete audit retains the counts below unchanged:
`/tmp/table-reads-prover-check.log`, `/tmp/table-reads-registry-audit.json`,
`/tmp/table-reads-full-tests.log`.

The aggregate-projection follow-up passes both cost gates and six scoped Table
registry baseline checks, after regenerating `table_option`'s changed diagnostic
baseline. The official fixture still times out at 25000; it is not newly verified.
Diagnostic source probes must set both `maxHeartbeats` and
`leaner.verifyHeartbeats` to reproduce that budget. The new aggregate payload
source proof passes, and a false payload length bound is rejected. Logs:
`/tmp/table-projections-check.log`, `/tmp/table-projections-option-recheck.log`.
The preceding 437-test run remains the latest complete registry checkpoint.

The historical Table-read checkpoint diagnostics were:

| Outcome | Files |
|---|---:|
| No diagnostics | 122 |
| Warnings/notes only | 5 |
| Rejected before verification | 47 |
| Verification diagnostics | 231 |
| Compiler/other errors | 32 |

These are diagnostic categories, not parity counts. Many fixtures deliberately
contain invalid specifications. In particular, abort-code and constant-vector
fixes expose their intended negative cases; an empty baseline is not the
expected result for those files. A subsequent stored-field-frame batch now
reaches verification in `stored_fun_values` (33/42 targets prove in the scoped
run; see V26). The nominal function-field quantifier restriction remains a
soundness guard, not a proved positive case.
The C18 resource-projection fix moves one more file from preverification
rejection to verification diagnostics. Field-update normalization and the
cross-resource companion plus prepared-call-fact simplification now prove all
six positive targets at 25k. These changes
are included in the full suites above. The full registry audit additionally
finds `anchor_mutation_pre_state::valid_derived_claim` now proves, while its
wrong attached claim still fails. No previously proved target regressed.
`state_labels/aliasing` reports an extra unresolved clause before its existing
timeout; its failed target names are unchanged. Audit:
`/tmp/update-normalization-registry-audit.{json,log}`.
The function ledger below tracks known positive obligations still unproved;
it is being reconciled as previously rejected files reach verification.

The latest regular benchmark was regenerated before its broad suites, with
HTML generated against main CI run 37250691414 (`ea4ecc43e7`). The full run
`/tmp/leaner-benchmark-table-routing.json` has 30 verified samples, one expected
AMM rejection, and no unexpected failures or timeouts. It uses 15,826,709,038
raw heartbeats (−0.0012% from the returned-storage checkpoint). All four full
suites and all 437 registry checks pass (`/tmp/table-routing-full-tests.log`),
with no baseline changes. This full E2E run covers the fixed closure-frame
binder regression and reaches the VM stages. Native Table entry-loan primitives
are tested, but frontend roles remain disabled and V7 outcomes are unchanged. Framework
ordered_map is 29/29 at 4.158G, calculator is 8/8 at 1.072G, and pool_u64 is
22/22 at 3.189G. AMM's six valid targets prove; its noncompliant fee constructor
is rejected normally at 185.036M heartbeats. All seven AMM targets contribute
to its 1.548G total. The removed type_info sample remains excluded. Sample
statuses alone do not establish target-level parity.

The final registry audit found no diagnostic changes after the cost-gate and
Order proof corrections (`/tmp/short-circuit-final-registry-audit.log`). The
preceding refresh changed only two diagnostics, with no new failed functions:
shorter aborts_if residuals and bug_15044 failing normally instead of timing out.
The run includes generic enums, stored-field frames, implicit function wrappers,
and the registry AMM constant_product companion. `result_of_old_label` and
`fun_arg_frame_check_valid` also have no diagnostics.

The generic-caller companion applies the already proved `count_is_len` lemma
at u64 for the loop entry, exit, and back edge. Inlining intentionally omits the
callee's proof block, so the caller needs these instances itself. No source
specification or heartbeat limit changed. Scoped refresh and ordinary baseline
check pass (`/tmp/specialize-generic-caller-registry-{update,check}.log`); the
current audit records only that removed failed target and no additions
(`/tmp/specialize-generic-caller-registry-audit.log`). The full suites above
precede this proof-only follow-up.

The older 414-file survey found 120 MVP-positive functions/lemmas unproved
in 43 files. That historical count is no longer a current parity measurement;
current entries must distinguish unsupported constructs, budget exhaustion,
and intended negative specifications.

## General

### G0. How the test driver runs a test

A test file is verified alone, with the driver's dependencies: the Move
standard library and its nursery, and `move-table-extension`
(`std = 0x1`, `extensions = 0x2`), or the Aptos standard library under
`// use-aptos-stdlib`; `// flag:` lines add options (`testsuite.rs`,
`get_flags_and_baseline`). The Leaner path (`lib.rs`,
`run_move_prover_lean`) builds the model with them and exports the target
file's modules with what they read. Of the flags, it honors the extra
`--dependency` files (`script`, `script_incorrect`, `exists_only_memory*`)
and, since 2026-10-08, `--verify-only`: a target function the scope leaves out
is exported with `pragma verify = false`, one it names with
`pragma verify = true` (`leaner.rs`, `apply_scope`), as the Prover's
`should_verify` selects them (`verify_only_list` now matches the Prover). The
Boogie options (`--vector-theory`, `--split-vcs-by-assert`, `--timeout`,
`--trace`) have no counterpart; `--check-inconsistency` is G11.

Scripts were not verified at all until 2026-10-08: the verifier looked up a
module by its path's raw spelling, which keeps the quotes of a script's
`«<SELF>_0»`, found none, and verified nothing. `script_incorrect` now fails
as in the Prover (`SourceVerify/script_false.move`). The same lookup skipped
every module whose rendered name is quoted, `std::string` among them.

### G1. Import diagnostics — returned as reports; precise locations still open

`SourceVerify.verifySource` now returns LIR import/validation errors as error
reports rather than throwing an uncaught exception (2026-10-07). This lets
the benchmark retain the failure and its measured frontend work, and lets
the source baseline suite continue to subsequent fixtures. The report is
anchored at the input's first line; embedded byte ranges still need mapping
back to precise source locations. Exporter/IO exceptions remain separate.
An invalid unit still prevents verification of the whole file or package,
including functions without an error; this change does not add recovery.

### G2. Baselines carry residual goal states

7 baselines record the full goal state of a residual obligation. The
largest, `functional/type_dependent_code.lean_exp`, has 638 lines. Such
baselines change with any change to the closer's
normalization.

### G3. A hypothesis can hide a parameter in an obligation

In `functional/fixed_point_arithm.move::mul_div`, the obligation has a
hypothesis named `x`. It hides the parameter `x`, which becomes the
inaccessible `x✝⁴`, so a hand proof cannot name it. The proofs in
`fixed_point_arithm.proof.lean` work around this with `‹…›` patterns.

### G4. A proof file needs a single-module Move file

`foo.proof.lean` accompanies only a Move file that declares one module.
34 of the test files declare several modules, so their functions cannot
get a hand proof.

### G8. Inconsistent invariant pragmas are resolved, not rejected

The Move Prover rejects two uses of the invariant pragmas as inconsistent:
delegating from a public function, and `disable_invariants_in_body` on a
function whose invariants a caller already carries. Leaner resolves both
soundly: a public function carries its invariants itself, and a function
whose caller carries them carries none.

Example (`functional/disable_inv.move`): the Prover rejects the module;
Leaner reports `f1_incorrect`, a public function delegating its invariants,
at its first write, and verifies the other functions.

The Prover also rejects a public function called, at any depth, from a
function with `disable_invariants_in_body` (`functional/disable_inv_indirect`):
the public function assumes the invariants at entry, which the disabled body
need not keep. Leaner verifies the module; a call in such a body does not
give the callee's postcondition where the invariants are broken (a function
breaking an invariant, calling a public reader whose postcondition holds by
the invariant, and restoring it before its exit, fails its own
postcondition).

Tests: `functional/disable_inv.move`, `functional/disable_inv_indirect.move`.

### G10. `pragma unroll` is bounded checking

With `pragma unroll = N`, the Move Prover unrolls a loop N times and cuts
the paths that iterate more often, so it checks the function only for runs
within N iterations. Leaner does not mirror the cut (decided 2026-10-02): a
theorem states full correctness. It reads the pragma as a sound proof rule
instead: up to N iterations and the final condition check are exposed, and a
path that would iterate further must be unreachable. A function whose loop
can run more often verifies only by a loop invariant. `math8::floor_log2`
(`unroll = 2`) always runs three iterations, so the Prover's check of it is
vacuous; Leaner does not accept it.

Example (`functional/verify_vector.move`, `verify_contains_with_unroll`):
the Prover verifies the assertion after the loop for vectors of at most
three elements; Leaner reports `the assertion `!(∃ (x in v), x == e)`
does not hold`.

Tests: `functional/loop_unroll.move`, `functional/verify_vector.move`
(`verify_append_with_unroll`, `verify_contains_with_unroll`,
`verify_index_of_with_unroll`, `verify_remove_with_unroll`,
`verify_reverse_with_unroll`).

### G11. Inconsistent assumptions are not reported

With `--check-inconsistency`, the Move Prover reports a function whose
verification holds only because something it assumes is inconsistent.
Leaner states a function's in-body assumptions as a hypothesis of its
theorem ([`denotation.md`](denotation.md), "Assumption ledger") and
does not check that the hypothesis can hold.

Example (`functional/inconsistency.move`, `assume_false`): `spec { assume
false; }` before a call that returns lets `ensures false` verify; the Prover
reports `there is an inconsistent assumption in the function`, and Leaner
verifies the theorem, whose hypothesis holds only of a function no run of
which returns or aborts.

Tests: `functional/inconsistency.move` (`assume_false`).

### G12. A mutably borrowed place reads as its final value

While a `&mut` loan of a place lives, a specification reading the place
sees the value the loan ends with, its prophecy; the Move Prover sees the
value before the write-back. Code cannot observe the place during the
loan. The difference is inherent to the prophetic model and stays (decided
2026-10-03).

Example (`functional/bug-17117.move`, `test_input_param_as_mut_ref`): after
`let y = &mut r.x`, an assertion of `r.x` before the write through `y` reads
the written value.

Tests: `functional/bug-17117.move` (`test_input_param_as_mut_ref`).

### G13. A destructuring that does not fit aborts with the match code

A value no arm of a match fits, one a destructuring `let` or assignment
does not fit, and an access to a field its variant lacks abort with Move's
incomplete-match code `0xCA26CBD9BE0B0001` (decided 2026-10-03). The Move
VM aborts a match, and a selection of a field several variants share, with
that code too, but a destructuring and a field of one variant with the
execution failure `STRUCT_VARIANT_MISMATCH`, which the Prover reports as an
execution failure; the two differ only for `aborts_with`.

### G14. The Prover's bit-vector representation is not modeled

`pragma bv`, `bv_ret`, `bv_internal` and the classification that bitwise
operations induce select the Move Prover's SMT bit-vector encoding. Under it the
Prover wraps specification arithmetic over encoded values at their width, and a
narrowing specification cast yields an unspecified value of the target type.
Leaner treats the encoding as a backend choice (decided 2026-10-07):
specification arithmetic stays mathematical, bitwise operations are exact on
integers, and executable arithmetic is checked as always. The explicit
conversions are modeled: `int2bv(e)` wraps `e` into its fixed-width result type,
two's complement for a signed one, and `bv2int` reads the value back. (The
Prover renders signed values as integers, so its signed `int2bv` does not wrap.)

The two verifiers differ only where a specification's truth depends on that
wrapping. With `pragma bv = b"0"` and `x == 255`, `ensures x + 1 > 255` verifies
in Leaner and fails in the Prover; `ensures int2bv(x) + int2bv(1u8) == 0u8` is
the reverse. `SourceVerify/bv_encoding{,_false}.move` pin both directions;
`bv_conversion{,_false}.move` pin the conversions, on which both agree.

The former representation-guard files now reach verification; ten verify
cleanly. Of those, `functional/bv_cast.move` differs by the narrowing-cast rule
(the Prover rejects `(v as u8) == (v as u8)`), and the Prover's errors in
`functional/bv_internal_invalid.move` and `functional/bv_internal_aggregate.move`
concern its encoding only. An `int2bv` the compiler types at `num` (e.g.
`int2bv(value + 1)`) wraps at `u64` (decided 2026-10-08), so
`MoveToLeanerLang/constants.move` translates again. Open: one at a type
parameter has no width and is rejected (`functional/bv_signed_generic.move`).

An `int2bv` of a literal, also seen through specification casts, is the
wrapped literal, so `functional/bitwise_operators.move` decreases its measure
and verifies. `functional/bv_internal.move` verifies now that its axiom over
values is assumed (V2 below) and printed as an axiom. In
`functional/bitwise_features.move`, `contains` and `is_enabled` verify: the
closer states that a conjunction a leaf mentions with its operands in both
orders is one value, and reads a specification's truncating remainder of a
nonnegative shift as the runtime's remainder (`SourceVerify/module_axioms{,_false}.move`). `set` and
`disable_feature_flags` still exceed the budget: clearing a bit of a vector
element (`m & (v & (255 ^ m)) = 0`) needs bit-level reasoning through the
write.

Tests: `functional/bv_cast.move`, `functional/bv_internal_aggregate.move`, `functional/bv_internal_invalid.move`, `functional/bv_signed_generic.move`.

### G15. Specification functions are partial

A Move specification function is defined where each parameter declared with a
fixed-width integer type holds a value of that type; elsewhere its value is
unspecified (decided 2026-10-08). Specification typing lets a caller pass any
integer to such a parameter, and a specification function cannot abort. The
importer keeps the declared parameter types, and the body reads such a
parameter as an `Int`. The verifier derives the domain from the types: a
recursive definition unfolds inside it, an expansion is guarded by it (an
argument whose own form bounds it, such as a certified integer of a fitting
type, a literal or a vector length, needs no guard), and outside it both read
one uninterpreted value of the function at the arguments (`f.spec.outside`). The closer decides a domain guard from the bounds the
arguments' types carry, and keeps an unfolding only where it also decides the
body's own first condition, as for a function without a domain. A `num`
parameter is unbounded. The specification version of a Move function is
derived again from the function and is not guarded.

The Prover's specification functions are total over the integers: for
`spec fun successor(x: u8): num { x + 1 }`, `successor(300) == 301` verifies
there and not in Leaner. Inside the domain the two agree.

Tests: `SourceVerify/spec_fun_domain{,_false}.move`.

### G16. Verified where the Prover's encoding or solver gives up

Some tests pin a failure of the Prover that is not a property of the
program: a limit of its encoding, of its specification inference, or of the
solver. Leaner proves these functions, which is the intended outcome: its
behavioral predicates are defined from a function's meaning, not from an
inferred contract, and its theorems do not depend on solver heuristics.

- `proof/weight_too_large`: a `[weight = 1000]` axiom is never instantiated
  by Z3; `id_num(0) == 0` unfolds in Leaner.
- `closures/lambda_spec_loop_anchor`, `closures/lambda_captured_fun_loop`:
  the inferred contract of a looping lambda is not provable; Leaner reads
  the lambda's behavior from its body.
- `closures/behavioral_underivable_body`: `result_of<sum_to>` of a function
  without a specification whose body loops.
- `closures/lambda_funparam_memory_err`,
  `closures/lambda_funparam_declared_memory_err`,
  `closures/lambda_nested_hof_memory_err`: behavioral predicates of
  function-typed parameters whose targets access global memory, which the
  Prover's encoding does not thread yet.
- `state_labels/labeled_state_arg_mismatch`: labeled states over two
  different `&mut` arguments, which the Prover's witness keyed by label
  cannot tell apart.
- `regression/generic_aliasing_cap`: more than 256 type-aliasing cases; the
  memory of a generic resource is one typed memory per instantiation.

### G17. Diagnostics of the Prover not reported

The Prover rejects some specifications Leaner reads without harm:
`old(..)` of an expression that does not depend on state
(`functional/old_param_err`; Leaner reads it as the expression), and a
function accessing a resource its `reads` clause does not cover
(`functional/reads_check`). A function's `reads` clause is not used by
Leaner: a caller takes the callee to read all memory, so the unchecked
clause cannot be relied on.

## Rejected before verification

### C2. Map intrinsic declarations rejected

27 tests (21 also rejected by the Prover, marked †), 173 messages.

Messages:

- 114 × `` Move map intrinsic is missing required role `…` ``
- 30 × `` Move map intrinsic role `…` target has an incompatible physical signature ``
- 29 × `` Move map intrinsic role `…` requires `…` ``

Example (`functional/ghost_field_intrinsic_map_data_inv.move`):

```text
LIR-MOVE-INTRINSIC-ROLE-REQUIRED: Move map intrinsic is missing required role `map_spec_del`
```

Tests: `functional/ghost_field_intrinsic_map_data_inv.move`, `functional/ghost_field_intrinsic_map_full.move`, `functional/ghost_field_iter_abort_native.move`, `functional/intrinsic_iter_abort_role.move`, `functional/intrinsic_iter_abort_sig_err.move`†, `functional/intrinsic_iter_abort_uninterp.move`, `functional/intrinsic_iter_role_err.move`†, `functional/intrinsic_iter_role_err2.move`†, `functional/intrinsic_iter_role_err3.move`†, `functional/intrinsic_iter_role_err4.move`†, `functional/intrinsic_iter_role_err5.move`†, `functional/intrinsic_iter_role_err6.move`†, `functional/intrinsic_iter_role_err7.move`†, `functional/intrinsic_iter_role_err8.move`†, `functional/intrinsic_map_conv_ghost_err.move`†, `functional/intrinsic_map_enum_err.move`†, `functional/intrinsic_map_field_access_err.move`†, `functional/intrinsic_map_invariant_err.move`†, `functional/intrinsic_map_mut_no_get_err.move`†, `functional/intrinsic_map_native_role_err.move`†, `functional/intrinsic_map_rank_pair_err.move`†, `functional/intrinsic_map_rank_sig_err.move`†, `functional/intrinsic_map_role_sig_err.move`†, `functional/intrinsic_map_spec_pack_err.move`†, `functional/intrinsic_map_update_field_err.move`†, `functional/intrinsic_validity_data_inv_err.move`†, `functional/verify_iterator_validity.move`.

### C8. `update` condition must target a spec variable

9 tests, 9 messages.

Example (`functional/ghost_field.move`):

```text
an `update` condition must target a spec variable
```

Tests: `functional/ghost_field.move`, `functional/ghost_field_bp.move`, `functional/ghost_field_closure_equality.move`, `functional/ghost_field_closure_equality_ext.move`, `functional/ghost_field_equality.move`, `functional/ghost_field_invariants.move`, `functional/ghost_field_map_equality.move`, `functional/ghost_field_map_equality_ext.move`, `functional/loop_memory_havoc.move`.

### C11. Equality operand types differ

1 test, 2 messages.

Example (`functional/verify_custom_table.move`):

```text
LIR-SEMANTIC-TYPE: in fun create_and_insert_fail2: equality primitive operand types differ at [7698, 7734)
```

Tests: `functional/verify_custom_table.move`.

### C12. Call or closure target does not resolve — opaque-inline gap fixed

The adapter previously discarded every retained inline declaration even when
an opaque call or behavioral predicate still referred to it. It now keeps opaque
inline declarations and their comments in both owning modules and interfaces.
Non-opaque declarations already expanded by compiler-v2 remain omitted.

The three positive-gap fixtures now reach verification. `behavioral_predicate_inline_fun`
verifies completely. `opaque_inline_body_fail` rejects the bad `inc` body and its
dependent caller, while the explicitly trusted variant verifies. An authored
companion proves `opaque_inline_loop_sum::sum` and `test_sum_twice` at the unchanged
25k budget (13.196M and 5.093M raw heartbeats). All positive targets in that file
verify; only the deliberately wrong `test_sum_wrong` postcondition fails.
All three scoped ordinary baseline checks pass. The fresh full benchmark has
30 verified problems and one expected rejection; its generated main-relative
report preceded the Move package suite and the 437-file registry audit. Both
pass, and no previously verified target regresses.

The remaining 1 test (also rejected by the Prover, marked †) has 4 messages:
`specification call target … does not resolve to a declared function or specification function`.
It refers to `f2` and `f4` in `functional/restrictions.move`†.

### C16. Implicit freezing of returned references — frontend gap fixed

Compiler-v2 can type a call's result as `(&u64, &mut u64)` while its callee
returns `(&mut u64, &mut u64)`. The frontend now preserves the instantiated
callee result, evaluates it once, destructures it, and explicitly freezes the
components whose context expects shared references. Computed freeze operands
are held in a temporary for the denotation. Other type mismatches remain errors.

`SourceVerify/returned_reference_freeze.move` verifies mixed/shared tuple
results, a single returned reference, and mutation through the retained mutable
reference at 25k; its false postcondition still fails. The follow-on lexical
scope bug in `bp_forwarding` is fixed too: body quantifier domains are
predeclared in their lexical scope, so a nested lambda's local cannot shadow an
outer vector in a later assertion. `SourceVerify/quantifier_shadowing.move`
verifies both positive cases and rejects the false assertion.

Both affected registry files now reach verification. In `bp_forwarding`,
`check_keys_bounded` and `check_all_positive` prove; `set_values` and
`all_values_bounded` still exceed 25k. In `folds_of_wrapper_mut`, `mirror_keys`
and `set_both` still exceed 25k. Their intentional negative cases remain failures.
Scoped logs: `/tmp/returned-freeze-{bp_forwarding,folds_of_wrapper_mut}-registry.log`.
Both cost gates, frontend tests, and new-fixture non-update baseline checks pass.
The full benchmark/main report, all four suites, and registry refresh completed
successfully under `/tmp/returned-freeze-validation.py`.

### C18. Borrow result is not a reference type — resource projection fixed

`functional/state_labels/spec_fun_cross_resource.move` now reaches verification.
Compiler-v2 retained `borrow_global` in a derived specification function after
projecting its result to a resource value. The frontend now treats that logical,
value-typed node as a global read; actual reference-typed borrows are unchanged.
`SourceVerify/projected_global_reads.move` proves four positive targets at 25k,
including a caller using an opaque contract's invocation-defined label without
a callee program point. Its incorrect increment-by-two claim still fails.
Closure-frame widening now normalizes `Weave.compose` when comparing a verified
callee's frame with the wrapper's frame, as the precondition proof already did.
That removes two residual frame obligations in the larger callers. The focused
`closure_frame_widening` fixture proves six positives at 25k, including captures,
and rejects a callback writing an unauthorized captured address; both cost gates
pass. The subsequent field-update and prepared-fact changes below close the
remaining cross-resource obligations.

The scoped registry update and ordinary baseline check now pass with no
diagnostics: all six targets prove at 25k. Field-update normalization and codec
round trips close the two higher-order callers. The final direct-call target
now tries its already available call facts before repeating behavioral contract
derivation; preparation drops from 28.437M to 20.974M raw heartbeats. Failed
speculative attempts restore the original goal and use the existing higher-order
fallback. The 25k acceptance budget is unchanged. A short companion completes
the proof. `SourceVerify/behavior_manual_facts` proves three positives and rejects
the wrong final counter value at both 25k and the suite default. Both cost gates
pass. Scoped evidence: `/tmp/prepared-behavior-registry-{update,check}.log` and
`/tmp/behavior-manual-fixture-{check,default}.log`. Fresh benchmark/main HTML, all four full suites, and the 437-file registry
refresh passed (`/tmp/prepared-behavior-validation.log`). A final literal-closure
guard bounds failed speculation to two attempts per target; scoped checks and
both cost gates pass (`/tmp/behavior-guard-two-*.log`).

`functional/closures/bp_mut_ref_selector.move` remains rejected. Its projected
field borrow also needs the behavioral predicate's implicit pre/post argument
expansion, followed by V20's mutable-reference invocation support. Simply
removing the borrow does not implement those semantics.

### C20. Function-value typing — frontend gap fixed

`functional/closures/amm_example.move` previously stopped with three
`closure construction result is not a function type` diagnostics and one
`invoked operand is not a function type`. Compiler-v2 implicitly packs a
function into a positional single-function-field wrapper and unwraps it for
invocation. The frontend now emits ordinary constructor and field-selection
operations, instantiating the field type with the wrapper's type arguments.
This preserves the wrapper's data-invariant and field-frame checks.

Both printer stages quote numeric field-frame targets (`modifies_of<«0»>`).
`SourceVerify/function_wrappers.move` proves all eight valid targets at 25k and
rejects its invalid implicit pack. The frontend formatting test checks addressed
and wildcard positional frames through two formatting passes.

The scoped registry run now reaches verification. A follow-up combines pure
short-circuit guards whose branches leave identical environments, avoiding
repeated verification of their continuations. `constant_product` now proves at
25k with its arithmetic companion (`/tmp/short-circuit-registry-amm.log`), as
`create_pool` already did. Its closer is 15.1M raw heartbeats and 17 leaves,
versus exhaustion before the manual proof in the earlier 25k run.
The two fee functions, `swap`, and `create_constant_product_pool` still exceed
25k; the other two constructors depend on the unproved fee functions. The
full three-function arithmetic candidate remains only in
`/tmp/registry-amm-arithmetic-candidate.proof.lean`. No budget was raised.

A bounded arithmetic-only omega attempt now precedes the full-context solver
in the cheap bound decider when memory-dependent hypotheses are present. It avoids preprocessing unrelated memory and
closure facts; `ArithmeticContext.lean` checks both the fast path and fallback.
The final regular AMM benchmark retains its six valid targets:
`constant_product` is 16.1M heartbeats (previous 26.8M, down 39.8%). Total
AMM is 2.86G versus 2.87G. An earlier unrestricted fast-path experiment reduced
the non-compliant pricing function by 51%, but grew proof terms on the core
cost gate; the final memory-context guard does not retain that pricing gain.
Both cost gates now pass without baseline changes. `Scalars/Order::ordered3`
introduces both comparison premises after guard fusion.
The fresh full benchmark `/tmp/leaner-benchmark-returned-freeze-final.json`
retains 28/32 passing samples and all previously proved targets. The HTML was
regenerated against main before the full suites. The controller is
`/tmp/short-circuit-final-validation.py`; all four suites and the 437-file
registry refresh passed. The final registry audit found no changed diagnostics.
The first 437-file refresh introduced no failed functions; its diagnostic changes
were shorter aborts_if residuals and bug_15044 failing normally instead of timing
out (`/tmp/short-circuit-registry-audit.log`).

The focused probes also exposed open-generic closure-frame and state-labelled
`result_of`/wildcard-frame proof gaps. The checked regression covers concrete
generic instantiation and `ensures_of` invariants; it does not claim those broader
proof gaps are solved. This scoped follow-up is after the full 437-file counts
above (one pre-verification rejection now reaches verification).

### C21. Field selection not on every variant

1 test, 2 messages.

Example (`functional/uninterpreted_spec_fun_congruence.move`):

```text
LIR-SEMANTIC-TARGET: in fun ghost_sensitive_function_is_not_move_congruent: selected field does not exist on every possible variant at [3413, 3420)
```

Tests: `functional/uninterpreted_spec_fun_congruence.move`.

### C23. Spec variable with an initializer

1 test, 1 message.

Example (`functional/global_vars.move`):

```text
spec variable `sum_of_T2` has an initializer, which is not supported
```

Tests: `functional/global_vars.move`.

### C24. State label neither bound nor defined

A state label a specification reads is bound by a quantifier over the
state domain or defined by a two-state operation whose post-state it is
([`state-labels.md`](state-labels.md)). The Prover reads a free label on a
function of linear control flow at a program point and fails the clause
on non-linear control flow; validation rejects the unit.

1 test (the Prover fails the clause), 1 message.

Example (`functional/state_labels/nonlinear_cfg_error.move`):

```text
LIR-SEMANTIC-STATE-LABEL: in fun branching_with_labels: state label `S` is neither bound by a quantifier over the state domain nor defined by a state-change predicate
```

Tests: `functional/state_labels/nonlinear_cfg_error.move`.

## Errors during verification

### V1. Rendering outside the LeanerLang parser

12 tests, 12 messages (2026-10-05 refresh; generic axioms parse since
2026-10-08).

Messages:

- 6 × `generic module invariants are outside the current LeanerLang parser`
- 2 × `` condition kind `…` is outside the current LeanerLang parser ``
- 2 × `this quantifier kind is outside the current LeanerLang parser`
- 1 × `` in-body specification condition `…` with N properties and N auxiliary expressions is outside the current LeanerLang parser ``
- 1 × `generic update module invariants are outside the current LeanerLang parser`

Example (`functional/emits.move`):

```text
condition kind `LeanerIR.ConditionKind.emits` is outside the current LeanerLang parser
```

Functional `update_field` is supported for ordinary structs and enum fields
present in every variant, including generic and nested updates. The eleven
former render failures now reach proof obligations or a more specific gap.
`aborts_with` is now supported too: the two abort-clause registry files prove
all 12 positive functions and reject all 15 deliberately invalid functions.
Allowed codes remain constrained with partial abort conditions; arithmetic
traps are distinguished from explicit aborts and map to `EXECUTION_FAILURE`.

Partial-variant enum updates and mutable-reference behavioral predicates are
listed under V2; `mono` now reaches its unsupported `emits` clause.

A generic axiom (`axiom<T>`, LeanerLang `axiom {T} e`) is assumed at each
instantiation a verification applies a specification function of it at, as
the Move Prover monomorphizes it: the applications in the function's body and
contract, in its callees' contracts and in the specification functions these
expand, a callee's type parameter read as the call's argument. The axiom's
binders range over the instance types' values (a bounded integer's range
included). A specification function applied at `num` takes it as a type
argument of its own (`SpecTypeArgument.integer`). A module's
`pragma aborts_if_is_strict` (and `_partial`) now holds for its functions. So
`axiom_generic`, `axioms`, `mono_on_axiom_spec_type` and `opaque_native`
verify (`SourceVerify/generic_axioms{,_false}.move`,
`SourceVerify/inherited_strictness{,_false}.move`). Generic module invariants
remain open.

Tests: `functional/choice.move`, `functional/emits.move`, `functional/generic_invariants.move`, `functional/loop_unroll.move`, `functional/mono.move`, `functional/uninst_global_invariant.move`, `regression/generic_aliasing_ghost_main.move`, `regression/generic_aliasing_ghost_pair.move`, `regression/generic_aliasing_ghost_params.move`, `regression/mono_after_global_invariant.move`, `regression/type_param_bug_121721.move`, `regression/write_back_local_type_inst.move`.

### V2. Construct not supported in generated contracts

5 tests, 7 messages (2026-10-05 refresh; `abort_in_fun`, `bv_aborts`,
`defines` and `performance_200511` reach verification since 2026-10-08).

Messages:

- 2 × `a function value with type arguments is not carried in generated contracts`
- 1 × `specification operation LeanerIR.Operation.call (LeanerIR.CallKind.invoke) is not supported in generated contracts`
- 1 × `generated contracts currently expand specification functions with one result`
- 2 × `a field update on an enum whose variants do not all carry the field is not supported yet`
- 1 × `a behavioral predicate over a function with mutable reference parameters is not carried yet`

The state-change predicates `publish`, `remove`, and `update` now translate
in contracts, including labels they define (S2b of
[`state-labels.md`](state-labels.md), 2026-10-05).

An arbitrary value (`__leaner_arbitrary_…`, the value of a specification
function's aborting call) takes no type arguments, so `abort_in_fun` verifies.
A module the specifications read (an abort code's constant function) is
exported with the module closure, so `bv_aborts` reaches verification; its
clause fails, as in the Prover. An equality the compiler keeps at a schema's
declared type (`num`) after the inclusion substitutes `u64` values compares at
the operands' type, so `defines` verifies (`SourceVerify/schema_equality.move`).

A specification function applied at `num` takes it as a type argument
(`SpecTypeArgument.integer`), so `performance_200511` reaches verification;
two of its functions exceed the budget.

Example (`functional/closures/closure_in_spec_expr.move`):

```text
specification operation LeanerIR.Operation.call (LeanerIR.CallKind.invoke) is not supported in generated contracts
```

Tests: `functional/closures/behavioral_soundness.move`, `functional/closures/closure_in_spec_expr.move`, `functional/spec_fun_tuple_errors.move`, `regression/enum_update_out_of_variant.move`, `functional/closures/inline/discarded_mut_ref_result.move`.

### V6. Construct not carried by the denotation

7 tests, 11 messages.

Messages:

- 4 × `` no denotation for the callee `…`: reference operation is not carried by the denotation ``
- 1 × `` no denotation for `…`: a literal of this type is not carried by the denotation ``
- 2 × `` no denotation for `…`: a closure whose rows are not its target's is not carried by the denotation ``
- 4 × `` no denotation for `…`: a type argument without values is not carried by the denotation ``

A closure's rows are not its target's where the target returns a reference
(`closure_refs.move::borrow_a`) or several results; a function type marking
its shared results is the recorded fix ([`higher-order-functions.md`](higher-order-functions.md),
"Closures in memory"). A function type as a type argument
(`Option<|u64|u64>`) has no values of its own (`NTy.inhabitable`): the
values of a function type are the unit's typed closures, and the induced
frame of a generic call defaults each parameter to an inhabitant of its
argument (`Carriers.default_instantiate`).

Example (`functional/type_reflection.move:71`):

```text
no denotation for `test_type_info_symbolic`: a literal of this type is not carried by the denotation
```

Tests: `functional/bitwise_table.move`, `functional/verify_table.move`, `functional/closures/closure_refs.move`, `functional/fun_field_nested_ability_variants.move`, `functional/fun_type_unused_ctor.move`, `functional/type_reflection.move`, `regression/fun_type_arity_injectivity.move`.

### V7. Intrinsic map representation not carried

5 tests, 31 messages.

Example (`functional/ghost_field_intrinsic_map_ops.move:6`):

```text
the intrinsic map role `map_spec_get` belongs to a map whose representation is not carried
```

Tests: `functional/closures/inline/folds_of_map_intrinsic.move`, `functional/ghost_field_intrinsic_map_ops.move`, `functional/intrinsic_map_insertion_order.move`, `functional/intrinsic_map_rank.move`, `functional/intrinsic_map_rank_bulk.move`.

### V10. Other elaboration errors in the rendering

5 tests, 5 messages.

Messages:

- 2 × `` LEANER-CALL-NAME: unknown function `…` ``
- 3 × `LEANER-SPEC-ARITY: behavior predicate expects N value argument(s), got N`

The constant-vector indexing error in `functional/consts.move` is fixed:
its three valid functions verify, and its five invalid functions fail their
clauses. The index head now resolves constants as values, just like locals.

Generic enum invariants now instantiate their `this` type with the enum's
type parameters, as struct invariants already do. `enum_19575` has no
remaining diagnostics. The source fixture `generic_enum_invariants.move`
checks generic constructors/readers and an opaque concrete caller, and rejects
both an invalid generic constructor and a false postcondition.

Vector index domains `range(v)` now share bound translation with explicit
`lower..upper` ranges, in both quantifiers and slices. The active old/labeled
observation context is preserved. `Check/Specifications/VectorRanges` proves
five positive targets at 25k and rejects an existential at the excluded upper
endpoint; `TableSnapshots` also checks a range over logical snapshot values.
`macro_verification::foreach` reaches verification instead of rejecting the
range syntax, but still exceeds 25k. Its intentionally false +2 postcondition
has not become provable. A probe retaining only its valid postconditions also
times out: loop setup consumes about 7.6M raw heartbeats, and normalization of
the unchanged-tail invariant after a mutable element write exhausts the
remaining budget before the authored tactic runs
(`/tmp/vector-ranges-foreach-{positive,debug}.log`). The owning runner refreshed that diagnostic baseline
(`/tmp/vector-ranges-registry-update.log`); no new whole-file success is claimed.

The follow-up clears unused continuation alias chains and normalizes encoded
integer reads to certified values, preserving signed/unsigned bounds even for
computed optional reads. Four `CertifiedReads` positives pass at 25k, and a
false signed nonnegativity claim remains rejected. The same instrumented
positive-only `foreach` diagnostic drops from 48.483M to 36.190M raw heartbeats
(`/tmp/foreach-auto-diagnostic.log`, `/tmp/certified-read-foreach-profile.log`).
Those profiling runs use a diagnostic 100k allowance; the ordinary 25k probe
still times out. This is a preparation-cost improvement, not a fixed fixture.

Tests: `functional/closures/behavioral_results.move`, `functional/closures/inline/folds_of_idx.move`, `functional/closures/inline/folds_of_multi.move`, `functional/closures/result_of_mut_ref_soundness.move`, `functional/state_labels/followed_by_mut_ref.move`.

### V16. Native without specification or prelude model

11 tests, 50 messages.

In `bitwise_table`, `bitwise_table_mixed_instances` and `verify_table` the
natives are those behind `extensions::table`, the test driver's
`move-table-extension` dependency (`Table.spec.move` declares the map and its
roles, and the export carries them). A handle-backed table carries only its
read roles, `map_borrow` and `map_has_key` (`Contract.mapRoleOf?`); its
allocating and mutating roles (`new`, `add`, `remove`, `length`) are disabled
until stage 2 of [`intrinsic-maps.md`](intrinsic-maps.md), so a caller enters
their bodies and meets the natives.

Example (`functional/type_reflection.move:80`):

```text
`test_type_info_ignores_type_param` calls the native `0x2::type_info::type_of`, which has neither a specification nor a prelude model; specify it
```

Tests: `functional/bitwise_table.move`, `functional/bitwise_table_mixed_instances.move`, `functional/closures/inline/bp_invariant_weakening_scope.move`, `functional/data_invariant_in_map.move`, `functional/intrinsic_map_insertion_order.move`, `functional/type_reflection.move`, `functional/type_reflection_ext.move`, `functional/verify_table.move`, `regression/type_reflection_runtime_names.move`, `regression/vector_theory_boogie_array_intern.move`, `regression/vector_theory_smt_seq.move`.

### V18. Recursion through an unspecified function

1 test, 1 message.

Example (`functional/recursive_move_funs_multi_hop.move:13`):

```text
`foo_3` reaches itself through `foo_2`, which is unspecified; a function on a cycle of calls is used through its contract, so specify `foo_2`
```

Tests: `functional/recursive_move_funs_multi_hop.move`.

### V20. Behavioral predicates with mutable-reference parameters

3 tests, 3 messages.

Quantified labels and both state-change and invocation-defined labels are
carried in contracts (S2 and S3 of [`state-labels.md`](state-labels.md)).
Behavioral predicates over functions with `&mut` parameters remain open.

Message:

- 3 × `a behavioral predicate over a function with mutable reference parameters is not carried yet`

Tests: `functional/closures/lambda_spec_global_memory.move`, `functional/state_labels/bp_requires_aborts_labeled_mut.move`, `functional/state_labels/closure_bp_post_sub_pre_only.move`.

The invocation-label increment moves `aborting_result_definition` to its
intended no-abort clause failures. `aborts_if_at_state_label::caller` now
verifies at the native 25k budget (2026-10-07): reuse call observations
before reconstructing runs or re-deriving contracts. The timeout baseline
is removed by the owning runner.

### V23. Storage clause over a resource without a native type

A specification variable (`global x: num`) is a resource whose field has
the unbounded type `num`, which has no native type, so the memory has no
slot for it.

1 test, 1 message.

Example (`functional/bitwise_error_2.move:1`):

```text
a storage clause names a type that is not a resource
```

Tests: `functional/bitwise_error_2.move`.

### V25. Quantified nominal values with function fields need a validity domain

MVP's quantified nominal domain ties each function-valued field to its owning
struct instantiation. Leaner's native carrier is a field row; treating all
rows as valid lets a quantified `G<u64>` invariant constrain a closure taken
from `G<bool>`. These quantifiers are explicitly rejected until the validity
domain is represented. Expanding native-row quantifiers is not a sound parity
fix: it proved the negative `other_instantiation` in a scratch experiment.

1 registry test, 3 messages: `regression/behavior_axiom_target_field.move`.
The valid `same_type_quantified` and `fun_inst` remain unsupported, as does the
negative `other_instantiation`. The direct-field positives `f_lower`, `g_upper`,
`own_field_generic`, and `enum_field` now verify under the existing 25k budget.
`fixed_value`, `g_lower`, and `f_upper` remain rejected.

The former V22 resource certificate error is fixed: open generic entries are
frame templates, not concrete runtime keys. Encoding, unnamed-memory agreement,
and the resource typing certificate share the closed-key lookup; frame
coherence retains the template lookup. All eight intentionally invalid claims
in `generic_aliasing_write_routes` remain rejected. Scoped refresh log:
`/tmp/registry-resource-targets.log`.

`functional/nonlinear_arithm.move::mul5` now has a proof companion: its
five increasing unsigned factors yield zero or a product of at least 120,
so the result is not 72. The deliberately invalid `mul5_incorrect` and other
negative cases remain rejected (`/tmp/registry-nonlinear-target.log`).

The proof companion for `functional/invariants_with_quant.move` now proves
all four quantified postconditions at the existing 25k budget by enumerating
the three valid indices and computing the vector reads. The file has no
remaining diagnostics (`/tmp/registry-quant-vector-target.log`).
All eleven positives in `for_loop_invariants.move` now verify. Its three
triangular-sum proofs use a proved conversion from truncating to Euclidean
division: `i * (i - 1)` is nonnegative for every integer. This prevents costly
absolute-value case splits during preparation, retaining the 25k budget.
Both intentionally incorrect loop invariants remain rejected
(`/tmp/registry-for-loops-final.log`).

`functional/simple_vector_client.move` now verifies completely. Its companion
normalizes literal list spines before branch exploration and supplies the
four membership witnesses for `test_contains`. `test_index_of` and
`option_type` close by simplification at the unchanged 25k budget
(`/tmp/registry-simple-vector-target.log`).

### V26. Stored write frames and global invariant preservation

Explicit struct-field `modifies_of` now reaches verification, including
referent-typed formals for shared-reference arguments. All 42 targets in
`functional/closures/stored_fun_values.move` run within the 25k budget; 33
prove. Seven remaining failures are intentional negative specifications.
The two addressed-frame callers, `use_counter_modifier` and
`use_config_aware_modifier`, now have a kernel-checked companion proof: a
frame can alter Counter, whose invariant is trivial, while preserving the
invariants of all other resources. Two MVP-positive targets remain:

- `use_any_modifier`: a wildcard frame alone does not establish that a call
  preserves the data invariants of every resource it may overwrite. The current
  field contract has no such preservation guarantee. Adding one requires
  checking it at construction, including its recursive relationship with
  stored function fields; assuming it at invocation would be unsound.
- `create_modifier_valid`: it packs `safe_increment`, which writes Counter,
  into a field with the implicit empty write frame. MVP's backend also treats
  undeclared field access as pure (`derive_struct_field_frame_access`), but its
  fixture accepts this construction. Leaner checks the implicit frame at pack
  time. This discrepancy needs a specification/validity decision rather than
  an unchecked proof shortcut.

Scoped evidence: `/tmp/stored-frames-prover-followup.log`. The formerly expensive
`increment_counter` and `use_transformer` use 7.95M and 5.13M closer heartbeats,
respectively, after computing resource-invariant dispatch locally. All four
suites passed. Dependency interfaces still omit field contracts; explicit write
frames are therefore rejected at that boundary rather than silently becoming
empty frames. Owned declarations retain their frames.

### V27. Specification observations after moving a mutable reference

`functional/specs_in_fun_ref.move::simple7` fails its in-body `assert x == y`
after `let a = x; *a = y`. The denotation consumes `x` with `Term.take`;
`assertionConditions`/`overDefined` then requires that emptied slot to be defined.
The residual is `False`, not budget exhaustion. Reproduction at the registry's
25k cap: `/tmp/specs-in-fun-ref-residual.{lean,log}`.

The Move observation needs the current value held by `a`. Substituting the
function-entry value or the final prophecy is incorrect: a later write can
change the final value. A static `x → a` rewrite also needs to handle further
moves, reassignment, reborrowing (including field reborrows), branch joins,
and lexical shadowing. `old(x)` and saved-label observations retain their
own states. The borrow certificate currently records loan-site holders across
the function, not a per-observation current-owner map. A fix must establish
that map or an equivalent sound observation mechanism; choosing any holder
with the same type or prophecy value would be unsound.

## Functions left unproved

| Test | Functions |
|---|---|
| `functional/address_serialization_constant_size.move` | `serialized_addresses_same_len` |
| `functional/bitwise_features.move` | `set` (budget), `disable_feature_flags` (budget) |
| `functional/bug-17117.move` | `get_s_error` (budget), `get_s_no_error` (budget), `test_input_param_as_mut_ref` |
| `functional/bug_15044.move` | `compare_u8_vector` |
| `functional/bug_15880.move` | `test2` |
| `functional/bv_mutual_recursion.move` | `split_nibbles` (not attempted) |
| `functional/closures/amm_example.move` | `constant_product_with_fee` (budget), `constant_product_with_fee_non_compliant` (budget), `swap` (budget), `create_constant_product_pool` (budget), `create_compliant_fee_pool` (not attempted) |
| `functional/closures/behavioral_predicates_examples.move` | `contains_test_not_found` (budget), `contains_opaque_test_not_found` (not attempted), `index` (budget), `index_opaque` (budget), `index_test_found` (budget), `index_opaque_test_found` (not attempted), `reduce_test_ok` (budget), `reduce_opaque_test_ok` |
| `functional/closures/behavioral_target_two_masks.move` | `pending` |
| `functional/closures/inline/bp_forwarding.move` | `set_values` (budget), `all_values_bounded` (budget) |
| `functional/closures/inline/fold_symbolic.move` | `sum` (budget) |
| `functional/closures/inline/folds_of_collect.move` | `fold_is_prefix` (lemma), `collect` (budget) |
| `functional/closures/inline/folds_of_consuming.move` | `sum_literal` (budget), `digits_forward` (budget), `digits_reverse` (budget), `sum_noncopy` (budget) |
| `functional/closures/inline/folds_of_wrapper.move` | `sum_values` (budget), `sum_kv` (budget), `collect_keys` (budget), `sum_values_three_levels` (budget), `sum_values_both` (budget), `sum_values_through_inline` (budget) |
| `functional/closures/inline/folds_of_wrapper_mut.move` | `mirror_keys` (budget), `set_both` (budget) |
| `functional/closures/inline/result_of_attached_state_ok.move` | `map_add_global` (budget), `map_add_global_bare` (budget) |
| `functional/closures/inline/vector_hofs_fold.move` | `count_even_concrete` (budget) |
| `functional/closures/inline/vector_hofs_for_each.move` | `find_value` (budget), `increment_all` (budget), `scale_all` (budget), `increment_all_inferred` (budget), `clamp_all_inferred` (budget), `clamp_all` (budget) |
| `functional/closures/inline/vector_hofs_mut_receiver.move` | `bump_field` (budget), `bump_resource` (budget) |
| `functional/closures/stored_fun_values.move` | `use_any_modifier`, `create_modifier_valid` (V26 semantic gaps) |
| `functional/inline_fun.move` | `test_filter` (budget) |
| `functional/loops_with_memory_ops.move` | `nested_loop1` (budget) |
| `functional/math8.move` | `pow` (budget), `floor_log2` (budget), `sqrt` (not attempted) |
| `functional/math_fixed8.move` | `pow_raw` (budget), `exp_raw` (not attempted), `exp` (not attempted) |
| `functional/mut_ref.move` | `call_return_ref_different_path_vec` (budget), `call_return_ref_different_path_vec2` (budget) |
| `functional/nested_loop_inv.move` | `assert_no_duplicate` (budget) |
| `functional/opaque.move` | `opaque_caller` (not attempted) |
| `functional/serialize_model.move` | `bcs_test1` |
| `functional/specs_in_fun_ref.move` | `simple7`: in-body assertion reads emptied reference slot after move (not a timeout) |
| `functional/state_labels/unmodified_memory_at_label.move` | `swap` (a read after removal; Boogie retains absent resource contents) |
| `functional/verify_vector.move` | `verify_reverse` (budget), `verify_reverse_with_unroll` (budget), `verify_append` (budget), `verify_append_with_unroll`, `verify_index_of_with_unroll`, `verify_contains_with_unroll`, `verify_remove` (budget), `verify_remove_with_unroll` (budget) |
| `regression/behavior_axiom_target_field.move` | `same_type_quantified`, `fun_inst` (unsupported quantified field-validity domain; V25) |
| `regression/generic_aliasing_all_partitions.move` | `true_in_every_case` (budget), `never_alias` (budget) |
| `regression/performance_200511.move` | `fresh_guid` (budget), `new_event_handle_impl` (budget) |
| `regression/vector_theory_boogie_array_intern.move` | `f1` (budget) |
| `regression/vector_theory_smt_seq.move` | `f1` (budget) |

The functions are marked as follows:

- No mark: the automatic verification failed within its budget.
- (budget): the function exceeds the 25 thousand `maxHeartbeats` budget.
  Of the tests that reached verification in the first run of this
  registry, such functions also fail with an unlimited budget; the tests
  admitted by later fixes were not run without one.
- (not attempted): a callee is not verified, so the function is not tried.
- (lemma): a `spec lemma` whose theorem is not established
  ([`lemmas.md`](lemmas.md)).

A function can fail because of a V entry of the same test. The functions
proved by the hand proof beside `fixed_point_arithm` are not listed.

## Outside the unit tests

### O1. `vector` functions with lambdas are unsupported declarations

`move_stdlib_lean_prover_tests` (`aptos-move/framework/tests`) verifies the
framework's `move-stdlib` with `prove --lean`. Since 5354e9abfe exports
inline functions, the export skips `vector`'s functions whose bodies hold a
lambda, and each one is reported as an error at the module:

```text
<FRAMEWORK_DIR>/move-stdlib/sources/vector.move:11:1: error: unsupported Move declaration `for_each`: in function `vector::for_each`: lambda is not supported by XAST (function values are out of scope)
```

The functions are `destroy`, `enumerate_ref`, `for_each`,
`for_each_reverse`, `zip`, `zip_ref`, and `zip_reverse`. The test's
baseline expects a clean run, so the test fails.
