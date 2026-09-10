-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeObservedFlow

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep Denotation

variable {Locals : Type} {body : ExprDenotation}
  {frames : Locals → RuntimeFrame → Prop}
  {computation : Locals → Spec RuntimeState Failure (NativeFlow.Flow Locals)}
  {frame finalFrame : RuntimeFrame} {initial finalState : RuntimeState} {control : Control}

private theorem observed_loop_sound
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames frames) flowControl (computation locals))
    (executed : Denotation.NativeLoop body frame initial finalFrame finalState control) :
    ∀ locals, frames locals frame →
      (∃ result, NativeLoop.Runs (fun locals => NativeFlow.iteration (computation locals))
        locals initial result finalState ∧ frames result finalFrame ∧ control = .value .unit) ∨
      (∃ error, NativeLoop.Fails (fun locals => NativeFlow.iteration (computation locals))
        locals initial error ∧ control = .throw_ error.1 error.2) := by
  induction executed with
  | repeatValue value _ step _ ih =>
    intro locals related
    obtain ⟨flow, ran, same, equal⟩ := (agreement locals _ related).sound _ _ _ (.value value) trivial step
    cases flow with
    | normal next =>
      cases equal
      rcases ih next same with ⟨result, rest, same, equal⟩ | ⟨error, rest, equal⟩
      · exact Or.inl ⟨result, .next ⟨.normal next, ran, rfl⟩ rest, same, equal⟩
      · exact Or.inr ⟨error, .next ⟨.normal next, ran, rfl⟩ rest, equal⟩
    | continue_ _ => cases equal
    | break_ _ => cases equal
  | repeatContinue _ step _ ih =>
    intro locals related
    obtain ⟨flow, ran, same, equal⟩ := (agreement locals _ related).sound _ _ _ (.continue_ 0) trivial step
    cases flow with
    | normal _ => cases equal
    | continue_ next =>
      rcases ih next same with ⟨result, rest, same, equal⟩ | ⟨error, rest, equal⟩
      · exact Or.inl ⟨result, .next ⟨.continue_ next, ran, rfl⟩ rest, same, equal⟩
      · exact Or.inr ⟨error, .next ⟨.continue_ next, ran, rfl⟩ rest, equal⟩
    | break_ _ => cases equal
  | break_ value step =>
    intro locals related
    obtain ⟨flow, ran, same, equal⟩ := (agreement locals _ related).sound _ _ _ (.break_ 0 value) trivial step
    cases flow with
    | normal _ => cases equal
    | continue_ _ => cases equal
    | break_ next =>
      cases equal
      exact Or.inl ⟨next, .done ⟨.break_ next, ran, rfl⟩, same, rfl⟩
  | throw_ kind values step =>
    intro locals related
    exact Or.inr ⟨(kind, values), .here ((agreement locals _ related).aborts _ _ |>.mp ⟨_, _, step⟩), rfl⟩
  | outerContinue nest step =>
    intro locals related
    obtain ⟨flow, _, _, equal⟩ := (agreement locals _ related).sound _ _ _ (.continue_ (nest + 1)) trivial step
    cases flow <;> cases equal
  | outerBreak nest value step =>
    intro locals related
    obtain ⟨flow, _, _, equal⟩ := (agreement locals _ related).sound _ _ _ (.break_ (nest + 1) value) trivial step
    cases flow <;> cases equal
  | return_ values step =>
    intro locals related
    obtain ⟨flow, _, _, equal⟩ := (agreement locals _ related).sound _ _ _ (.return_ values) trivial step
    cases flow <;> cases equal

private theorem observed_loop_runs
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames frames) flowControl (computation locals))
    (executed : NativeLoop.Runs (fun locals => NativeFlow.iteration (computation locals))
      locals initial result final) :
    ∀ frame, frames locals frame → ∃ finalFrame,
      Denotation.NativeLoop body frame initial finalFrame final (.value .unit) ∧ frames result finalFrame := by
  induction executed with
  | done step =>
    intro frame related
    obtain ⟨flow, step, equal⟩ := step
    obtain ⟨finalFrame, ran, same⟩ := (agreement _ frame related).complete _ _ _ step
    cases flow with
    | normal _ => cases equal
    | continue_ _ => cases equal
    | break_ result =>
      cases equal
      exact ⟨finalFrame, .break_ none ran, same⟩
  | next step _ ih =>
    intro frame related
    obtain ⟨flow, step, equal⟩ := step
    obtain ⟨middleFrame, ran, same⟩ := (agreement _ frame related).complete _ _ _ step
    cases flow with
    | normal next =>
      cases equal
      obtain ⟨finalFrame, rest, related⟩ := ih middleFrame same
      exact ⟨finalFrame, .repeatValue .unit _ ran rest, related⟩
    | continue_ next =>
      cases equal
      obtain ⟨finalFrame, rest, related⟩ := ih middleFrame same
      exact ⟨finalFrame, .repeatContinue _ ran rest, related⟩
    | break_ _ => cases equal

