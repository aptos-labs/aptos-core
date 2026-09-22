-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.RowSpec

/-!
# Control and invariant boundaries for generic normalization

All ordinary control operations compute over `RowState`. At a loop, reuse
the checked finite native relation for agreement, and discharge its weakest
precondition by an entry proof and one normalized iteration. Runtime frames
occur only in this shared transport, not in the iteration proof vocabulary.
This establishes partial correctness, not termination.
-/

namespace LeanerIR.Proofs.Denotation.RowSpec

def return_ (arguments : RowSpec Values) : RowSpec Control := do
  match ← arguments with
  | .ok values => pure (.return_ values.toArray)
  | .error control => pure control

def break_ (nest : Nat) : RowSpec Control := pure (.break_ nest none)

def continue_ (nest : Nat) : RowSpec Control := pure (.continue_ nest)

theorem return_agrees {arguments : RowSpec Values} {relation : ValuesDenotation}
    (agrees : ValuesAgrees arguments relation) :
    ExprAgrees (return_ arguments) (nativeReturn relation) := by
  intro initial control frame state
  simp only [return_, nativeReturn, bind_ok]
  constructor
  · rintro ⟨result, middle, step, step'⟩
    cases result with
    | ok values =>
        obtain ⟨rfl, equal⟩ := step'
        cases equal
        exact .inl ⟨values, ((agrees initial _ _).1 values).mp step, rfl⟩
    | error control' =>
        obtain ⟨rfl, equal⟩ := step'
        cases equal
        exact .inr (((agrees initial _ _).2 control).mp step)
  · rintro (⟨values, step, rfl⟩ | step)
    · exact ⟨.ok values, RowState.ofFrame frame state,
        ((agrees initial _ _).1 values).mpr step, rfl, rfl⟩
    · exact ⟨.error control, RowState.ofFrame frame state,
        ((agrees initial _ _).2 control).mpr step, rfl, rfl⟩

theorem break_agrees (nest : Nat) : ExprAgrees (break_ nest) (nativeBreak nest none) := by
  intro initial control frame state
  simp only [break_, pure_ok, ofFrame_eq_iff, nativeBreak]
  simp only [and_comm, and_left_comm, and_assoc]

theorem continue_agrees (nest : Nat) : ExprAgrees (continue_ nest) (nativeContinue nest) := by
  intro initial control frame state
  simp only [continue_, pure_ok, ofFrame_eq_iff, nativeContinue]
  simp only [and_comm, and_left_comm, and_assoc]

theorem total_return {arguments : RowSpec Values} (total : Total arguments) :
    Total (return_ arguments) := by
  refine total_bind total fun result => ?_
  cases result <;> exact total_pure _

def assignLocal (localId : LocalId) (value : RowSpec Control) : RowSpec Control := do
  match ← value with
  | .value runtimeValue =>
      let state ← get
      if localId.index < state.row.size then
        set { state with row := state.row.set! localId.index (some runtimeValue) }
        pure (.value .unit)
      else stuck
  | control => pure control

theorem assignLocal_agrees {localId : LocalId} {value : RowSpec Control}
    {relation : ExprDenotation} (agrees : ExprAgrees value relation) :
    ExprAgrees (assignLocal localId value) (nativeAssignLocal localId relation) := by
  intro initial control frame state
  simp only [assignLocal, nativeAssignLocal, bind_ok]
  constructor
  · rintro ⟨valueControl, middle, valueStep, step⟩
    have related := (agrees initial _ middle.frame middle.state).mp (by simpa using valueStep)
    cases valueControl with
    | value runtimeValue =>
        obtain ⟨_, _, ⟨rfl, rfl⟩, step⟩ := step
        dsimp only at step
        split at step
        · rename_i bound
          obtain ⟨_, _, ⟨rfl, rfl⟩, pureStep⟩ := step
          obtain ⟨rfl, equal⟩ := pureStep
          have shape := congrArg RowState.frame equal
          have stable := congrArg RowState.state equal
          exact .inr ⟨_, _, runtimeValue, related, bound,
            shape, stable, rfl⟩
        · exact step.elim
    | break_ n v =>
        obtain ⟨rfl, equal⟩ := step
        cases equal
        exact .inl ⟨related, .break_ n v⟩
    | continue_ n =>
        obtain ⟨rfl, equal⟩ := step
        cases equal
        exact .inl ⟨related, .continue_ n⟩
    | return_ values =>
        obtain ⟨rfl, equal⟩ := step
        cases equal
        exact .inl ⟨related, .return_ values⟩
    | throw_ kind values =>
        obtain ⟨rfl, equal⟩ := step
        cases equal
        exact .inl ⟨related, .throw_ kind values⟩
  · rintro (⟨step, abrupt⟩ | ⟨valueFrame, valueState, runtimeValue, step, bound, rfl, rfl, rfl⟩)
    · refine ⟨control, RowState.ofFrame frame state, (agrees initial _ _ _).mpr step, ?_⟩
      cases abrupt <;> exact ⟨rfl, rfl⟩
    · refine ⟨.value runtimeValue, RowState.ofFrame valueFrame _,
        (agrees initial _ _ _).mpr step, _, _, ⟨rfl, rfl⟩, ?_⟩
      simp only [RowState.ofFrame, bound, ↓reduceIte, bind_ok, set_ok, pure_ok]
      exact ⟨(), _, ⟨trivial, rfl⟩, trivial, rfl⟩

