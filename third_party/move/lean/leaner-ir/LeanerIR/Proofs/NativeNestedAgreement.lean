-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeControlRoute

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep Denotation

/-- One successful loop exit, including the IR's depth decrement. -/
inductive LoopExit : Control → Control → Prop
  | break_ (value : Option RuntimeValue) : LoopExit (.break_ 0 value) (.value (value.getD .unit))
  | outerContinue (depth : Nat) : LoopExit (.continue_ (depth + 1)) (.continue_ depth)
  | outerBreak (depth : Nat) (value : Option RuntimeValue) :
      LoopExit (.break_ (depth + 1) value) (.break_ depth value)
  | return_ (values : Array RuntimeValue) : LoopExit (.return_ values) (.return_ values)

namespace LoopExit

theorem input_valid (exit : LoopExit before after) : NonThrow before := by
  cases exit <;> trivial

theorem functional (left : LoopExit before first) (right : LoopExit before second) : first = second := by
  cases left <;> cases right <;> rfl

theorem not_repeats (exit : LoopExit before after) (repeats : Repeats before) : False := by
  cases exit <;> cases repeats

theorem raised (control : Control) (abrupt : Abrupt control) (valid : NonThrow control) :
    LoopExit (ControlRoute.raise control) control := by
  cases abrupt with
  | continue_ depth => exact .outerContinue depth
  | break_ depth value => exact .outerBreak depth value
  | return_ values => exact .return_ values
  | throw_ _ _ => exact valid.elim

theorem execution (exit : LoopExit before after)
    (ran : body frame initial finalFrame finalState before) :
    Denotation.NativeLoop body frame initial finalFrame finalState after := by
  cases exit with
  | break_ value => exact .break_ value ran
  | outerContinue depth => exact .outerContinue depth ran
  | outerBreak depth value => exact .outerBreak depth value ran
  | return_ values => exact .return_ values ran

end LoopExit

def sumFrames (current : Locals → RuntimeFrame → Prop) (outer : Outer → RuntimeFrame → Prop) :
    Sum Locals Outer → RuntimeFrame → Prop
  | .inl locals => current locals
  | .inr target => outer target

def stepFrames (current : Locals → RuntimeFrame → Prop) (outer : Outer → RuntimeFrame → Prop) :
    NativeLoop.Step Locals (NativeFlow.Flow Locals Outer) → RuntimeFrame → Prop
  | .next locals => current locals
  | .done flow => flowFrames current outer flow

theorem nested_step_frames (current : Locals → RuntimeFrame → Prop) (outer : Outer → RuntimeFrame → Prop)
    (flow : NativeFlow.Flow Locals (Sum Locals Outer)) (frame : RuntimeFrame) :
    flowFrames current (sumFrames current outer) flow frame ↔
      stepFrames current outer (NativeNestedFlow.step flow) frame := by
  cases flow with
  | normal locals => rfl
  | continue_ target => cases target <;> rfl
  | break_ target => cases target <;> rfl

theorem nested_step_classify (route : ControlRoute Outer)
    (flow : NativeFlow.Flow Locals (Sum Locals Outer)) :
    (∃ next, Repeats (route.push.encode flow) ∧ NativeNestedFlow.step flow = .next next) ∨
    (∃ result, LoopExit (route.push.encode flow) (route.encode result) ∧
      NativeNestedFlow.step flow = .done result) := by
  cases flow with
  | normal locals => exact Or.inl ⟨locals, .value .unit, rfl⟩
  | continue_ target =>
    cases target with
    | inl locals => exact Or.inl ⟨locals, .continue_, rfl⟩
    | inr target => exact Or.inr ⟨.continue_ target,
        .raised _ (route.continueAbrupt target) (route.continueValid target), rfl⟩
  | break_ target =>
    cases target with
    | inl locals => exact Or.inr ⟨.normal locals, .break_ none, rfl⟩
    | inr target => exact Or.inr ⟨.break_ target,
        .raised _ (route.breakAbrupt target) (route.breakValid target), rfl⟩

section Iteration

variable {Locals Outer : Type} {body : ExprDenotation} {route : ControlRoute Outer}
  {frames : Locals → RuntimeFrame → Prop} {outerFrames : Outer → RuntimeFrame → Prop}
  {computation : Locals → Spec RuntimeState Failure (NativeFlow.Flow Locals (Sum Locals Outer))}
  {locals : Locals} {frame finalFrame : RuntimeFrame} {initial finalState : RuntimeState}
  {control after : Control}

