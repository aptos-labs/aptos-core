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

-- A failed checking step produces no normal outcome.
example : ¬(Spec.certified (Invariant := False) False.elim : Spec Unit Nat Nat).ok () 0 () :=
  fun ⟨holds, _⟩ => holds

-- Same-frame refinement relates outcomes only where the refined computation
-- is defined. A run that goes on past a failed data invariant keeps its
-- outcome, while the checking step has none and is undefined there: the
-- outcome is no outcome of the checking step, so the refinement can hold
-- only because the step is undefined, and a weakest precondition, which
-- excludes that start, still transfers.
example : Spec.Refines (Spec.pure 0 : Spec Unit Nat Nat)
    (Spec.certified (Invariant := False) False.elim) :=
  ⟨fun _ _ _ _ => .inr not_false, fun _ _ aborts => aborts.elim,
    fun _ undefined => undefined.elim⟩

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
    · intros
      trivial
    · intros
      trivial
  refine Contract.RelationalCertificate.transfer ⟨{
    argsRel := Eq, stateRel, resultRel, errorRel := Eq
    admissible := fun _ _ => True
    refines, compatible, admissibleStarts := ?_, relatedStarts := ?_
  }⟩ ?_
  · intros
    trivial
  · intro args _ _ _ _
    exact ⟨args, false, rfl, rfl⟩
  · intro _ initial _ _ _
    refine ⟨?_, ?_, ?_⟩
    · intro result final execution
      exact ⟨trivial, fun _ => execution.1, execution.2, fun impossible => impossible⟩
    · intro _ impossible
      exact impossible
    · intro impossible
      exact impossible

-- Instantiating a Given-verified generic `identity<T>(value : T) -> T` at an
-- invariant-bearing type `Percent` (`value ≤ 100`):
-- On the Given side, `GivenFrame.toCarriers` sets parameter validity to `True`
-- (`validArgs = fun _ => True`, `validResult = fun _ => True`).
-- On the concrete side, `validArgs` and `validResult` enforce `· ≤ 100`.
section GivenInvariantInstantiation

private def percentIdentityConcreteContract : Contract Unit Nat Nat Nat where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun value => value ≤ 100
  validResult := fun result => result ≤ 100

private def givenIdentityContract (α : Type) : Contract Unit Nat α α where
  requires := fun _ _ => True
  ensures := fun args _ result _ => result = args
  aborts := fun _ _ _ => False
  mayAbort := fun _ _ => False
  validArgs := fun _ => True
  validResult := fun _ => True

-- Negative control (first): uncertified plain `Eq` transfer at `Percent` is
-- uninhabited because `givenIdentityContract` supplies only `True` in
-- `validResult`, which cannot establish `result ≤ 100` at `result := 101`.
theorem uncertified_percent_transfer_refused :
    ¬ Nonempty (Contract.RelationalTransfer Eq Eq Eq Eq
      percentIdentityConcreteContract (givenIdentityContract Nat)) := by
  rintro ⟨compatible⟩
  have invalid : 101 ≤ 100 := compatible.validResult 101 101 rfl trivial
  omega

