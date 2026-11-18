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

/-- A second target, whose contract states the validity of the value its
runs return: its derived fact is the fact another candidate's argument
needs. -/
def otherHandle : FunctionHandle := ⟨⟨0⟩, ⟨1⟩⟩

abbrev producerRow : NRow := .cons (.int 64 false) .nil
abbrev producerCaptured : NRow := .nil
abbrev producerSupplied : NRow := .cons (.int 64 false) .nil
abbrev producerWeave : Weave producerRow producerCaptured producerSupplied := .supplied .nil

/-- A target whose theorem holds at arguments that hold their data
invariants, and states nothing else. -/
def targetContract (unit : Validation.ValidatedUnit) [FunctionCarriers unit] (row : NRow) :
    Contract (Memory unit) Failure (@HList ((Carriers.runtime unit)).toSorts row)
      (@ResultShape.carrier ((Carriers.runtime unit)).toSorts .none) where
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
    let dispatch ← dispatchBehavior goal callees
    owed.set dispatch.owed
    let some next := dispatch.continuation
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

-- A statement names its target's theorem `verified`, which the check finds in
-- the context by that name: a use the unused-variable linter does not see.
set_option linter.unusedVariables false in
#guard_dispatch_residuals ["guarded_target", 2] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! The fact of the receiver's current value is not the prophecy's: the
residual stays. -/

set_option linter.unusedVariables false in
#guard_dispatch_residuals ["guarded_target", 2] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded current)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! ## The paired cases, with the prophecy's fact

The fact in the context pays the argument. -/

set_option linter.unusedVariables false in
#guard_dispatch_residuals [] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
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

set_option linter.unusedVariables false in
#guard_dispatch_residuals [] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_stated : @EnsuresOfEntries unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, current), ()) (prophecy, ()) () pre post),
    False

/-! ## A capture with no fact

The capture is the target's second argument and the closure's only one: the
residual is at the target's position. -/

set_option linter.unusedVariables false in
#guard_dispatch_residuals ["guarded_target", 2] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle captureRow .none)
      (targetContract unit captureRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (captured : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded)
    (typed : @SiteTyped unit _ (Skolems.runtime unit) captureSupplied shared .nil handle
      captureWeave.mask #[] captureCaptured)
    (pre post : Memory unit)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) captureSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) captureSupplied shared .nil handle
        captureWeave.mask #[] captureCaptured (captured, ()) typed)
      (count, ()) () pre post),
    False

