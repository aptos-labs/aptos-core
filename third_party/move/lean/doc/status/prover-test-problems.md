# Move Prover tests under Verex: problem registry

The Move Prover's unit tests under `third_party/move/move-prover/tests/sources`
run with the Verex verifier through the `lean` test feature
([`source-verification.md`](../designs/source-verification.md), "From the Move Prover").
This registry records unsupported constructs, proof gaps and diagnostic
differences from the `.lean_exp` baselines beside the tests. Entries describe
symptoms and identify causes where known.

A baseline match is not a proof-success count: the suite includes intended
negative specifications and unsupported cases. Use the fixture output for
individual verdicts and the test runner's summary for the number of tests run.

## Updating the registry

After a fix, run from `third_party/move/move-prover`:

```bash
MVP_TEST_FEATURE=lean UPBL=1 cargo test -p move-prover --test testsuite
```

Review the `.lean_exp` diffs. Remove the tests that no longer show an
entry's message, and the entry once no test does. Identifiers stay stable:
a new problem takes the next free number of its section. Test paths are
relative to `tests/sources`.

## General

### G0. How the test driver runs a test

A test file is verified alone, with the driver's dependencies: the Move
standard library and its nursery, and `move-table-extension`
(`std = 0x1`, `extensions = 0x2`), or the Aptos standard library under
`// use-aptos-stdlib`; `// flag:` lines add options (`testsuite.rs`,
`get_flags_and_baseline`). The Verex path (`lib.rs`,
`run_move_prover_lean`) builds the model with them and exports the target
file's modules with what they read. Of the flags, it honors the extra
`--dependency` files (`script`, `script_incorrect`, `exists_only_memory*`)
and `--verify-only`: a target function the scope leaves out
is exported with `pragma verify = false`, one it names with
`pragma verify = true` (`verex.rs`, `apply_scope`), as the Prover's
`should_verify` selects them (`verify_only_list` now matches the Prover). The
Boogie options (`--vector-theory`, `--split-vcs-by-assert`, `--timeout`,
`--trace`) have no counterpart; `--check-inconsistency` is G11.

Module lookup handles quoted names such as a script's `«<SELF>_0»` and
`std::string`; otherwise it could silently verify no targets.
`SourceVerify/script_false.move` checks that an incorrect script is rejected.

### G1. Import diagnostics — returned as reports; precise locations still open

`SourceVerify.verifySource` now returns VIR import/validation errors as error
reports rather than throwing an uncaught exception. This lets
the benchmark retain the failure and its measured frontend work, and lets
the source baseline suite continue to subsequent fixtures. The report is
anchored at the input's first line; embedded byte ranges still need mapping
back to precise source locations. Exporter/IO exceptions remain separate.
An invalid unit still prevents verification of the whole file or package,
including functions without an error; this change does not add recovery.

### G2. Baselines carry residual goal states

7 baselines record the full goal state of a residual obligation. The
largest, `functional/type_dependent_code.lean_exp`, has 638 lines. Such
baselines change with any change to the solver's
normalization.

### G3. A hypothesis can hide a parameter in an obligation

In `functional/fixed_point_arithm.move::mul_div`, the obligation has a
hypothesis named `x`. It hides the parameter `x`, which becomes the
inaccessible `x✝⁴`, so a hand proof cannot name it; authored proofs work
around this with `‹…›` patterns. (`mul_div` itself verifies automatically.)

### G4. A proof file needs a single-module Move file

`foo.proof.lean` accompanies only a Move file that declares one module.
34 of the test files declare several modules, so their functions cannot
get a hand proof.

### G8. Inconsistent invariant pragmas are resolved, not rejected

The Move Prover rejects two uses of the invariant pragmas as inconsistent:
delegating from a public function, and `disable_invariants_in_body` on a
function whose invariants a caller already carries. Verex resolves both
soundly: a public function carries its invariants itself, and a function
whose caller carries them carries none.

Example (`functional/disable_inv.move`): the Prover rejects the module;
Verex reports `f1_incorrect`, a public function delegating its invariants,
at its first write, and verifies the other functions.

