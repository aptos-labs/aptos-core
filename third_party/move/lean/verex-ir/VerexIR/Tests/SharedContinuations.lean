-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.SharedContinuations

/-! Kernel-checked edge construction, including dependent captures and memory.
The source-level scheduler fixtures live in `VerexSL.Tests.ContinuationJoins`. -/

open Lean Meta Elab Tactic
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open VerexIR.Proofs.Denote.SharedContinuations

-- A captured path condition conflicts with the linked outgoing row.
-- Reject it before distributing the earlier entry's two alternatives.
private def earlierEntry (choice : Bool) : Prop := choice = true ∨ choice = false
private def linkedEntry (slot : Option Nat) (choice : Bool) : Prop :=
  ∃ (_ : earlierEntry choice) (value : Option Nat), ∃ (_ : value = none), value = slot
opaque entryComputation (value : Nat) : Nat := value
private def computedEntry (left right : Nat) : Prop :=
  (some (entryComputation left), entryComputation right) =
    (some (entryComputation right), entryComputation left)

attribute [irreducible] earlierEntry linkedEntry computedEntry

elab "check_entry_cases " count:num : tactic => do
  let goals ← exposeEntries (← getMainGoal) #[``linkedEntry, ``earlierEntry]
  unless goals.length == count.getNat do
    throwError "expected {count.getNat} incoming alternatives, got {goals.length}"
  setGoals goals

example (choice : Bool) (entry : linkedEntry (some 1) choice) : False := by
  check_entry_cases 0

-- The same entry with a compatible row keeps both possible predecessors.
example (choice : Bool) (entry : linkedEntry none choice) : choice = true ∨ choice = false := by
  check_entry_cases 2
  all_goals simp

-- Constructor injection preserves equalities between opaque computations
-- for ordinary reasoning instead of trying dependent elimination on them.
elab "open_computed_entry" : tactic => do
  setGoals (← exposeEntries (← getMainGoal) #[``computedEntry])

example (left right : Nat) (entry : computedEntry left right) :
    entryComputation left = entryComputation right := by
  open_computed_entry
  assumption

example (value : Nat) (_entry : computedEntry value value) : True := by
  open_computed_entry
  fail_if_success contradiction
  trivial

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

set_option maxHeartbeats 1000000
set_option Elab.async false

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] [Skolems unit]
variable {ρ : ResultShape} {Γ : NRow}

-- Failure and abrupt-flow arms bypass the normal continuation. A matcher
-- at a read and a conditional inside a value do not justify a shared node.
elab "check_single_exit" : tactic => do
  let action ← (← getMainGoal).withContext do
    return (← instantiateMVars (← (← getMainGoal).getType)).consumeMData.getArg! 3
  if ← mayJoin action then throwError "an action without a normal join allocated a candidate"

example (slot : Option Nat) (error : Failure) (env : HEnv Γ) (state : Memory unit) :
    wp (match slot with
      | none => Spec.abort error
      | some value => Spec.pure (.value value env))
      (fun (_ : Flow ρ Γ Nat) _ => True) (fun _ => True) state := by
  check_single_exit
  cases slot <;> simp [wp_abort, wp_pure]

example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.break_ 0 env) else Spec.pure (.value (2 : Nat) env))
      (fun (_ : Flow ρ Γ Nat) _ => True) (fun (_ : Failure) => False) state := by
  check_single_exit
  cases b <;> simp [wp_pure]

example (b : Bool) (env : HEnv Γ) (state : Memory unit) :
    wp (Spec.pure (.value (if b then (1 : Nat) else 2) env))
      (fun (_ : Flow ρ Γ Nat) _ => True) (fun (_ : Failure) => False) state := by
  check_single_exit
  simp [wp_pure]

-- The unit, skolem instance, result shape and local row form a dependent
-- telescope. Branch substitution must preserve their identities and types.
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

-- Both edges keep a slot present while changing its payload. The next slot
-- retains a dominating value. The summary can read those structural facts
-- without distributing its entry relation into predecessor cases.
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

-- Preserve the nominal type argument of `some` when its payload is shaped.
-- Inferring that argument as `α × α` instead of `EntryPair α` leaves this
-- elimination stuck under the simplifier's reducible matching.
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

