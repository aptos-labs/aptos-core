-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.SharedContinuations

/-! Kernel-checked edge construction, including dependent captures and memory.
The source-level scheduler fixtures live in `VerexSL.Tests.ContinuationJoins`. -/

open Lean Meta Elab Tactic
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open VerexIR.Proofs.Denote.SharedContinuations

-- ============================================================================
-- Section 1: Entry Relation Exposure & Conflict Pruning (`exposeEntries`)
-- ============================================================================

section EntryRelationTests

-- A captured path condition conflicts with the linked outgoing row.
-- Reject it before distributing the earlier entry's two alternatives.
private def earlierEntry (choice : Bool) : Prop := choice = true ∨ choice = false
private def linkedEntry (slot : Option Nat) (choice : Bool) : Prop :=
  ∃ (_ : earlierEntry choice) (value : Option Nat), ∃ (_ : value = none), value = slot
opaque entryComputation (value : Nat) : Nat := value
private def computedEntry (left right : Nat) : Prop :=
  (some (entryComputation left), entryComputation right) =
    (some (entryComputation right), entryComputation left)

opaque opaqueOpt (k : Nat) : Option Nat := some k
private def mirroredConflictEntry (k : Nat) (choice : Bool) : Prop :=
  ∃ (_ : earlierEntry choice) (_ : opaqueOpt k = some 1), none = opaqueOpt k
private def forwardConflictEntry (k : Nat) (choice : Bool) : Prop :=
  ∃ (_ : earlierEntry choice) (_ : opaqueOpt k = some 1), opaqueOpt k = none

attribute [irreducible] earlierEntry linkedEntry computedEntry
  mirroredConflictEntry forwardConflictEntry

elab "check_entry_cases " count:num : tactic => do
  let goals ← exposeEntries (← getMainGoal) #[``linkedEntry, ``earlierEntry]
  unless goals.length == count.getNat do
    throwError "expected {count.getNat} incoming alternatives, got {goals.length}"
  setGoals goals