The Prover also rejects a public function called, at any depth, from a
function with `disable_invariants_in_body` (`functional/disable_inv_indirect`):
the public function assumes the invariants at entry, which the disabled body
need not keep. Verex verifies the module; a call in such a body does not
give the callee's postcondition where the invariants are broken (a function
breaking an invariant, calling a public reader whose postcondition holds by
the invariant, and restoring it before its exit, fails its own
postcondition).

Tests: `functional/disable_inv.move`, `functional/disable_inv_indirect.move`.

### G10. `pragma unroll` is bounded checking

With `pragma unroll = N`, the Move Prover unrolls a loop N times and cuts
the paths that iterate more often, so it checks the function only for runs
within N iterations. Verex does not mirror the cut: a
theorem states full correctness. It reads the pragma as a sound proof rule
instead: up to N iterations and the final condition check are exposed, and a
path that would iterate further must be unreachable. A function whose loop
can run more often verifies only by a loop invariant. `math8::floor_log2`
(`unroll = 2`) always runs three iterations, so the Prover's check of it is
vacuous; Verex does not accept it.

Example (`functional/verify_vector.move`, `verify_contains_with_unroll`):
the Prover verifies the assertion after the loop for vectors of at most
three elements; Verex exhausts its 25k budget before the loop's exhausted
bound is reported, since each iteration's return path is taken first.

Tests: `functional/loop_unroll.move`, `functional/verify_vector.move`
(`verify_append_with_unroll`, `verify_contains_with_unroll`,
`verify_index_of_with_unroll`, `verify_remove_with_unroll`,
`verify_reverse_with_unroll`).

### G11. Inconsistent assumptions are not reported

With `--check-inconsistency`, the Move Prover reports a function whose
verification holds only because something it assumes is inconsistent.
Verex states a function's in-body assumptions as a hypothesis of its
theorem ([`denotation.md`](../designs/denotation.md), "Assumption ledger") and
does not check that the hypothesis can hold.

Example (`functional/inconsistency.move`, `assume_false`): `spec { assume
false; }` before a call that returns lets `ensures false` verify; the Prover
reports `there is an inconsistent assumption in the function`, and Verex
verifies the theorem, whose hypothesis holds only of a function no run of
which returns or aborts.

Tests: `functional/inconsistency.move` (`assume_false`).

### G12. A mutably borrowed place reads as its final value

While a `&mut` loan of a place lives, a specification reading the place
sees the value the loan ends with, its prophecy; the Move Prover sees the
value before the write-back. Code cannot observe the place during the
loan. The difference is inherent to the prophetic model.

Example (`functional/bug-17117.move`, `test_input_param_as_mut_ref`): after
`let y = &mut r.x`, an assertion of `r.x` before the write through `y` reads
the written value.

Tests: `functional/bug-17117.move` (`test_input_param_as_mut_ref`).

### G13. A destructuring that does not fit aborts with the match code

A value no arm of a match fits, one a destructuring `let` or assignment
does not fit, and an access to a field its variant lacks abort with Move's
incomplete-match code `0xCA26CBD9BE0B0001`. The Move
VM aborts a match, and a selection of a field several variants share, with
that code too, but a destructuring and a field of one variant with the
execution failure `STRUCT_VARIANT_MISMATCH`, which the Prover reports as an
execution failure; the two differ only for `aborts_with`.

### G14. The Prover's bit-vector representation is not modeled

`pragma bv`, `bv_ret`, `bv_internal` and the classification that bitwise
operations induce select the Move Prover's SMT bit-vector encoding. Under it the
Prover wraps specification arithmetic over encoded values at their width, and a
narrowing specification cast yields an unspecified value of the target type.
Verex treats the encoding as a backend choice:
specification arithmetic stays mathematical, bitwise operations are exact on
integers, and executable arithmetic is checked as always. The explicit
conversions are modeled: `int2bv(e)` wraps `e` into its fixed-width result type,
two's complement for a signed one, and `bv2int` reads the value back. (The
Prover renders signed values as integers, so its signed `int2bv` does not wrap.)

The two verifiers differ only where a specification's truth depends on that
wrapping. With `pragma bv = b"0"` and `x == 255`, `ensures x + 1 > 255` verifies
in Verex and fails in the Prover; `ensures int2bv(x) + int2bv(1u8) == 0u8` is
the reverse. `SourceVerify/bv_encoding{,_false}.move` pin both directions;
`bv_conversion{,_false}.move` pin the conversions, on which both agree.