-- `have` can bind data as well as proofs. This nondependent let is read by
-- the continuation and must be generalized as a capture argument, while
-- ordinary let-bound observations below retain their definitions.
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

-- Each predecessor writes its own memory. A let-bound captured observation
-- retains its definition, and is not generalized to an unrelated value.
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

-- Closing a summary must retain definitions on which later local types
-- depend, even when its proof uses their concrete values by reduction.
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

-- Conjoining incompatible predecessor assumptions would prove this false
-- postcondition. The generated disjunction must keep it unproved.
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

-- ============================================================================
-- Phase 1.6c & 1.6d Tests: Contract validArgs/validResult, atPlain, and Join Holds
-- ============================================================================

section Phase16cdTests

private def positiveContract : Contract Unit Unit Nat Nat where
  requires := fun _ _ => True
  ensures := fun _ _ _ _ => True
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun n => n > 0
  validResult := fun r => r > 0

-- `wp_summary` demands `validArgs` and supplies `validResult` to the continuation.
example (n : Nat) (hn : n > 0) :
    wp (positiveContract.summary n) (fun r _ => r > 0) (fun _ => False) () := by
  rw [wp_summary]
  exact ⟨hn, trivial, fun _ _ hr _ _ _ => hr, fun _ h => h⟩

-- Outside `validArgs`, the summary is undefined so `wp` fails even when `requires` holds.
example : ¬wp (positiveContract.summary 0) (fun _ _ => True) (fun _ => True) () := by
  rw [wp_summary]
  rintro ⟨hvalid, _⟩
  exact Nat.lt_irrefl 0 hvalid

-- Negative test: returning an invalid result fails `Satisfies` even when `ensures` and `frame` hold.
example : ¬Satisfies (fun _ => Spec.pure 0) positiveContract := by
  intro verified
  have h := (verified 1 () (show 1 > 0 by decide) trivial trivial).1 0 () ⟨rfl, rfl⟩
  exact Nat.lt_irrefl 0 h.1

-- `satisfies_atPlain_iff` and `satisfies_specAtPlain_iff` over non-`certificateFree` types.
section PlainContractTests
variable [Carriers]

private abbrev paramRow : NRow := .cons (.param 0) .nil
private abbrev paramShape : ResultShape := .one (.param 0)

private def plainIdentityContract :
    Contract Unit Failure (HListPlain paramRow) paramShape.plain where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args.1
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

example :
    Satisfies (fun (args : HList paramRow) => (Spec.pure args.1 : Spec Unit Failure paramShape.carrier))
      plainIdentityContract.atPlain := by
  rw [satisfies_atPlain_iff]
  intro args holds _ _ _ _
  have hforget : paramShape.forget (NTy.ofHolds (.param 0) args.1 holds.1) = args.1 :=
    ResultShape.forget_ofHolds paramShape args.1 holds.1
  simp [wp_pure, HListPlain.ofHolds, plainIdentityContract]
  exact hforget

example :
    Satisfies (Spec.atPlain (shape := paramShape)
      (fun (args : HList paramRow) => (Spec.pure args.1 : Spec Unit Failure paramShape.carrier)))
      plainIdentityContract.plainValidated := by
  rw [satisfies_specAtPlain_iff, satisfies_atPlain_iff]
  intro args holds _ _ _ _
  have hforget : paramShape.forget (NTy.ofHolds (.param 0) args.1 holds.1) = args.1 :=
    ResultShape.forget_ofHolds paramShape args.1 holds.1
  simp [wp_pure, HListPlain.ofHolds, plainIdentityContract]
  exact hforget

end PlainContractTests

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

