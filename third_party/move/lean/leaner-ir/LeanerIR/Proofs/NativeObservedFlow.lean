-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeFlowAgreement

/-! Execution agreement may forget dead lexical locals without storing them
in the native computation. Both simulation directions retain the exact store
and control; completeness constructs a real execution frame, rather than
asserting that every frame satisfying the observation is reachable. -/

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep LeanerIR.SemanticOperations Denotation

def BorrowFreeCell : Option RuntimeValue → Prop
  | none => True
  | some value => outermostBorrows value = #[]

theorem optional_cell (value : Option A) (encode : A → RuntimeValue)
    (plain : ∀ value, outermostBorrows (encode value) = #[]) :
    BorrowFreeCell (value.map encode) := by
  cases value with
  | none => trivial
  | some value => exact plain value

theorem integer_cell (value : Option Int) : BorrowFreeCell (value.map RuntimeValue.integer) :=
  optional_cell value _ (by intro value; simp [outermostBorrows, collectPruned, borrowEntry?])

theorem bool_cell (value : Option Bool) : BorrowFreeCell (value.map RuntimeValue.bool) :=
  optional_cell value _ (by intro value; simp [outermostBorrows, collectPruned, borrowEntry?])

theorem unit_cell (value : Option Unit) : BorrowFreeCell (value.map (fun _ => RuntimeValue.unit)) :=
  optional_cell value _ (by intro value; simp [outermostBorrows, collectPruned, borrowEntry?])

inductive CellsBorrowFree : List (Option RuntimeValue) → Prop
  | nil : CellsBorrowFree []
  | cons {value : Option RuntimeValue} {values : List (Option RuntimeValue)}
      (head : BorrowFreeCell value) (tail : CellsBorrowFree values) : CellsBorrowFree (value :: values)

theorem borrowFreeCells (values : List (Option RuntimeValue))
    (plain : CellsBorrowFree values) :
    frameBorrows { locals := values.toArray } = #[] := by
  have fold : ∀ initial, values.foldl (fun borrows slot =>
      match slot with
      | some value => borrows ++ outermostBorrows value
      | none => borrows) initial = initial := by
    induction plain with
    | nil => intro initial; rfl
    | @cons value values plain rest ih =>
      intro initial
      cases value with
      | none => exact ih initial
      | some value =>
        simpa only [List.foldl, show outermostBorrows value = #[] from plain, Array.append_empty] using ih initial
  simp only [frameBorrows, List.foldl_toArray']
  exact fold #[]

structure Observed (body : ExprDenotation) (entry : RuntimeFrame)
    (frames : Result → RuntimeFrame → Prop) (encode : Result → Control)
    (computation : Spec RuntimeState Failure Result) : Prop where
  sound : ∀ initial finalFrame finalState control, NonThrow control →
    body entry initial finalFrame finalState control →
      ∃ result, computation.ok initial result finalState ∧
        frames result finalFrame ∧ control = encode result
  complete : ∀ initial result finalState, computation.ok initial result finalState →
    ∃ finalFrame, body entry initial finalFrame finalState (encode result) ∧ frames result finalFrame
  aborts : ∀ initial error,
    (∃ finalFrame finalState, body entry initial finalFrame finalState (.throw_ error.1 error.2)) ↔
      computation.aborts initial error
  defined : ∀ initial, ¬computation.undefined initial
  valid : ∀ result, NonThrow (encode result)

theorem observed_of_controlled
    (agreement : Controlled body entry frame encode computation)
    (observes : ∀ initial result final, computation.ok initial result final → frames result (frame result)) :
    Observed body entry frames encode computation := by
  constructor
  · intro initial finalFrame finalState control valid ran
    obtain ⟨result, step, rfl, equal⟩ := (agreement.normal _ _ _ _ valid).mp ran
    exact ⟨result, step, observes _ _ _ step, equal⟩
  · intro initial result finalState ran
    exact ⟨frame result, (agreement.normal _ _ _ _ (agreement.valid result)).mpr
      ⟨result, ran, rfl, rfl⟩, observes _ _ _ ran⟩
  · exact agreement.aborts
  · exact agreement.defined
  · exact agreement.valid

theorem observed_congr (left right : ExprDenotation) (entry : RuntimeFrame)
    (frames : Result → RuntimeFrame → Prop) (encode : Result → Control)
    (computation : Spec RuntimeState Failure Result)
    (equal : ∀ initial finalFrame finalState control,
      left entry initial finalFrame finalState control ↔ right entry initial finalFrame finalState control)
    (agreement : Observed right entry frames encode computation) :
    Observed left entry frames encode computation := by
  constructor
  · intro initial finalFrame finalState control valid ran
    exact agreement.sound _ _ _ _ valid ((equal _ _ _ _).mp ran)
  · intro initial result finalState ran
    obtain ⟨frame, ran, same⟩ := agreement.complete _ _ _ ran
    exact ⟨frame, (equal _ _ _ _).mpr ran, same⟩
  · intro initial error; simp only [equal]; exact agreement.aborts _ _
  · exact agreement.defined
  · exact agreement.valid

/-- Forgetting a lexical local changes only the typed result and its frame
observation; it cannot introduce executions or hide failures. -/
theorem observed_map (map : A → B)
    (agreement : Observed body entry frames encode computation)
    (controls : ∀ value, encode value = nextEncode (map value))
    (observes : ∀ value frame, frames value frame → nextFrames (map value) frame)
    (valid : ∀ value, NonThrow (nextEncode value)) :
    Observed body entry nextFrames nextEncode
      (Spec.bind computation (fun value => Spec.pure (map value))) := by
  constructor
  · intro initial frame final control nonThrow ran
    obtain ⟨value, ran, related, equal⟩ := agreement.sound _ _ _ _ nonThrow ran
    exact ⟨map value, ⟨value, final, ran, rfl, rfl⟩, observes _ _ related,
      equal.trans (controls value)⟩
  · rintro initial result final ⟨value, middle, ran, sameValue, sameState⟩
    cases sameValue
    cases sameState
    obtain ⟨frame, ran, related⟩ := agreement.complete _ _ _ ran
    exact ⟨frame, controls value ▸ ran, observes _ _ related⟩
  · intro initial error
    constructor
    · intro ran; exact Or.inl ((agreement.aborts _ _).mp ran)
    · rintro (failed | ⟨_, _, _, impossible⟩)
      · exact (agreement.aborts _ _).mpr failed
      · cases impossible
  · intro initial undefined
    rcases undefined with undefined | ⟨_, _, _, impossible⟩
    · exact agreement.defined _ undefined
    · cases impossible
  · exact valid

theorem observed_let (binder : NativePatternBinder) (head body : ExprDenotation)
    (entry : RuntimeFrame) (encodeHead : Head → RuntimeValue) (bound : Head → RuntimeFrame)
    (first : Spec RuntimeState Failure Head) (next : Head → Spec RuntimeState Failure Result)
    (frames : Result → RuntimeFrame → Prop) (encode : Result → Control)
    (scalar : Scalar head entry encodeHead first)
    (binds : ∀ value, binder.bind entry (encodeHead value) = some (bound value))
    (continuation : ∀ value, Observed body (bound value) frames encode (next value))
    (valid : ∀ result, NonThrow (encode result)) :
    Observed (letNativeValue binder head body) entry frames encode (Spec.bind first next) := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    rintro (⟨ran, abrupt⟩ | ⟨headFrame, headState, value, boundFrame, ran, binding, continued⟩)
    · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ ran abrupt
      exact nonThrow.elim
    · obtain ⟨value, step, rfl, rfl⟩ := (scalar.normal _ _ _ _).mp ran
      rw [binds value] at binding
      cases binding
      obtain ⟨result, rest, same, equal⟩ :=
        (continuation value).sound _ _ _ _ nonThrow continued
      exact ⟨result, ⟨value, headState, step, rest⟩, same, equal⟩
  · rintro initial result finalState ⟨value, middle, step, rest⟩
    obtain ⟨finalFrame, ran, same⟩ := (continuation value).complete _ _ _ rest
    exact ⟨finalFrame, Or.inr ⟨entry, middle, encodeHead value, bound value,
      (scalar.normal _ _ _ _).mpr ⟨value, step, rfl, rfl⟩, binds value, ran⟩, same⟩
  · intro initial error
    constructor
    · rintro ⟨frame, state, (⟨ran, _⟩ | ⟨headFrame, headState, value, boundFrame, ran, binding, continued⟩)⟩
      · exact Or.inl ((scalar.aborts _ _).mp ⟨frame, state, ran⟩)
      · obtain ⟨value, step, rfl, rfl⟩ := (scalar.normal _ _ _ _).mp ran
        rw [binds value] at binding
        cases binding
        exact Or.inr ⟨value, headState, step, ((continuation value).aborts _ _).mp ⟨frame, state, continued⟩⟩
    · rintro (failed | ⟨value, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (scalar.aborts _ _).mpr failed
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · obtain ⟨frame, state, ran⟩ := ((continuation value).aborts _ _).mpr failed
        exact ⟨frame, state, Or.inr ⟨entry, middle, encodeHead value, bound value,
          (scalar.normal _ _ _ _).mpr ⟨value, step, rfl, rfl⟩, binds value, ran⟩⟩
  · intro initial undefined
    rcases undefined with undefined | ⟨value, middle, _, undefined⟩
    · exact scalar.defined _ undefined
    · exact (continuation value).defined _ undefined
  · exact valid

def flowFrames (normal : Locals → RuntimeFrame → Prop) (abrupt : LoopLocals → RuntimeFrame → Prop) :
    NativeFlow.Flow Locals LoopLocals → RuntimeFrame → Prop
  | .normal locals => normal locals
  | .continue_ locals | .break_ locals => abrupt locals

theorem observed_sequence (head body : ExprDenotation) (tail : StatementsDenotation)
    (entry : RuntimeFrame) (joinFrames : Join → RuntimeFrame → Prop)
    (frames : Locals → RuntimeFrame → Prop) (loopFrames : LoopLocals → RuntimeFrame → Prop)
    (first : Spec RuntimeState Failure (NativeFlow.Flow Join LoopLocals))
    (next : Join → Spec RuntimeState Failure (NativeFlow.Flow Locals LoopLocals))
    (headAgreement : Observed head entry (flowFrames joinFrames loopFrames) flowControl first)
    (continuation : ∀ locals frame, joinFrames locals frame →
      Observed (blockResult tail body) frame (flowFrames frames loopFrames) flowControl (next locals)) :
    Observed (blockResult (statementsCons head tail) body) entry
      (flowFrames frames loopFrames) flowControl (NativeFlow.sequence first next) := by
  rw [block_discard]
  constructor
  · intro initial finalFrame finalState control nonThrow
    rintro (⟨ran, abrupt⟩ | ⟨headFrame, headState, actual, boundFrame, ran, binds, continued⟩)
    · obtain ⟨flow, step, same, equal⟩ := headAgreement.sound _ _ _ _ nonThrow ran
      cases flow with
      | normal locals => cases equal; cases abrupt
      | continue_ locals =>
        exact ⟨.continue_ locals, ⟨.continue_ locals, finalState, step, rfl, rfl⟩, same, equal⟩
      | break_ locals =>
        exact ⟨.break_ locals, ⟨.break_ locals, finalState, step, rfl, rfl⟩, same, equal⟩
    · obtain ⟨flow, step, same, equal⟩ := headAgreement.sound _ _ _ (.value actual) trivial ran
      cases binds
      cases flow with
      | normal locals =>
        cases equal
        obtain ⟨result, rest, same, equal⟩ :=
          (continuation locals headFrame same).sound _ _ _ _ nonThrow continued
        exact ⟨result, ⟨.normal locals, headState, step, rest⟩, same, equal⟩
      | continue_ _ => cases equal
      | break_ _ => cases equal
  · rintro initial result finalState ⟨flow, middle, step, rest⟩
    obtain ⟨headFrame, ran, same⟩ := headAgreement.complete _ _ _ step
    cases flow with
    | normal locals =>
      obtain ⟨finalFrame, continued, observes⟩ := (continuation locals headFrame same).complete _ _ _ rest
      exact ⟨finalFrame, Or.inr ⟨headFrame, middle, .unit, headFrame, ran, rfl, continued⟩, observes⟩
    | continue_ locals =>
      obtain ⟨rfl, rfl⟩ := rest
      exact ⟨headFrame, Or.inl ⟨ran, .continue_ _⟩, same⟩
    | break_ locals =>
      obtain ⟨rfl, rfl⟩ := rest
      exact ⟨headFrame, Or.inl ⟨ran, .break_ _ _⟩, same⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, finalState, (⟨ran, _⟩ |
        ⟨headFrame, headState, actual, boundFrame, ran, binds, continued⟩)⟩
      · exact Or.inl ((headAgreement.aborts _ _).mp ⟨_, _, ran⟩)
      · obtain ⟨flow, step, same, equal⟩ := headAgreement.sound _ _ _ (.value actual) trivial ran
        cases binds
        cases flow with
        | normal locals =>
          cases equal
          exact Or.inr ⟨.normal locals, headState, step,
            (continuation locals headFrame same).aborts _ _ |>.mp ⟨_, _, continued⟩⟩
        | continue_ _ => cases equal
        | break_ _ => cases equal
    · rintro (failed | ⟨flow, middle, step, failed⟩)
      · obtain ⟨frame, state, ran⟩ := (headAgreement.aborts _ _).mpr failed
        exact ⟨frame, state, Or.inl ⟨ran, .throw_ _ _⟩⟩
      · obtain ⟨headFrame, step, same⟩ := headAgreement.complete _ _ _ step
        cases flow with
        | normal locals =>
          obtain ⟨lastFrame, state, ran⟩ := (continuation locals headFrame same).aborts _ _ |>.mpr failed
          exact ⟨lastFrame, state, Or.inr ⟨headFrame, middle, .unit, headFrame, step, rfl, ran⟩⟩
        | continue_ _ => cases failed
        | break_ _ => cases failed
  · intro initial undefined
    rcases undefined with undefined | ⟨flow, middle, ran, undefined⟩
    · exact headAgreement.defined _ undefined
    · obtain ⟨frame, _, same⟩ := headAgreement.complete _ _ _ ran
      cases flow with
      | normal locals => exact (continuation locals frame same).defined _ undefined
      | continue_ _ => cases undefined
      | break_ _ => cases undefined
  · exact flowControl_valid

theorem observed_branch (condition yes no : ExprDenotation) (entry : RuntimeFrame)
    (test : Spec RuntimeState Failure Bool) (left right : Spec RuntimeState Failure Result)
    (frame : Result → RuntimeFrame → Prop) (encode : Result → Control)
    (scalar : Scalar condition entry RuntimeValue.bool test)
    (yesAgreement : Observed yes entry frame encode left)
    (noAgreement : Observed no entry frame encode right) :
    Observed (nativeBranch condition yes (some no)) entry frame encode
      (Spec.bind test (fun flag => if flag then left else right)) := by
  constructor
  · intro initial finalFrame finalState control nonThrow
    rintro (⟨ran, abrupt⟩ | ⟨testFrame, testState, ran, continued⟩ |
        ⟨testFrame, testState, ran, continued⟩)
    · obtain ⟨kind, values, rfl⟩ := scalar.abrupt _ _ _ _ ran abrupt
      exact nonThrow.elim
    · obtain ⟨flag, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp ran
      cases equal
      obtain ⟨result, rest, same, equal⟩ := yesAgreement.sound _ _ _ _ nonThrow continued
      exact ⟨result, ⟨true, testState, step, rest⟩, same, equal⟩
    · obtain ⟨flag, step, rfl, equal⟩ := (scalar.normal _ _ _ _).mp ran
      cases equal
      obtain ⟨result, rest, same, equal⟩ := noAgreement.sound _ _ _ _ nonThrow continued
      exact ⟨result, ⟨false, testState, step, rest⟩, same, equal⟩
  · rintro initial result finalState ⟨flag, middle, step, rest⟩
    cases flag
    · obtain ⟨frame, ran, same⟩ := noAgreement.complete _ _ _ rest
      exact ⟨frame, Or.inr (Or.inr ⟨entry, middle,
        (scalar.normal _ _ _ _).mpr ⟨false, step, rfl, rfl⟩, ran⟩), same⟩
    · obtain ⟨frame, ran, same⟩ := yesAgreement.complete _ _ _ rest
      exact ⟨frame, Or.inr (Or.inl ⟨entry, middle,
        (scalar.normal _ _ _ _).mpr ⟨true, step, rfl, rfl⟩, ran⟩), same⟩
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

theorem observed_blockUnit (statements : StatementsDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame → Prop) (encode : Result → Control) (computation : Spec RuntimeState Failure Result)
    (agreement : Observed (blockResult statements (value .unit)) entry frame encode computation) :
    Observed (blockUnit statements) entry frame encode computation := by
  simpa only [block_unit_result] using agreement

theorem observed_blockNil (body : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame → Prop) (encode : Result → Control) (computation : Spec RuntimeState Failure Result)
    (agreement : Observed body entry frame encode computation) :
    Observed (blockResult statementsNil body) entry frame encode computation := by
  simpa only [block_nil_eq] using agreement

theorem observed_assign (slot : LocalId) (value : ExprDenotation) (entry : RuntimeFrame)
    (frame : Result → RuntimeFrame → Prop) (encode : Result → Control) (computation : Spec RuntimeState Failure Result)
    (agreement : Observed (blockUnit (statementsCons (nativeAssignLocal slot value) statementsNil))
      entry frame encode computation) :
    Observed (nativeAssignLocal slot value) entry frame encode computation := by
  simpa only [unit_single_assign] using agreement

theorem observed_blockAssign (slot : LocalId) (value body : ExprDenotation) (tail : StatementsDenotation)
    (entry : RuntimeFrame) (frame : Result → RuntimeFrame → Prop) (encode : Result → Control)
    (computation : Spec RuntimeState Failure Result)
    (agreement : Observed (letNativeValue ⟨1, .variable slot⟩ value (blockResult tail body))
      entry frame encode computation) :
    Observed (blockResult (statementsCons (nativeAssignLocal slot value) tail) body)
      entry frame encode computation :=
  observed_congr _ _ _ _ _ _ (block_assign _ _ _ _ _) agreement

theorem observed_pure (body : ExprDenotation) (entry : RuntimeFrame)
    (frames : Result → RuntimeFrame → Prop) (encode : Result → Control) (result : Result)
    (frame : RuntimeFrame)
    (evaluates : ∀ initial finalFrame finalState control,
      body entry initial finalFrame finalState control ↔
        finalFrame = frame ∧ finalState = initial ∧ control = encode result)
    (related : frames result frame)
    (valid : ∀ result, NonThrow (encode result)) :
    Observed body entry frames encode (Spec.pure result) := by
  apply observed_of_controlled
    (controlled_pure body entry (fun _ => frame) encode result evaluates valid)
  rintro initial result final ⟨rfl, rfl⟩
  exact related

end LeanerIR.Proofs.ComputationAgreement
