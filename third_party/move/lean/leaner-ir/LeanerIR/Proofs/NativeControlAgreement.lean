-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeStatementAgreement
namespace LeanerIR.Proofs.ComputationAgreement
open LeanerIR.SemanticOperations LeanerIR.BigStep Denotation
theorem scalar_throw (kind : ThrowKind) (operands : ValuesDenotation) (entry : RuntimeFrame)
    (encodeArgs : Args → List RuntimeValue) (arguments : Spec RuntimeState Failure Args)
    (encode : Result → RuntimeValue)
    (agreement : Operands operands entry encodeArgs arguments) :
    Scalar (nativeThrow kind operands) entry encode
      (Spec.bind arguments (fun args => Spec.abort (kind, (encodeArgs args).toArray))) := by
  constructor
  · intro initial finalFrame finalState actual
    constructor
    · rintro (⟨values, _, impossible⟩ | controlled)
      · cases impossible
      · obtain ⟨_, _, impossible⟩ := agreement.control _ _ _ _ controlled
        cases impossible
    · rintro ⟨_, ⟨_, _, _, impossible⟩, _, _⟩
      cases impossible
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨values, ran, equal⟩ | controlled)⟩
      · obtain ⟨args, step, rfl, rfl⟩ := (agreement.normal _ _ _ _).mp ran
        cases error with | mk actualKind actualValues =>
          cases equal
          exact Or.inr ⟨args, state, step, rfl⟩
      · exact Or.inl ((agreement.aborts _ _).mp ⟨frame, state, controlled⟩)
    · rintro (aborted | ⟨args, state, step, equal⟩)
      · obtain ⟨frame, state, controlled⟩ := (agreement.aborts _ _).mpr aborted
        exact ⟨frame, state, Or.inr controlled⟩
      · change error = (kind, (encodeArgs args).toArray) at equal
        cases equal
        exact ⟨entry, state, Or.inl ⟨encodeArgs args,
          (agreement.normal _ _ _ _).mpr ⟨args, step, rfl, rfl⟩, rfl⟩⟩
  · intro initial finalFrame finalState flow executed _
    rcases executed with ⟨values, _, equal⟩ | controlled
    · exact ⟨kind, values.toArray, equal⟩
    · exact agreement.control _ _ _ _ controlled
  · intro initial undefined
    rcases undefined with undefined | ⟨_, _, _, impossible⟩
    · exact agreement.defined _ undefined
    · exact impossible

theorem scalar_discard (head body : ExprDenotation) (entry : RuntimeFrame)
    (encodeHead : Head → RuntimeValue) (encode : Result → RuntimeValue)
    (first : Spec RuntimeState Failure Head) (next : Spec RuntimeState Failure Result)
    (headAgreement : Scalar head entry encodeHead first)
    (bodyAgreement : Scalar body entry encode next) :
    Scalar (letNativeValue ⟨1, .wildcard⟩ head body) entry encode
      (Spec.bind first (fun _ => next)) := by
  have binding (value : RuntimeValue) :
      (NativePatternBinder.mk 1 .wildcard).bind entry value = some entry := rfl
  constructor
  · intro initial finalFrame finalState value
    constructor
    · rintro (⟨_, abrupt⟩ | ⟨frame, state, actual, bound, ran, binds, continued⟩)
      · cases abrupt
      · obtain ⟨native, step, rfl, rfl⟩ := (headAgreement.normal _ _ _ _).mp ran
        rw [binding] at binds
        cases binds
        obtain ⟨result, resultStep, same, equal⟩ := (bodyAgreement.normal _ _ _ _).mp continued
        exact ⟨result, ⟨native, state, step, resultStep⟩, same, equal⟩
    · rintro ⟨result, ⟨native, state, step, resultStep⟩, same, equal⟩
      exact Or.inr ⟨entry, state, encodeHead native, entry,
        (headAgreement.normal _ _ _ _).mpr ⟨native, step, rfl, rfl⟩, binding _,
        (bodyAgreement.normal _ _ _ _).mpr ⟨result, resultStep, same, equal⟩⟩
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨ran, _⟩ | ⟨headFrame, headState, value, bound, ran, binds, continued⟩)⟩
      · exact Or.inl ((headAgreement.aborts _ _).mp ⟨frame, state, ran⟩)
      · obtain ⟨native, step, rfl, rfl⟩ := (headAgreement.normal _ _ _ _).mp ran
        rw [binding] at binds
        cases binds
        exact Or.inr ⟨native, headState, step,
          (bodyAgreement.aborts _ _).mp ⟨frame, state, continued⟩⟩
    · rintro (aborted | ⟨native, state, step, aborted⟩)
      · obtain ⟨frame, state, ran⟩ := (headAgreement.aborts _ _).mpr aborted
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · obtain ⟨frame, last, ran⟩ := (bodyAgreement.aborts _ _).mpr aborted
        exact ⟨frame, last, Or.inr ⟨entry, state, encodeHead native, entry,
          (headAgreement.normal _ _ _ _).mpr ⟨native, step, rfl, rfl⟩, binding _, ran⟩⟩
  · intro initial finalFrame finalState flow executed abrupt
    rcases executed with ⟨ran, _⟩ | ⟨frame, state, value, bound, ran, binds, continued⟩
    · exact headAgreement.abrupt _ _ _ _ ran abrupt
    · obtain ⟨native, _, rfl, rfl⟩ := (headAgreement.normal _ _ _ _).mp ran
      rw [binding] at binds
      cases binds
      exact bodyAgreement.abrupt _ _ _ _ continued abrupt
  · intro initial undefined
    rcases undefined with undefined | ⟨_, state, _, undefined⟩
    · exact headAgreement.defined _ undefined
    · exact bodyAgreement.defined state undefined
/-- Discard a statement value before the function continuation. Its native
result type is arbitrary; the execution frame remains an agreement boundary. -/
theorem fromFrame_discard (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (head body : ExprDenotation) (tail : StatementsDenotation) (entry : RuntimeFrame)
    (encode : Head → RuntimeValue) (first : Spec RuntimeState Failure Head)
    (next : Spec RuntimeState Failure Result) (codec : Codec Result (Array RuntimeValue))
    (scalar : Scalar head entry encode first) (preserves : StatePreserving first)
    (continuation : Spec.Equiv (fromFrame unit shape (blockResult tail body) entry)
      (encodeSpec codec next)) :
    Spec.Equiv (fromFrame unit shape (blockResult (statementsCons head tail) body) entry)
      (encodeSpec codec (Spec.bind first (fun _ => next))) := by
  rw [block_discard]
  exact fromFrame_let_scalar unit shape ⟨1, .wildcard⟩ head (blockResult tail body)
    entry encode (fun _ => entry) first (fun _ => next) codec scalar preserves
    (fun _ => rfl) (fun _ => continuation)

theorem fromFrame_unit (unit : Validation.ExecutableUnit) (shape : FunctionShape)
    (statements : StatementsDenotation) (entry : RuntimeFrame) :
    Spec.Equiv (fromFrame unit shape (blockUnit statements) entry)
      (fromFrame unit shape (blockResult statements (value .unit)) entry) := by
  rw [block_unit_result]
  exact Spec.Equiv.refl _

end LeanerIR.Proofs.ComputationAgreement