elab "open_computed_entry" : tactic => do
  setGoals (← exposeEntries (← getMainGoal) #[``computedEntry])

-- Negative 1: conflicting path condition prunes both predecessors (`0` alternatives).
example (choice : Bool) (entry : linkedEntry (some 1) choice) : False := by
  check_entry_cases 0

-- Negative 2: transitive constructor conflict in `closeImpossibleEntry`
-- (mirrored orientation `x = C₁ …` followed by `C₂ … = x` on non-variable `opaqueOpt k`).
example (k : Nat) (choice : Bool) (entry : mirroredConflictEntry k choice) : False := by
  run_tac do
    let goals ← exposeEntries (← getMainGoal) #[``mirroredConflictEntry, ``earlierEntry]
    unless goals.isEmpty do
      throwError "expected 0 surviving alternatives, got {goals.length}"
    setGoals goals

-- Negative 3: transitive constructor conflict in forward orientation (`x = C₁ …`, `x = C₂ …`).
example (k : Nat) (choice : Bool) (entry : forwardConflictEntry k choice) : False := by
  run_tac do
    let goals ← exposeEntries (← getMainGoal) #[``forwardConflictEntry, ``earlierEntry]
    unless goals.isEmpty do
      throwError "expected 0 surviving alternatives, got {goals.length}"
    setGoals goals

-- Negative 4: non-conflicting reflexive `computedEntry` does not fabricate `False`.
example (value : Nat) (_entry : computedEntry value value) : True := by
  open_computed_entry
  fail_if_success contradiction
  trivial

-- Positive 1: compatible row keeps both possible predecessors.
example (choice : Bool) (entry : linkedEntry none choice) : choice = true ∨ choice = false := by
  check_entry_cases 2
  all_goals simp

-- Positive 2: constructor injection preserves equalities between opaque computations
-- for ordinary reasoning instead of trying dependent elimination on them.
example (left right : Nat) (entry : computedEntry left right) :
    entryComputation left = entryComputation right := by
  open_computed_entry
  assumption

end EntryRelationTests

-- ============================================================================
-- Section 2: Shared Diamond Continuations & Single-Exit Classification
-- ============================================================================

section SharedDiamondTests

-- Deliberately small harness: collect predecessors, prove the summary and
-- connect its theorem to every original proof destination.
elab "check_shared " memory:("keeping_memory ")? closed:("without_entries ")?
    incoming:tacticSeq " with " suffix:tacticSeq : tactic => do
  let action ← (← getMainGoal).withContext do
    return (← instantiateMVars (← (← getMainGoal).getType)).consumeMData.getArg! 3
  unless ← mayJoin action do throwError "a visible diamond was not recognized"
  let some (folded, node) ← fold? (← getMainGoal) | throwError "not a Flow continuation"
  setGoals [folded]
  evalTactic incoming
  let mut node := node
  for goal in ← getGoals do
    let some arrival ← arrival? node goal | throwError "not a normal arrival: {← goal.getType}"
    node := { node with arrivals := node.arrivals.push arrival }
  node ← sealNode (← closeArrivals (← prepareArrivals node {}))
  if memory.isSome && node.summaryArguments.any (·.parameter == node.captures + 2) then
    throwError "the summary generalized unchanged memory"
  let some summary := node.summary? | throwError "no summary"
  let (_, goal) ← summary.introNP (node.summaryArguments.size + node.hypotheses + 1)
  let goal ← VerexIR.Proofs.Denote.SharedContinuations.unfold node goal
  let goals ← if closed.isSome then pure [goal]
    else exposeEntries goal #[node.relation?.get!.constName!]
  setGoals goals
  evalTactic suffix
  unless (← getUnsolvedGoals).isEmpty do throwError "shared summary incomplete"
  let some finished ← finish? node | throwError "summary has unresolved dependencies"
  unless finished.finished && finished.certified do throwError "summary not certified and connected"
  for arrival in node.arrivals do
    unless ← arrival.goal.isAssigned do throwError "lost incoming proof destination"
  setGoals []

-- Failure and abrupt-flow arms bypass the normal continuation. A matcher
-- at a read and a conditional inside a value do not justify a shared node.
elab "check_single_exit" : tactic => do
  let action ← (← getMainGoal).withContext do
    return (← instantiateMVars (← (← getMainGoal).getType)).consumeMData.getArg! 3
  if ← mayJoin action then throwError "an action without a normal join allocated a candidate"

set_option maxHeartbeats 1000000
set_option Elab.async false

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
variable {ρ : ResultShape} {Γ : NRow}

-- Negative 1 (first): conjoining incompatible predecessor assumptions would
-- prove this false postcondition; the generated disjunction must keep it unproved.
/-- error: shared summary incomplete -/
#guard_msgs in
example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value (1 : Nat) env) else Spec.pure (.value 2 env))
      (fun (_ : Flow ρ Γ Nat) _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    all_goals try simp_all

-- Negative 2: abort arm bypasses normal continuation (`mayJoin` is false).
example (slot : Option Nat) (error : Failure) (env : HEnv Γ) (state : Memory unit) :
    wp (match slot with
      | none => Spec.abort error
      | some value => Spec.pure (.value value env))
      (fun (_ : Flow ρ Γ Nat) _ => True) (fun _ => True) state := by
  check_single_exit
  cases slot <;> simp [wp_abort, wp_pure]

-- Negative 3: `break_` arm bypasses normal continuation (`mayJoin` is false).
example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.break_ 0 env) else Spec.pure (.value (2 : Nat) env))
      (fun (_ : Flow ρ Γ Nat) _ => True) (fun (_ : Failure) => False) state := by
  check_single_exit
  cases b <;> simp [wp_pure]

-- Negative 4: value-level conditional does not justify a shared node (`mayJoin` is false).
example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    wp (Spec.pure (.value (if b then (1 : Nat) else 2) env))
      (fun (_ : Flow ρ Γ Nat) _ => True) (fun (_ : Failure) => False) state := by
  check_single_exit
  simp [wp_pure]

