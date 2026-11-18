-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
import VerexIR.Tests.GivenExecution

/-!
The normalizer and walker run real Given bodies and the concrete adapter.
The final proof inspection requires each changed marker rule to occur, so a
simplifier cannot bypass the interface under test.
-/

open VerexIR.Proofs VerexIR.Proofs.Denote
namespace VerexIR.Tests.GivenWalker
open VerexIR.Tests.GivenExecution
set_option maxHeartbeats 100000
set_option maxRecDepth 2000

def immediateLoop (unit : VerexIR.Validation.ValidatedUnit) : Function unit where
  params := .nil
  locals := .nil
  frame := .nil
  frame_eq := rfl
  result := .none
  body := .loop 7 (.break_ 0)
  mutables := .nil
  exitSite := 0

def handle : VerexIR.FunctionHandle := ⟨⟨0⟩, ⟨0⟩⟩

-- Real Given body evaluation and leaves use the ordinary solver.
theorem given_identity (G : GivenFrame) (State : Type) (unit : VerexIR.Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State)
    (value : @NTy.carrier (G.toCarriers).toSorts (.param 0)) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (identity unit) (value, ()))
      (fun result final => result = value ∧ final = initial) (fun _ => False) initial := by
  simp only [identity, Function.denoteIn]
  verex_normalize
  all_goals verex_obligations []
  all_goals verex_solve

-- The invariant is already at the Given family and state; it has no
-- ValidatedUnit or Skolems factory to reconstruct at the loop marker.
theorem given_loop (G : GivenFrame) (State : Type) (unit : VerexIR.Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (immediateLoop unit) ())
      (fun _ final => final = initial) (fun _ => False) initial := by
  simp only [immediateLoop, Function.denoteIn]
  verex_normalize
  verex_obligations [(7, SiteCondition.Kind.loopInvariant, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv G.toCarriers.toSorts .nil) (entry : State)
      (_ : @HEnv G.toCarriers.toSorts .nil) (current : State) => current = entry)]
  all_goals verex_solve

-- Concrete predicates retain their frame explicitly; the result is the
-- same loop invariant under the concrete adapter.
theorem concrete_loop {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [Θ : Skolems unit]
    {executable : VerexIR.Validation.ExecutableUnit unit}
    (meanings : Meanings executable) (initial : Memory unit) :
    wp ((immediateLoop unit).denoteWith meanings ())
      (fun _ final => final = initial) (fun _ => False) initial := by
  simp only [immediateLoop, Function.denoteWith]
  verex_normalize
  verex_obligations [(7, SiteCondition.Kind.loopInvariant, handle, concreteCondition Θ (fun frame =>
    fun (_ : Unit) (_ : Memory unit) (_ : @HEnv frame.toCarriers.toSorts .nil) (entry : Memory unit)
      (_ : @HEnv frame.toCarriers.toSorts .nil) (current : Memory unit) => current = entry))]
  all_goals verex_solve

-- State-only call markers use the recorded ambient frame.
theorem concrete_call_marker {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit] [_Θ : Skolems unit]
    (initial : Memory unit) :
    wp (callAt 8 (Spec.pure true : Comp unit Bool))
      (fun result final => result = true ∧ final = initial) (fun _ => False) initial := by
  verex_obligations [(8, SiteCondition.Kind.afterCall, handle, concreteCondition _Θ (fun _ =>
    fun (_ : Unit) (_ : Memory unit) (before after : Memory unit) => after = before))]
  all_goals verex_solve

section Markers
variable [C : Carriers] {State : Type}

-- Negative control first: `ghostAt` checks `checking τ value`, so a chosen ghost
-- value violating `NTy.Holds` fails `wp` even under trivial postconditions.
theorem generic_ghost_invalid (value : NTy.carrier (.param 0))
    (broken : ¬ NTy.Holds (.param 0) value) (initial : State) :
    ¬ wp (@ghostAt C State .none .nil (.param 0) 13 (fun _ => some value) ())
      (fun _ _ => True) (fun _ => True) initial := by
  unfold ghostAt
  rw [wp_bind, wp_certifyState]
  rintro ⟨_, hget⟩
  rw [wp_bind, wp_get, wp_bind, wp_choose] at hget
  have hval := hget value
  rw [wp_bind, wp_assume] at hval
  have hcheck := hval rfl
  rw [wp_bind, wp_checkingAt] at hcheck
  exact broken hcheck.1

theorem generic_ghost (value : NTy.carrier (.param 0))
    (holds : NTy.Holds (.param 0) value) (initial : State) :
    wp (@ghostAt C State .none .nil (.param 0) 13 (fun _ => some value) ())
      (fun result final => result = Flow.value value () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

theorem generic_assertion (initial : State) :
    wp (@assertAt C State .none .nil 11 ())
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(11, SiteCondition.Kind.assertion, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => True)]
  all_goals verex_solve

theorem generic_memory (initial : State) :
    wp (Flow.bind (@anchorAt C State .none .nil 12 ()) fun _ env => memoryWrittenAt 12 env)
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(12, SiteCondition.Kind.memoryWritten, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => True)]
  all_goals verex_solve

theorem generic_construction (value : NTy.carrier (.param 0)) (initial : State) :
    wp (@constructedAt C State .none .nil _ 14 value ())
      (fun result final => result = Flow.value value () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(14, SiteCondition.Kind.construction, handle,
    fun (_ : Unit) (_ : State) (current : NTy.carrier (.param 0)) (_ : State) => current = value)]
  all_goals verex_solve

theorem generic_mutation_end (initial : State) :
    wp (@mutationEndAt C State .none .nil 15 ())
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(15, SiteCondition.Kind.mutationEnd, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => True)]
  all_goals verex_solve