-- 1. Certified non-`certificateFree` variable (`NTy.param 0`): `sealNode` emits 1 mandatory `Holds`
-- premise and discharges both arrivals via `NTy.holds_forget`.
example (b : Bool) (left right : (NTy.param 0).carrier) (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value left env) else Spec.pure (.value right env))
      (fun (flow : Flow ρ Γ (NTy.param 0).carrier) _ => match flow with
        | .value v _ => NTy.Holds (.param 0) (NTy.forget (.param 0) v)
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- 2. Plain non-`certificateFree` variable (`(NTy.param 0).plain`) WITH `NTy.Holds` at both arrivals:
-- `sealNode` emits 1 mandatory `Holds` premise, proves it from `LCtx` at each arrival, and supplies
-- it to the shared suffix without re-splitting the entry disjunction!
example (b : Bool) (left right : (NTy.param 0).plain)
    (hLeft : NTy.Holds (.param 0) left) (hRight : NTy.Holds (.param 0) right)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value left env) else Spec.pure (.value right env))
      (fun (flow : Flow ρ Γ (NTy.param 0).plain) _ => match flow with
        | .value v _ => NTy.Holds (.param 0) v
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- 3. Mutable reference `(NTy.ref (.param 0)).plain`: open reference currents
-- `curLeft`, `curRight` lack `NTy.Holds` (they may be open across arrivals!), while prophecies
-- `prophLeft`, `prophRight` satisfy `NTy.Holds`. `sealNode` checks ONLY the prophecy (holdsCount = 1),
-- NEVER the open current!
example (b : Bool) (curLeft curRight prophLeft prophRight : (NTy.param 0).plain)
    (hProphLeft : NTy.Holds (.param 0) prophLeft) (hProphRight : NTy.Holds (.param 0) prophRight)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value ((curLeft, prophLeft) : (NTy.ref (.param 0)).plain) env)
        else Spec.pure (.value ((curRight, prophRight) : (NTy.ref (.param 0)).plain) env))
      (fun (flow : Flow ρ Γ (NTy.ref (.param 0)).plain) _ => match flow with
        | .value r _ => NTy.Holds (.param 0) r.2
        | _ => False)
      (fun (_ : Failure) => False) state := by
  check_shared_holds 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- 4. Mandatory arrival obligation: `cases b` puts `false` (`right`, missing `NTy.Holds`) at
-- arrival 0 and `true` (`left`, carrying `_hLeft`) at arrival 1. `sealNode` seals the join
-- (`holdsCount = 1`), records only arrival 0's open obligation in `pendingHolds`, keeps arrival 1's
-- proof closed and unaffected, and `finish?` refuses to certify until arrival 0's obligation closes.
example (b : Bool) (left right : (NTy.param 0).plain)
    (_hLeft : NTy.Holds (.param 0) left)
    (env : HEnv Γ) (state : Memory unit) :
    wp (if b then Spec.pure (.value left env) else Spec.pure (.value right env))
      (fun (_ : Flow ρ Γ (NTy.param 0).plain) _ => True)
      (fun (_ : Failure) => False) state := by
  check_pending_holds 1 1 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  cases b <;> simp [wp_pure]

-- 5. Transitive constructor conflict in `closeImpossibleEntry` (both orientations on a
-- non-variable term `opaqueOpt k`, including `x = C₁ …` followed by `C₂ … = x`).
opaque opaqueOpt (k : Nat) : Option Nat := some k
private def mirroredConflictEntry (k : Nat) (choice : Bool) : Prop :=
  ∃ (_ : earlierEntry choice) (_ : opaqueOpt k = some 1), none = opaqueOpt k
private def forwardConflictEntry (k : Nat) (choice : Bool) : Prop :=
  ∃ (_ : earlierEntry choice) (_ : opaqueOpt k = some 1), opaqueOpt k = none
attribute [irreducible] mirroredConflictEntry forwardConflictEntry

example (k : Nat) (choice : Bool) (entry : mirroredConflictEntry k choice) : False := by
  run_tac do
    let goals ← exposeEntries (← getMainGoal) #[``mirroredConflictEntry, ``earlierEntry]
    unless goals.isEmpty do
      throwError "expected 0 surviving alternatives, got {goals.length}"
    setGoals goals

example (k : Nat) (choice : Bool) (entry : forwardConflictEntry k choice) : False := by
  run_tac do
    let goals ← exposeEntries (← getMainGoal) #[``forwardConflictEntry, ``earlierEntry]
    unless goals.isEmpty do
      throwError "expected 0 surviving alternatives, got {goals.length}"
    setGoals goals

end Phase16cdTests
