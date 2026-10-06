# Move Prover tests under Leaner: problem registry

Status: 2026-10-02, with the Move Prover's `lean` test feature. C1, C3,
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
414 files) run with the Leaner verifier through the test feature `lean`
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

The full 2026-10-06 run discovers 437 files. All four Leaner packages build
and pass their full suites, including the 122 Check fixtures, both cost
gates, source verification, MonoVM, and differential tests. Logs:
`/tmp/registry-prepared-behavior-<package>-{build,test}.log`.

The owning Move Prover test runner refreshed all 437 baselines successfully
(`/tmp/behavior-guard-prover-refresh.log`). The final guard’s diagnostic audit
shows no changes from its scoped-validated snapshot
(`/tmp/behavior-guard-registry-audit.log`). `bp_pure_callee::count_all` now proves
at 25k with its fold-equation companion; `remove_all_found` still times out.
Returned-reference freezing
and quantifier lexical scope now let both C16 files reach verification; two
previously blocked `bp_forwarding` targets prove. The generic-caller companion
closes `specialize_generic_caller::use_concrete` at 25k, leaving only its existing
compiler warning. The latest full-run diagnostics are:

| Outcome | Files |
|---|---:|
| No diagnostics | 119 |
| Warnings/notes only | 4 |
| Rejected before verification | 50 |
| Verification diagnostics | 232 |
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

The regular benchmark was regenerated before these suites and its HTML
generated against main CI run 37250691414 (`ea4ecc43e7`). The fresh full run
`/tmp/leaner-benchmark-update-normalization.json` retains 28/32 passing samples,
all six valid benchmark AMM targets, and no newly failed targets. Framework
ordered_map is 29/29 at 4.146G heartbeats; calculator is 8/8 at 1.072G.
Benchmark constant_product is 16.1M versus 26.8M before guard fusion (-39.8%).
Total AMM is 2.860G versus 2.870G; the deliberately invalid constructor retains
its timeout. Sample-status counts alone can hide target regressions.

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

### G1. A rejection is an uncaught exception

Every test of the C entries ends the verifier with `uncaught exception:
<file>: <code>: … at [start, end)`. The location is a byte range of the
export, not a line and column in the Move source. The verifier lists the
validation errors of the file and stops: no function of the file is
verified, including those without an error.

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

Tests: `functional/disable_inv.move`.

### G10. `pragma unroll` is bounded checking

With `pragma unroll = N`, the Move Prover unrolls a loop N times and cuts
the paths that iterate more often, so it checks the function only for runs
within N iterations. Leaner does not mirror this (decided 2026-10-02): a
theorem states full correctness, and such a function verifies only by a
loop invariant.

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

### C6. Vector operation typing

1 test, 1 message.

Example (`functional/bv_signed_generic.move`):

```text
LIR-SEMANTIC-TYPE: in spec fun roundtrip: int-to-bit-vector operand is not logical num at [4093, 4110)
```

Tests: `functional/bv_signed_generic.move`.

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

### C12. Call or closure target does not resolve

4 tests (1 also rejected by the Prover, marked †), 15 messages.

Messages:

- 9 × `` call target `…` does not resolve to a declared function ``
- 4 × `` specification call target `…` does not resolve to a declared function or specification function ``
- 2 × `` closure target `…` does not resolve to a declared function ``

Example (`functional/behavioral_predicate_inline_fun.move`):

```text
LIR-SEMANTIC-TARGET: call target `increment` does not resolve to a declared function at [147, 159)
```

Tests: `functional/behavioral_predicate_inline_fun.move`, `functional/closures/inline/opaque_inline_body_fail.move`, `functional/closures/inline/opaque_inline_loop_sum.move`, `functional/restrictions.move`†.

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

### C17. Bit-vector conversion typing

2 tests, 4 messages.

Messages:

- 2 × `bit-vector-to-int result is not logical num`
- 1 × `int-to-bit-vector result is not a fixed-width integer`
- 1 × `bit-vector-to-int operand is not a fixed-width integer`

Example (`functional/bv_internal.move`):

```text
LIR-SEMANTIC-TYPE: in fun bv2int_boundary: bit-vector-to-int result is not logical num at [5660, 5674)
```

Tests: `functional/bv_internal.move`, `functional/bv_signed_generic.move`.

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

16 tests, 16 messages (2026-10-05 refresh).

Messages:

- 6 × `generic module invariants are outside the current LeanerLang parser`
- 4 × `generic module axioms are outside the current LeanerLang parser`
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