private theorem observed_loop_fails
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames frames) flowControl (computation locals))
    (executed : NativeLoop.Fails (fun locals => NativeFlow.iteration (computation locals))
      locals initial error) :
    ∀ frame, frames locals frame → ∃ finalFrame finalState,
      Denotation.NativeLoop body frame initial finalFrame finalState (.throw_ error.1 error.2) := by
  induction executed with
  | here step =>
    intro frame related
    obtain ⟨finalFrame, finalState, ran⟩ := (agreement _ frame related).aborts _ _ |>.mpr step
    exact ⟨finalFrame, finalState, .throw_ _ _ ran⟩
  | next step _ ih =>
    intro frame related
    obtain ⟨flow, step, equal⟩ := step
    obtain ⟨middleFrame, ran, same⟩ := (agreement _ frame related).complete _ _ _ step
    cases flow with
    | normal next =>
      cases equal
      obtain ⟨finalFrame, finalState, rest⟩ := ih middleFrame same
      exact ⟨finalFrame, finalState, .repeatValue .unit _ ran rest⟩
    | continue_ next =>
      cases equal
      obtain ⟨finalFrame, finalState, rest⟩ := ih middleFrame same
      exact ⟨finalFrame, finalState, .repeatContinue _ ran rest⟩
    | break_ _ => cases equal

private theorem observed_loop_defined
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames frames) flowControl (computation locals))
    (steps : NativeLoop.Undefined (fun locals => NativeFlow.iteration (computation locals)) locals initial) :
    ∀ frame, frames locals frame → False := by
  induction steps with
  | here undefined =>
    intro frame related
    exact (agreement _ frame related).defined _ undefined
  | next step _ ih =>
    intro frame related
    obtain ⟨flow, step, equal⟩ := step
    obtain ⟨middleFrame, _, same⟩ := (agreement _ frame related).complete _ _ _ step
    cases flow with
    | normal next => cases equal; exact ih middleFrame same
    | continue_ next => cases equal; exact ih middleFrame same
    | break_ _ => cases equal

theorem observed_loop (site : ExprId) (locals : Locals)
    (invariant : Locals → RuntimeState → Prop) (frame : RuntimeFrame)
    (related : frames locals frame)
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames frames) flowControl (computation locals)) :
    Observed (nativeLoop site body) frame frames (fun _ => .value .unit)
      (NativeLoop.run (fun locals => NativeFlow.iteration (computation locals)) locals invariant) := by
  constructor
  · intro initial finalFrame finalState control valid ran
    rcases observed_loop_sound agreement ran locals related with
      ⟨result, ran, same, equal⟩ | ⟨error, ran, equal⟩
    · exact ⟨result, (NativeLoop.run_ok ..).mpr ran, same, equal⟩
    · cases equal; exact valid.elim
  · intro initial result finalState ran
    exact observed_loop_runs agreement ((NativeLoop.run_ok ..).mp ran) frame related
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, ran⟩
      rcases observed_loop_sound agreement ran locals related with
        ⟨result, ran, same, equal⟩ | ⟨actual, ran, equal⟩
      · cases equal
      · cases error; cases actual; cases equal
        exact (NativeLoop.run_aborts ..).mpr ran
    · intro failed
      exact observed_loop_fails agreement ((NativeLoop.run_aborts ..).mp failed) frame related
  · intro initial undefined
    exact observed_loop_defined agreement ((NativeLoop.run_undefined ..).mp undefined) frame related
  · exact fun _ => trivial

