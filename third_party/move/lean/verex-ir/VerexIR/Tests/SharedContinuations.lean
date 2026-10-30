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

example (value : Nat) (entry : computedEntry value value) : True := by
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
  node ← sealNode node
  if memory.isSome && node.summaryArguments.any (·.parameter == node.captures + 2) then
    throwError "the summary generalized unchanged memory"
  let some summary := node.summary? | throwError "no summary"
  let (_, goal) ← summary.introNP (node.summaryArguments.size + 1)
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

variable {unit : Validation.ValidatedUnit} [Skolems unit]
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
