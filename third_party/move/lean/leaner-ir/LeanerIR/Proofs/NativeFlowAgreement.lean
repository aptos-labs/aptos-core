-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeFlow
import LeanerIR.Proofs.NativeLoopAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep LeanerIR.SemanticOperations Denotation

def NonThrow : Control → Prop
  | .throw_ .. => False
  | _ => True

/-- Exact typed results of a statement, including its control and local
updates. The encoded frame/control are used only by execution agreement. -/
structure Controlled (body : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame) (encode : Result → Control)
    (computation : Spec RuntimeState Failure Result) : Prop where
  normal : ∀ initial finalFrame finalState control, NonThrow control →
    (body entry initial finalFrame finalState control ↔
      ∃ result, computation.ok initial result finalState ∧
        finalFrame = frame result ∧ control = encode result)
  aborts : ∀ initial error,
    (∃ finalFrame finalState, body entry initial finalFrame finalState (.throw_ error.1 error.2)) ↔
      computation.aborts initial error
  defined : ∀ initial, ¬computation.undefined initial
  valid : ∀ result, NonThrow (encode result)

theorem controlled_scalar (body : ExprDenotation) (entry : RuntimeFrame)
    (encode : Result → RuntimeValue) (computation : Spec RuntimeState Failure Result)
    (scalar : Scalar body entry encode computation) :
    Controlled body entry (fun _ => entry) (fun value => .value (encode value)) computation := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    cases control with
    | value value => simpa only [Control.value.injEq] using scalar.normal initial finalFrame finalState value
    | throw_ kind values => exact nonThrow.elim
    | continue_ nest | break_ nest value | return_ values =>
      constructor
      · intro ran
        obtain ⟨_, _, equal⟩ := scalar.abrupt _ _ _ _ ran (by constructor)
        cases equal
      · rintro ⟨_, _, _, equal⟩; cases equal
  · exact scalar.aborts
  · exact scalar.defined
  · exact fun _ => trivial

theorem controlled_map (body : ExprDenotation) (entry : RuntimeFrame)
    (frame : A → RuntimeFrame) (encode : A → Control) (first : Spec RuntimeState Failure A)
    (mappedFrame : B → RuntimeFrame) (mappedControl : B → Control) (map : A → B)
    (agreement : Controlled body entry frame encode first)
    (frames : ∀ value, mappedFrame (map value) = frame value)
    (controls : ∀ value, mappedControl (map value) = encode value)
    (valid : ∀ value, NonThrow (mappedControl value)) :
    Controlled body entry mappedFrame mappedControl (Spec.bind first (fun value => Spec.pure (map value))) := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    constructor
    · intro ran
      obtain ⟨value, step, same, equal⟩ := (agreement.normal _ _ _ _ nonThrow).mp ran
      exact ⟨map value, ⟨value, finalState, step, rfl, rfl⟩,
        same.trans (frames value).symm, equal.trans (controls value).symm⟩
    · rintro ⟨_, ⟨value, _, step, rfl, rfl⟩, same, equal⟩
      exact (agreement.normal _ _ _ _ nonThrow).mpr
        ⟨value, step, same.trans (frames value), equal.trans (controls value)⟩
  · intro initial error
    constructor
    · intro ran; exact Or.inl ((agreement.aborts _ _).mp ran)
    · rintro (failed | ⟨_, _, _, impossible⟩)
      · exact (agreement.aborts _ _).mpr failed
      · exact impossible.elim
  · intro initial undefined
    rcases undefined with undefined | ⟨_, _, _, impossible⟩
    · exact agreement.defined _ undefined
    · exact impossible
  · exact valid

theorem controlled_congr (left right : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame) (encode : Result → Control)
    (computation : Spec RuntimeState Failure Result)
    (equal : ∀ initial finalFrame finalState control,
      left entry initial finalFrame finalState control ↔ right entry initial finalFrame finalState control)
    (agreement : Controlled right entry frame encode computation) :
    Controlled left entry frame encode computation := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    exact (equal _ _ _ _).trans (agreement.normal _ _ _ _ nonThrow)
  · intro initial error; simp only [equal]; exact agreement.aborts _ _
  · exact agreement.defined
  · exact agreement.valid

