-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.RelationalTransfer

namespace VerexIR.Tests.RelationalTransfer

open VerexIR.Proofs

-- Erasing an undefined concrete execution cannot justify verification.
example : ¬Spec.RelationalRefinesAt Eq Eq Eq
    (Spec.stuck : Spec Unit Nat Unit) (Spec.pure ()) () () := by
  intro refines
  exact refines.undefined trivial

-- A concrete abort cannot be erased by a normally returning interpretation.
example : ¬Spec.RelationalRefinesAt Eq Eq Eq
    (Spec.abort 7 : Spec Unit Nat Unit) (Spec.pure ()) () () := by
  intro refines
  obtain ⟨error, aborts, _⟩ := refines.aborts 7 rfl
  exact aborts

-- Same-frame refinement relates outcomes only where the refined computation
-- is defined. A run that goes on past a failed data invariant keeps its
-- outcome, while the certifying step has none and is undefined there: the
-- outcome is no outcome of the certifying step, so the refinement can hold
-- only because the step is undefined, and a weakest precondition, which
-- excludes that start, still transfers.
example : Spec.Refines (Spec.pure 0 : Spec Unit Nat Nat)
    (Spec.certified (Invariant := False) False.elim) :=
  ⟨fun _ _ _ _ => .inr not_false, fun _ _ aborts => aborts.elim,
    fun _ undefined => undefined.elim⟩

example : ¬(Spec.certified (Invariant := False) False.elim : Spec Unit Nat Nat).ok () 0 () :=
  fun ⟨holds, _⟩ => holds

-- Failures need not have the same representation, but a permitted given
-- failure must justify the actual concrete failure along the error relation.
example : wp (Spec.abort 7 : Spec Unit Nat Unit)
    (fun _ _ => False) (fun error => error = 7) () := by
  have related : Spec.RelationalRefinesAt Eq Eq
      (fun concrete given : Nat => concrete = 7 ∧ given = 8)
      (Spec.abort 7 : Spec Unit Nat Unit) (Spec.abort 8) () () := by
    refine ⟨?_, ?_, ?_⟩
    · intro _ _ impossible
      exact impossible.elim
    · intro error failed
      exact ⟨8, rfl, failed, rfl⟩
    · intro impossible
      exact impossible
  apply wp_of_relationalRefinesAt related
    (givenEnsures := fun _ _ => False) (givenAborts := fun error => error = 8)
  · intro _ _ _ _ _ _ impossible
    exact impossible
  · intro _ _ relatedError _
    exact relatedError.1
  · exact ⟨fun _ _ impossible => impossible.elim, fun _ failed => failed,
      fun impossible => impossible⟩

private def stateRel (_ : Unit) (given : Bool) : Prop := given = false
private def resultRel (concrete : Nat) (given : Bool) : Prop := concrete = 7 ∧ given = true

private theorem relatedPure : Spec.RelationalRefinesAt stateRel resultRel Eq
    (Spec.pure 7 : Spec Unit Nat Nat) (Spec.pure true) () false := by
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    exact ⟨true, false, ⟨rfl, rfl⟩, ⟨execution.1, rfl⟩, rfl⟩
  · intro _ impossible
    exact impossible.elim
  · intro impossible
    exact impossible

private def concreteContract : Contract Unit Nat Unit Nat where
  requires := fun _ _ => True
  ensures := fun _ _ result _ => result = 7
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

private def givenContract : Contract Bool Nat Unit Bool where
  requires := fun _ _ => True
  ensures := fun _ _ result _ => result = true
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False

-- Contract export supports different state and result types. Related-start
-- coverage is restricted to permitted concrete starts, not a total inverse.
example : Satisfies (fun _ : Unit => (Spec.pure 7 : Spec Unit Nat Nat)) concreteContract := by
  have refines : FunctionRelationalRefines Eq stateRel resultRel Eq (fun _ _ => True)
      (fun _ : Unit => (Spec.pure 7 : Spec Unit Nat Nat))
      (fun _ : Unit => (Spec.pure true : Spec Bool Nat Bool)) := by
    intro args givenArgs initial givenInitial _ relatedInitial _
    cases initial
    change givenInitial = false at relatedInitial
    subst givenInitial
    exact relatedPure
  have compatible : Contract.RelationalTransfer Eq stateRel resultRel Eq
      concreteContract givenContract := by
    constructor
    · intros
      trivial
    · intros
      trivial
    · intro _ _ _ _ _ _ _ _ _ _ relatedResult _ _
      exact relatedResult.1
    · intro _ _ initial _ _ _ final _ _ _ _ _ _
      cases initial
      cases final
      rfl
    · intro _ _ _ _ _ _ _ _ _ impossible
      exact impossible
    · intro _ _ _ _ _ _ impossible
      exact impossible
    · intro _ _ _ _ _ _ impossible
      exact impossible
  refine Contract.RelationalCertificate.transfer ⟨{
    argsRel := Eq, stateRel, resultRel, errorRel := Eq
    admissible := fun _ _ => True
    refines, compatible, admissibleStarts := ?_, relatedStarts := ?_
  }⟩ ?_
  · intros
    trivial
  · intro args _ _ _
    exact ⟨args, false, rfl, rfl⟩
  · intro _ initial _ _
    refine ⟨?_, ?_, ?_⟩
    · intro result final execution
      exact ⟨fun _ => execution.1, execution.2, fun impossible => impossible⟩
    · intro _ impossible
      exact impossible
    · intro impossible
      exact impossible

end VerexIR.Tests.RelationalTransfer