/-- A statement's dead frame cells stay in execution witnesses, not in the
typed continuation argument. Intermediate stores need not equal loop entry. -/
theorem fromFrame_observed_discard (unit : Validation.ExecutableUnit)
    (shape : SemanticOperations.FunctionShape) (head result : ExprDenotation)
    (tail : StatementsDenotation) (entry : RuntimeFrame)
    (frames : Locals → RuntimeFrame → Prop)
    (first : Spec RuntimeState Failure Locals) (next : Locals → Spec RuntimeState Failure Result)
    (codec : Codec Result (Array RuntimeValue))
    (agreement : Observed head entry frames (fun _ => .value .unit) first)
    (continuation : ∀ value frame, frames value frame →
      Spec.Equiv (fromFrame unit shape (blockResult tail result) frame) (encodeSpec codec (next value))) :
    Spec.Equiv (fromFrame unit shape (blockResult (statementsCons head tail) result) entry)
      (encodeSpec codec (Spec.bind first next)) := by
  have abrupt : ∀ initial finalFrame finalState control,
      head entry initial finalFrame finalState control → Abrupt control →
      ∃ kind values, control = .throw_ kind values := by
    intro initial finalFrame finalState control ran abrupt
    cases control with
    | throw_ kind values => exact ⟨kind, values, rfl⟩
    | value value => cases abrupt
    | continue_ nest | break_ nest value | return_ values =>
      obtain ⟨_, _, _, equal⟩ := agreement.sound _ _ _ _ (by trivial) ran
      cases equal
  rw [block_discard]
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, (⟨ran, isAbrupt⟩ |
        ⟨headFrame, headState, value, bound, ran, binds, continued⟩), ended, exported⟩
      · obtain ⟨kind, values, rfl⟩ := abrupt _ _ _ _ ran isAbrupt
        cases ended
      · obtain ⟨value, step, same, equal⟩ := agreement.sound _ _ _ (.value value) trivial ran
        cases equal
        cases binds
        obtain ⟨native, executed, encoded⟩ := (continuation value headFrame same).ok _ _ _ |>.mp
          ⟨finalFrame, state, control, continued, ended, exported⟩
        exact ⟨native, ⟨value, headState, step, executed⟩, encoded⟩
    · rintro ⟨native, ⟨value, middle, step, executed⟩, encoded⟩
      obtain ⟨headFrame, ran, related⟩ := agreement.complete _ _ _ step
      obtain ⟨finalFrame, state, control, continued, ended, exported⟩ :=
        (continuation value headFrame related).ok _ _ _ |>.mpr ⟨native, executed, encoded⟩
      exact ⟨finalFrame, state, control,
        Or.inr ⟨headFrame, middle, .unit, headFrame, ran, rfl, continued⟩, ended, exported⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, (⟨ran, isAbrupt⟩ |
        ⟨headFrame, headState, value, bound, ran, binds, continued⟩), ended⟩
      · obtain ⟨kind, values, rfl⟩ := abrupt _ _ _ _ ran isAbrupt
        cases error with | mk actualKind actualValues =>
          cases ended
          exact Or.inl (agreement.aborts _ _ |>.mp ⟨_, _, ran⟩)
      · obtain ⟨value, step, same, equal⟩ := agreement.sound _ _ _ (.value value) trivial ran
        cases equal
        cases binds
        exact Or.inr ⟨value, headState, step,
          (continuation value headFrame same).aborts _ _ |>.mp ⟨finalFrame, state, control, continued, ended⟩⟩
    · rintro (failed | ⟨value, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := agreement.aborts _ _ |>.mpr failed
        exact ⟨frame, state, .throw_ error.1 error.2, Or.inl ⟨ran, .throw_ _ _⟩, rfl⟩
      · obtain ⟨headFrame, ran, related⟩ := agreement.complete _ _ _ step
        obtain ⟨frame, state, control, continued, ended⟩ :=
          (continuation value headFrame related).aborts _ _ |>.mpr failed
        exact ⟨frame, state, control,
          Or.inr ⟨headFrame, middle, .unit, headFrame, ran, rfl, continued⟩, ended⟩
  · intro initial
    constructor
    · exact False.elim
    · rintro (undefined | ⟨value, state, ran, undefined⟩)
      · exact agreement.defined _ undefined
      · obtain ⟨frame, _, related⟩ := agreement.complete _ _ _ ran
        exact (continuation value frame related).undefined state |>.mpr undefined

end LeanerIR.Proofs.ComputationAgreement