The former representation-guard files now reach verification; ten verify
cleanly. Of those, `functional/bv_cast.move` differs by the narrowing-cast rule
(the Prover rejects `(v as u8) == (v as u8)`), and the Prover's errors in
`functional/bv_internal_invalid.move` and `functional/bv_internal_aggregate.move`
concern its encoding only. An `int2bv` the compiler types at `num` (e.g.
`int2bv(value + 1)`) wraps at `u64`, so
`MoveToVerexSL/constants.move` translates again. Open: one at a type
parameter has no width and is rejected (`functional/bv_signed_generic.move`).

An `int2bv` of a literal, also seen through specification casts, is the
wrapped literal, so `functional/bitwise_operators.move` decreases its measure
and verifies. `functional/bv_internal.move` verifies now that its axiom over
values is assumed (V2 below) and printed as an axiom. In
`functional/bitwise_features.move`, `contains` and `is_enabled` verify: the
solver states that a conjunction a leaf mentions with its operands in both
orders is one value, and reads a specification's truncating remainder of a
nonnegative shift as the runtime's remainder (`SourceVerify/module_axioms{,_false}.move`). `set` and
`disable_feature_flags` still exceed the budget: clearing a bit of a vector
element (`m & (v & (255 ^ m)) = 0`) needs bit-level reasoning through the
write.

Tests: `functional/bv_cast.move`, `functional/bv_internal_aggregate.move`, `functional/bv_internal_invalid.move`, `functional/bv_signed_generic.move`.

### G15. Specification functions are partial

A Move specification function is defined where each parameter declared with a
fixed-width integer type holds a value of that type; elsewhere its value is
unspecified. Specification typing lets a caller pass any
integer to such a parameter, and a specification function cannot abort. The
importer keeps the declared parameter types, and the body reads such a
parameter as an `Int`. The verifier derives the domain from the types: a
recursive definition unfolds inside it, an expansion is guarded by it (an
argument whose own form bounds it, such as a certified integer of a fitting
type, a literal or a vector length, needs no guard), and outside it both read
one uninterpreted value of the function at the arguments (`f.spec.outside`). The solver decides a domain guard from the bounds the
arguments' types carry, and keeps an unfolding only where it also decides the
body's own first condition, as for a function without a domain. A `num`
parameter is unbounded. The specification version of a Move function is
derived again from the function and is not guarded.

The Prover's specification functions are total over the integers: for
`spec fun successor(x: u8): num { x + 1 }`, `successor(300) == 301` verifies
there and not in Verex. Inside the domain the two agree.

Tests: `SourceVerify/spec_fun_domain{,_false}.move`.

### G16. Verified where the Prover's encoding or solver gives up

Some tests pin a failure of the Prover that is not a property of the
program: a limit of its encoding, of its specification inference, or of the
solver. Verex proves these functions, which is the intended outcome: its
behavioral predicates are defined from a function's meaning, not from an
inferred contract, and its theorems do not depend on solver heuristics.

- `proof/weight_too_large`: a `[weight = 1000]` axiom is never instantiated
  by Z3; `id_num(0) == 0` unfolds in Verex.
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

The Prover rejects some specifications Verex reads without harm:
`old(..)` of an expression that does not depend on state
(`functional/old_param_err`; Verex reads it as the expression), and a
function accessing a resource its `reads` clause does not cover
(`functional/reads_check`). A function's `reads` clause is not used by
Verex: a caller takes the callee to read all memory, so the unchecked
clause cannot be relied on.

## Rejected before verification

### C2. Map intrinsic declarations rejected

26 tests (21 also rejected by the Prover, marked †), 172 messages.

Messages:

- 114 × `` Move map intrinsic is missing required role `…` ``
- 29 × `` Move map intrinsic role `…` target has an incompatible physical signature ``
- 29 × `` Move map intrinsic role `…` requires `…` ``

Example (`functional/ghost_field_intrinsic_map_data_inv.move`):

```text
VIR-MOVE-INTRINSIC-ROLE-REQUIRED: Move map intrinsic is missing required role `map_spec_del`
```