set_option linter.unusedVariables false in
#guard_dispatch_residuals [] in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle captureRow .none)
      (targetContract unit captureRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (captured : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded)
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

/-! ## The candidates a witness search takes

A witness search has no goal to take what a derived fact owes: it decides
the atoms where it meets the candidate, and one it cannot pay for is
rejected there and its statement set aside, leaving no fact and no record of
it, so that a candidate accepted afterwards can still make it payable
(`witnessBehaviors`).  The controls read how many candidates the search took
and that a rejected one left nothing behind.

A leaf holding two runs of the target, one whose receiver's prophecy has a
fact and one whose has none, is the case that separates this reading from
one that takes every candidate and pays afterwards: the search takes the
paid run only, in either order of the two hypotheses.
-/

/-- Runs the behavior reading of a witness search on the leaf `statement`
states.  The candidates it took are `expected`: it records one
`BehaviorDispatched` and asserts one `verexBehavior` for each, and a
candidate it rejected leaves neither.  The targets it may read through are
the hypotheses `verified` and, where the leaf states one, `verifiedProducer`. -/
elab "#guard_witness_candidates " expected:num " in " statement:term : command =>
    liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let counts ← IO.mkRef (0, 0)
  let _ ← Tactic.run goal do
    let callees ← goal.withContext do
      let some verified := (← getLCtx).findFromUserName? `verified
        | throwError "the leaf states no theorem of the target"
      let mut found := #[(mkConst ``handle, "guarded_target", verified.toExpr)]
      if let some producer := (← getLCtx).findFromUserName? `verifiedProducer then
        found := found.push (mkConst ``otherHandle, "producer_target", producer.toExpr)
      pure found
    let some next ← witnessBehaviors goal callees #[] #[]
      | throwError "the reading closed the leaf"
    next.withContext do
      let mut records := 0
      let mut facts := 0
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        if (← instantiateMVars decl.type).isAppOfArity ``BehaviorDispatched 1 then
          records := records + 1
        if decl.userName == `verexBehavior then facts := facts + 1
      counts.set (records, facts)
    setGoals []
  let (records, facts) ← counts.get
  let wanted := expected.getNat
  unless records == wanted && facts == wanted do
    throwError "expected {wanted} candidates, the search took {records} \
      and left {facts} facts"

/-! The receiver's prophecy has no fact: the candidate is rejected where it
was met and the leaf holds no fact of its run. -/

set_option linter.unusedVariables false in
#guard_witness_candidates 0 in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded current)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! The prophecy's fact is in hand: the candidate is taken. -/

set_option linter.unusedVariables false in
#guard_witness_candidates 1 in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded prophecy)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! Two runs of the target, the first unpaid and the second paid: the search
takes the paid one and leaves nothing of the other. -/

set_option linter.unusedVariables false in
#guard_witness_candidates 1 in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (currentA prophecyA currentB prophecyB : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded)
    (pre post : Memory unit)
    (_ranA : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((currentA, prophecyA), ()) () pre post)
    (_heldB : @NTy.Holds (Carriers.runtime unit) Guarded prophecyB)
    (_ranB : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((currentB, prophecyB), ()) () pre post),
    False

/-! The same leaf with the paid run first. -/

set_option linter.unusedVariables false in
#guard_witness_candidates 1 in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (currentA prophecyA currentB prophecyB : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded)
    (pre post : Memory unit)
    (_heldB : @NTy.Holds (Carriers.runtime unit) Guarded prophecyB)
    (_ranB : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((currentB, prophecyB), ()) () pre post)
    (_ranA : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((currentA, prophecyA), ()) () pre post),
    False

/-! Both runs paid: both candidates are taken, so the rejection above is the
unpaid atom's and not an effect of there being two runs. -/

set_option linter.unusedVariables false in
#guard_witness_candidates 2 in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (currentA prophecyA currentB prophecyB : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded)
    (pre post : Memory unit)
    (_heldA : @NTy.Holds (Carriers.runtime unit) Guarded prophecyA)
    (_ranA : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((currentA, prophecyA), ()) () pre post)
    (_heldB : @NTy.Holds (Carriers.runtime unit) Guarded prophecyB)
    (_ranB : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((currentB, prophecyB), ()) () pre post),
    False

/-! ## The proof and the debit of a failed attempt

`SharedBudgets.run` is the innermost of the boundaries a leaf step runs in,
and what it answers for is the tactic state and the debit: a failure after
the dispatch restores the goals, the assignments and the records of the
facts it derived, while the work it did stays charged to its owners.  The
control raises the failure in its own action, after a dispatch that owes an
atom.  The scheduler's state and the walk's caches are answered for by
`runOwned` and `runAutomatic` above it, and the control of those is
`VerexIR/Tests/InvocationResidualScheduling.lean`. -/

open VerexIR.Proofs.Denote.SharedBudgets in
/-- Runs a dispatch of the leaf `statement` states inside one of those
attempts.  With `fails` the action raises after the dispatch: the attempt is
refused, the goals and the assignments are as they were, and the debit
remains.  With `commits` the same action returns: the derived fact and the
atom the dispatch owes are there, for a debit of the same work. -/
elab "#guard_owned_transaction " mode:(&"fails" <|> &"commits") " in " statement:term :
    command => liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let raises := (mode.raw.reprint.getD "").trimAscii.toString == "fails"
  let ledger ← IO.mkRef (({} : Ledger).allocate 20000000).1
  let _ ← Tactic.run goal do
    let callees ← goal.withContext do
      let some verified := (← getLCtx).findFromUserName? `verified
        | throwError "the leaf states no theorem of the target"
      pure #[(mkConst ``handle, "guarded_target", verified.toExpr)]
    let before ← getGoals
    let owing ← IO.mkRef 0
    let (outcome, cost) ← SharedBudgets.run (α := Unit) ledger #[0] .leaf do
      let dispatch ← dispatchBehavior goal callees
      owing.set dispatch.owed.size
      if dispatch.owed.isEmpty then throwError "the dispatch owed no atom"
      if raises then throwError "the action of an owned transaction fails"
    unless (← owing.get) == 1 do
      throwError "the dispatch owed {← owing.get} atoms, not one"
    unless cost > 0 do throwError "the transaction measured no work"
    unless (← ledger.get).owners[0]!.spent == cost do
      throwError "the owner's debit is {(← ledger.get).owners[0]!.spent}, not the {cost} spent"
    if raises then
      match outcome with
      | .failed _ => pure ()
      | _ => throwError "the failing action was accepted"
      unless (← getGoals) == before do throwError "the goals were not restored"
      if ← goal.isAssigned then throwError "an assignment of the dispatch escaped the rollback"
      goal.withContext do
        for decl in ← getLCtx do
          if decl.isImplementationDetail then continue
          if (← instantiateMVars decl.type).isAppOfArity ``BehaviorDispatched 1 then
            throwError "a record of the abandoned dispatch survived its transaction"
          if decl.userName == `verexBehavior then
            throwError "a fact of the abandoned dispatch survived its transaction"
    else
      match outcome with
      | .done _ => pure ()
      | _ => throwError "the returning action was refused"
      unless ← goal.isAssigned do throwError "the dispatch left the leaf unchanged"
    setGoals []

/-! The receiver's prophecy has no fact, so the dispatch owes one atom. The
failing transaction leaves neither the fact nor its record; the returning
one leaves both, and the debit is the same either way. -/

set_option linter.unusedVariables false in
#guard_owned_transaction fails in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

set_option linter.unusedVariables false in
#guard_owned_transaction commits in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    False

/-! ## The witness a search finds, and the candidate it cannot pay for

A state label's existential is witnessed at a program point from the facts
the leaf holds there and the behaviors its candidates derive
(`behaviorWitness`).  A target whose contract states that its runs keep the
memory gives a conjunct no hypothesis of the leaf states, so the witness
needs the derived fact and the cheap deciders alone do not reach it.  Where
the candidate's argument validity is unpaid the search takes no witness and
leaves the goal, its context and the proof as they were.
-/

/-- A target whose contract states that its runs keep the memory: a
conjunct of a witness that only the derived fact gives. -/
def keepingContract (unit : Validation.ValidatedUnit) [FunctionCarriers unit] (row : NRow) :
    Contract (Memory unit) Failure (@HList ((Carriers.runtime unit)).toSorts row)
      (@ResultShape.carrier ((Carriers.runtime unit)).toSorts .none) where
  requires := fun _ _ => True
  ensures := fun _ pre _ post => post = pre
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun args => @HList.Holds (Carriers.runtime unit) row args

/-- Runs the witness search on the goal `statement` states and compares its
answer with `found`.  Where it finds none, what is compared with the state
before the search is the list of goals, whether the goal is assigned, the
goal's statement, and the number of declarations in its context. -/
elab "#guard_behavior_witness " found:(&"reads" <|> &"drops") " in " statement:term :
    command => liftTermElabM do
  let statement ← instantiateMVars (← Term.elabType statement)
  Term.synthesizeSyntheticMVarsNoPostponing
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let wanted := (found.raw.reprint.getD "").trimAscii.toString == "reads"
  let answered ← IO.mkRef false
  let _ ← Tactic.run goal do
    let callees ← goal.withContext do
      let some verified := (← getLCtx).findFromUserName? `verified
        | throwError "the leaf states no theorem of the target"
      pure #[(mkConst ``handle, "guarded_target", verified.toExpr)]
    let goals ← getGoals
    let context ← goal.withContext do pure (← getLCtx).decls.size
    let target ← instantiateMVars (← goal.getType)
    answered.set (← behaviorWitness goal callees)
    unless ← answered.get do
      unless (← getGoals) == goals do throwError "a search that found none moved the goals"
      if ← goal.isAssigned then throwError "a search that found none assigned the goal"
      unless (← instantiateMVars (← goal.getType)) == target do
        throwError "a search that found none changed the goal"
      goal.withContext do
        unless (← getLCtx).decls.size == context do
          throwError "a search that found none changed the context"
    setGoals []
  unless (← answered.get) == wanted do
    throwError "the search {if wanted then "found no" else "found a"} witness"

/-! The receiver's prophecy has a fact, so the candidate is paid and its
derived fact witnesses the label. -/

set_option linter.unusedVariables false in
#guard_behavior_witness reads in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (keepingContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_point : @ProgramPoint (Carriers.runtime unit) (Memory unit) .none .nil PUnit
      (@Flow.value (Carriers.runtime unit) .none .nil PUnit PUnit.unit ()) post)
    (_held : @NTy.Holds (Carriers.runtime unit) Guarded prophecy)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    ∃ (memory : Memory unit),
      (@EnsuresOf unit _ (Skolems.runtime unit)
        (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
        (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
          receiverWeave.mask #[] receiverCaptured (count, ()) typed)
        ((current, prophecy), ()) () pre memory) ∧ memory = pre

/-! The same label with no fact of the prophecy: the candidate is rejected,
no other candidate derives the conjunct, and the search finds no witness. -/

set_option linter.unusedVariables false in
#guard_behavior_witness drops in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (keepingContract unit receiverRow))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (current prophecy : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_point : @ProgramPoint (Carriers.runtime unit) (Memory unit) .none .nil PUnit
      (@Flow.value (Carriers.runtime unit) .none .nil PUnit PUnit.unit ()) post)
    (_ran : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, prophecy), ()) () pre post),
    ∃ (memory : Memory unit),
      (@EnsuresOf unit _ (Skolems.runtime unit)
        (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
        (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
          receiverWeave.mask #[] receiverCaptured (count, ()) typed)
        ((current, prophecy), ()) () pre memory) ∧ memory = pre

/-! ## A candidate the fact of a later candidate pays for

A candidate rejected for an unpaid argument is set aside, not excluded: the
statements set aside are forgotten as soon as a candidate is accepted or a
composition opened, so the same candidate is met again in the enriched
context.  Here the second target's contract states the validity of the value
its runs return, and that value is the first target's receiver prophecy: the
first candidate is unpaid when it is first met, and payable once the second
candidate's fact is derived.  Both candidates are taken.
-/

/-- A target whose contract states the validity of the value its runs
return. -/
def producerContract (unit : Validation.ValidatedUnit) [FunctionCarriers unit] :
    Contract (Memory unit) Failure (@HList ((Carriers.runtime unit)).toSorts producerRow)
      (@ResultShape.carrier ((Carriers.runtime unit)).toSorts (.one Guarded)) where
  requires := fun _ _ => True
  ensures := fun _ _ result _ => @NTy.Holds (Carriers.runtime unit) Guarded result
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun args => @HList.Holds (Carriers.runtime unit) producerRow args

set_option linter.unusedVariables false in
#guard_witness_candidates 2 in
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit)
    (verified : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] handle receiverRow .none)
      (targetContract unit receiverRow))
    (verifiedProducer : Satisfies
      (@propheticMeaning unit _ executable (Skolems.runtime unit) #[] otherHandle producerRow
        (.one Guarded))
      (producerContract unit))
    (shared : Sharing) (count : @NTy.carrier ((Carriers.runtime unit)).toSorts (.int 64 false))
    (typed : @SiteTyped unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
      receiverWeave.mask #[] receiverCaptured)
    (typedProducer : @SiteTyped unit _ (Skolems.runtime unit) producerSupplied shared
      (.cons Guarded .nil) otherHandle producerWeave.mask #[] producerCaptured)
    (current produced : @NTy.carrier ((Carriers.runtime unit)).toSorts Guarded) (pre post : Memory unit)
    (_ranReceiver : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) receiverSupplied shared .none
      (@Skolems.construct unit _ (Skolems.runtime unit) receiverSupplied shared .nil handle
        receiverWeave.mask #[] receiverCaptured (count, ()) typed)
      ((current, produced), ()) () pre post)
    (_ranProducer : @EnsuresOf unit _ (Skolems.runtime unit)
      (@closureMeaning unit _ executable (Skolems.runtime unit)) producerSupplied shared
      (.one Guarded)
      (@Skolems.construct unit _ (Skolems.runtime unit) producerSupplied shared
        (.cons Guarded .nil) otherHandle producerWeave.mask #[] producerCaptured ()
        typedProducer)
      (count, ()) produced pre post),
    False

end VerexIR.Tests.BehaviorDispatchResidual
