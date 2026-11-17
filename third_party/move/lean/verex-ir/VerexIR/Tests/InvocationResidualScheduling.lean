-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve

/-! # What an invocation's argument validity is scheduled as

The rule a dictionary's contract gives an invocation of a function value
owes the validity of the arguments it passes.  The step runs that rule and
queues what it leaves: the atoms the leaf does not state, each under the
origin of the argument it belongs to, and the goals the leaf goes on with,
under the step's own origin.  All of them keep the owners and the clause
ranges the step inherited, as one queue took them both before, so an
argument of an invocation and what follows it share one allowance.

The controls run the real step (`invocationSteps`) on a leaf of the
solver's own shape and read the queue it leaves.

The clause range the step inherited is registered as a declared type's
invariant, as it is of any leaf that opens one.  An invocation's argument is
keyed by the argument whatever clause its invariant belongs to, so its atom
keeps the argument's key; an origin with no key of its own — what follows
the invocation here — takes that invariant's key instead, which is what the
atom's key would become without the arm that reads an argument first
(`leafKey`).
-/

namespace VerexIR.Tests.InvocationResidualScheduling

open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open VerexIR.Proofs.Denote.SharedBudgets (Ledger)
open Lean Elab Command Tactic Meta

set_option maxRecDepth 4000

/-- A declared type with one field, whose invariant is the unit's own and so
is no consequence of the value. -/
abbrev Guarded : NTy := .struct ⟨⟨0⟩, 0⟩ .nil (.cons (.int 64 false) .nil)

/-- The invoked value's row: an integer, which states nothing, and then the
declared type, so the position an open atom belongs to is the second. -/
abbrev row : NRow := .cons (.int 64 false) (.cons Guarded .nil)
abbrev shape : ResultShape := .none

/-- The clause range the step inherited. -/
def inherited : ObligationRange := { file := "control", startByte := 7, endByte := 9 }

/-- The origin the step carries, which what follows the invocation keeps. -/
def stepOrigin : Provenance := .continuation "sibling" (some 3)

/-- What one run of the invocation step queued: each goal's origin, its
clause ranges, and the owners it was queued under. -/
structure Queued where
  origin : Option Provenance
  clauses : Array ObligationRange
  owners : Array (List Bool × SharedBudgets.OwnerId)
  /-- The key the leaf is reported and selected under (`leafKey`). -/
  key : ObligationKey