Tests: `functional/axiom_generic.move`, `functional/axioms.move`, `functional/choice.move`, `functional/emits.move`, `functional/generic_invariants.move`, `functional/loop_unroll.move`, `functional/mono.move`, `functional/opaque_native.move`, `functional/uninst_global_invariant.move`, `regression/generic_aliasing_ghost_main.move`, `regression/generic_aliasing_ghost_pair.move`, `regression/generic_aliasing_ghost_params.move`, `regression/mono_after_global_invariant.move`, `regression/mono_on_axiom_spec_type.move`, `regression/type_param_bug_121721.move`, `regression/write_back_local_type_inst.move`.

### V2. Construct not supported in generated contracts

9 tests, 12 messages (2026-10-05 refresh).

Messages:

- 3 × `` a type argument of the specification function `…` has no native type (LeanerIR.GenericArgument.typeArg …) ``
- 2 × `a function value with type arguments is not carried in generated contracts`
- 1 × `specification function call namespace is out of range`
- 1 × `specification operation LeanerIR.Operation.call (LeanerIR.CallKind.invoke) is not supported in generated contracts`
- 1 × `` primitive `…` has non-inferable generic operation arguments ``
- 1 × `generated contracts currently expand specification functions with one result`
- 2 × `a field update on an enum whose variants do not all carry the field is not supported yet`
- 1 × `a behavioral predicate over a function with mutable reference parameters is not carried yet`

The state-change predicates `publish`, `remove`, and `update` now translate
in contracts, including labels they define (S2b of
[`state-labels.md`](state-labels.md), 2026-10-05).

Example (`functional/abort_in_fun.move:1`):

```text
a type argument of the specification function `0x42::TestAbortInFunction::__leaner_arbitrary_4_62` has no native type (LeanerIR.GenericArgument.typeArg …)
```

Tests: `functional/abort_in_fun.move`, `functional/bv_aborts.move`, `functional/closures/behavioral_soundness.move`, `functional/closures/closure_in_spec_expr.move`, `functional/defines.move`, `functional/spec_fun_tuple_errors.move`, `regression/performance_200511.move`, `regression/enum_update_out_of_variant.move`, `functional/closures/inline/discarded_mut_ref_result.move`.

### V6. Construct not carried by the denotation

5 tests, 7 messages.

Messages:

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

Tests: `functional/closures/closure_refs.move`, `functional/fun_field_nested_ability_variants.move`, `functional/fun_type_unused_ctor.move`, `functional/type_reflection.move`, `regression/fun_type_arity_injectivity.move`.

### V7. Intrinsic map representation not carried

12 tests, 51 messages.

Example (`functional/ghost_field_intrinsic_map_ops.move:6`):

```text
the intrinsic map role `map_spec_get` belongs to a map whose representation is not carried
```

Tests: `functional/bitwise_table.move`, `functional/bitwise_table_mixed_instances.move`, `functional/closures/inline/folds_of_map_intrinsic.move`, `functional/ghost_field_intrinsic_map_ops.move`, `functional/intrinsic_map_rank.move`, `functional/intrinsic_map_rank_bulk.move`, `functional/table_contais_to_length.move`, `functional/table_option.move`, `functional/verify_table.move`, `regression/map_equality_encoding.move`, `regression/vector_theory_boogie_array_intern.move`, `regression/vector_theory_smt_seq.move`.

### V10. Other elaboration errors in the rendering

6 tests, 6 messages.

Messages:

- 2 × `` LEANER-CALL-NAME: unknown function `…` ``
- 3 × `LEANER-SPEC-ARITY: behavior predicate expects N value argument(s), got N`
- 1 × `a quantifier range must be written as a range`

The constant-vector indexing error in `functional/consts.move` is fixed:
its three valid functions verify, and its five invalid functions fail their
clauses. The index head now resolves constants as values, just like locals.

Generic enum invariants now instantiate their `this` type with the enum's
type parameters, as struct invariants already do. `enum_19575` has no
remaining diagnostics. The source fixture `generic_enum_invariants.move`
checks generic constructors/readers and an opaque concrete caller, and rejects
both an invalid generic constructor and a false postcondition.

Tests: `functional/closures/behavioral_results.move`, `functional/closures/inline/folds_of_idx.move`, `functional/closures/inline/folds_of_multi.move`, `functional/closures/result_of_mut_ref_soundness.move`, `functional/macro_verification.move`, `functional/state_labels/followed_by_mut_ref.move`.

