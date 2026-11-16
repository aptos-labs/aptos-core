-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! # The residual a behavior's argument validity leaves

A leaf reads the behavior of a known function value through its target's
theorem (`dispatchBehavior`), which owes the validity of the target's
arguments: the captures and the supplied arguments, woven.  Where the leaf
holds no fact of one of them, the dispatch queues that atom as a residual of
the callee the behavior belongs to, at the argument's position in the
target's own row.  The controls run the dispatch itself on a leaf and read
the origin it queues.

The target takes an integer and a mutable reference to a value of a declared
type, whose data invariant is the unit's own (`FunctionCarriers.own`) and so
is no consequence of the value.  The reference's validity is its prophecy's.
The integer states nothing and stands first, so the position read is the
row's and not the one left after the trivial conjunct is erased; it is the
closure's capture, so the position is the woven row's and not the supplied
row's.
-/

namespace VerexIR.Tests.BehaviorDispatchResidual

open Lean Elab Command Tactic Meta

def handle : FunctionHandle := ⟨⟨0⟩, ⟨0⟩⟩

/-- A declared type with one field: its invariant is the unit's. -/
abbrev Guarded : NTy := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil)

/-- The target's row: an integer, then a mutable reference. -/
abbrev receiverRow : NRow := .cons (.int 64 false) (.cons (.ref Guarded) .nil)
abbrev receiverCaptured : NRow := .cons (.int 64 false) .nil
abbrev receiverSupplied : NRow := .cons (.ref Guarded) .nil
abbrev receiverWeave : Weave receiverRow receiverCaptured receiverSupplied :=
  .captured (.supplied .nil)

/-- A target's row whose second argument, a value of the declared type, is
the closure's capture. -/
abbrev captureRow : NRow := .cons (.int 64 false) (.cons Guarded .nil)
abbrev captureCaptured : NRow := .cons Guarded .nil
abbrev captureSupplied : NRow := .cons (.int 64 false) .nil
abbrev captureWeave : Weave captureRow captureCaptured captureSupplied :=
  .supplied (.captured .nil)

/-- A target whose theorem holds at arguments that hold their data
invariants, and states nothing else. -/
def targetContract (unit : Validation.ValidatedUnit) [FunctionCarriers unit] (row : NRow) :
    Contract (Memory unit) Failure (@HList (Carriers.runtime unit) row)
      (@ResultShape.carrier (Carriers.runtime unit) .none) where
  requires := fun _ _ => True
  ensures := fun _ _ _ _ => True
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun args => @HList.Holds (Carriers.runtime unit) row args

/-- Runs the behavior dispatch on the leaf `statement` states, whose
hypothesis `verified` is the theorem of the target `handle`, named
`guarded_target`.  The dispatch must read a behavior through that theorem,
and the residuals it queues must be exactly those `expected` lists: the
callee each belongs to and its argument position.  A residual states a fact
of the hypothesis `prophecy` or `captured`, whichever the leaf has, and of
no other value. -/
elab "#guard_dispatch_residuals " "[" expected:(str ", " num),* "]" " in " statement:term :
    command => liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let owed ← IO.mkRef (#[] : Array (Provenance × MVarId))
  let dispatched ← IO.mkRef false
  let _ ← Tactic.run goal do
    let callees ← goal.withContext do
      let some verified := (← getLCtx).findFromUserName? `verified
        | throwError "the leaf states no theorem of the target"
      pure #[(mkConst ``handle, "guarded_target", verified.toExpr)]
    let some next ← dispatchBehavior goal callees (owed := some owed)
      | throwError "the dispatch closed the leaf"
    next.withContext do
      dispatched.set ((← getLCtx).findFromUserName? `verexBehavior).isSome
    setGoals []
  unless ← dispatched.get do
    throwError "the dispatch read no behavior through the target's theorem"
  let mut found : Array (String × Nat) := #[]
  for (origin, residual) in ← owed.get do
    let .callArgument callee none position := origin
      | throwError "a residual of another origin"
    found := found.push (callee, position)
    residual.withContext do
      let stated ← instantiateMVars (← residual.getType)
      let context ← getLCtx
      let subject := [`prophecy, `captured].filterMap context.findFromUserName?
      let others := [`current, `count].filterMap context.findFromUserName?
      unless subject.any fun decl => stated.containsFVar decl.fvarId do
        throwError "the residual {stated} is not a fact of the unpaid value"
      if others.any fun decl => stated.containsFVar decl.fvarId then
        throwError "the residual {stated} is a fact of another value"
  let wanted := expected.getElems.toList.map fun entry =>
    (entry.raw[0].isStrLit?.getD "", entry.raw[2].isNatLit?.getD 0)
  unless found.toList == wanted do
    throwError "expected the residuals {wanted}, got {found.toList}"

