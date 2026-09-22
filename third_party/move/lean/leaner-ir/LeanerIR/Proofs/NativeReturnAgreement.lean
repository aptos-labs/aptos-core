-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeNestedAgreement
import LeanerIR.Proofs.NativeReturnFlow

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep LeanerIR.SemanticOperations Denotation

def ControlRoute.returned (encode : Result → Array RuntimeValue) : ControlRoute Result where
  continue_ := fun result => .return_ (encode result)
  break_ := fun result => .return_ (encode result)
  continueAbrupt := fun _ => .return_ _
  breakAbrupt := fun _ => .return_ _
  continueValid := fun _ => trivial
  breakValid := fun _ => trivial

theorem controlled_return (values : ValuesDenotation) (entry : RuntimeFrame)
    (encode : Result → List RuntimeValue) (computation : Spec RuntimeState Failure Result)
    (agreement : Operands values entry encode computation) :
    Controlled (nativeReturn values) entry (fun _ => entry)
      (fun result => .return_ (encode result).toArray) computation := by
  constructor
  · intro initial finalFrame finalState control valid
    constructor
    · rintro (⟨values, ran, equal⟩ | ran)
      · obtain ⟨result, ran, same, encoded⟩ := (agreement.normal _ _ _ _).mp ran
        exact ⟨result, ran, same, encoded ▸ equal⟩
      · obtain ⟨_, _, equal⟩ := agreement.control _ _ _ _ ran
        cases equal; exact valid.elim
    · rintro ⟨result, ran, same, equal⟩
      exact Or.inl ⟨encode result, (agreement.normal _ _ _ _).mpr ⟨result, ran, same, rfl⟩, equal⟩
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨_, _, impossible⟩ | ran)⟩
      · cases impossible
      · exact (agreement.aborts _ _).mp ⟨frame, state, ran⟩
    · intro failed
      obtain ⟨frame, state, ran⟩ := (agreement.aborts _ _).mpr failed
      exact ⟨frame, state, Or.inr ran⟩
  · exact agreement.defined
  · exact fun _ => trivial

/-- Singleton return evaluation retains the native scalar, without allocating
a native operand product just to discard its Unit tail. -/
theorem operands_single (head : ExprDenotation) (entry : RuntimeFrame)
    (encode : Result → RuntimeValue) (computation : Spec RuntimeState Failure Result)
    (agreement : Scalar head entry encode computation) :
    Operands (valuesCons head valuesNil) entry (fun result => [encode result]) computation := by
  constructor
  · intro initial finalFrame finalState values
    constructor
    · rintro (⟨_, _, _, _, _, impossible⟩ | ⟨frame, state, value, _, _, tail, ran, ended, equal⟩ |
        ⟨_, _, _, _, _, _, _, impossible, _⟩)
      · cases impossible
      · cases ended
        cases equal
        obtain ⟨result, ran, same, equal⟩ := (agreement.normal _ _ _ _).mp ran
        exact ⟨result, ran, same, equal ▸ rfl⟩
      · cases impossible
    · rintro ⟨result, ran, same, equal⟩
      exact Or.inr (Or.inl ⟨finalFrame, finalState, encode result, finalFrame, finalState, [],
        (agreement.normal _ _ _ _).mpr ⟨result, ran, same, rfl⟩, rfl, equal ▸ rfl⟩)
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨_, _, _, ran, _, equal⟩ | ⟨_, _, _, _, _, _, _, _, impossible⟩ |
        ⟨_, _, _, _, _, _, _, impossible, _⟩)⟩
      · cases equal; exact (agreement.aborts _ _).mp ⟨_, _, ran⟩
      · cases impossible
      · cases impossible
    · intro failed
      obtain ⟨frame, state, ran⟩ := (agreement.aborts _ _).mpr failed
      exact ⟨frame, state, Or.inl ⟨frame, state, .throw_ error.1 error.2, ran, .throw_ _ _, rfl⟩⟩
  · intro initial frame state control
    rintro (⟨_, _, _, ran, abrupt, equal⟩ | ⟨_, _, _, _, _, _, _, _, impossible⟩ |
      ⟨_, _, _, _, _, _, _, impossible, _⟩)
    · cases equal; exact agreement.abrupt _ _ _ _ ran abrupt
    · cases impossible
    · cases impossible
  · exact agreement.defined

private theorem finished_throw (count : Nat) (control : Control) (error : Failure)
    (ended : finishControl? count control = some (.threw error.1 error.2)) :
    control = .throw_ error.1 error.2 := by
  cases control with
  | value value =>
    cases equal : unpackFallthrough count value <;> simp [finishControl?, equal] at ended
  | return_ values =>
    simp only [finishControl?] at ended
    split at ended <;> cases ended
  | throw_ kind values => cases error; cases ended; rfl
  | break_ depth value => cases ended
  | continue_ depth => cases ended

