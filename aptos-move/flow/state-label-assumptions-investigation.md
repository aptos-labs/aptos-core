# State-label verification repair

The gas-schedule mutation regression exposed a verifier soundness bug: a mixed
postcondition containing an intermediate-state definition was assumed in full,
then skipped as a proof obligation. The final write was therefore assumed rather
than verified. This is a verification-instrumentation bug; WP must not invoke
the prover to compensate for it.

## Implementation

- Project only free intermediate-state definitions, preserving guards and let
  scope. Quantified states remain proof obligations.
- Always assert full postconditions. Keep successful callee postconditions even
  when their defining fragments were assumed before the call.
- Share structural projection with behavioral-predicate generation so abort
  predicates cannot import final-state claims from an ensures clause.
- Guard call definitions by their requires/non-aborting domain and mutation
  definitions by resource existence/absence. An aborting operation must not
  make verification vacuous by assuming its success conditions.
- Snapshot implicit pre-states explicitly, including post-labeled behavioral
  calls and mutations. Callee snapshots refer to call entry, not caller entry.
- Connect referenced memory-free opaque-call results to their result carriers.
  Avoid generating unnecessary behavioral summaries for unrelated calls.
- Preserve memory state across value-only dynamic calls whose conservative
  effects contain neither memory writes nor mutable-reference outputs.
- Retain intermediate field-update definitions when later writes can alias
  them. Only final field updates may be weakened to a leaf equality. This also
  repairs the three-address and unrolled-loop inference fixtures, which had
  previously recorded verification failures in their baselines.
- Expose plain-struct value-state witnesses through constructor fields with
  constructor triggers. Supply reverse evaluator triggers for memory-free
  function-parameter postconditions. These support E-matching with MBQI disabled.
- Keep typed compiler-internal lambda summaries even when their synthetic
  closure references have no source spelling. User-facing inferred conditions
  still undergo source-representability checks.

## Regression coverage

- `guarded_state_labels`: the original contract verifies; replacing the final
  update with a zero-valued configuration must be rejected.
- `mixed_callee_postcondition`: successful callee properties survive definition
  extraction, including after the caller has changed the resource.
- `aborting_result_definition`: incorrect non-aborting claims are rejected for
  both an always-aborting callee and a missing resource.
- Quantified-state fixtures retain their false claims as explicit negative
  cases and add valid counterparts. No integer lies strictly between n and
  n + 1; incrementing an unconstrained counter twice does not ensure it is <100.
- Existing chained opaque-call, calculator, mutable-reference composition,
  bitvector, and nested-lambda fixtures exercise the associated repairs.
- The three-address mutation fixture now verifies and rejects a wrong final
  write; the unrolled-loop fixture now verifies with its distinct write states.

Solver settings are unchanged. Inference and verification remain separate runs.
The obsolete experimental patch has been removed; it did not contain the full
repair and must not be applied.

## Validation

Using the `ci` profile and the configured Boogie/Z3 executables, without MBQI:

- All 386 functional/regression prover tests pass. The `choice` negative test
  uses an explicit, parenthesized return without a trailing semicolon to keep
  its diagnostic on one return location; its verification errors are unchanged.
- All 54 inference baseline tests pass, including the final-write mutation
  rejection checks. These tests compare expected diagnostics as well as proofs;
  passing the suite is not a claim that every negative fixture verifies.
- All 234 library unit tests across `move-model`, `move-prover`,
  `move-prover-bytecode-pipeline`, and `move-prover-boogie-backend` pass.
- Scoped Clippy with repository lint flags and the Flow compile check pass.
  The Flow binary also builds successfully with `cargo build -p aptos-move-flow
  --profile ci` (one unrelated existing consensus dead-code warning).
- Repository formatting passes `cargo +nightly fmt --all -- --check`.

The v1.2 corpus still needs fresh screening with the repaired toolchain before
the requested full round. No evaluation sessions have been launched as part
of this repair, and the pinned Etna revision has not changed.