private theorem nested_repeats
    (agreement : Observed body frame (flowFrames frames (sumFrames frames outerFrames))
      route.push.encode (computation locals))
    (ran : body frame initial finalFrame finalState control) (repeats : Repeats control) :
    ∃ next, (NativeNestedFlow.iteration (computation locals)).ok initial (.next next) finalState ∧
      frames next finalFrame := by
  have valid : NonThrow control := by cases repeats <;> trivial
  obtain ⟨flow, step, related, equal⟩ := agreement.sound _ _ _ _ valid ran
  subst control
  have related := (nested_step_frames frames outerFrames flow finalFrame).mp related
  rcases nested_step_classify route flow with ⟨next, _, shape⟩ | ⟨result, exit, shape⟩
  · rw [shape] at related
    exact ⟨next, ⟨flow, step, shape.symm⟩, related⟩
  · exact (exit.not_repeats repeats).elim

private theorem nested_exits
    (agreement : Observed body frame (flowFrames frames (sumFrames frames outerFrames))
      route.push.encode (computation locals))
    (ran : body frame initial finalFrame finalState control) (exit : LoopExit control after) :
    ∃ result, (NativeNestedFlow.iteration (computation locals)).ok initial (.done result) finalState ∧
      flowFrames frames outerFrames result finalFrame ∧ after = route.encode result := by
  obtain ⟨flow, step, related, equal⟩ := agreement.sound _ _ _ _ exit.input_valid ran
  subst control
  have related := (nested_step_frames frames outerFrames flow finalFrame).mp related
  rcases nested_step_classify route flow with ⟨next, repeats, shape⟩ | ⟨result, actual, shape⟩
  · exact (exit.not_repeats repeats).elim
  · rw [shape] at related
    exact ⟨result, ⟨flow, step, shape.symm⟩, related, exit.functional actual⟩

private theorem nested_next
    (agreement : Observed body frame (flowFrames frames (sumFrames frames outerFrames))
      route.push.encode (computation locals))
    (ran : (NativeNestedFlow.iteration (computation locals)).ok initial (.next next) finalState) :
    ∃ finalFrame control, body frame initial finalFrame finalState control ∧
      Repeats control ∧ frames next finalFrame := by
  obtain ⟨flow, step, equal⟩ := ran
  obtain ⟨finalFrame, ran, related⟩ := agreement.complete _ _ _ step
  have related := (nested_step_frames frames outerFrames flow finalFrame).mp related
  rcases nested_step_classify route flow with ⟨next, repeats, shape⟩ | ⟨result, actual, shape⟩
  · rw [shape] at equal related
    cases equal
    exact ⟨finalFrame, _, ran, repeats, related⟩
  · rw [shape] at equal; cases equal

private theorem nested_done
    (agreement : Observed body frame (flowFrames frames (sumFrames frames outerFrames))
      route.push.encode (computation locals))
    (ran : (NativeNestedFlow.iteration (computation locals)).ok initial (.done result) finalState) :
    ∃ finalFrame, Denotation.NativeLoop body frame initial finalFrame finalState (route.encode result) ∧
      flowFrames frames outerFrames result finalFrame := by
  obtain ⟨flow, step, equal⟩ := ran
  obtain ⟨finalFrame, ran, related⟩ := agreement.complete _ _ _ step
  have related := (nested_step_frames frames outerFrames flow finalFrame).mp related
  rcases nested_step_classify route flow with ⟨next, repeats, shape⟩ | ⟨result, actual, shape⟩
  · rw [shape] at equal; cases equal
  · rw [shape] at equal related
    cases equal
    exact ⟨finalFrame, actual.execution ran, related⟩

private theorem nested_loop_sound
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames (sumFrames frames outerFrames)) route.push.encode (computation locals))
    (executed : Denotation.NativeLoop body frame initial finalFrame finalState control) :
    ∀ locals, frames locals frame →
      (∃ result, NativeLoop.Runs (fun locals => NativeNestedFlow.iteration (computation locals))
        locals initial result finalState ∧ flowFrames frames outerFrames result finalFrame ∧
          control = route.encode result) ∨
      (∃ error, NativeLoop.Fails (fun locals => NativeNestedFlow.iteration (computation locals))
        locals initial error ∧ control = .throw_ error.1 error.2) := by
  induction executed with
  | repeatValue value _ step _ ih =>
    intro locals related
    obtain ⟨next, ran, related⟩ := nested_repeats (agreement locals _ related) step (.value value)
    rcases ih next related with ⟨result, rest, related, equal⟩ | ⟨error, rest, equal⟩
    · exact Or.inl ⟨result, .next ran rest, related, equal⟩
    · exact Or.inr ⟨error, .next ran rest, equal⟩
  | repeatContinue _ step _ ih =>
    intro locals related
    obtain ⟨next, ran, related⟩ := nested_repeats (agreement locals _ related) step .continue_
    rcases ih next related with ⟨result, rest, related, equal⟩ | ⟨error, rest, equal⟩
    · exact Or.inl ⟨result, .next ran rest, related, equal⟩
    · exact Or.inr ⟨error, .next ran rest, equal⟩
  | break_ value step =>
    intro locals related
    obtain ⟨result, ran, related, equal⟩ := nested_exits (agreement locals _ related) step (.break_ value)
    exact Or.inl ⟨result, .done ran, related, equal⟩
  | outerContinue depth step =>
    intro locals related
    obtain ⟨result, ran, related, equal⟩ := nested_exits (agreement locals _ related) step (.outerContinue depth)
    exact Or.inl ⟨result, .done ran, related, equal⟩
  | outerBreak depth value step =>
    intro locals related
    obtain ⟨result, ran, related, equal⟩ := nested_exits (agreement locals _ related) step (.outerBreak depth value)
    exact Or.inl ⟨result, .done ran, related, equal⟩
  | return_ values step =>
    intro locals related
    obtain ⟨result, ran, related, equal⟩ := nested_exits (agreement locals _ related) step (.return_ values)
    exact Or.inl ⟨result, .done ran, related, equal⟩
  | throw_ kind values step =>
    intro locals related
    exact Or.inr ⟨(kind, values), .here ((agreement locals _ related).aborts _ _ |>.mp ⟨_, _, step⟩), rfl⟩