theorem controlled_blockUnit (statements : StatementsDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame) (encode : Result → Control) (computation : Spec RuntimeState Failure Result)
    (agreement : Controlled (blockResult statements (value .unit)) entry frame encode computation) :
    Controlled (blockUnit statements) entry frame encode computation := by
  simpa only [block_unit_result] using agreement

theorem controlled_blockNil (body : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame) (encode : Result → Control) (computation : Spec RuntimeState Failure Result)
    (agreement : Controlled body entry frame encode computation) :
    Controlled (blockResult statementsNil body) entry frame encode computation := by
  simpa only [block_nil_eq] using agreement

theorem controlled_assign (slot : LocalId) (value : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame) (encode : Result → Control) (computation : Spec RuntimeState Failure Result)
    (agreement : Controlled (blockUnit (statementsCons (nativeAssignLocal slot value) statementsNil))
      entry frame encode computation) :
    Controlled (nativeAssignLocal slot value) entry frame encode computation := by
  simpa only [unit_single_assign] using agreement

theorem controlled_blockAssign (slot : LocalId) (value body : ExprDenotation) (tail : StatementsDenotation)
    (entry : RuntimeFrame) (frame : Result → RuntimeFrame) (encode : Result → Control)
    (computation : Spec RuntimeState Failure Result)
    (agreement : Controlled (letNativeValue ⟨1, .variable slot⟩ value (blockResult tail body))
      entry frame encode computation) :
    Controlled (blockResult (statementsCons (nativeAssignLocal slot value) tail) body)
      entry frame encode computation :=
  controlled_congr _ _ _ _ _ _ (block_assign _ _ _ _ _) agreement

theorem controlled_pure (body : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame) (encode : Result → Control) (result : Result)
    (evaluates : ∀ initial finalFrame finalState control,
      body entry initial finalFrame finalState control ↔
        finalFrame = frame result ∧ finalState = initial ∧ control = encode result)
    (valid : ∀ result, NonThrow (encode result)) :
    Controlled body entry frame encode (Spec.pure result) := by
  constructor
  · intro initial finalFrame finalState control _
    simp [evaluates, Spec.pure, and_assoc, and_comm, and_left_comm]
  · intro initial error
    constructor
    · rintro ⟨frame, state, ran⟩
      obtain ⟨_, _, equal⟩ := (evaluates _ _ _ _).mp ran
      have nonThrow := valid result
      rw [← equal] at nonThrow
      exact nonThrow
    · exact False.elim
  · exact fun _ => False.elim
  · exact valid

