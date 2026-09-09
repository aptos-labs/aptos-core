-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation

/-! Typed loop state and its finite-execution semantics. Locals and exits are
ordinary type parameters: neither the computation nor its invariant contains
a runtime frame. The fixed point has no runtime fuel limit. -/

namespace LeanerIR.Proofs.NativeLoop

inductive Step (Locals Result : Type) where
  | next (locals : Locals)
  | done (result : Result)

def body (iteration : Locals → Spec State Error (Step Locals Result))
    (recursive : Locals → Spec State Error Result) (locals : Locals) :
    Spec State Error Result :=
  Spec.bind (iteration locals) fun
    | .next next => recursive next
    | .done result => Spec.pure result

def run (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop) : Spec State Error Result :=
  Spec.withInvariant (body iteration) locals invariant

/-- Finite successful iterations, retaining every intermediate store. -/
inductive Runs (iteration : Locals → Spec State Error (Step Locals Result)) :
    Locals → State → Result → State → Prop where
  | done (step : (iteration locals).ok initial (.done result) final) :
      Runs iteration locals initial result final
  | next (step : (iteration locals).ok initial (.next next) middle)
      (rest : Runs iteration next middle result final) :
      Runs iteration locals initial result final

/-- Failures retain the state at the failing iteration, not the loop entry. -/
inductive Fails (iteration : Locals → Spec State Error (Step Locals Result)) :
    Locals → State → Error → Prop where
  | here (step : (iteration locals).aborts initial error) :
      Fails iteration locals initial error
  | next (step : (iteration locals).ok initial (.next next) middle)
      (rest : Fails iteration next middle error) :
      Fails iteration locals initial error

inductive Undefined (iteration : Locals → Spec State Error (Step Locals Result)) :
    Locals → State → Prop where
  | here (step : (iteration locals).undefined initial) :
      Undefined iteration locals initial
  | next (step : (iteration locals).ok initial (.next next) middle)
      (rest : Undefined iteration next middle) :
      Undefined iteration locals initial

theorem run_ok (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop) (initial final : State)
    (result : Result) :
    (run iteration locals invariant).ok initial result final ↔
      Runs iteration locals initial result final := by
  constructor
  · rintro ⟨fuel, executed⟩
    induction fuel generalizing locals initial with
    | zero => exact False.elim executed
    | succ fuel ih =>
      obtain ⟨action, middle, step, rest⟩ := executed
      cases action with
      | next next => exact .next step (ih next middle rest)
      | done value =>
        obtain ⟨rfl, rfl⟩ := rest
        exact .done step
  · intro executed
    change ∃ fuel, (Spec.fixApprox (body iteration) fuel locals).ok initial result final
    induction executed with
    | done step => exact ⟨1, .done _, _, step, rfl, rfl⟩
    | next step _ ih =>
      obtain ⟨fuel, rest⟩ := ih
      exact ⟨fuel + 1, .next _, _, step, rest⟩

theorem run_aborts (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop) (initial : State)
    (error : Error) :
    (run iteration locals invariant).aborts initial error ↔
      Fails iteration locals initial error := by
  constructor
  · rintro ⟨fuel, executed⟩
    induction fuel generalizing locals initial with
    | zero => exact False.elim executed
    | succ fuel ih =>
      rcases executed with failed | ⟨action, middle, step, rest⟩
      · exact .here failed
      · cases action with
        | next next => exact .next step (ih next middle rest)
        | done _ => exact False.elim rest
  · intro executed
    change ∃ fuel, (Spec.fixApprox (body iteration) fuel locals).aborts initial error
    induction executed with
    | here step => exact ⟨1, Or.inl step⟩
    | next step _ ih =>
      obtain ⟨fuel, rest⟩ := ih
      exact ⟨fuel + 1, Or.inr ⟨.next _, _, step, rest⟩⟩

theorem run_undefined (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop) (initial : State) :
    (run iteration locals invariant).undefined initial ↔
      Undefined iteration locals initial := by
  constructor
  · rintro ⟨fuel, executed⟩
    induction fuel generalizing locals initial with
    | zero => exact False.elim executed
    | succ fuel ih =>
      rcases executed with failed | ⟨action, middle, step, rest⟩
      · exact .here failed
      · cases action with
        | next next => exact .next step (ih next middle rest)
        | done _ => exact False.elim rest
  · intro executed
    change ∃ fuel, (Spec.fixApprox (body iteration) fuel locals).undefined initial
    induction executed with
    | here step => exact ⟨1, Or.inl step⟩
    | next step _ ih =>
      obtain ⟨fuel, rest⟩ := ih
      exact ⟨fuel + 1, Or.inr ⟨.next _, _, step, rest⟩⟩

theorem preserves (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop)
    (step : ∀ locals, StatePreserving (iteration locals)) :
    StatePreserving (run iteration locals invariant) := by
  intro initial result final executed
  have ran := (run_ok _ _ _ _ _ _).mp executed
  clear executed
  induction ran with
  | done executed => exact step _ _ _ _ executed
  | next executed _ ih => exact ih.trans (step _ _ _ _ executed)

theorem defined (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop)
    (step : ∀ locals initial, ¬(iteration locals).undefined initial) (initial : State) :
    ¬(run iteration locals invariant).undefined initial := by
  intro undefined
  have ran := (run_undefined _ _ _ _).mp undefined
  clear undefined
  induction ran with
  | here failed => exact step _ _ failed
  | next _ _ ih => exact ih

/-- One invariant-entry obligation and one iteration obligation. No proof is
generated for any particular iteration count. -/
theorem wp_run (iteration : Locals → Spec State Error (Step Locals Result))
    (locals : Locals) (invariant : Locals → State → Prop)
    (post : Result → State → Prop) (aborts : Error → Prop) (initial : State)
    (entry : invariant locals initial)
    (step : ∀ locals state, invariant locals state →
      wp (iteration locals)
        (fun action final => match action with
          | .next next => invariant next final
          | .done result => post result final)
        aborts state) :
    wp (run iteration locals invariant) post aborts initial := by
  apply wp_withInvariant_fix entry
  intro recursive hypothesis locals state holds
  rw [body, wp_bind]
  apply wp_mono (step locals state holds)
  · intro action final established
    cases action with
    | next next => exact hypothesis next final established
    | done result => exact (wp_pure _ _ _ _).mpr established
  · exact fun _ => id

end LeanerIR.Proofs.NativeLoop