private theorem nested_loop_runs
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames (sumFrames frames outerFrames)) route.push.encode (computation locals))
    (executed : NativeLoop.Runs (fun locals => NativeNestedFlow.iteration (computation locals))
      locals initial result finalState) :
    ∀ frame, frames locals frame → ∃ finalFrame,
      Denotation.NativeLoop body frame initial finalFrame finalState (route.encode result) ∧
        flowFrames frames outerFrames result finalFrame := by
  induction executed with
  | done step =>
    intro frame related
    exact nested_done (agreement _ frame related) step
  | next step _ ih =>
    intro frame related
    obtain ⟨middleFrame, control, ran, repeats, related⟩ := nested_next (agreement _ frame related) step
    obtain ⟨finalFrame, rest, related⟩ := ih middleFrame related
    cases repeats with
    | value value => exact ⟨finalFrame, .repeatValue value _ ran rest, related⟩
    | continue_ => exact ⟨finalFrame, .repeatContinue _ ran rest, related⟩

private theorem nested_loop_fails
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames (sumFrames frames outerFrames)) route.push.encode (computation locals))
    (executed : NativeLoop.Fails (fun locals => NativeNestedFlow.iteration (computation locals))
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
    obtain ⟨middleFrame, control, ran, repeats, related⟩ := nested_next (agreement _ frame related) step
    obtain ⟨finalFrame, finalState, rest⟩ := ih middleFrame related
    cases repeats with
    | value value => exact ⟨finalFrame, finalState, .repeatValue value _ ran rest⟩
    | continue_ => exact ⟨finalFrame, finalState, .repeatContinue _ ran rest⟩

private theorem nested_loop_defined
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames (sumFrames frames outerFrames)) route.push.encode (computation locals))
    (executed : NativeLoop.Undefined (fun locals => NativeNestedFlow.iteration (computation locals)) locals initial) :
    ∀ frame, frames locals frame → False := by
  induction executed with
  | here undefined =>
    intro frame related
    exact (agreement _ frame related).defined _ undefined
  | next step _ ih =>
    intro frame related
    obtain ⟨middleFrame, _, _, _, related⟩ := nested_next (agreement _ frame related) step
    exact ih middleFrame related

theorem observed_nested_loop (site : ExprId) (locals : Locals)
    (invariant : Locals → RuntimeState → Prop) (frame : RuntimeFrame)
    (related : frames locals frame)
    (agreement : ∀ locals frame, frames locals frame → Observed body frame
      (flowFrames frames (sumFrames frames outerFrames)) route.push.encode (computation locals)) :
    Observed (nativeLoop site body) frame (flowFrames frames outerFrames) route.encode
      (NativeLoop.run (fun locals => NativeNestedFlow.iteration (computation locals)) locals invariant) := by
  constructor
  · intro initial finalFrame finalState control valid ran
    rcases nested_loop_sound agreement ran locals related with
      ⟨result, ran, related, equal⟩ | ⟨error, ran, equal⟩
    · exact ⟨result, (NativeLoop.run_ok ..).mpr ran, related, equal⟩
    · cases equal; exact valid.elim
  · intro initial result finalState ran
    exact nested_loop_runs agreement ((NativeLoop.run_ok ..).mp ran) frame related
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, ran⟩
      rcases nested_loop_sound agreement ran locals related with
        ⟨result, ran, related, equal⟩ | ⟨actual, ran, equal⟩
      · have valid := route.valid result
        rw [← equal] at valid
        exact valid.elim
      · cases error; cases actual; cases equal
        exact (NativeLoop.run_aborts ..).mpr ran
    · intro failed
      exact nested_loop_fails agreement ((NativeLoop.run_aborts ..).mp failed) frame related
  · intro initial undefined
    exact nested_loop_defined agreement ((NativeLoop.run_undefined ..).mp undefined) frame related
  · exact route.valid

end Iteration

end LeanerIR.Proofs.ComputationAgreement