-- Positive 1: dependent telescope preservation across branch substitution.
example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value (1 : Nat) env) else Spec.pure (.value 2 env))
      (fun (flow : Flow ρ Γ Nat) final => match flow with
        | .value n locals => locals = env ∧ final = state ∧
            ((b = true ∧ n = 1) ∨ (b = false ∧ n = 2))
        | _ => False)
      (fun (_ : Failure) => False) state := by
  -- A fact unused by the body stays in the enclosing proof context.
  have witnessed : True := True.intro
  -- Saved computations unused by the named suffix stay out of its entry
  -- witnesses, including aliases that otherwise keep their bodies alive.
  let discarded := fun (n : Nat) =>
    wp (Spec.pure n) (fun _ (_ : Memory unit) => True) (fun (_ : Failure) => False) state
  let discardedAlias := discarded
  check_shared keeping_memory
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    all_goals have _ := witnessed
    all_goals fail_if_success have _ := discarded
    all_goals fail_if_success have _ := discardedAlias
    all_goals simp_all

-- Positive 2: structural slot facts read without distributing entry relation.
example (b : Bool) (left right stable : String) (state : Memory unit) :
    wp (if b then Spec.pure (.value (1 : Nat) (some left, some stable, PUnit.unit))
      else Spec.pure (.value 2 (some right, some stable, PUnit.unit)))
      (fun (flow : Flow ρ (.cons .address (.cons .address .nil)) Nat) final => match flow with
        | .value _ locals => locals.1.isSome = true ∧ locals.2.1 = some stable ∧ final = state
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared keeping_memory without_entries
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    all_goals simp

private def EntryPair (α : Type) := α × α

-- Positive 3: preserve nominal type argument of `some` when its payload is shaped.
example {α : Type} (b : Bool) (stable left right : α) (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value (some (stable, left) : Option (EntryPair α)) env)
      else Spec.pure (.value (some (stable, right) : Option (EntryPair α)) env))
      (fun (flow : Flow ρ Γ (Option (EntryPair α))) _ => match flow with
        | .value value _ => value.elim False (fun pair => pair.1 = stable)
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared keeping_memory without_entries
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    all_goals simp only [Option.elim_some]

-- Positive 4: `have`-bound data generalized as capture argument.
example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    ∃ bound : Nat, 2 ≤ bound ∧
      wp (if b then Spec.pure (.value (1 : Nat) env) else Spec.pure (.value 2 env))
        (fun (flow : Flow ρ Γ Nat) final => match flow with
          | .value n _ => n ≤ bound ∧ final = state
          | _ => False)
        (fun (_ : Failure) => False) state := by
  have bound : { n : Nat // 2 ≤ n } := ⟨3, by decide⟩
  refine ⟨bound.val, bound.property, ?_⟩
  check_shared keeping_memory
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    all_goals have lower := bound.property
    all_goals simp_all
    all_goals omega

-- Positive 5: distinct predecessor memory writes keep let-bound captured observation.
example (b : Bool) (env : HEnv Γ) (initial left right : Memory unit) :
    let old := initial
    wp (if b then Spec.bind (Spec.set left) (fun _ => Spec.pure (.value (1 : Nat) env))
      else Spec.bind (Spec.set right) (fun _ => Spec.pure (.value 2 env)))
      (fun (flow : Flow ρ Γ Nat) final => match flow with
        | .value n locals => old = initial ∧ locals = env ∧
            ((b = true ∧ n = 1 ∧ final = left) ∨ (b = false ∧ n = 2 ∧ final = right))
        | _ => False)
      (fun (_ : Failure) => False) initial := by
  intro old
  check_shared
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_bind, wp_set, wp_pure]
  with
    all_goals simp_all

-- Positive 6: closing a summary retains definitions on which later local types depend.
example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    let limit := 3
    let bound : Fin limit := ⟨2, by decide⟩
    wp (if b then Spec.pure (.value (1 : Nat) env) else Spec.pure (.value 2 env))
      (fun (flow : Flow ρ Γ Nat) _ => match flow with
        | .value n _ => n ≤ bound.val ∧ bound.val < limit
        | _ => False)
      (fun (_ : Failure) => False) state := by
  intro limit bound
  check_shared keeping_memory
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    all_goals simp_all

end SharedDiamondTests

-- ============================================================================
-- Section 3: Contract `validArgs` / `validResult` Over Single-Carrier `HList`
-- ============================================================================

section ContractDomainTests

private def positiveContract : Contract Unit Unit Nat Nat where
  requires := fun _ _ => True
  ensures := fun _ _ _ _ => True
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun n => n > 0
  validResult := fun r => r > 0

private abbrev paramRow : NRow := .cons (.param 0) .nil
private abbrev paramShape : ResultShape := .one (.param 0)

private def paramIdentityContract [Carriers] :
    Contract Unit Failure (HList paramRow) paramShape.carrier where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := HList.Holds paramRow
  validResult := paramShape.Holds

-- Negative 1 (first): outside `validArgs`, the summary is undefined so `wp`
-- fails even when `requires` holds.
example : ¬wp (positiveContract.summary 0) (fun _ _ => True) (fun _ => True) () := by
  rw [wp_summary]
  rintro ⟨hvalid, _⟩
  exact Nat.lt_irrefl 0 hvalid

-- Negative 2: returning an invalid result fails `Satisfies` even when `ensures`
-- and `frame` hold.
example : ¬Satisfies (fun _ => Spec.pure 0) positiveContract := by
  intro verified
  have h := (verified 1 () (show 1 > 0 by decide) trivial trivial).1 0 () ⟨rfl, rfl⟩
  exact Nat.lt_irrefl 0 h.1

-- Negative 3: over single-carrier `HList`, `paramIdentityContract.summary`
-- refuses an argument lacking `NTy.Holds`.
example [Carriers] (x : (NTy.param 0).carrier) (bad : ¬NTy.Holds (.param 0) x) :
    ¬wp (paramIdentityContract.summary (x, ())) (fun _ _ => True) (fun _ => True) () := by
  rw [wp_summary]
  rintro ⟨hvalid, _⟩
  exact bad hvalid.1

-- Negative 4: returning an unvalidated `badRes` (`¬NTy.Holds (.param 0) badRes`)
-- on a valid input `goodArg` fails `Satisfies` even when `ensures` is `True`.
example [Carriers] (goodArg badRes : (NTy.param 0).carrier)
    (hGood : NTy.Holds (.param 0) goodArg) (hBad : ¬NTy.Holds (.param 0) badRes) :
    ¬Satisfies
        (fun _ : HList paramRow => (Spec.pure badRes : Spec Unit Failure paramShape.carrier))
        { paramIdentityContract with ensures := fun _ _ _ _ => True } := by
  intro verified
  have h := (verified (goodArg, ()) () ⟨hGood, trivial⟩ trivial trivial).1 badRes () ⟨rfl, rfl⟩
  exact hBad h.1

-- Positive 1: `wp_summary` demands `validArgs` and supplies `validResult` to the continuation.
example (n : Nat) (hn : n > 0) :
    wp (positiveContract.summary n) (fun r _ => r > 0) (fun _ => False) () := by
  rw [wp_summary]
  exact ⟨hn, trivial, fun _ _ hr _ _ _ => hr, fun _ h => h⟩

-- Positive 2: `Spec.pure args.1` satisfies `paramIdentityContract` on single-carrier
-- `HList paramRow` because `validArgs` (`HList.Holds paramRow args`) supplies
-- `NTy.Holds (.param 0) args.1` directly to `validResult` (`paramShape.Holds`).
example [Carriers] :
    Satisfies
      (fun (args : HList paramRow) => (Spec.pure args.1 : Spec Unit Failure paramShape.carrier))
      paramIdentityContract := by
  intro args _ valid _ _
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    rcases execution with ⟨rfl, rfl⟩
    exact ⟨valid.1, fun _ => rfl, rfl, fun impossible => impossible⟩
  · intro _ impossible
    exact impossible
  · intro impossible
    exact impossible

end ContractDomainTests

-- ============================================================================
-- Section 4: Mandatory Join `Holds` Obligations Over Single-Carrier Values
-- ============================================================================

section JoinHoldsTests

set_option maxHeartbeats 1000000
set_option Elab.async false

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
variable {ρ : ResultShape} {Γ : NRow}

-- Harness verifying `holdsCount` on sealed nodes and checking refusal-to-seal (`none`).
elab "check_shared_holds " expectedHolds:num " : "
    incoming:tacticSeq " with " suffix:tacticSeq : tactic => do
  let some (folded, node) ← fold? (← getMainGoal) | throwError "not a Flow continuation"
  setGoals [folded]
  evalTactic incoming
  let mut node := node
  for goal in ← getGoals do
    let some arrival ← arrival? node goal | throwError "not a normal arrival"
    node := { node with arrivals := node.arrivals.push arrival }
  let sealedNode ← sealNode (← closeArrivals (← prepareArrivals node {}))
  unless sealedNode.holdsCount == expectedHolds.getNat do
    throwError "expected holdsCount = {expectedHolds.getNat}, got {sealedNode.holdsCount}"
  let some summary := sealedNode.summary? | throwError "no summary"
  let (_, goal) ← summary.introNP (sealedNode.summaryArguments.size + sealedNode.hypotheses + 1)
  let goal ← VerexIR.Proofs.Denote.SharedContinuations.unfold sealedNode goal
  setGoals [goal]
  evalTactic suffix
  unless (← getUnsolvedGoals).isEmpty do throwError "shared summary incomplete"
  let some finished ← finish? sealedNode | throwError "summary has unresolved dependencies"
  unless finished.finished && finished.certified do throwError "summary not certified"
  setGoals []

elab "check_pending_holds " expectedHolds:num expectedPending:num " : " incoming:tacticSeq : tactic => do
  let saved ← Lean.Elab.Tactic.saveState
  let some (folded, node) ← fold? (← getMainGoal) | throwError "not a Flow continuation"
  setGoals [folded]
  evalTactic incoming
  let mut node := node
  for goal in ← getGoals do
    let some arrival ← arrival? node goal | throwError "not a normal arrival"
    node := { node with arrivals := node.arrivals.push arrival }
  let sealedNode ← sealNode (← closeArrivals (← prepareArrivals node {}))
  unless sealedNode.sealed do
    throwError "sealNode did not seal the node"
  unless sealedNode.holdsCount == expectedHolds.getNat do
    throwError "expected holdsCount = {expectedHolds.getNat}, got {sealedNode.holdsCount}"
  unless sealedNode.pendingHolds.size == expectedPending.getNat do
    throwError "expected pendingHolds.size = {expectedPending.getNat}, got {sealedNode.pendingHolds.size}"
  unless sealedNode.pendingHolds[0]!.1 == 0 do
    throwError "expected pending obligation at arrival 0, got arrival {sealedNode.pendingHolds[0]!.1}"
  if (← instantiateMVars sealedNode.holdsProofs[1]![0]!).hasExprMVar then
    throwError "arrival 1's Holds proof should be closed and unaffected, but has open mvars"
  if (← finish? sealedNode).isSome then
    throwError "finish? certified a summary while an arrival's Holds obligation was still open!"
  saved.restore

-- Negative 1 (first): `cases b` puts `false` (`right`, missing `NTy.Holds`) at
-- arrival 0 and `true` (`left`, carrying `_hLeft`) at arrival 1. Because no theorem
-- derives `NTy.Holds` from bare `(NTy.param 0).carrier` alone, `sealNode` emits 1
-- mandatory `Holds` premise (`holdsCount = 1`), records arrival 0's open obligation
-- in `pendingHolds`, keeps arrival 1's proof closed, and `finish?` refuses to certify.
example (b : Bool) (left right : (NTy.param 0).carrier)
    (_hLeft : NTy.Holds (.param 0) left)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value left env) else Spec.pure (.value right env))
      (fun (_ : Flow ρ Γ (NTy.param 0).carrier) _ => True)
      (fun (_ : Failure) => False) state := by
  check_pending_holds 1 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  cases b <;> simp [wp_pure]