/-! ## A mutable receiver whose prophecy has no fact

The leaf holds a run of the closure at a row it names, and no fact of the
receiver's prophecy: the residual is the target's second argument's. -/

#guard_dispatch_residuals ["guarded_target", 2] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier (Carriers.runtime unit) Guarded) (pre post : Memory unit)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! The fact of the receiver's current value is not the prophecy's: the
residual stays. -/

#guard_dispatch_residuals ["guarded_target", 2] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier (Carriers.runtime unit) Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded current)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! ## The paired cases, with the prophecy's fact

The fact in the context pays the argument. -/

#guard_dispatch_residuals [] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier (Carriers.runtime unit) Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded prophecy)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! A clause's statement of the run, at the receiver's entry value and its
final value, quantifies a row of valid values: opening it gives the
prophecy's fact, which pays the argument. -/

#guard_dispatch_residuals [] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier (Carriers.runtime unit) Guarded) (pre post : Memory unit)
    (_stated : @EnsuresOfEntries unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, current), ()) (prophecy, ()) () pre post),
    False

/-! ## A capture with no fact

The capture is the target's second argument and the closure's only one: the
residual is at the target's position. -/

#guard_dispatch_residuals ["guarded_target", 2] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle captureRow .none)
      (targetContract unit captureRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (captured : @NTy.carrier (Carriers.runtime unit) Guarded)
    (typed : @SiteTyped unit _ (Skolems.runtime unit) captureSupplied shared .nil handle
      captureWeave.mask #[] captureCaptured)
    (pre post : Memory unit)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) captureSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) captureSupplied shared .nil handle
        captureWeave.mask #[] captureCaptured (captured, ()) typed)
      (count, ()) () pre post),
    False

#guard_dispatch_residuals [] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle captureRow .none)
      (targetContract unit captureRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (captured : @NTy.carrier (Carriers.runtime unit) Guarded)
    (typed : @SiteTyped unit _ (Skolems.runtime unit) captureSupplied shared .nil handle
      captureWeave.mask #[] captureCaptured)
    (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded captured)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) captureSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) captureSupplied shared .nil handle
        captureWeave.mask #[] captureCaptured (captured, ()) typed)
      (count, ()) () pre post),
    False

/-! ## A dispatch that owes no residual

A witness search reads a behavior at a candidate point and owes nothing for
a candidate it may reject, so its dispatch takes no list of residuals: an
atom of the argument validity is decided on the spot, and where it stays
open the behavior is not read at all.  The leaf then holds no fact of the
run through the target's theorem, and no obligation was queued for it. -/

/-- Runs the behavior dispatch as a witness search does, with no list of
residuals, on the leaf `statement` states, and compares whether it read a
behavior through the target's theorem with `read`. -/
elab "#guard_dispatch_without_residuals " read:(&"reads" <|> &"drops") " in " statement:term :
    command => liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let dispatched ← IO.mkRef false
  let _ ← Tactic.run goal do
    let callees ← goal.withContext do
      let some verified := (← getLCtx).findFromUserName? `verified
        | throwError "the leaf states no theorem of the target"
      pure #[(mkConst ``handle, "guarded_target", verified.toExpr)]
    let some next ← dispatchBehavior goal callees
      | throwError "the dispatch closed the leaf"
    next.withContext do
      dispatched.set ((← getLCtx).findFromUserName? `verexBehavior).isSome
    setGoals []
  let wanted := (read.raw.reprint.getD "").trimAscii.toString == "reads"
  unless (← dispatched.get) == wanted do
    throwError "the dispatch {if wanted then "read no" else "read a"} behavior through the \
      target's theorem"

#guard_dispatch_without_residuals drops in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier (Carriers.runtime unit) Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded current)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

#guard_dispatch_without_residuals reads in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier (Carriers.runtime unit) (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier (Carriers.runtime unit) Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded prophecy)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

end VerexIR.Tests.BehaviorDispatchResidual
