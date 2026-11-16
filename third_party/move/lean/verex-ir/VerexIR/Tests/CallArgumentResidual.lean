-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # The residual a call's argument validity leaves

A callee's contract states the validity of its arguments as a condition per
row position. Where the caller does not hold the fact of one argument, the
solver leaves that atom, at its position in the callee's row, and reports it
without a source clause: no source program reaches that report, since every
value it passes owes its data invariants where it comes about.
-/

namespace VerexIR.Tests.CallArgumentResidual

open Lean Elab Command

set_option maxRecDepth 2000

def handle : FunctionHandle := ⟨⟨0⟩, ⟨0⟩⟩

/-- Two argument slots: the first states nothing, so the position of the
second is the one the row gives it and not the one left after erasure. -/
abbrev row : NRow := .cons (.int 64 false) (.cons (.param 0) .nil)

/-- A callee whose arguments are valid where the second satisfies `P`. -/
def guardedContract [C : Carriers] (State : Type)
    (P : @NTy.carrier C (.param 0) → Prop) :
    Contract State Failure (@HList C row) (@NTy.carrier C (.param 0)) where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.2.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun args => True ∧ P args.2.1 ∧ True

def caller (unit : Validation.ValidatedUnit) : Function unit where
  params := row
  locals := .nil
  frame := row
  frame_eq := rfl
  result := .one (.param 0)
  body := .call 1 handle (.one (.param 0))
    (.cons (.take .here) (.cons (.take (.there .here)) .nil))
  mutables := .nil
  exitSite := 0

/-- Elaborating `command` reports exactly `expected`, as errors, in order. -/
elab "#guard_reports " expected:str,+ " in " command:command : command => do
  let saved ← get
  let before := saved.messages.toList.length
  try elabCommand command catch _ => pure ()
  let fresh := ((← get).messages.toList.drop before).filter (·.severity == .error)
  let texts ← fresh.mapM fun message => message.data.toString
  modify fun _ => saved
  let wanted := expected.getElems.toList.map (·.getString)
  unless texts == wanted do
    throwError "expected the reports {wanted}, got {texts}"

/-! ## The residual, without the argument's fact

The caller holds no fact of its second argument, so the call's argument
validity stays owed and is reported at that argument's position in the
callee's row, with no clause of the target naming it. The first slot states
nothing, so the position is the row's and not the one left after erasure.
-/