-- Negative 2: joining an empty local slot `none` (arrival 1) with an unvalidated
-- `some right` (arrival 0, missing `NTy.Holds`) emits `Slot.Holds (.param 0)`
-- (`holdsCount = 1`), closes `none` at arrival 1, leaves arrival 0 pending, and
-- `finish?` refuses to certify.
example (b : Bool) (right : (NTy.param 0).carrier) (state : Memory unit) :
    wp (if b then Spec.pure (.value () (none, PUnit.unit))
        else Spec.pure (.value () (some right, PUnit.unit)))
      (fun (_ : Flow ρ (.cons (.param 0) .nil) Unit) _ => True)
      (fun (_ : Failure) => False) state := by
  check_pending_holds 1 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  cases b <;> simp [wp_pure]

-- Positive 1: non-`invariantFree` variable (`(NTy.param 0).carrier`) WITH `NTy.Holds`
-- at both arrivals: `sealNode` emits 1 mandatory `Holds` premise, proves it from `LCtx`
-- at each arrival, and supplies it to the shared suffix without splitting the entry relation.
example (b : Bool) (left right : (NTy.param 0).carrier)
    (hLeft : NTy.Holds (.param 0) left) (hRight : NTy.Holds (.param 0) right)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value left env) else Spec.pure (.value right env))
      (fun (flow : Flow ρ Γ (NTy.param 0).carrier) _ => match flow with
        | .value v _ => NTy.Holds (.param 0) v
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- Positive 2: mutable reference `(NTy.ref (.param 0)).carrier` where open currents
-- `curLeft`, `curRight` lack `NTy.Holds` while prophecies `prophLeft`, `prophRight`
-- satisfy `NTy.Holds`. `sealNode` checks ONLY the prophecy (`holdsCount = 1`),
-- NEVER the open current!
example (b : Bool) (curLeft curRight prophLeft prophRight : (NTy.param 0).carrier)
    (hProphLeft : NTy.Holds (.param 0) prophLeft) (hProphRight : NTy.Holds (.param 0) prophRight)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value ((curLeft, prophLeft) : (NTy.ref (.param 0)).carrier) env)
        else Spec.pure (.value ((curRight, prophRight) : (NTy.ref (.param 0)).carrier) env))
      (fun (flow : Flow ρ Γ (NTy.ref (.param 0)).carrier) _ => match flow with
        | .value r _ => NTy.Holds (.param 0) r.2
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- Positive 3: `Slot.Holds` across `none` and `some right` (carrying `NTy.Holds`):
-- `sealNode` emits 1 `Slot.Holds` premise (`holdsCount = 1`), closes both arrivals,
-- and supplies `Slot.Holds (.param 0) locals.1` to the continuation.
example (b : Bool) (right : (NTy.param 0).carrier)
    (hRight : NTy.Holds (.param 0) right) (state : Memory unit) :
    wp (if b then Spec.pure (.value () (none, PUnit.unit))
        else Spec.pure (.value () (some right, PUnit.unit)))
      (fun (flow : Flow ρ (.cons (.param 0) .nil) Unit) _ => match flow with
        | .value _ locals => Slot.Holds (.param 0) locals.1
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- Positive 4: `invariantFree` scalar `(NTy.int 64 false).carrier` emits zero
-- `Holds` premises (`holdsCount = 0`).
example (b : Bool) (left right : (NTy.int 64 false).carrier)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value left env) else Spec.pure (.value right env))
      (fun (_ : Flow ρ Γ (NTy.int 64 false).carrier) _ => True)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

end JoinHoldsTests