-- Positive control 1 (inhabited subtype carrier): choosing `{ n : Nat // n ≤ 100 }`
-- on the Given side threads `concreteContract.validArgs` (`x ≤ 100`) through
-- `relatedStarts` and discharges `compatible.validResult` from the subtype witness.
private abbrev CertifiedPercent := { n : Nat // n ≤ 100 }
private def percentValRel (concrete : Nat) (given : CertifiedPercent) : Prop :=
  concrete = given.val

private def percentIdentityCertificate :
    Contract.RelationalCertificate
      (fun x : Nat => (Spec.pure x : Spec Unit Nat Nat))
      (fun gx : CertifiedPercent => (Spec.pure gx : Spec Unit Nat CertifiedPercent))
      percentIdentityConcreteContract
      (givenIdentityContract CertifiedPercent) where
  argsRel := percentValRel
  stateRel := Eq
  resultRel := percentValRel
  errorRel := Eq
  admissible := fun x _ => x ≤ 100
  refines := by
    intro args givenArgs initial givenInitial relatedArgs relatedInitial _
    cases initial
    cases givenInitial
    refine ⟨?_, ?_, ?_⟩
    · intro result final execution
      exact ⟨givenArgs, (), ⟨rfl, rfl⟩, execution.1.trans relatedArgs, execution.2⟩
    · intro _ impossible
      exact impossible.elim
    · intro impossible
      exact impossible.elim
  compatible := {
    assumes := fun _ _ _ _ _ _ _ => trivial
    requires := fun _ _ _ _ _ _ _ => trivial
    ensures := fun _ _ _ _ _ _ _ _ relatedArgs _ relatedResult _ eqGiven =>
      relatedResult.trans ((congrArg Subtype.val eqGiven).trans relatedArgs.symm)
    frame := fun _ _ initial _ _ _ final _ _ _ _ _ _ => by
      cases initial
      cases final
      rfl
    aborts := fun _ _ _ _ _ _ _ _ _ impossible => impossible.elim
    mayAbort := fun _ _ _ _ _ _ impossible => impossible.elim
    mustAbort := fun _ _ _ _ _ _ impossible => impossible.elim
    validArgs := fun _ _ _ _ => trivial
    validResult := fun _ givenResult relatedResult _ =>
      relatedResult ▸ givenResult.property
  }
  admissibleStarts := fun _ _ valid _ _ => valid
  relatedStarts := fun args initial valid _ _ => ⟨⟨args, valid⟩, initial, rfl, rfl⟩

theorem certified_percent_identity_satisfies :
    Satisfies (fun x : Nat => (Spec.pure x : Spec Unit Nat Nat))
      percentIdentityConcreteContract := by
  refine Contract.RelationalCertificate.transfer ⟨percentIdentityCertificate⟩ ?_
  intro gx initial _ _ _
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    exact ⟨trivial, fun _ => execution.1, execution.2, fun impossible => impossible.elim⟩
  · intro _ impossible
    exact impossible.elim
  · intro impossible
    exact impossible.elim

-- Positive control 2 (plain sort `Nat`, no subtype carrier): validity travels
-- purely in the relational evidence `concrete = given ∧ concrete ≤ 100`
-- seeded from `admissible := fun x _ => x ≤ 100` and `concreteContract.validArgs`,
-- without instantiating the Given sort at a subtype.
private def plainPercentIdentityCertificate :
    Contract.RelationalCertificate
      (fun x : Nat => (Spec.pure x : Spec Unit Nat Nat))
      (fun gx : Nat => (Spec.pure gx : Spec Unit Nat Nat))
      percentIdentityConcreteContract
      (givenIdentityContract Nat) where
  argsRel := fun concrete given : Nat => concrete = given ∧ concrete ≤ 100
  stateRel := Eq
  resultRel := fun concrete given : Nat => concrete = given ∧ concrete ≤ 100
  errorRel := Eq
  admissible := fun x _ => x ≤ 100
  refines := by
    intro args givenArgs initial givenInitial relatedArgs relatedInitial admissibleStart
    cases initial
    cases givenInitial
    refine ⟨?_, ?_, ?_⟩
    · intro result final execution
      refine ⟨givenArgs, (), ⟨rfl, rfl⟩, ⟨execution.1.trans relatedArgs.1, ?_⟩, execution.2⟩
      rw [execution.1]
      exact admissibleStart
    · intro _ impossible
      exact impossible.elim
    · intro impossible
      exact impossible.elim
  compatible := {
    assumes := fun _ _ _ _ _ _ _ => trivial
    requires := fun _ _ _ _ _ _ _ => trivial
    ensures := fun _ _ _ _ _ _ _ _ relatedArgs _ relatedResult _ eqGiven =>
      relatedResult.1.trans (eqGiven.trans relatedArgs.1.symm)
    frame := fun _ _ initial _ _ _ final _ _ _ _ _ _ => by
      cases initial
      cases final
      rfl
    aborts := fun _ _ _ _ _ _ _ _ _ impossible => impossible.elim
    mayAbort := fun _ _ _ _ _ _ impossible => impossible.elim
    mustAbort := fun _ _ _ _ _ _ impossible => impossible.elim
    validArgs := fun _ _ _ _ => trivial
    validResult := fun _ _ relatedResult _ => relatedResult.2
  }
  admissibleStarts := fun _ _ valid _ _ => valid
  relatedStarts := fun args initial valid _ _ => ⟨args, initial, ⟨rfl, valid⟩, rfl⟩

theorem plain_percent_identity_satisfies :
    Satisfies (fun x : Nat => (Spec.pure x : Spec Unit Nat Nat))
      percentIdentityConcreteContract := by
  refine Contract.RelationalCertificate.transfer ⟨plainPercentIdentityCertificate⟩ ?_
  intro gx initial _ _ _
  refine ⟨?_, ?_, ?_⟩
  · intro result final execution
    exact ⟨trivial, fun _ => execution.1, execution.2, fun impossible => impossible.elim⟩
  · intro _ impossible
    exact impossible.elim
  · intro impossible
    exact impossible.elim

end GivenInvariantInstantiation

end VerexIR.Tests.RelationalTransfer