/-- A typed scalar initializer feeds a control-producing continuation. The
initializer may change the store; neither normal nor failure state is frozen. -/
theorem controlled_let (binder : NativePatternBinder) (head body : ExprDenotation)
    (entry : RuntimeFrame) (encodeHead : Head → RuntimeValue) (bound : Head → RuntimeFrame)
    (first : Spec RuntimeState Failure Head) (next : Head → Spec RuntimeState Failure Result)
    (frame : Result → RuntimeFrame) (encode : Result → Control)
    (scalar : Scalar head entry encodeHead first)
    (binds : ∀ value, binder.bind entry (encodeHead value) = some (bound value))
    (continuation : ∀ value, Controlled body (bound value) frame encode (next value))
    (valid : ∀ result, NonThrow (encode result)) :
    Controlled (letNativeValue binder head body) entry frame encode (Spec.bind first next) := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    constructor
    · rintro (⟨ran, abrupt⟩ | ⟨headFrame, headState, value, boundFrame, ran, binding, continued⟩)
      · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ ran abrupt
        exact nonThrow.elim
      · obtain ⟨value, step, rfl, rfl⟩ := (scalar.normal _ _ _ _).mp ran
        rw [binds value] at binding
        cases binding
        obtain ⟨result, rest, same, equal⟩ :=
          ((continuation value).normal _ _ _ _ nonThrow).mp continued
        exact ⟨result, ⟨value, headState, step, rest⟩, same, equal⟩
    · rintro ⟨result, ⟨value, middle, step, rest⟩, same, equal⟩
      exact Or.inr ⟨entry, middle, encodeHead value, bound value,
        (scalar.normal _ _ _ _).mpr ⟨value, step, rfl, rfl⟩, binds value,
        ((continuation value).normal _ _ _ _ nonThrow).mpr ⟨result, rest, same, equal⟩⟩
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨ran, _⟩ | ⟨headFrame, headState, value, boundFrame, ran, binding, continued⟩)⟩
      · exact Or.inl ((scalar.aborts _ _).mp ⟨frame, state, ran⟩)
      · obtain ⟨value, step, rfl, rfl⟩ := (scalar.normal _ _ _ _).mp ran
        rw [binds value] at binding
        cases binding
        exact Or.inr ⟨value, headState, step,
          (continuation value).aborts _ _ |>.mp ⟨frame, state, continued⟩⟩
    · rintro (failed | ⟨value, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (scalar.aborts _ _).mpr failed
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · obtain ⟨frame, state, ran⟩ := (continuation value).aborts _ _ |>.mpr failed
        exact ⟨frame, state, Or.inr ⟨entry, middle, encodeHead value, bound value,
          (scalar.normal _ _ _ _).mpr ⟨value, step, rfl, rfl⟩, binds value, ran⟩⟩
  · intro initial undefined
    rcases undefined with undefined | ⟨value, middle, _, undefined⟩
    · exact scalar.defined _ undefined
    · exact (continuation value).defined _ undefined
  · exact valid

theorem controlled_branch (condition yes no : ExprDenotation) (entry : RuntimeFrame)
    (test : Spec RuntimeState Failure Bool) (left right : Spec RuntimeState Failure Result)
    (frame : Result → RuntimeFrame) (encode : Result → Control)
    (scalar : Scalar condition entry RuntimeValue.bool test)
    (yesAgreement : Controlled yes entry frame encode left)
    (noAgreement : Controlled no entry frame encode right) :
    Controlled (nativeBranch condition yes (some no)) entry frame encode
      (Spec.bind test (fun flag => if flag then left else right)) := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    constructor
    · rintro (⟨ran, abrupt⟩ | ⟨testFrame, testState, ran, continued⟩ |
        ⟨testFrame, testState, ran, continued⟩)
      · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ ran abrupt
        exact nonThrow.elim
      · obtain ⟨flag, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp ran
        cases equal
        obtain ⟨result, rest, same, equal⟩ := (yesAgreement.normal _ _ _ _ nonThrow).mp continued
        exact ⟨result, ⟨true, testState, step, rest⟩, same, equal⟩
      · obtain ⟨flag, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp ran
        cases equal
        obtain ⟨result, rest, same, equal⟩ := (noAgreement.normal _ _ _ _ nonThrow).mp continued
        exact ⟨result, ⟨false, testState, step, rest⟩, same, equal⟩
    · rintro ⟨result, ⟨flag, middle, step, rest⟩, same, equal⟩
      cases flag
      · exact Or.inr (Or.inr ⟨entry, middle,
          (scalar.normal _ _ _ _).mpr ⟨false, step, rfl, rfl⟩,
          (noAgreement.normal _ _ _ _ nonThrow).mpr ⟨result, rest, same, equal⟩⟩)
      · exact Or.inr (Or.inl ⟨entry, middle,
          (scalar.normal _ _ _ _).mpr ⟨true, step, rfl, rfl⟩,
          (yesAgreement.normal _ _ _ _ nonThrow).mpr ⟨result, rest, same, equal⟩⟩)
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨ran, _⟩ | ⟨testFrame, testState, ran, continued⟩ |
        ⟨testFrame, testState, ran, continued⟩)⟩
      · exact Or.inl ((scalar.aborts _ _).mp ⟨frame, state, ran⟩)
      · obtain ⟨flag, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp ran
        cases equal
        exact Or.inr ⟨true, testState, step,
          (yesAgreement.aborts _ _).mp ⟨frame, state, continued⟩⟩
      · obtain ⟨flag, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp ran
        cases equal
        exact Or.inr ⟨false, testState, step,
          (noAgreement.aborts _ _).mp ⟨frame, state, continued⟩⟩
    · rintro (failed | ⟨flag, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (scalar.aborts _ _).mpr failed
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · cases flag
        · obtain ⟨frame, state, ran⟩ := (noAgreement.aborts _ _).mpr failed
          exact ⟨frame, state, Or.inr (Or.inr ⟨entry, middle,
            (scalar.normal _ _ _ _).mpr ⟨false, step, rfl, rfl⟩, ran⟩)⟩
        · obtain ⟨frame, state, ran⟩ := (yesAgreement.aborts _ _).mpr failed
          exact ⟨frame, state, Or.inr (Or.inl ⟨entry, middle,
            (scalar.normal _ _ _ _).mpr ⟨true, step, rfl, rfl⟩, ran⟩)⟩
  · intro initial undefined
    rcases undefined with undefined | ⟨flag, middle, _, undefined⟩
    · exact scalar.defined _ undefined
    · cases flag
      · exact noAgreement.defined _ undefined
      · exact yesAgreement.defined _ undefined
  · exact yesAgreement.valid

def flowControl : NativeFlow.Flow Locals LoopLocals → Control
  | .normal _ => .value .unit
  | .continue_ _ => .continue_ 0
  | .break_ _ => .break_ 0 none

theorem flowControl_valid (flow : NativeFlow.Flow Locals LoopLocals) : NonThrow (flowControl flow) := by
  cases flow <;> trivial

theorem controlled_sequence (head body : ExprDenotation) (tail : StatementsDenotation)
    (entry : RuntimeFrame) (frame : Locals → RuntimeFrame)
    (first : Spec RuntimeState Failure (NativeFlow.Flow Locals))
    (next : Locals → Spec RuntimeState Failure (NativeFlow.Flow Locals))
    (headAgreement : Controlled head entry (fun flow => frame flow.locals) flowControl first)
    (continuation : ∀ locals, Controlled (blockResult tail body) (frame locals)
      (fun flow => frame flow.locals) flowControl (next locals)) :
    Controlled (blockResult (statementsCons head tail) body) entry
      (fun flow => frame flow.locals) flowControl (NativeFlow.sequence first next) := by
  rw [block_discard]
  constructor
  · intro initial finalFrame finalState control nonThrow
    constructor
    · rintro (⟨ran, abrupt⟩ | ⟨headFrame, headState, actual, boundFrame, ran, binds, continued⟩)
      · obtain ⟨flow, step, same, equal⟩ := (headAgreement.normal _ _ _ _ nonThrow).mp ran
        cases flow with
        | normal locals => cases equal; cases abrupt
        | continue_ locals =>
          exact ⟨.continue_ locals, ⟨.continue_ locals, finalState, step, rfl, rfl⟩, same, equal⟩
        | break_ locals =>
          exact ⟨.break_ locals, ⟨.break_ locals, finalState, step, rfl, rfl⟩, same, equal⟩
      · obtain ⟨flow, step, same, equal⟩ :=
          (headAgreement.normal _ _ _ (.value actual) trivial).mp ran
        cases binds
        cases flow with
        | normal locals =>
          cases equal
          subst headFrame
          obtain ⟨result, rest, same, equal⟩ :=
            ((continuation locals).normal _ _ _ _ nonThrow).mp continued
          exact ⟨result, ⟨.normal locals, headState, step, rest⟩, same, equal⟩
        | continue_ _ => cases equal
        | break_ _ => cases equal
    · rintro ⟨result, ⟨flow, middle, step, rest⟩, same, equal⟩
      cases flow with
      | normal locals =>
        exact Or.inr ⟨frame locals, middle, .unit, frame locals,
          (headAgreement.normal _ _ _ (.value .unit) trivial).mpr ⟨.normal locals, step, rfl, rfl⟩,
          rfl, ((continuation locals).normal _ _ _ _ nonThrow).mpr ⟨result, rest, same, equal⟩⟩
      | continue_ locals =>
        obtain ⟨rfl, rfl⟩ := rest
        cases equal
        exact Or.inl ⟨(headAgreement.normal _ _ _ (.continue_ 0) trivial).mpr
          ⟨.continue_ locals, step, same, rfl⟩, .continue_ _⟩
      | break_ locals =>
        obtain ⟨rfl, rfl⟩ := rest
        cases equal
        exact Or.inl ⟨(headAgreement.normal _ _ _ (.break_ 0 none) trivial).mpr
          ⟨.break_ locals, step, same, rfl⟩, .break_ _ _⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, (⟨ran, _⟩ |
        ⟨headFrame, headState, actual, boundFrame, ran, binds, continued⟩)⟩
      · exact Or.inl ((headAgreement.aborts _ _).mp ⟨_, _, ran⟩)
      · obtain ⟨flow, step, same, equal⟩ :=
          (headAgreement.normal _ _ _ (.value actual) trivial).mp ran
        cases binds
        cases flow with
        | normal locals =>
          cases equal
          subst headFrame
          exact Or.inr ⟨.normal locals, headState, step,
            (continuation locals).aborts _ _ |>.mp ⟨_, _, continued⟩⟩
        | continue_ _ => cases equal
        | break_ _ => cases equal
    · rintro (failed | ⟨flow, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (headAgreement.aborts _ _).mpr failed
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · cases flow with
        | normal locals =>
          obtain ⟨lastFrame, state, ran⟩ := (continuation locals).aborts _ _ |>.mpr failed
          exact ⟨lastFrame, state, Or.inr ⟨frame locals, middle, .unit, frame locals,
            (headAgreement.normal _ _ _ (.value .unit) trivial).mpr
              ⟨.normal locals, step, rfl, rfl⟩, rfl, ran⟩⟩
        | continue_ _ => cases failed
        | break_ _ => cases failed
  · intro initial undefined
    rcases undefined with undefined | ⟨flow, middle, _, undefined⟩
    · exact headAgreement.defined _ undefined
    · cases flow with
      | normal locals => exact (continuation locals).defined _ undefined
      | continue_ _ => cases undefined
      | break_ _ => cases undefined
  · exact flowControl_valid

/-- Consume the iteration's control distinction only at its enclosing loop. -/
theorem loopIteration_of_controlled
    (body : ExprDenotation) (frame : Locals → RuntimeFrame)
    (computation : Locals → Spec RuntimeState Failure (NativeFlow.Flow Locals))
    (agreement : ∀ locals, Controlled body (frame locals)
      (fun flow => frame flow.locals) flowControl (computation locals)) :
    LoopIteration body frame frame (fun locals => NativeFlow.iteration (computation locals)) := by
  constructor
  · intro locals initial finalFrame finalState
    constructor
    · rintro ⟨control, repeats, ran⟩
      have nonThrow : NonThrow control := by cases repeats <;> trivial
      obtain ⟨flow, step, same, equal⟩ := ((agreement locals).normal _ _ _ _ nonThrow).mp ran
      cases flow with
      | normal next => exact ⟨next, ⟨.normal next, step, rfl⟩, same⟩
      | continue_ next => exact ⟨next, ⟨.continue_ next, step, rfl⟩, same⟩
      | break_ next => cases equal; cases repeats
    · rintro ⟨next, ⟨flow, step, equal⟩, same⟩
      cases flow with
      | normal next =>
        cases equal
        exact ⟨.value .unit, .value _, ((agreement locals).normal _ _ _ (.value .unit) trivial).mpr
          ⟨.normal next, step, same, rfl⟩⟩
      | continue_ next =>
        cases equal
        exact ⟨.continue_ 0, .continue_, ((agreement locals).normal _ _ _ (.continue_ 0) trivial).mpr
          ⟨.continue_ next, step, same, rfl⟩⟩
      | break_ next => cases equal
  · intro locals initial finalFrame finalState
    constructor
    · intro ran
      obtain ⟨flow, step, same, equal⟩ := ((agreement locals).normal _ _ _ (.break_ 0 none) trivial).mp ran
      cases flow with
      | normal _ => cases equal
      | continue_ _ => cases equal
      | break_ next => exact ⟨next, ⟨.break_ next, step, rfl⟩, same⟩
    · rintro ⟨next, ⟨flow, step, equal⟩, same⟩
      cases flow with
      | normal _ => cases equal
      | continue_ _ => cases equal
      | break_ next =>
        cases equal
        exact ((agreement locals).normal _ _ _ (.break_ 0 none) trivial).mpr ⟨.break_ next, step, same, rfl⟩
  · intro locals initial error; exact (agreement locals).aborts _ _
  · intro locals initial finalFrame finalState control ran
    cases control with
    | throw_ kind values => exact Or.inr (Or.inr ⟨kind, values, rfl⟩)
    | value value | continue_ nest | break_ nest value | return_ values =>
      obtain ⟨flow, _, _, equal⟩ := ((agreement locals).normal _ _ _ _ (by trivial)).mp ran
      cases flow with
      | normal next => cases equal <;> exact Or.inl (.value _)
      | continue_ next => cases equal <;> exact Or.inl .continue_
      | break_ next => cases equal <;> exact Or.inr (Or.inl rfl)
  · intro locals initial; exact (agreement locals).defined _

end LeanerIR.Proofs.ComputationAgreement