-- One site owes conditions of two kinds, as a mutable borrow of a resource
-- does: the namespace invariants where its write of memory ends, over the
-- anchor the write saved, and the data invariant where its mutation ends,
-- over the locals and state. Each marker reads the condition of its own
-- kind, so the write's predicate never stands for the mutation's: applied
-- there it would leave a binder and no proposition.
theorem markers_of_two_kinds (initial : State) :
    wp (Flow.bind (@anchorAt C State .none .nil 20 ()) fun _ env =>
        Flow.bind (memoryWrittenAt 20 env) fun _ env => mutationEndAt 20 env)
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [
    (20, SiteCondition.Kind.memoryWritten, handle,
      fun (_ : Unit) (_ : State) (_ : @AnchorOf C State 20 .nil) (_ : @HEnv C.toSorts .nil)
        (_ : State) => True),
    (20, SiteCondition.Kind.mutationEnd, handle,
      fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => True)]
  all_goals verex_solve

-- Two conditions of one kind at one site, where the step reads one: the
-- second would be passed over without a word, so the elaborator refuses
-- the pair. Sharing a site across kinds stays allowed
-- (`markers_of_two_kinds`).
/--
error: two conditions of the kind `mutationEnd` at the site 22: a step reads one
-/
#guard_msgs in
theorem conditions_of_one_kind (initial : State) :
    wp (@mutationEndAt C State .none .nil 22 ())
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [
    (22, SiteCondition.Kind.mutationEnd, handle,
      fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => True),
    (22, SiteCondition.Kind.mutationEnd, handle,
      fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => False)]
  all_goals verex_solve

-- A condition whose interface is not that of the rule of its step leaves a
-- goal that is no proposition. The walk stops there with an internal error
-- naming the step: the goal is never handed on as a leaf, where it would be
-- reported as a proof that failed, and nothing is put in its place.
/--
error: internal: malformed obligation of the `mutationEnd` condition at the site 21: what it owes is no proposition
  Nat
---
error: (kernel) application type mismatch
  wp_mutationEndAt 21 () fun x x_1 => Nat
argument has type
  HEnv NRow.nil → State → Type
but function has type
  ∀ (invariant : HEnv NRow.nil → State → Prop) (ensures : Flow ResultShape.none NRow.nil Unit → State → Prop)
    (aborts : Failure → Prop) (initial : State),
    invariant () initial →
      (invariant () initial → ensures (Flow.value () ()) initial) → wp (mutationEndAt 21 ()) ensures aborts initial
-/
#guard_msgs in
theorem malformed_mutation_end (initial : State) :
    wp (@mutationEndAt C State .none .nil 21 ())
      (fun result final => result = Flow.value () () ∧ final = initial)
      (fun _ => False) initial := by
  verex_obligations [(21, SiteCondition.Kind.mutationEnd, handle,
    fun (_ : Unit) (_ : State) (_ : @HEnv C.toSorts .nil) (_ : State) => Nat)]
  all_goals verex_solve
end Markers

theorem given_certification (G : GivenFrame) (State : Type)
    (value : @NTy.carrier (G.toCarriers).toSorts (.param 0)) (initial : State) :
    wp (@checkingAt G.toCarriers State 16 .copyOut (.param 0) value)
      (fun _ final => final = initial) (fun _ => False) initial := by
  verex_obligations []
  all_goals verex_solve

theorem given_unroll (G : GivenFrame) (State : Type) (unit : VerexIR.Validation.ValidatedUnit)
    (ops : InterpreterOps unit G.toCarriers State) (initial : State) :
    wp (@Function.denoteIn unit G.toCarriers State ops (immediateLoop unit) ())
      (fun _ final => final = initial) (fun _ => False) initial := by
  simp only [immediateLoop, Function.denoteIn]
  verex_normalize
  verex_obligations [] unrolling [(7, 1)]
  all_goals verex_solve

open Lean Elab Command in
run_cmd do
  for (name, rules) in [
      (``given_loop, #[``wp_loopAt]),
      (``concrete_loop, #[``wp_loopAt]),
      (``concrete_call_marker, #[``wp_callAt_checked]),
      (``generic_assertion, #[``wp_assertAt]),
      (``generic_memory, #[``wp_memoryWrittenAt]),
      (``generic_memory, #[``wp_anchorAt]),
      (``generic_ghost, #[``wp_ghostAt]),
      (``generic_construction, #[``wp_constructedAt]),
      (``generic_mutation_end, #[``wp_mutationEndAt]),
      (``markers_of_two_kinds, #[``wp_memoryWrittenAt]),
      (``markers_of_two_kinds, #[``wp_mutationEndAt]),
      (``given_certification, #[`VerexIR.Proofs.Denote.wp_checkingAt_intro,
        `VerexIR.Proofs.Denote.wp_checkingAt_intro_holds]),
      (``given_unroll, #[``wp_loopAt_unroll]),
      (``given_unroll, #[``wp_loopUnroll_step])] do
    let .thmInfo theoremInfo ← getConstInfo name | throwError "expected theorem"
    unless (theoremInfo.value.find? (fun expression => rules.any expression.isConstOf)).isSome do
      throwError "{name} did not exercise any expected walker rule in {rules}"

open Lean Elab Command in
run_cmd do
  for name in [``generic_ghost_invalid, ``given_identity, ``given_loop, ``concrete_loop,
      ``concrete_call_marker, ``generic_assertion, ``generic_memory, ``generic_ghost,
      ``generic_construction, ``generic_mutation_end, ``markers_of_two_kinds,
      ``given_certification, ``given_unroll] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end VerexIR.Tests.GivenWalker
