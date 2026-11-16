-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
/-! Calls in the shared Given interpreter consume an explicit contract for
the target capability, registered at its caller-side signature. -/

namespace VerexIR.Tests.GivenCalls
set_option maxRecDepth 2000

def handle : FunctionHandle := ⟨⟨0⟩, ⟨0⟩⟩
abbrev row : NRow := .cons (.param 0) .nil
def arguments : TypeArgs := ⟨row, rfl⟩

def caller (unit : Validation.ValidatedUnit) : Function unit where
  params := row
  locals := .nil
  frame := row
  frame_eq := rfl
  result := .one (.param 0)
  body := .callGeneric (σs := row) 1 handle #[] arguments (.one (.param 0))
    (.one (.param 0)) (by decide) (.cons (.take .here) .nil)
  mutables := .nil
  exitSite := 0

def directCaller (unit : Validation.ValidatedUnit) : Function unit where
  params := row
  locals := .nil
  frame := row
  frame_eq := rfl
  result := .one (.param 0)
  body := .call 2 handle (.one (.param 0)) (.cons (.take .here) .nil)
  mutables := .nil
  exitSite := 0

-- Negative control first: a callee contract demanding an unestablished `validArgs`
-- premise cannot be discharged at a Given call site without that validity proof.
def guardedContract [C : Carriers] (State : Type)
    (P : @NTy.carrier C (.param 0) → Prop) :
    Contract State Failure (@HList C row) (@NTy.carrier C (.param 0)) where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun args => P args.1

set_option linter.unusedVariables false in
example (G : GivenFrame) (State : Type) (unit : Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (P : @NTy.carrier G.toCarriers (.param 0) → Prop)
    (verified : Satisfies (ops.call handle row (.one (.param 0)))
      (@guardedContract G.toCarriers State P))
    (value : @NTy.carrier G.toCarriers (.param 0)) (initial : State) : True := by
  fail_if_success
    have : wp (@Function.denoteIn unit G.toCarriers State ops (directCaller unit) (value, ()))
        (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
      simp only [directCaller, row, Function.denoteIn]
      verex_normalize
      verex_obligations [] with [PProd.mk (ops.call handle row (.one (.param 0)))
        (PProd.mk "guarded_direct_callee" verified)]
      all_goals verex_decide
  trivial

def identityContract [C : Carriers] (State : Type) :
    Contract State Failure (@HList C row) (@NTy.carrier C (.param 0)) where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

theorem given_caller (G : GivenFrame) (State : Type) (unit : Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (verified : Satisfies (ops.generic handle #[] arguments row (.one (.param 0)))
      (@identityContract G.toCarriers State))
    (value : @NTy.carrier G.toCarriers (.param 0)) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (caller unit) (value, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [caller, row, Function.denoteIn]
  verex_normalize
  verex_obligations [] with [PProd.mk (ops.generic handle #[] arguments row (.one (.param 0)))
    (PProd.mk "abstract_callee" verified)]
  all_goals verex_solve

theorem given_direct_caller (G : GivenFrame) (State : Type) (unit : Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (verified : Satisfies (ops.call handle row (.one (.param 0)))
      (@identityContract G.toCarriers State))
    (value : @NTy.carrier G.toCarriers (.param 0)) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (directCaller unit) (value, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [directCaller, row, Function.denoteIn]
  verex_normalize
  verex_obligations [] with [PProd.mk (ops.call handle row (.one (.param 0)))
    (PProd.mk "abstract_direct_callee" verified)]
  all_goals verex_solve

open Lean Elab Command in
run_cmd do
  let callRules : Array Name := #[
    ``wp_call_strict_valid, ``wp_call_normal_valid,
    ``wp_call_equations_valid, ``wp_call_valid
  ]
  -- A call may apply a rule stating the contract's conditions by their normal
  -- forms, which the solver holds privately; the private prefix comes off the
  -- name, the namespace does not.
  let ruleNames := callRules ++ #[
    `VerexIR.Proofs.Denote.wp_call_valid_eq,
    `VerexIR.Proofs.Denote.wp_call_strict_valid_eq,
    `VerexIR.Proofs.Denote.wp_call_normal_valid_eq]
  for name in [``given_caller, ``given_direct_caller] do
    let .thmInfo theoremInfo ← getConstInfo name | throwError "expected theorem"
    unless (theoremInfo.value.find? fun expression => match expression.constName? with
        | some rule => ruleNames.contains (privateToUserName rule)
        | none => false).isSome do
      throwError "{name} did not use the callee contract rule"
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

#print axioms given_caller
#print axioms given_direct_caller
end VerexIR.Tests.GivenCalls
