-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeValueAgreement

namespace LeanerIR.Proofs.ComputationAgreement
open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation

/-- An effectful typed test runs before either branch. Its abort state is not
assumed equal to entry; successful evaluation has a separate state certificate. -/
theorem fromFrame_branch_scalar (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (condition yes no : ExprDenotation) (entry : RuntimeFrame)
    (test : Spec RuntimeState Failure Bool) (left right : Spec RuntimeState Failure Result)
    (codec : Codec Result (Array RuntimeValue))
    (scalar : Scalar condition entry RuntimeValue.bool test)
    (preserves : StatePreserving test)
    (yesAgreement : Spec.Equiv (fromFrame unit shape yes entry) (encodeSpec codec left))
    (noAgreement : Spec.Equiv (fromFrame unit shape no entry) (encodeSpec codec right)) :
    Spec.Equiv (fromFrame unit shape (nativeBranch condition yes (some no)) entry)
      (encodeSpec codec (Spec.bind test (fun value => if value then left else right))) := by
  have evaluated : ∀ initial finalFrame finalState control,
      nativeBranch condition yes (some no) entry initial finalFrame finalState control ↔
        (condition entry initial finalFrame finalState control ∧ Abrupt control) ∨
        ∃ value, test.ok initial value initial ∧
          (if value then yes else no) entry initial finalFrame finalState control := by
    intro initial finalFrame finalState control
    constructor
    · rintro (aborted | ⟨frame, state, tested, ran⟩ | ⟨frame, state, tested, ran⟩)
      · exact Or.inl aborted
      · obtain ⟨value, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp tested
        cases equal
        have same := preserves _ _ _ step
        subst state
        exact Or.inr ⟨true, step, ran⟩
      · obtain ⟨value, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp tested
        cases equal
        have same := preserves _ _ _ step
        subst state
        exact Or.inr ⟨false, step, ran⟩
    · rintro (aborted | ⟨value, step, ran⟩)
      · exact Or.inl aborted
      · cases value with
        | false => exact Or.inr (Or.inr ⟨entry, initial,
            (scalar.normal _ _ _ _).mpr ⟨false, step, rfl, rfl⟩, ran⟩)
        | true => exact Or.inr (Or.inl ⟨entry, initial,
            (scalar.normal _ _ _ _).mpr ⟨true, step, rfl, rfl⟩, ran⟩)
  have branch : ∀ value : Bool,
      Spec.Equiv (fromFrame unit shape (if value then yes else no) entry)
        (encodeSpec codec (if value then left else right)) := by
    intro value; cases value <;> assumption
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended, exported⟩
      rcases (evaluated _ _ _ _).mp executed with ⟨initialized, abrupt⟩ | ⟨value, step, ran⟩
      · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ initialized abrupt
        cases ended
      · obtain ⟨native, nextStep, encoded⟩ := (branch value).ok _ _ _ |>.mp
          ⟨finalFrame, state, control, ran, ended, exported⟩
        exact ⟨native, ⟨value, initial, step, nextStep⟩, encoded⟩
    · rintro ⟨native, ⟨value, middle, firstStep, nextStep⟩, encoded⟩
      have same := preserves _ _ _ firstStep
      subst middle
      obtain ⟨finalFrame, state, control, ran, ended, exported⟩ :=
        (branch value).ok _ _ _ |>.mpr ⟨native, nextStep, encoded⟩
      exact ⟨finalFrame, state, control,
        (evaluated _ _ _ _).mpr (Or.inr ⟨value, firstStep, ran⟩), ended, exported⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, executed, ended⟩
      rcases (evaluated _ _ _ _).mp executed with ⟨initialized, abrupt⟩ | ⟨value, step, ran⟩
      · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ initialized abrupt
        cases error with | mk actualKind actualValues =>
          cases ended
          exact Or.inl ((scalar.aborts _ _).mp ⟨finalFrame, state, initialized⟩)
      · exact Or.inr ⟨value, initial, step,
          (branch value).aborts _ _ |>.mp ⟨finalFrame, state, control, ran, ended⟩⟩
    · rintro (aborted | ⟨value, middle, firstStep, aborted⟩)
      · obtain ⟨finalFrame, state, ran⟩ := (scalar.aborts _ _).mpr aborted
        exact ⟨finalFrame, state, .throw_ error.1 error.2,
          (evaluated _ _ _ _).mpr (Or.inl ⟨ran, .throw_ _ _⟩), rfl⟩
      · have same := preserves _ _ _ firstStep
        subst middle
        obtain ⟨finalFrame, state, control, ran, ended⟩ :=
          (branch value).aborts _ _ |>.mpr aborted
        exact ⟨finalFrame, state, control,
          (evaluated _ _ _ _).mpr (Or.inr ⟨value, firstStep, ran⟩), ended⟩
  · intro initial
    constructor
    · exact False.elim
    · rintro (undefined | ⟨value, middle, _, undefined⟩)
      · exact scalar.defined initial undefined
      · exact (branch value).undefined middle |>.mpr undefined

/-- Boolean subexpressions compose without evaluating the unselected arm.
Unlike the function-finalization boundary, this scalar rule can retain the
condition's intermediate state directly. -/
theorem scalar_branch (condition yes no : ExprDenotation) (entry : RuntimeFrame)
    (test : Spec RuntimeState Failure Bool) (left right : Spec RuntimeState Failure Result)
    (encode : Result → RuntimeValue)
    (conditionAgreement : Scalar condition entry RuntimeValue.bool test)
    (yesAgreement : Scalar yes entry encode left)
    (noAgreement : Scalar no entry encode right) :
    Scalar (nativeBranch condition yes (some no)) entry encode
      (Spec.bind test (fun value => if value then left else right)) := by
  have branch : ∀ value : Bool,
      Scalar (if value then yes else no) entry encode (if value then left else right) := by
    intro value; cases value <;> assumption
  have evaluated : ∀ initial finalFrame finalState control,
      nativeBranch condition yes (some no) entry initial finalFrame finalState control ↔
        (condition entry initial finalFrame finalState control ∧ Abrupt control) ∨
        ∃ value middle, test.ok initial value middle ∧
          (if value then yes else no) entry middle finalFrame finalState control := by
    intro initial finalFrame finalState control
    constructor
    · rintro (aborted | ⟨frame, state, tested, ran⟩ | ⟨frame, state, tested, ran⟩)
      · exact Or.inl aborted
      · obtain ⟨value, step, rfl, equal⟩ := (conditionAgreement.normal _ _ _ _).mp tested
        cases equal
        exact Or.inr ⟨true, state, step, ran⟩
      · obtain ⟨value, step, rfl, equal⟩ := (conditionAgreement.normal _ _ _ _).mp tested
        cases equal
        exact Or.inr ⟨false, state, step, ran⟩
    · rintro (aborted | ⟨value, middle, step, ran⟩)
      · exact Or.inl aborted
      · cases value with
        | false => exact Or.inr (Or.inr ⟨entry, middle,
            (conditionAgreement.normal _ _ _ _).mpr ⟨false, step, rfl, rfl⟩, ran⟩)
        | true => exact Or.inr (Or.inl ⟨entry, middle,
            (conditionAgreement.normal _ _ _ _).mpr ⟨true, step, rfl, rfl⟩, ran⟩)
  constructor
  · intro initial finalFrame finalState actual
    constructor
    · intro executed
      rcases (evaluated _ _ _ _).mp executed with ⟨_, abrupt⟩ | ⟨value, middle, step, ran⟩
      · cases abrupt
      · obtain ⟨native, nextStep, sameFrame, equal⟩ := (branch value).normal _ _ _ _ |>.mp ran
        exact ⟨native, ⟨value, middle, step, nextStep⟩, sameFrame, equal⟩
    · rintro ⟨native, ⟨value, middle, step, nextStep⟩, sameFrame, equal⟩
      exact (evaluated _ _ _ _).mpr (Or.inr ⟨value, middle, step,
        (branch value).normal _ _ _ _ |>.mpr ⟨native, nextStep, sameFrame, equal⟩⟩)
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, executed⟩
      rcases (evaluated _ _ _ _).mp executed with ⟨ran, _⟩ | ⟨value, middle, step, ran⟩
      · exact Or.inl ((conditionAgreement.aborts _ _).mp ⟨finalFrame, finalState, ran⟩)
      · exact Or.inr ⟨value, middle, step,
          (branch value).aborts _ _ |>.mp ⟨finalFrame, finalState, ran⟩⟩
    · rintro (aborted | ⟨value, middle, step, aborted⟩)
      · obtain ⟨finalFrame, finalState, ran⟩ := (conditionAgreement.aborts _ _).mpr aborted
        exact ⟨finalFrame, finalState, (evaluated _ _ _ _).mpr (Or.inl ⟨ran, .throw_ _ _⟩)⟩
      · obtain ⟨finalFrame, finalState, ran⟩ := (branch value).aborts _ _ |>.mpr aborted
        exact ⟨finalFrame, finalState, (evaluated _ _ _ _).mpr (Or.inr ⟨value, middle, step, ran⟩)⟩
  · intro initial finalFrame finalState control executed abrupt
    rcases (evaluated _ _ _ _).mp executed with ⟨ran, _⟩ | ⟨value, middle, _, ran⟩
    · exact conditionAgreement.abrupt _ _ _ _ ran abrupt
    · exact (branch value).abrupt _ _ _ _ ran abrupt
  · intro initial undefined
    rcases undefined with undefined | ⟨value, middle, _, undefined⟩
    · exact conditionAgreement.defined _ undefined
    · exact (branch value).defined _ undefined

/-- Pure choices join as a typed value before any continuation. -/
theorem scalar_branch_pure (condition yes no : ExprDenotation) (entry : RuntimeFrame)
    (test : Bool) (left right : Result) (encode : Result → RuntimeValue)
    (conditionAgreement : Scalar condition entry RuntimeValue.bool (Spec.pure test))
    (yesAgreement : Scalar yes entry encode (Spec.pure left))
    (noAgreement : Scalar no entry encode (Spec.pure right)) :
    Scalar (nativeBranch condition yes (some no)) entry encode
      (Spec.pure (if test then left else right)) := by
  have composed := scalar_branch condition yes no entry (Spec.pure test)
    (Spec.pure left) (Spec.pure right) encode conditionAgreement yesAgreement noAgreement
  cases test <;> simpa only [Spec.pure_bind, Bool.false_eq_true, if_false, if_true] using composed

end LeanerIR.Proofs.ComputationAgreement