#guard_reports "argument 2 of the call to `guarded_callee` is not established" in
example (G : GivenFrame) (State : Type) (unit : Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (P : @NTy.carrier G.toCarriers (.param 0) → Prop)
    (verified : Satisfies (ops.call handle row (.one (.param 0)))
      (@guardedContract G.toCarriers State P))
    (count : @NTy.carrier G.toCarriers (.int 64 false))
    (value : @NTy.carrier G.toCarriers (.param 0)) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (caller unit) (count, value, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [caller, row, Function.denoteIn]
  verex_normalize
  verex_obligations [] with [PProd.mk (ops.call handle row (.one (.param 0)))
    (PProd.mk "guarded_callee" verified)]
  all_goals verex_solve

/-! ## The paired case, with the argument's fact -/

theorem caller_holds (G : GivenFrame) (State : Type) (unit : Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (P : @NTy.carrier G.toCarriers (.param 0) → Prop)
    (verified : Satisfies (ops.call handle row (.one (.param 0)))
      (@guardedContract G.toCarriers State P))
    (count : @NTy.carrier G.toCarriers (.int 64 false))
    (value : @NTy.carrier G.toCarriers (.param 0)) (held : P value) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (caller unit) (count, value, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [caller, row, Function.denoteIn]
  verex_normalize
  verex_obligations [] with [PProd.mk (ops.call handle row (.one (.param 0)))
    (PProd.mk "guarded_callee" verified)]
  all_goals verex_solve

#print axioms caller_holds

/-! ## The same contract with its slots swapped

The predicate is the first slot's and the slot that states nothing is the
second, so the position the report names is the first.
-/

abbrev swappedRow : NRow := .cons (.param 0) (.cons (.int 64 false) .nil)

def swappedContract [C : Carriers] (State : Type)
    (P : @NTy.carrier C (.param 0) → Prop) :
    Contract State Failure (@HList C swappedRow) (@NTy.carrier C (.param 0)) where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun args => P args.1 ∧ True ∧ True

def swappedCaller (unit : Validation.ValidatedUnit) : Function unit where
  params := swappedRow
  locals := .nil
  frame := swappedRow
  frame_eq := rfl
  result := .one (.param 0)
  body := .call 1 handle (.one (.param 0))
    (.cons (.take .here) (.cons (.take (.there .here)) .nil))
  mutables := .nil
  exitSite := 0

#guard_reports "argument 1 of the call to `guarded_callee` is not established" in
example (G : GivenFrame) (State : Type) (unit : Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (P : @NTy.carrier G.toCarriers (.param 0) → Prop)
    (verified : Satisfies (ops.call handle swappedRow (.one (.param 0)))
      (@swappedContract G.toCarriers State P))
    (value : @NTy.carrier G.toCarriers (.param 0))
    (count : @NTy.carrier G.toCarriers (.int 64 false)) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (swappedCaller unit) (value, count, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [swappedCaller, swappedRow, Function.denoteIn]
  verex_normalize
  verex_obligations [] with [PProd.mk (ops.call handle swappedRow (.one (.param 0)))
    (PProd.mk "guarded_callee" verified)]
  all_goals verex_solve

/-! ## The names a validity statement's atoms enter under

A row position holding a wrapper's or a tuple's reading states more than one
atom. `introValidityAtoms` numbers the atoms of the whole statement once, in
its order, so each name is distinct, and the binders after the replaced
premise keep the names and the types they were given.
-/

open Lean Meta in
/-- The binders of the goal `premise` names, after `introValidityAtoms`
replaced the binder `c_valid` of it, as `name : type`. -/
def validityBinders (premise : Lean.Expr) : MetaM (List String) := do
  let goal ← mkFreshExprSyntheticOpaqueMVar premise
  let next ← introValidityAtoms goal.mvarId! `c_valid fun atoms index =>
    if atoms == 1 then `c_valid else Name.mkSimple s!"c_valid_{index + 1}"
  forallTelescope (← next.getType) fun binders _ => do
    let named ← binders.mapM fun binder => do
      let declaration ← binder.fvarId!.getDecl
      pure s!"{declaration.userName} : {← ppExpr declaration.type}"
    pure named.toList

open Lean Elab Command in
/-- The binders of the goal `premise` leaves are exactly `expected`. -/
elab "#guard_binders " expected:str,+ " in " premise:term : command =>
  runTermElabM fun _ => do
    let premise ← Term.elabType premise
    Term.synthesizeSyntheticMVarsNoPostponing
    let binders ← validityBinders (← instantiateMVars premise)
    let wanted := expected.getElems.toList.map (·.getString)
    unless binders == wanted do
      throwError "expected the binders {wanted}, got {binders}"

set_option linter.unusedVariables false

-- A statement of one atom keeps the premise's name.
#guard_binders "c_valid : 0 = 0", "c_ensures : 1 = 1", "c_frame : 2 = 2",
    "c_returns : 3 = 3" in
  ∀ (c_valid : (0 = 0) ∧ True) (c_ensures : 1 = 1) (c_frame : 2 = 2)
    (c_returns : 3 = 3), True

-- Two atoms at the first row position and one at the second: a number per
-- row position would name the first two alike, so the atoms are numbered
-- once over the whole statement.
#guard_binders "c_valid_1 : 0 = 0", "c_valid_2 : 1 = 1", "c_valid_3 : 2 = 2",
    "c_ensures : 3 = 3", "c_frame : 4 = 4", "c_returns : 5 = 5" in
  ∀ (c_valid : ((0 = 0) ∧ (1 = 1) ∧ True) ∧ (2 = 2) ∧ True) (c_ensures : 3 = 3)
    (c_frame : 4 = 4) (c_returns : 5 = 5), True

-- A statement of no atom leaves no binder.
#guard_binders "c_ensures : 0 = 0", "c_frame : 1 = 1", "c_returns : 2 = 2" in
  ∀ (c_valid : True ∧ True) (c_ensures : 0 = 0) (c_frame : 1 = 1)
    (c_returns : 2 = 2), True

end VerexIR.Tests.CallArgumentResidual