Tests: `functional/ghost_field_intrinsic_map_data_inv.move`, `functional/ghost_field_intrinsic_map_full.move`, `functional/ghost_field_iter_abort_native.move`, `functional/intrinsic_iter_abort_role.move`, `functional/intrinsic_iter_abort_sig_err.move`†, `functional/intrinsic_iter_abort_uninterp.move`, `functional/intrinsic_iter_role_err.move`†, `functional/intrinsic_iter_role_err2.move`†, `functional/intrinsic_iter_role_err3.move`†, `functional/intrinsic_iter_role_err4.move`†, `functional/intrinsic_iter_role_err5.move`†, `functional/intrinsic_iter_role_err6.move`†, `functional/intrinsic_iter_role_err7.move`†, `functional/intrinsic_iter_role_err8.move`†, `functional/intrinsic_map_conv_ghost_err.move`†, `functional/intrinsic_map_enum_err.move`†, `functional/intrinsic_map_field_access_err.move`†, `functional/intrinsic_map_invariant_err.move`†, `functional/intrinsic_map_mut_no_get_err.move`†, `functional/intrinsic_map_native_role_err.move`†, `functional/intrinsic_map_rank_pair_err.move`†, `functional/intrinsic_map_rank_sig_err.move`†, `functional/intrinsic_map_role_sig_err.move`†, `functional/intrinsic_map_spec_pack_err.move`†, `functional/intrinsic_map_update_field_err.move`†, `functional/intrinsic_validity_data_inv_err.move`†.

### C8. `update` condition must target a spec variable — carried

Ghost fields and spec variables are carried through the unit's model
(`ghost-fields.md`); the message no longer occurs. Where the
tests stand:

- `functional/ghost_field.move`, `functional/ghost_field_equality.move`,
  `functional/ghost_field_invariants.move`,
  `functional/ghost_field_closure_equality.move`: the Prover's results, but
  for functions without a specification (`copy_num_to_u8_unbounded`,
  `ovf_*`: Verex verifies specified functions).