/-- A typed early return skips the remaining source statements. The only
frame condition on that path is the actual function-boundary loan export;
normal paths retain their precise live-local observation. -/
theorem fromFrame_returning_discard (unit : Validation.ExecutableUnit)
    (shape : FunctionShape) (head result : ExprDenotation) (tail : StatementsDenotation)
    (entry : RuntimeFrame) (frames : Locals → RuntimeFrame → Prop)
    (first : Spec RuntimeState Failure (NativeFlow.Flow Locals Result))
    (next : Locals → Spec RuntimeState Failure Result) (codec : Codec Result (Array RuntimeValue))
    (agreement : Observed head entry
      (flowFrames frames (fun _ frame => frameBorrows frame = #[]))
      (ControlRoute.returned codec.encode).encode first)
    (finished : ∀ value, finishControl? shape.resultCount (.return_ (codec.encode value)) =
      some (.returned (codec.encode value)))
    (continuation : ∀ value frame, frames value frame →
      Spec.Equiv (fromFrame unit shape (blockResult tail result) frame) (encodeSpec codec (next value))) :
    Spec.Equiv (fromFrame unit shape (blockResult (statementsCons head tail) result) entry)
      (encodeSpec codec (NativeReturnFlow.finish first next)) := by
  rw [block_discard]
  constructor
  · intro initial results final
    constructor
    · rintro ⟨finalFrame, state, control, (⟨ran, abrupt⟩ |
        ⟨headFrame, headState, value, bound, ran, binds, continued⟩), ended, exported⟩
      · have valid : NonThrow control := by
          cases control <;> try trivial
          cases ended
        obtain ⟨flow, step, related, equal⟩ := agreement.sound _ _ _ _ valid ran
        cases flow with
        | normal locals => cases equal; cases abrupt
        | continue_ value | break_ value =>
          cases equal
          change finishControl? shape.resultCount (.return_ (codec.encode value)) =
            some (.returned results) at ended
          rw [finished] at ended
          cases ended
          have same : state = final := by
            simpa only [finalizeFunctionState,
              exportReturnedFrameLoans_borrowFree _ _ _ related] using exported
          subst state
          exact ⟨value, ⟨_, final, step, rfl, rfl⟩, rfl⟩
      · obtain ⟨flow, step, related, equal⟩ := agreement.sound _ _ _ (.value value) trivial ran
        cases binds
        cases flow with
        | normal locals =>
          cases equal
          obtain ⟨native, executed, encoded⟩ := (continuation locals headFrame related).ok _ _ _ |>.mp
            ⟨finalFrame, state, control, continued, ended, exported⟩
          exact ⟨native, ⟨.normal locals, headState, step, executed⟩, encoded⟩
        | continue_ value | break_ value => cases equal
    · rintro ⟨native, ⟨flow, middle, step, executed⟩, encoded⟩
      obtain ⟨headFrame, ran, related⟩ := agreement.complete _ _ _ step
      cases flow with
      | normal locals =>
        obtain ⟨finalFrame, state, control, continued, ended, exported⟩ :=
          (continuation locals headFrame related).ok _ _ _ |>.mpr ⟨native, executed, encoded⟩
        exact ⟨finalFrame, state, control,
          Or.inr ⟨headFrame, middle, .unit, headFrame, ran, rfl, continued⟩, ended, exported⟩
      | continue_ value | break_ value =>
        obtain ⟨sameValue, sameState⟩ := executed
        subst native
        subst middle
        subst results
        exact ⟨headFrame, final, .return_ (codec.encode value), Or.inl ⟨ran, .return_ _⟩,
          finished value, exportReturnedFrameLoans_borrowFree _ _ _ related⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, (⟨ran, _⟩ |
        ⟨headFrame, headState, value, bound, ran, binds, continued⟩), ended⟩
      · have equal := finished_throw _ _ _ ended
        cases equal
        exact Or.inl ((agreement.aborts _ _).mp ⟨_, _, ran⟩)
      · obtain ⟨flow, step, related, equal⟩ := agreement.sound _ _ _ (.value value) trivial ran
        cases binds
        cases flow with
        | normal locals =>
          cases equal
          exact Or.inr ⟨.normal locals, headState, step,
            (continuation locals headFrame related).aborts _ _ |>.mp
              ⟨finalFrame, state, control, continued, ended⟩⟩
        | continue_ value | break_ value => cases equal
    · rintro (failed | ⟨flow, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (agreement.aborts _ _).mpr failed
        exact ⟨frame, state, .throw_ error.1 error.2, Or.inl ⟨ran, .throw_ _ _⟩, rfl⟩
      · obtain ⟨headFrame, ran, related⟩ := agreement.complete _ _ _ step
        cases flow with
        | normal locals =>
          obtain ⟨frame, state, control, continued, ended⟩ :=
            (continuation locals headFrame related).aborts _ _ |>.mpr failed
          exact ⟨frame, state, control,
            Or.inr ⟨headFrame, middle, .unit, headFrame, ran, rfl, continued⟩, ended⟩
        | continue_ value | break_ value => cases failed
  · intro initial
    constructor
    · exact False.elim
    · rintro (undefined | ⟨flow, state, ran, undefined⟩)
      · exact agreement.defined _ undefined
      · obtain ⟨frame, _, related⟩ := agreement.complete _ _ _ ran
        cases flow with
        | normal locals => exact (continuation locals frame related).undefined state |>.mpr undefined
        | continue_ value | break_ value => cases undefined

end LeanerIR.Proofs.ComputationAgreement
