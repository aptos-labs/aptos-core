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

| Outcome | Tests |
|---|---|
| Verify | 109 |
| Verify with the Move compiler's warnings only | 3 |
| Compile errors, the same as the Prover's | 31 |
| Rejected before verification ([C entries](#rejected-before-verification)) | 51 |
| Errors during verification ([V entries](#errors-during-verification), [unproved functions](#functions-left-unproved)) | 220 |

Of the 51 rejected tests, 22 are rejected by the Prover too (bytecode
transformation or condition generation errors). For those, the rejection
itself is expected and only its report is a problem ([G1](#g1-a-rejection-is-an-uncaught-exception)).

The 220 tests with verification errors hold 399 functions and lemmas whose
verification fails. The problems are the V entries and the 120 functions
and lemmas in 43 tests that the Prover proves but Leaner does not, listed under
[functions left unproved](#functions-left-unproved): 111 of the failing
functions and 9 that are not attempted. The other 288 failing functions
are deliberately wrong specifications the Prover fails too, such as the
`_incorrect` functions, or belong to tests the Prover rejects.

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

### C16. Call result differs from the callee signature

2 tests, 6 messages.

Example (`functional/closures/inline/bp_forwarding.move`):

```text
LIR-SEMANTIC-TYPE: in fun set_values: function call result differs from the callee signature at [3981, 4001)
```

Tests: `functional/closures/inline/bp_forwarding.move`, `functional/closures/inline/folds_of_wrapper_mut.move`.

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

### C18. Borrow result is not a reference type

2 tests, 4 messages.

Example (`functional/closures/bp_mut_ref_selector.move`):

```text
LIR-SEMANTIC-TYPE: in fun apply_to_x: borrow result is not a reference type at [924, 932)
```

Tests: `functional/closures/bp_mut_ref_selector.move`, `functional/state_labels/spec_fun_cross_resource.move`.

### C20. Function-value typing

1 test, 4 messages.

Messages:

- 3 × `closure construction result is not a function type`
- 1 × `invoked operand is not a function type`

Example (`functional/closures/amm_example.move`):

```text
LIR-SEMANTIC-TYPE: in fun swap: invoked operand is not a function type at [9270, 9358)
```

Tests: `functional/closures/amm_example.move`.

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

28 tests, 28 messages.

Messages:

- 11 × `` operation `…` is outside the current LeanerLang parser `` (all `update_field`)
- 6 × `generic module invariants are outside the current LeanerLang parser`
- 4 × `generic module axioms are outside the current LeanerLang parser`
- 3 × `` condition kind `…` is outside the current LeanerLang parser ``
- 2 × `this quantifier kind is outside the current LeanerLang parser`
- 1 × `` in-body specification condition `…` with N properties and N auxiliary expressions is outside the current LeanerLang parser ``
- 1 × `generic update module invariants are outside the current LeanerLang parser`

Example (`functional/aborts_if.move:170`):

```text
condition kind `LeanerIR.ConditionKind.abortsWith` is outside the current LeanerLang parser
```

Tests: `functional/aborts_if.move`, `functional/aborts_if_with_code.move`, `functional/axiom_generic.move`, `functional/axioms.move`, `functional/choice.move`, `functional/closures/inline/anchor_aliasing.move`, `functional/closures/inline/anchor_mutation_pre_state.move`, `functional/closures/inline/discarded_mut_ref_result.move`, `functional/closures/inline/folds_of.move`, `functional/closures/inline/folds_of_callee_ensures.move`, `functional/closures/inline/folds_of_ref.move`, `functional/emits.move`, `functional/generic_invariants.move`, `functional/loop_unroll.move`, `functional/mono.move`, `functional/opaque_native.move`, `functional/state_labels/aliasing.move`, `functional/state_labels/bp_with_state_labels.move`, `functional/state_labels/two_state_labels.move`, `functional/uninst_global_invariant.move`, `regression/enum_update_out_of_variant.move`, `regression/generic_aliasing_ghost_main.move`, `regression/generic_aliasing_ghost_pair.move`, `regression/generic_aliasing_ghost_params.move`, `regression/mono_after_global_invariant.move`, `regression/mono_on_axiom_spec_type.move`, `regression/type_param_bug_121721.move`, `regression/write_back_local_type_inst.move`.

### V2. Construct not supported in generated contracts

11 tests, 14 messages.

Messages:

- 3 × `` a type argument of the specification function `…` has no native type (LeanerIR.GenericArgument.typeArg …) ``
- 2 × `a function value with captures or type arguments is not carried in generated contracts`
- 2 × `specification operation … (LeanerIR.SpecOperation.remove …) is not supported in generated contracts`
- 2 × `specification operation … (LeanerIR.SpecOperation.update …) is not supported in generated contracts`
- 1 × `specification operation … (LeanerIR.SpecOperation.publish …) is not supported in generated contracts`
- 1 × `specification function call namespace is out of range`
- 1 × `specification operation LeanerIR.Operation.call (LeanerIR.CallKind.invoke) is not supported in generated contracts`
- 1 × `` primitive `…` has non-inferable generic operation arguments ``
- 1 × `generated contracts currently expand specification functions with one result`

The state-change predicates `publish`, `remove`, and `update` are carried
to LIR since S1 of [`state-labels.md`](state-labels.md); their contracts are
S2.

Example (`functional/abort_in_fun.move:1`):

```text
a type argument of the specification function `0x42::TestAbortInFunction::__leaner_arbitrary_4_62` has no native type (LeanerIR.GenericArgument.typeArg …)
```

Tests: `functional/abort_in_fun.move`, `functional/bv_aborts.move`, `functional/closures/behavioral_soundness.move`, `functional/closures/closure_in_spec_expr.move`, `functional/closures/inline/bp_inline_derive.move`, `functional/defines.move`, `functional/spec_fun_tuple_errors.move`, `functional/state_labels/aborting_result_definition.move`, `functional/state_labels/mixed_callee_postcondition.move`, `functional/state_labels/negated_definition.move`, `regression/performance_200511.move`.

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

8 tests, 8 messages.

Messages:

- 2 × `` LEANER-CALL-NAME: unknown function `…` ``
- 3 × `LEANER-SPEC-ARITY: behavior predicate expects N value argument(s), got N`
- 1 × `LEANER-INTEGER-CONTEXT: an integer literal appears in a non-integer context`
- 1 × `LEANER-ENUM-INVARIANT-GENERIC: generic enum invariants are not supported yet`
- 1 × `a quantifier range must be written as a range`

Example (`functional/consts.move:44`):

```text
LEANER-INTEGER-CONTEXT: an integer literal appears in a non-integer context
```

Tests: `functional/closures/behavioral_results.move`, `functional/closures/inline/folds_of_idx.move`, `functional/closures/inline/folds_of_multi.move`, `functional/closures/result_of_mut_ref_soundness.move`, `functional/consts.move`, `functional/enum_19575.move`, `functional/macro_verification.move`, `functional/state_labels/followed_by_mut_ref.move`.

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

### V20. State labels and behavioral predicate forms not carried yet

5 tests, 5 messages.

Quantified labels, over memory and the values of the `&mut` parameters at
their state, are bound in contracts (S2a and S3a of
[`state-labels.md`](state-labels.md)); a label a state-change predicate or
an invocation defines is not carried yet (S2b), nor a behavioral predicate
over a function with `&mut` parameters.

Messages:

- 3 × `a behavioral predicate over a function with mutable reference parameters is not carried yet`
- 2 × `` state label `…` is not bound in this clause; a label a state-change predicate or an invocation defines is not carried yet ``

Example (`functional/state_labels/aborts_if_at_state_label.move:1`):

```text
state label `S1` is not bound in this clause; a label a state-change predicate or an invocation defines is not carried yet
```

Tests: `functional/closures/lambda_spec_global_memory.move`, `functional/state_labels/aborting_result_definition.move`, `functional/state_labels/aborts_if_at_state_label.move`, `functional/state_labels/bp_requires_aborts_labeled_mut.move`, `functional/state_labels/closure_bp_post_sub_pre_only.move`.

### V22. Resource typing not decided for the unit

A theorem that invokes a function value its proof cannot see needs the
unit's resource types to correspond to their keys' types (`ResourcesTyped`,
[`static-memory.md`](static-memory.md)), decided by evaluation
(`resourcesTypedCheck`, through `NTy.TypedAs`). Enum resources, fields of
generic declarations' instances, and function-typed fields are decided; a
generic resource type over its parameters has no closed type to be typed
at.

1 test, 2 messages.

Example (`regression/generic_aliasing_write_routes.move:7`):

```text
Tactic `decide` failed for proposition
  LeanerIR.Proofs.resourcesTypedCheck «0x42».generic_aliasing_write_routes.unit = true
```

Tests: `regression/generic_aliasing_write_routes.move`.

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

## Functions left unproved

| Test | Functions |
|---|---|
| `functional/address_serialization_constant_size.move` | `serialized_addresses_same_len` |
| `functional/bitwise_features.move` | `contains` (budget), `is_enabled` (not attempted), `set` (budget), `disable_feature_flags` (budget) |
| `functional/bug-17117.move` | `get_s_error` (budget), `get_s_no_error` (budget), `test_input_param_as_mut_ref` |
| `functional/bug_15044.move` | `compare_u8_vector` (budget) |
| `functional/bug_15880.move` | `test2` |
| `functional/bv_mutual_recursion.move` | `split_nibbles` (not attempted) |
| `functional/closures/behavioral_predicates_examples.move` | `contains_test_not_found` (budget), `contains_opaque_test_not_found` (not attempted), `index` (budget), `index_opaque` (budget), `index_test_found` (budget), `index_opaque_test_found` (not attempted), `reduce_test_ok` (budget), `reduce_opaque_test_ok` |
| `functional/closures/behavioral_target_two_masks.move` | `pending` |
| `functional/closures/inline/bp_pure_callee.move` | `count_all` (budget), `remove_all_found` (budget) |
| `functional/closures/inline/fold_symbolic.move` | `sum` (budget) |
| `functional/closures/inline/folds_of_collect.move` | `fold_is_prefix` (lemma), `collect` (budget) |
| `functional/closures/inline/folds_of_consuming.move` | `sum_literal` (budget), `digits_forward` (budget), `digits_reverse` (budget), `sum_noncopy` (budget) |
| `functional/closures/inline/folds_of_wrapper.move` | `sum_values` (budget), `sum_kv` (budget), `collect_keys` (budget), `sum_values_three_levels` (budget), `sum_values_both` (budget), `sum_values_through_inline` (budget) |
| `functional/closures/inline/result_of_attached_state_ok.move` | `map_add_global` (budget), `map_add_global_bare` (budget) |
| `functional/closures/inline/specialize_generic_caller.move` | `use_concrete` |
| `functional/closures/inline/vector_hofs_fold.move` | `sum_concrete` (budget), `sum_inferred` (budget), `sum_scaled` (budget), `product_concrete` (budget), `count_even_concrete` (budget) |
| `functional/closures/inline/vector_hofs_for_each.move` | `find_value` (budget), `increment_all` (budget), `scale_all` (budget), `increment_all_inferred` (budget), `clamp_all_inferred` (budget), `clamp_all` (budget) |
| `functional/closures/inline/vector_hofs_mut_receiver.move` | `bump_field` (budget), `bump_resource` (budget) |
| `functional/closures/stored_fun_values.move` | `use_transformer` (budget), `use_monotone`, `use_reader`, `increment_counter` (budget), `safe_increment` (budget), `use_modifier`, `use_safe_op`, `use_counter_reader`, `use_counter_modifier`, `config_aware_increment` (budget), `use_config_aware_modifier`, `use_any_reader`, `use_any_modifier`, `use_action` (budget), `create_modifier_valid` (not attempted), `create_counter_modifier` (not attempted), `create_config_aware_modifier` (not attempted), `create_any_modifier` (not attempted), `create_any_modifier_multi` (not attempted) |
| `functional/for_loop_invariants.move` | `sum_range`, `sum_from` (budget), `sum_skip_first` (budget) |
| `functional/inline_fun.move` | `test_filter` (budget) |
| `functional/invariants_with_quant.move` | `vector_of_proper_positives` (budget) |
| `functional/loops_with_memory_ops.move` | `nested_loop1` (budget) |
| `functional/math8.move` | `pow` (budget), `floor_log2` (budget), `sqrt` (not attempted) |
| `functional/math_fixed8.move` | `pow_raw` (budget), `exp_raw` (not attempted), `exp` (not attempted) |
| `functional/mut_ref.move` | `call_return_ref_different_path_vec` (budget), `call_return_ref_different_path_vec2` (budget) |
| `functional/nested_loop_inv.move` | `assert_no_duplicate` (budget) |
| `functional/nonlinear_arithm.move` | `mul5` |
| `functional/opaque.move` | `opaque_caller` (not attempted) |
| `functional/serialize_model.move` | `bcs_test1` |
| `functional/simple_vector_client.move` | `test_contains` (budget), `test_index_of` (budget), `option_type` (budget) |
| `functional/specs_in_fun_ref.move` | `simple7` |
| `functional/state_labels/intermediate_states.move` | `test_config_preserved` (budget) |
| `functional/state_labels/mixed_callee_postcondition.move` | `caller` (not attempted), `create_then_call` (not attempted) |
| `functional/state_labels/spec_fun_old_param_labeled_with_memory.move` | `inc_under_cap_twice` (budget) |
| `functional/verify_vector.move` | `verify_reverse` (budget), `verify_reverse_with_unroll` (budget), `verify_append` (budget), `verify_append_with_unroll`, `verify_index_of` (budget), `verify_index_of_with_unroll` (budget), `verify_contains_with_unroll`, `verify_remove` (budget), `verify_remove_with_unroll` (budget), `verify_swap_remove` (budget), `verify_model_swap_remove` (budget) |
| `regression/behavior_axiom_quantifier_domain.move` | `whole_type` |
| `regression/behavior_axiom_shared_result.move` | `distinct_results` |
| `regression/behavior_axiom_target_field.move` | `f_lower` (budget), `g_upper` (budget), `own_field_generic`, `same_type_quantified`, `enum_field`, `fun_inst` |
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