- `functional/ghost_field_bp.move`,
  `functional/ghost_field_closure_equality_ext.move`: the Prover's results
  (G4b: closure targets routed in the model, behavioral predicates and
  stored-function frames over the model's invocation meaning).
- `functional/loop_memory_havoc.move`: the Prover's failures, besides
  Verex's loop frames, which keep only what an unannotated loop's body
  leaves alone (`untouched_memory_framed`, `ghost_untouched_framed`,
  `fv_field_untouched_framed`), its residual loop goals
  (`bump_loop_with_invariant`, `bump_loop_with_old_invariant`), and
  `fv_forward_depth2_fails`, reported at its loop invariant rather than its
  postcondition, as without ghosts.
- `functional/ghost_field_map_equality.move`,
  `functional/ghost_field_map_equality_ext.move`: C2.
- `functional/ghost_field_intrinsic_map_ops.move`: the Prover's results
  (G5: the role contracts state the validity slot), but for the bulk roles'
  quantified clauses and continuations (`app_disjoint`, `tr`, `rk`), open
  since the roles' contracts.
- `functional/verify_iterator_validity.move`: the Prover's thirteen
  failures (G5: keyed iterators, the role natives' declared preconditions,
  the iterator's slot read without its variant).

### C11. Equality operand types differ

1 test, 2 messages.

Example (`functional/verify_custom_table.move`):

```text
VIR-SEMANTIC-TYPE: in fun create_and_insert_fail2: equality primitive operand types differ at [7698, 7734)
```

Tests: `functional/verify_custom_table.move`.

### C12. Call or closure target does not resolve — opaque-inline gap fixed

The adapter previously discarded every retained inline declaration even when
an opaque call or behavioral predicate still referred to it. It now keeps opaque
inline declarations and their comments in both owning modules and interfaces.
Non-opaque declarations already expanded by compiler-v2 remain omitted.

The three positive-gap fixtures now reach verification. `behavioral_predicate_inline_fun`
verifies completely. `opaque_inline_body_fail` rejects the bad `inc` body and its
dependent caller, while the explicitly trusted variant verifies. `opaque_inline_loop_sum::sum` and
`test_sum_twice` verify automatically using facts about products of consecutive
integers; the deliberately wrong `test_sum_wrong` postcondition fails.

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
the wrong final counter value at both 25k and the suite default. A
literal-closure guard bounds failed speculation to two attempts per target.

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

Pure short-circuit guards whose branches leave identical environments share
their continuation. The `constant_product` companion supplies the arithmetic
argument directly. A bounded arithmetic-only omega attempt precedes the
full-context bound solver when memory-dependent hypotheses are present;
`ArithmeticContext.lean` covers the fast path and fallback. An unrestricted
variant grew proof terms on the core cost gate and was rejected.

The focused probes also exposed open-generic closure-frame and state-labelled
`result_of`/wildcard-frame proof gaps. The checked regression covers concrete
generic instantiation and `ensures_of` invariants; it does not claim those broader
proof gaps are solved. This scoped follow-up is after the full 437-file counts
above (one pre-verification rejection now reaches verification).

### C21. Field selection not on every variant — no longer occurs

The selection was of a ghost field (`input.proof`), which the unit model
carries (`ghost-fields.md`). The test now reaches
verification and shows V27.

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
([`state-labels.md`](../designs/state-labels.md)). The Prover reads a free label on a
function of linear control flow at a program point and fails the clause
on non-linear control flow; validation rejects the unit.

1 test (the Prover fails the clause), 1 message.

Example (`functional/state_labels/nonlinear_cfg_error.move`):

```text
VIR-SEMANTIC-STATE-LABEL: in fun branching_with_labels: state label `S` is neither bound by a quantifier over the state domain nor defined by a state-change predicate
```

Tests: `functional/state_labels/nonlinear_cfg_error.move`.

## Errors during verification

### V1. Rendering outside the VerexSL parser

Messages:

- 2 × `` condition kind `…` is outside the current VerexSL parser ``
- 3 × `this quantifier kind is outside the current VerexSL parser`
- 1 × `` in-body specification condition `…` with N properties and N auxiliary expressions is outside the current VerexSL parser ``

Generic module invariants are supported;
`type_param_bug_121721` now stops at a quantifier kind.

Example (`functional/emits.move`):

```text
condition kind `VerexIR.ConditionKind.emits` is outside the current VerexSL parser
```

Functional `update_field` is supported for ordinary structs and enum fields
present in every variant, including generic and nested updates. The eleven
former render failures now reach proof obligations or a more specific gap.
`aborts_with` is now supported too: the two abort-clause registry files prove
all 12 positive functions and reject all 15 deliberately invalid functions.
Allowed codes remain constrained with partial abort conditions; arithmetic
traps are distinguished from explicit aborts and map to `EXECUTION_FAILURE`.

Partial-variant enum updates are listed under V2, mutable-reference
behavioral predicates under V20; `mono` now reaches its unsupported `emits`
clause.

A generic axiom (`axiom<T>`, VerexSL `axiom {T} e`) is assumed at each
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
`SourceVerify/inherited_strictness{,_false}.move`). Generic module invariants are also supported.

Tests: `functional/choice.move`, `functional/emits.move`, `functional/loop_unroll.move`, `functional/mono.move`, `regression/type_param_bug_121721.move`.

### V2. Construct not supported in generated contracts

Messages:

- 2 × `a function value with type arguments is not carried in generated contracts`
- 1 × `specification operation VerexIR.Operation.call (VerexIR.CallKind.invoke) is not supported in generated contracts`
- 1 × `generated contracts currently expand specification functions with one result`
- 2 × `a field update on an enum whose variants do not all carry the field is not supported yet`

The state-change predicates `publish`, `remove`, and `update` now translate
in contracts, including labels they define (S2b of
[`state-labels.md`](../designs/state-labels.md)).

An arbitrary value (`__verex_arbitrary_…`, the value of a specification
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
specification operation VerexIR.Operation.call (VerexIR.CallKind.invoke) is not supported in generated contracts
```

Tests: `functional/closures/behavioral_soundness.move`, `functional/closures/closure_in_spec_expr.move`, `functional/spec_fun_tuple_errors.move`, `regression/enum_update_out_of_variant.move`.

### V6. Construct not carried by the denotation

5 tests, 7 messages.

Messages:

- 1 × `` no denotation for `…`: a literal of this type is not carried by the denotation ``
- 2 × `` no denotation for `…`: a closure whose rows are not its target's is not carried by the denotation ``
- 4 × `` no denotation for `…`: a type argument without values is not carried by the denotation ``

A closure's rows are not its target's where the target returns a reference
(`closure_refs.move::borrow_a`) or several results; a function type marking
its shared results is the recorded fix ([`higher-order-functions.md`](../designs/higher-order-functions.md),
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

### V7. Intrinsic map representation not carried — placeholder layouts carried

The abstract map owners the Prover's tests declare (`struct Map<K, V> {}`,
whose one field is the Boolean placeholder `dummy_field` the compilers give
an empty struct) are placeholder layouts of the table view: their entries
are the owner's hidden contents field, their identity is their entries, and
every role, the bulk roles included, is available to them
([`intrinsic-maps.md`](../designs/intrinsic-maps.md), "Views", "Laws of the bulk
roles"). `intrinsic_map_rank`, `intrinsic_map_rank_bulk` and
`intrinsic_map_insertion_order` fail exactly the functions the Prover fails
(`rank_wrong`, `build_wrong`, `same_entries_not_equal` and `canary`).

Still failing, 2 tests, 22 messages:

- `ghost_field_intrinsic_map_ops.move`: `map_spec_iter_preserved` is not
  carried (3 messages; iterator validity is stage 3 of the design), so
  `borrow_write`, `add_preserved_bad` and `addall_preserved_bad` are not
  verified, where the Prover fails the latter two; `app_disjoint`'s
  `aborts_if ∃ k: u64 …` needs the existential over a map's keys read at
  the key type; `tr` does not establish that the trimmed map and the
  returned rest are disjoint, and `rk` (`replace_key_inplace` under
  `aborts_if_is_partial`) does not establish its continuation. The other
  nine functions verify.
- `closures/inline/folds_of_map_intrinsic.move`: the `spec_fold` loop
  invariants over `spec_set` and `spec_remove` are not preserved by an
  iteration (`collect`, `collect_distinct`, `remove_all`,
  `remove_all_must_exist`, and the canary `collect_wrong`, which the Prover
  fails too).

Example (`functional/ghost_field_intrinsic_map_ops.move:6`):

```text
the intrinsic map role `map_spec_iter_preserved` is not carried
```

Tests: `functional/closures/inline/folds_of_map_intrinsic.move`, `functional/ghost_field_intrinsic_map_ops.move`.

### V10. Other elaboration errors in the rendering

5 tests, 5 messages.

Messages:

- 2 × `` VEREX-CALL-NAME: unknown function `…` ``
- 3 × `VEREX-SPEC-ARITY: behavior predicate expects N value argument(s), got N`

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
remaining budget before the authored tactic runs.

The follow-up clears unused continuation alias chains and normalizes encoded
integer reads to certified values, preserving signed/unsigned bounds even for
computed optional reads. Four `CertifiedReads` positives pass at 25k, and a
false signed nonnegativity claim remains rejected. The same instrumented
positive-only `foreach` diagnostic drops from 48.483M to 36.190M raw heartbeats.
Those profiling runs use a diagnostic 100k allowance; the ordinary 25k probe
still times out. This is a preparation-cost improvement, not a fixed fixture.

Tests: `functional/closures/behavioral_results.move`, `functional/closures/inline/folds_of_idx.move`, `functional/closures/inline/folds_of_multi.move`, `functional/closures/result_of_mut_ref_soundness.move`, `functional/state_labels/followed_by_mut_ref.move`.

### V16. Native without specification or prelude model

5 tests, 31 messages.

Example (`functional/type_reflection.move:80`):

```text
`test_type_info_ignores_type_param` calls the native `0x2::type_info::type_of`, which has neither a specification nor a prelude model; specify it
```

Tests: `functional/closures/inline/bp_invariant_weakening_scope.move`, `functional/intrinsic_map_insertion_order.move`, `functional/type_reflection.move`, `functional/type_reflection_ext.move`, `regression/type_reflection_runtime_names.move`.

### V18. Recursion through an unspecified function

1 test, 1 message.

Example (`functional/recursive_move_funs_multi_hop.move:13`):

```text
`foo_3` reaches itself through `foo_2`, which is unspecified; a function on a cycle of calls is used through its contract, so specify `foo_2`
```

Tests: `functional/recursive_move_funs_multi_hop.move`.

### V20. Behavioral predicates with mutable-reference parameters

3 tests, 3 messages.

`aborts_of`, `ensures_of` and `result_of` over function values with `&mut`
parameters are carried (`AbortsOfMut`, `EnsuresOfMut`,
`ResultOfMut`, [`higher-order-functions.md`](../designs/higher-order-functions.md),
"Behavioral predicates"): `lambda_spec_discarded_result`,
`lambda_spec_loop_anchor` and `lambda_spec_two_state_specfun` verify (the
Prover fails the inferred contract of the loop lambda in
`lambda_spec_loop_anchor`; Verex infers none). `lambda_spec_global_memory`
verifies since a literal closure's run is read once for all conjuncts of a
conjunction (19.8M heartbeats, [`performance.md`](../designs/performance.md)), and
`bp_requires_aborts_labeled_mut` since a label's existential over
behavioral predicates is witnessed at a program point
([`state-labels.md`](../designs/state-labels.md), S3b), and
`lambda_variant_field_access` (`write_ss`, `write_shared`: a lambda writing
a field of a `&mut` enum through an opaque `apply_e`/`apply_f`) since a run
is read at struct and enum arguments the normalization has unfolded to
their variant (`nativeOf?`); `two` and `write_ss_unchanged` fail as the
Prover does. What remains:

- `closure_bp_post_sub_pre_only`: invoking an unseen function value with
  `&mut` parameters leaves a residual obligation (its rule needs the H4b
  typing of lent arguments); the Prover fails the clause too.

Messages:

- 1 × `verification failed with a residual obligation`

Tests: `functional/state_labels/closure_bp_post_sub_pre_only.move`.

The invocation-label increment moves `aborting_result_definition` to its
intended no-abort clause failures. `aborts_if_at_state_label::caller` now
verifies at the native 25k budget: reuse call observations
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
struct instantiation. Verex's native carrier is a field row; treating all
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
in `generic_aliasing_write_routes` remain rejected.

`functional/nonlinear_arithm.move::mul5` now has a proof companion: its
five increasing unsigned factors yield zero or a product of at least 120,
so the result is not 72. The deliberately invalid `mul5_incorrect` and other
negative cases remain rejected.

`functional/invariants_with_quant.move` verifies its four quantified
postconditions automatically by enumerating the three valid indices and
computing the vector reads.
All eleven positives in `for_loop_invariants.move` now verify. Its three
triangular-sum proofs use a proved conversion from truncating to Euclidean
division: `i * (i - 1)` is nonnegative for every integer. This prevents costly
absolute-value case splits during preparation, retaining the 25k budget.
Both intentionally incorrect loop invariants remain rejected.

`functional/simple_vector_client.move` now verifies completely. Its companion
normalizes literal list spines before branch exploration and supplies the
four membership witnesses for `test_contains`. `test_index_of` closes by simplification at the unchanged 25k budget;
`option_type` verifies automatically.

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
  fixture accepts this construction. Verex checks the implicit frame at pack
  time. This discrepancy needs a specification/validity decision rather than
  an unchecked proof shortcut.

`increment_counter` and `use_transformer` measured 7.95M and 5.13M solver
heartbeats, respectively, after computing resource-invariant dispatch locally.
Dependency interfaces still omit field contracts; explicit write
frames are therefore rejected at that boundary rather than silently becoming
empty frames. Owned declarations retain their frames.

### V27. Specification observations after moving a mutable reference

`functional/specs_in_fun_ref.move::simple7` fails its in-body `assert x == y`
after `let a = x; *a = y`. The denotation consumes `x` with `Term.take`;
`assertionConditions`/`overDefined` then requires that emptied slot to be defined.
The residual is `False`, not budget exhaustion.

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
| `functional/bitwise_features.move` | `set`, `disable_feature_flags` (budget) |
| `functional/bug-17117.move` | `test_input_param_as_mut_ref` |
| `functional/bug_15044.move` | `compare_u8_vector` |
| `functional/bug_15880.move` | `test2` |
| `functional/closures/amm_example.move` | `constant_product_with_fee`, `constant_product_with_fee_non_compliant`, `swap`, `create_constant_product_pool`, `create_compliant_fee_pool` |
| `functional/closures/behavioral_predicates_examples.move` | `contains_test_not_found`, `contains_opaque_test_not_found`, `index_test_found`, `index_opaque_test_found`, `reduce_test_ok`, `reduce_opaque_test_ok` |
| `functional/closures/inline/bp_forwarding.move` | `set_values` |
| `functional/closures/inline/fold_symbolic.move` | `sum` |
| `functional/closures/inline/folds_of_collect.move` | `fold_is_prefix` (lemma), `collect` |
| `functional/closures/inline/folds_of_consuming.move` | `sum_literal`, `digits_forward`, `digits_reverse`, `sum_noncopy` |
| `functional/closures/inline/folds_of_wrapper_mut.move` | `mirror_keys`, `set_both` (budget) |
| `functional/closures/inline/result_of_attached_state_ok.move` | `map_add_global`, `map_add_global_bare` |
| `functional/closures/inline/vector_hofs_for_each.move` | `find_value` |
| `functional/closures/stored_fun_values.move` | `use_any_modifier`, `create_modifier_valid` (V26 semantic gaps) |
| `functional/inline_fun.move` | `test_filter` |
| `functional/math8.move` | `pow`, `floor_log2`, `sqrt` (budget) |
| `functional/math_fixed8.move` | `pow_raw` (budget), `exp_raw` |
| `functional/specs_in_fun_ref.move` | `simple7` |
| `functional/state_labels/unmodified_memory_at_label.move` | `swap` (a read after removal; Boogie retains absent resource contents) |
| `functional/verify_vector.move` | `verify_reverse`, `verify_reverse_with_unroll`, `verify_append`, `verify_append_with_unroll`, `verify_index_of_with_unroll`, `verify_contains_with_unroll`, `verify_remove`, `verify_remove_with_unroll` |
| `regression/behavior_axiom_target_field.move` | `same_type_quantified`, `fun_inst` (unsupported quantified field-validity domain; V25) |
| `regression/generic_aliasing_all_partitions.move` | `true_in_every_case`, `never_alias` |
| `regression/performance_200511.move` | `fresh_guid`, `new_event_handle_impl` |
| `regression/vector_theory_boogie_array_intern.move` | `f1`, `f2` |
| `regression/vector_theory_smt_seq.move` | `f1`, `f2` |

The functions are marked as follows:

- No mark: the automatic verification failed within its budget.
- (budget): the function exceeds the default target budget of 1000 thousand
  `maxHeartbeats` used by the registry.
- (lemma): a `spec lemma` whose theorem is not established
  ([`lemmas.md`](../designs/lemmas.md)).

A function can fail because of a V entry of the same test.

### V27. A ghost-sensitive function's theorem is not assembled

1 test. The theorem of a function whose in-body assumptions are carried
(`satisfies_of_assumptions`) and whose contract reads a ghost field through
an axiom is assembled without the `verexInvoke` binder its behavioral
predicates need:

```text
functional/uninterpreted_spec_fun_congruence.move:9:1: error: Unknown identifier `verexInvoke`
functional/uninterpreted_spec_fun_congruence.move:9:1: error: don't know how to synthesize implicit argument `assumed`
```

Tests: `functional/uninterpreted_spec_fun_congruence.move`
(`ghost_sensitive_function_is_not_move_congruent`). The cause is in the
assembly of the public theorem.

### V28. Function value equality up to ghosts does not carry a frame

2 tests. A function value's equality erases ghost slots
(`ghost-fields.md`): `result == p` of a function-valued result is
`(encode result).eraseGhosts = (encode p).eraseGhosts`. The data invariant
of a function-typed field (`KeepsMemoryAt`, the default read-only frame)
owed where `Box { f: id(p) }` is constructed is known of `p` and not
transferred to `result`, which the erased equality no longer identifies with
`p` syntactically:

```text
regression/fun_value_equality.move:58:5: error: the data invariant `f : Fn(u64) -> u64 has Copy, Drop` does not hold where a value is constructed
```

Tests: `regression/fun_value_equality.move`, `regression/fun_value_equality_ext.move`
(`boxed_equal`). The frame of a
function value does not depend on its captures' ghost slots, which the
runtime ignores, so a lemma transferring `KeepsMemoryAt` along
`eraseGhosts`-equality is the fix.

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
