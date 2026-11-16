-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

set_option Elab.async false

/-! ## A dictionary's contract handed on inside an adapter

A function that holds a dictionary's contract (`MethodContract`, the evidence
of its function-typed parameter) passes the dictionary on inside an adapter
closure. The adapter's contract assumes the dictionary's contract at the frame
the call's type arguments induce; the caller states it at its own. The
behavior the opaque callee ensures of the adapter is read through the
adapter's theorem, whose premise is that contract: the dispatch moves the
premise to the caller's frame (`MethodContract.of_instantiate`, applied at
the premise's own arguments) and takes it from the caller's hypothesis.

The contract is one fact (a structure): the step that splits a premise into
its conjuncts does not open it, so the goal the rules and the hypothesis are
compared with is the contract itself.

Negatives first: a caller that assumes no contract of the function value does
not obtain the adapter's behavior, and the contract of another function value
does not close the premise. -/

open Lean Meta Elab VerexIR VerexSL

namespace VerexSL.Tests.ForwardedMethodEvidence

-- ============================================================================
-- 1. Negative control
-- ============================================================================

/-- The module of the negative control: `forward` passes `call` on inside the
adapter and assumes no contract of it. -/
def uncheckedSource : String := "verex namespace unchecked where
  @[method_evidence = 1]
  fun unit_mut {F}(
    callback : &mut F, call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call_mut, &mut *callback)

  spec unit_mut where
    pragma opaque
    ensures ensures_of<call_mut>(old(callback), callback)

  @[method_evidence = 1, shared_method_adapter = 1]
  fun shared_as_mut {F}(
    receiver : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call, &*receiver)

  spec shared_as_mut where
    pragma opaque
    ensures receiver == old(receiver)
    ensures ensures_of<call>(old(receiver))

  fun forward {F}(
    callback : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](shared_as_mut::<F>, _, call)
    )

  spec forward where
    pragma opaque
    ensures ensures_of<call>(old(callback))
"

open LoanDeathControls in
run_cmd do
  let .ok command := Parser.runParserCategory (← getEnv) `command uncheckedSource
    | throwError "the negative control does not parse"
  let diagnostics ← rejectedDiagnostics `unchecked.forward ⟨command⟩
  -- The module is parsed from a string, so a clause is named by its kind: the
  -- one clause of `forward`, and the adapter's premise at the call.
  let stated := diagnostics.filter fun text =>
    !text.startsWith "verex verification failed: the automatic verification of `forward` failed;"
  let [premise, clause] := stated
    | throwError "a caller without the dictionary's contract: found {diagnostics}"
  unless premise == "the precondition of `shared_as_mut` is not established" &&
      clause.startsWith "the specification clause `" && clause.endsWith "` is not established" do
    throwError "a caller without the dictionary's contract: found {diagnostics}"

-- ============================================================================
-- 2. Positive controls
-- ============================================================================

verex namespace forwarded where
  @[method_evidence = 1]
  fun unit_mut {F}(
    callback : &mut F, call_mut : Fn(&mut F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call_mut, &mut *callback)

  spec unit_mut where
    pragma opaque
    ensures ensures_of<call_mut>(old(callback), callback)

  @[method_evidence = 1, shared_method_adapter = 1]
  fun shared_as_mut {F}(
    receiver : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    invoke(call, &*receiver)

  spec shared_as_mut where
    pragma opaque
    ensures receiver == old(receiver)
    ensures ensures_of<call>(old(receiver))

  -- The caller holds the contract of `call`.
  @[method_evidence = 1]
  fun forward {F}(
    callback : &mut F, call : Fn(&F) -> Unit has Copy, Drop
  ) -> Unit := do
    unit_mut(
      callback,
      function[Fn(&mut F) -> Unit has Copy, Drop](shared_as_mut::<F>, _, call)
    )

  spec forward where
    pragma opaque
    ensures callback == old(callback)
    ensures ensures_of<call>(old(callback))

  -- The same through a result that is a value.
  @[method_evidence = 1]
  fun value_mut {F}(
    callback : &mut F, call_mut : Fn(&mut F) -> u64 has Copy, Drop
  ) -> u64 :=
    invoke(call_mut, &mut *callback)

  spec value_mut where
    pragma opaque
    ensures ensures_of<call_mut>(old(callback), result, callback)

  @[method_evidence = 1, shared_method_adapter = 1]
  fun shared_value_as_mut {F}(
    receiver : &mut F, call : Fn(&F) -> u64 has Copy, Drop
  ) -> u64 :=
    invoke(call, &*receiver)

  spec shared_value_as_mut where
    pragma opaque
    ensures receiver == old(receiver)
    ensures ensures_of<call>(old(receiver), result)

  @[method_evidence = 1]
  fun forward_value {F}(
    callback : &mut F, call : Fn(&F) -> u64 has Copy, Drop
  ) -> u64 :=
    value_mut(
      callback,
      function[Fn(&mut F) -> u64 has Copy, Drop](shared_value_as_mut::<F>, _, call)
    )

  spec forward_value where
    pragma opaque
    ensures callback == old(callback)
    ensures ensures_of<call>(old(callback), result)

-- ============================================================================
-- 3. The premise step on a contract
-- ============================================================================

section PremiseStep

open VerexIR.Proofs VerexIR.Proofs.Denote

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit]
  {executable : Validation.ExecutableUnit unit}

/-- A conjunction splitter does not open a dictionary's contract, and the
contract of another function value does not close it; the caller's own does,
through the step as the dispatch runs it. -/
example [Skolems unit] {parameters : NRow} {shared : Sharing} {shape : ResultShape}
    {function other : (NTy.function parameters shared shape.row).carrier}
    (_unrelated : MethodContract executable parameters shared shape other) :
    MethodContract executable parameters shared shape function →
      MethodContract executable parameters shared shape function := by
  fail_if_success
    have : MethodContract executable parameters shared shape function := by
      (repeat apply And.intro) <;> first | assumption
  intro assumed
  fail_if_success apply And.intro
  (repeat apply And.intro) <;> first | assumption

set_option linter.unusedSimpArgs false in
/-- The contract owed at the frame a call's type arguments induce, of a
function value typed at the caller's carriers, is the caller's: the hypothesis
does not state it as it stands and no splitter opens it; the premise step
moves it whole to the caller's frame and reads the hypothesis there. -/
example [Θ : Skolems unit] {shared : Sharing}
    {function : @NTy.carrier Θ.toCarriers (.function (.cons (.param 0) .nil) shared .nil)}
    (assumed : MethodContract executable (.cons (.param 0) .nil) shared .none function) :
    @MethodContract unit _ executable (Skolems.instantiate ⟨.cons (.param 0) .nil, rfl⟩ Θ)
      (.cons (.param 0) .nil) shared .none function := by
  fail_if_success assumption
  fail_if_success apply And.intro
  (repeat apply And.intro) <;> first
    | assumption
    | (verex_method_contract_at_caller
       simp only [ResultShape.functionOfSkolem, NTy.ofSkolem_toSkolem, vir_denote_norm]
       assumption)

end PremiseStep

end VerexSL.Tests.ForwardedMethodEvidence