/-- Runs the invocation step on the leaf `statement` states, under one owner
that has already spent, and reports what it queued. -/
def runInvocationStep (statement : Lean.Expr) (spent preparationSpent : Nat) :
    TermElabM (Array Queued × Ledger) := do
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let charged := ((({} : Ledger).allocate 20000000).1.charge #[0] .leaf spent).charge #[0]
    .preparation preparationSpent
  let budgets ← IO.mkRef charged
  let queued ← IO.mkRef (#[] : Array Queued)
  let _ ← Tactic.run goal do
    let normalization ← simpContextOf (← `(tactic| simp only []))
    let started ← IO.getNumHeartbeats
    -- The clause range the step inherited is a declared type's invariant, as
    -- it is of any leaf that opens one: without the arm that reads an
    -- invocation's argument first, this is the key the leaf would take
    -- (`leafKey`).
    (← targetState).invariantKeys.set #[(inherited, "data_invariant_1")]
    let ctx : SolveContext := {
      invariants := #[]
      callees := #[]
      equations := #[]
      handOver := false
      labeled := false
      unrolls := #[]
      preconditions := #[]
      names := #[]
      ghostWrites := #[]
      debug := false
      profile := false
      shared := true
      budgets := budgets
      normalization := normalization
      startHeartbeats := started }
    let step : SolveStep :=
      { goal := goal, provenance := some stepOrigin, clauses := #[inherited],
        stageStart := started, callSite := some 3 }
    let state : SolveState := { shared := true, activeOwners := #[([], 0)] }
    let (taken, final) ← (invocationSteps ctx step).run state
    unless taken do throwError "the invocation step did not take the leaf"
    let mut entries := #[]
    for (goal, origin, clauses, _) in final.pending do
      entries := entries.push
        ⟨origin, clauses, final.goalOwners.getD goal #[], ← leafKey goal origin clauses #[]⟩
    queued.set entries
    setGoals []
  return (← queued.get, ← budgets.get)

/-- The leaf: a value the proof does not see, a contract of its invocations,
and an argument of the declared type whose fact the leaf does not state. -/
abbrev openAtom :=
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) [Skolems unit]
    (shared : Sharing)
    (function : @NTy.carrier Skolems.toCarriers (NTy.function row shared shape.row))
    (count : @NTy.carrier Skolems.toCarriers (.int 64 false))
    (x : @NTy.carrier Skolems.toCarriers Guarded)
    (initial : Memory unit) (P : Memory unit → Prop)
    (_contract : @MethodContract unit _ executable _ row shared shape function),
    wp (@closureMeaning unit _ executable _ row shared shape function (count, x, ()))
      (fun _ final => P final) (fun _ => True) initial

/-- The same leaf with the argument's fact in hand. -/
abbrev paidAtom :=
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) [Skolems unit]
    (shared : Sharing)
    (function : @NTy.carrier Skolems.toCarriers (NTy.function row shared shape.row))
    (count : @NTy.carrier Skolems.toCarriers (.int 64 false))
    (x : @NTy.carrier Skolems.toCarriers Guarded)
    (initial : Memory unit) (P : Memory unit → Prop)
    (_held : @NTy.Holds Skolems.toCarriers Guarded x)
    (_contract : @MethodContract unit _ executable _ row shared shape function),
    wp (@closureMeaning unit _ executable _ row shared shape function (count, x, ()))
      (fun _ final => P final) (fun _ => True) initial

/-- Reports the queue of one run as lines a control compares. -/
elab "#guard_invocation_queue " expected:str,* " in " statement:term : command =>
    liftTermElabM do
  let statement ← whnfR (← instantiateMVars (← Term.elabType statement))
  Term.synthesizeSyntheticMVarsNoPostponing
  let (queued, ledger) ← runInvocationStep statement 400000 90000
  let lines := queued.map fun entry =>
    let origin := match entry.origin with
      | some (.invocationArgument value call position) =>
          s!"invocation argument {position} of `{value}`{if call.isSome then s!" at call {call.get!}" else ""}"
      | some other => other.describe
      | none => "no origin"
    let owners := entry.owners.map (·.2)
    s!"{origin}; key {entry.key.kind}{entry.key.index.map (s!" {·}") |>.getD ""}; \
      clauses {(entry.clauses.map (·.startByte)).toList}; owners {owners.toList}"
  let wanted := expected.getElems.toList.map (·.getString)
  unless lines.toList == wanted do
    throwError "expected the queue {wanted}, got {lines.toList}"
  -- One owner identity for every queued goal, carrying the debit the step
  -- inherited: no owner was allocated for a residual, and the leaf and the
  -- preparation the step was charged for are still spent.
  unless ledger.owners.size == 1 do
    throwError "the step allocated {ledger.owners.size} owners, not one"
  unless ledger.owners[0]!.spent == 490000 && ledger.owners[0]!.preparationSpent == 90000 do
    throwError "the inherited debit is {ledger.owners[0]!.spent} spent, \
      {ledger.owners[0]!.preparationSpent} of it preparation, not 490000 and 90000"
  -- The allowance is one: charged to its limit through any one of the queued
  -- goals, every other queued goal observes none left.
  let remaining := (ledger.allowance #[0]).getD 0
  let exhausted := ledger.charge #[0] .leaf remaining
  unless exhausted.owners.size == 1 do
    throwError "exhausting one goal's owner allocated another"
  for entry in queued do
    unless exhausted.allowance (entry.owners.map (·.2)) == some 0 do
      throwError "a queued goal kept an allowance of \
        {exhausted.allowance (entry.owners.map (·.2))} after its owner was exhausted"

/-! The argument of the declared type is the second of the row: its atom is
queued under the invocation's own origin at that position, and the goals the
leaf goes on with keep the step's origin.  All of them carry the clause
range and the single owner the step inherited. -/

#guard_invocation_queue
  "invocation argument 2 of `function`; key argument 2; clauses [7]; owners [0]",
  "the continuation after call 3 of `sibling`; key data_invariant 1; clauses [7]; owners [0]" in openAtom

/-! With the argument's fact in hand nothing is owed: the queue holds the
goals the leaf goes on with and no residual. -/

#guard_invocation_queue
  "the continuation after call 3 of `sibling`; key data_invariant 1; clauses [7]; owners [0]" in paidAtom

/-! ## Several atoms at one position

A row position whose fact reads several atoms owes each of them at that
position: the number the origin carries is the argument's and never the
atom's ordinal.  The second position here is a tuple of two values of the
declared type, so it owes two atoms, both at position 2.
-/

abbrev pairRow : NRow :=
  .cons (.int 64 false) (.cons (.tuple (.cons Guarded (.cons Guarded .nil))) .nil)

abbrev twoClauses :=
  ∀ (unit : Validation.ValidatedUnit) [FunctionCarriers unit]
    (executable : Validation.ExecutableUnit unit) [Skolems unit]
    (shared : Sharing)
    (function : @NTy.carrier Skolems.toCarriers (NTy.function pairRow shared shape.row))
    (count : @NTy.carrier Skolems.toCarriers (.int 64 false))
    (x y : @NTy.carrier Skolems.toCarriers Guarded)
    (initial : Memory unit) (P : Memory unit → Prop)
    (_contract : @MethodContract unit _ executable _ pairRow shared shape function),
    wp (@closureMeaning unit _ executable _ pairRow shared shape function (count, (x, y, ()), ()))
      (fun _ final => P final) (fun _ => True) initial

#guard_invocation_queue
  "invocation argument 2 of `function`; key argument 2; clauses [7]; owners [0]",
  "invocation argument 2 of `function`; key argument 2; clauses [7]; owners [0]",
  "the continuation after call 3 of `sibling`; key data_invariant 1; clauses [7]; owners [0]" in twoClauses

/-! ## The owned transaction a leaf step runs in

A leaf step runs inside the boundary `runOwned` gives it: the tactic state
and the scheduler's state commit together, the caches a failed attempt
restores are the walk's (`saveWalkCaches`, through `runAutomatic`), and the
work a failed attempt did stays debited to its owners.  The control performs
a real invocation step inside that boundary — the step that queues the
atom its argument owes — and raises in its own action afterwards.
-/

/-- The caches a failed attempt restores, as far as they can be read: the
four sized ones and whether a preparation cache has been installed.  The
`PreparationCache.Cache` value exposes no size, its entries being private. -/
def cacheSizes : CoreM (Array (String × Nat)) := do
  let state ← targetState
  return #[("assertedBounds", (← state.assertedBounds.get).size),
    ("contextRules", (← state.contextRules.get).size),
    ("assumedClauses", (← state.assumedClauses.get).size),
    ("assumedSlotSizes", (← state.assumedSlotSizes.get).size),
    ("preparationProofs", if (← state.preparationProofs.get).isSome then 1 else 0)]

/-- Runs the invocation step on the leaf `statement` states inside the owned
boundary, under one owner that has already spent.  With `fails` the action
raises after the step: the attempt is refused, the queue and the owners of
its goals are the ones it started from, the proof is unchanged, and the work
stays debited.  With `commits` the same action returns and the queue holds
the atom and what follows the invocation. -/
elab "#guard_owned_leaf_step " mode:(&"fails" <|> &"commits") " in " statement:term :
    command => liftTermElabM do
  let statement ← whnfR (← instantiateMVars (← Term.elabType statement))
  Term.synthesizeSyntheticMVarsNoPostponing
  let goal ← mkFreshExprSyntheticOpaqueMVar statement
  let (_, goal) ← goal.mvarId!.introNP statement.getNumHeadForalls
  let raises := (mode.raw.reprint.getD "").trimAscii.toString == "fails"
  let charged := ((({} : Ledger).allocate 20000000).1.charge #[0] .leaf 400000).charge #[0]
    .preparation 90000
  let budgets ← IO.mkRef charged
  let _ ← Tactic.run goal do
    let normalization ← simpContextOf (← `(tactic| simp only []))
    let started ← IO.getNumHeartbeats
    (← targetState).invariantKeys.set #[(inherited, "data_invariant_1")]
    let ctx : SolveContext := {
      invariants := #[]
      callees := #[]
      equations := #[]
      handOver := false
      labeled := false
      unrolls := #[]
      preconditions := #[]
      names := #[]
      ghostWrites := #[]
      debug := false
      profile := false
      shared := true
      budgets := budgets
      normalization := normalization
      startHeartbeats := started }
    let step : SolveStep :=
      { goal := goal, provenance := some stepOrigin, clauses := #[inherited],
        stageStart := started, callSite := some 3 }
    let state : SolveState := { shared := true, activeOwners := #[([], 0)] }
    let caches ← cacheSizes
    let goals ← getGoals
    let owing ← IO.mkRef 0
    let action : SolveM Unit := do
      unless ← invocationSteps ctx step do throwError "the invocation step did not take the leaf"
      owing.set (← get).pending.size
      if (← get).pending.isEmpty then throwError "the step queued nothing"
      -- The step itself writes none of the caches a failed attempt restores,
      -- so the action writes one of them, to see it restored: a slot size
      -- the walk would have assumed.  That it was written inside the action
      -- is checked here, where the attempt still holds.
      (← targetState).assumedSlotSizes.modify (·.push (7, 9))
      unless (← (← targetState).assumedSlotSizes.get).contains (7, 9) do
        throwError "the action did not write the cache it means to see restored"
      if raises then throwError "the action of an owned leaf step fails"
    let (outcome, final) ← (runOwned ctx #[([], 0)] action).run state
    unless (← owing.get) == 2 do
      throwError "the step queued {← owing.get} goals inside the transaction, not two"
    unless (← budgets.get).owners[0]!.spent > 490000 do
      throwError "the transaction's work was refunded: {(← budgets.get).owners[0]!.spent} spent"
    -- The cache the action wrote: a refused attempt restores it, a
    -- returning one keeps what it built.
    let touched ← cacheSizes
    if raises then
      unless touched == caches do
        throwError "the caches of the refused attempt were not restored: \
          {touched} against {caches}"
    else
      let kept := caches.map fun (name, size) =>
        if name == "assumedSlotSizes" then (name, size + 1) else (name, size)
      unless touched == kept do
        throwError "the returning attempt lost the cache it built: \
          {touched} against {kept}"
    if raises then
      match outcome with
      | .failed _ => pure ()
      | _ => throwError "the failing action was accepted"
      unless final.pending.isEmpty do
        throwError "the queue kept {final.pending.size} goals of the refused attempt"
      unless final.goalOwners.isEmpty do throwError "the refused attempt kept its goals' owners"
      unless (← getGoals) == goals do throwError "the goals were not restored"
      if ← goal.isAssigned then throwError "an assignment of the refused attempt survived"
    else
      match outcome with
      | .done _ => pure ()
      | _ => throwError "the returning action was refused"
      unless final.pending.size == 2 do
        throwError "the committed queue holds {final.pending.size} goals, not two"
      unless final.pending.all (fun (goal, _, _, _) =>
          (final.goalOwners.getD goal #[]).map (·.2) == #[0]) do
        throwError "a committed goal does not carry the owner the step inherited"
      unless ← goal.isAssigned do throwError "the committed step left the leaf unchanged"
    setGoals []

/-! The atom the second argument owes is queued inside the transaction. A
failure after it leaves no queued goal, no owner of one, and no assignment,
and the work it did is still debited; the returning counterpart keeps the
atom and what follows the invocation, both under the inherited owner.

The invocation step writes none of the caches a failed attempt restores —
measured: the four sized ones and the preparation cache's installation are
equal across it — so the action writes one of them itself, a slot size the
walk would have assumed, and checks inside the attempt that it is there.
The refused attempt restores it and the returning one keeps it, which is
what the boundary owes the walk: work a leaf step committed is not thrown
away with it. -/

#guard_owned_leaf_step fails in openAtom
#guard_owned_leaf_step commits in openAtom

end VerexIR.Tests.InvocationResidualScheduling