theorem total_assignLocal (localId : LocalId) {value : RowSpec Control}
    (total : Total value) : Total (assignLocal localId value) := by
  refine total_bind total fun control => ?_
  cases control with
  | value runtimeValue =>
      refine total_bind total_get fun state => ?_
      split
      · exact total_bind (total_set _) fun _ => total_pure _
      · exact total_stuck
  | _ => exact total_pure _

/-- The relation is used only at the loop's agreement boundary. -/
def bodyRelation (body : RowSpec Control) : ExprDenotation :=
  fun frame state finalFrame finalState control =>
    body.ok (RowState.ofFrame frame state) control (RowState.ofFrame finalFrame finalState)

/-- Finite loop executions, sharing the checked native fixed point. -/
def loop (_site : ExprId) (body : RowSpec Control) : RowSpec Control where
  ok := fun initial control final =>
    NativeLoop (bodyRelation body) initial.frame initial.state final.frame final.state control
  aborts := fun _ _ => False

theorem loop_agrees {body : RowSpec Control} {relation : ExprDenotation}
    (site : ExprId) (agrees : ExprAgrees body relation) :
    ExprAgrees (loop site body) (nativeLoop site relation) := by
  have equal : bodyRelation body = relation := by
    funext frame state finalFrame finalState control
    exact propext (agrees (RowState.ofFrame frame state) control finalFrame finalState)
  intro initial control frame state
  simp only [loop, equal, RowState.frame_ofFrame, RowState.state_ofFrame, nativeLoop]

theorem total_loop (site : ExprId) (body : RowSpec Control) : Total (loop site body) :=
  ⟨fun _ _ => id, fun _ => id⟩

def loopPost (invariant : RowState → Prop) (post : Control → RowState → Prop)
    (control : Control) (state : RowState) : Prop :=
  match control with
  | .value _ | .continue_ 0 => invariant state
  | .continue_ (nest + 1) => post (.continue_ nest) state
  | .break_ 0 value => post (.value (value.getD .unit)) state
  | .break_ (nest + 1) value => post (.break_ nest value) state
  | .return_ values => post (.return_ values) state
  | .throw_ kind values => post (.throw_ kind values) state

theorem wp_loop_of_invariant (site : ExprId) (body : RowSpec Control)
    (invariant : RowState → Prop) (initial : RowState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (entry : invariant initial)
    (preserved : ∀ state, invariant state → wp body (loopPost invariant post) aborts state) :
    wp (loop site body) post aborts initial := by
  refine ⟨?_, fun _ h => h.elim, fun h => h.elim⟩
  intro control final step
  have checked := wpExpr_nativeLoop_of_invariant site (bodyRelation body)
    (fun frame state => invariant (RowState.ofFrame frame state))
    initial.frame initial.state
    (fun frame state control => post control (RowState.ofFrame frame state)) entry
    (by
      intro frame state holds
      unfold wpExpr
      intro finalFrame finalState control bodyStep
      have result := (preserved (RowState.ofFrame frame state) holds).1
        control (RowState.ofFrame finalFrame finalState) bodyStep
      cases control with
      | break_ nest value => cases nest <;> exact result
      | continue_ nest => cases nest <;> exact result
      | _ => exact result)
  unfold wpExpr at checked
  exact checked final.frame final.state control step

attribute [lir_wp_norm] return_ break_ continue_ assignLocal loopPost

end LeanerIR.Proofs.Denotation.RowSpec