### V16. Native without specification or prelude model

7 tests, 35 messages.

Example (`functional/type_reflection.move:80`):

```text
`test_type_info_ignores_type_param` calls the native `0x2::type_info::type_of`, which has neither a specification nor a prelude model; specify it
```

Tests: `functional/bitwise_table.move`, `functional/closures/inline/bp_invariant_weakening_scope.move`, `functional/data_invariant_in_map.move`, `functional/type_reflection.move`, `functional/type_reflection_ext.move`, `functional/verify_table.move`, `regression/type_reflection_runtime_names.move`.

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
reaches verification but exceeds the runner's 25k heartbeat budget.

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
| `functional/bitwise_features.move` | `contains` (budget), `is_enabled` (not attempted), `set` (budget), `disable_feature_flags` (budget) |
| `functional/bug-17117.move` | `get_s_error` (budget), `get_s_no_error` (budget), `test_input_param_as_mut_ref` |
| `functional/bug_15044.move` | `compare_u8_vector` |
| `functional/bug_15880.move` | `test2` |
| `functional/bv_mutual_recursion.move` | `split_nibbles` (not attempted) |
| `functional/closures/amm_example.move` | `constant_product_with_fee` (budget), `constant_product_with_fee_non_compliant` (budget), `swap` (budget), `create_constant_product_pool` (budget), `create_compliant_fee_pool` (not attempted) |
| `functional/closures/behavioral_predicates_examples.move` | `contains_test_not_found` (budget), `contains_opaque_test_not_found` (not attempted), `index` (budget), `index_opaque` (budget), `index_test_found` (budget), `index_opaque_test_found` (not attempted), `reduce_test_ok` (budget), `reduce_opaque_test_ok` |
| `functional/closures/behavioral_target_two_masks.move` | `pending` |
| `functional/closures/inline/bp_forwarding.move` | `set_values` (budget), `all_values_bounded` (budget) |
| `functional/closures/inline/bp_pure_callee.move` | `remove_all_found` (budget; `count_all` now proves with a companion at 25k) |
| `functional/closures/inline/fold_symbolic.move` | `sum` (budget) |
| `functional/closures/inline/folds_of_collect.move` | `fold_is_prefix` (lemma), `collect` (budget) |
| `functional/closures/inline/folds_of_consuming.move` | `sum_literal` (budget), `digits_forward` (budget), `digits_reverse` (budget), `sum_noncopy` (budget) |
| `functional/closures/inline/folds_of_wrapper.move` | `sum_values` (budget), `sum_kv` (budget), `collect_keys` (budget), `sum_values_three_levels` (budget), `sum_values_both` (budget), `sum_values_through_inline` (budget) |
| `functional/closures/inline/folds_of_wrapper_mut.move` | `mirror_keys` (budget), `set_both` (budget) |
| `functional/closures/inline/result_of_attached_state_ok.move` | `map_add_global` (budget), `map_add_global_bare` (budget) |
| `functional/closures/inline/vector_hofs_fold.move` | `sum_concrete` (budget), `sum_inferred` (budget), `sum_scaled` (budget), `product_concrete` (budget), `count_even_concrete` (budget) |
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
| `functional/state_labels/aborts_if_at_state_label.move` | `caller` (budget; invocation labels now translate) |
| `functional/state_labels/intermediate_states.move` | `test_config_preserved` (budget) |
| `functional/state_labels/unmodified_memory_at_label.move` | `swap` (a read after removal; Boogie retains absent resource contents) |
| `functional/state_labels/spec_fun_old_param_labeled_with_memory.move` | `inc_under_cap_twice` (budget) |
| `functional/verify_vector.move` | `verify_reverse` (budget), `verify_reverse_with_unroll` (budget), `verify_append` (budget), `verify_append_with_unroll`, `verify_index_of` (budget), `verify_index_of_with_unroll` (budget), `verify_contains_with_unroll`, `verify_remove` (budget), `verify_remove_with_unroll` (budget), `verify_swap_remove` (budget), `verify_model_swap_remove` (budget) |
| `regression/behavior_axiom_target_field.move` | `same_type_quantified`, `fun_inst` (unsupported quantified field-validity domain; V25) |
| `regression/generic_aliasing_all_partitions.move` | `true_in_every_case` (budget), `never_alias` (budget) |
| `regression/moved_local_with_refs.move` | `moved_local_in_loop` (budget) |
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
