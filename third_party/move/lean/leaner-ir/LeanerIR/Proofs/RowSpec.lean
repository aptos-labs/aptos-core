-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Native

/-!
# The computational denotation: verification by normalization

`ExprDenotation` is a relation over frames, states, and controls — the
right form for agreement with the big-step semantics, and the wrong form
for verification, where it must be *stepped*.  This file gives a body a
second denotation as a **computation**: a `Spec` over the symbolic state
a body runs over (the row, the registries, the runtime state), returning
a control.  Its weakest precondition normalizes by rewriting — one
`lir_wp_norm` lemma per monadic primitive, the combinators unfolded, the
closed evaluators reduced by evaluation lemmas — so verification is
`simp` followed by the closer, as it was in v0 (`generic-route.md`).

The two denotations agree combinator by combinator (`ExprAgrees` and
friends), which is what the generator composes, so the chain from a
verified computation to the big-step meaning stays exact.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR.Validation
open LeanerIR.SemanticOperations
open LeanerIR.BigStep

/-! ## The symbolic state -/

/-- What a body computes over: the locals as a row, the loan registries,
and the runtime state. -/
structure RowState where
  row : Row
  registries : Registries
  state : RuntimeState

/-- The frame a row state denotes. -/
def RowState.frame (s : RowState) : RuntimeFrame := rowFrame s.row s.registries

/-- The row state of a frame and a runtime state. -/
def RowState.ofFrame (frame : RuntimeFrame) (state : RuntimeState) : RowState :=
  ⟨frame.locals,
    ⟨frame.activeLoans, frame.loanLocations, frame.typeInstantiation⟩, state⟩

@[simp] theorem RowState.frame_ofFrame (frame : RuntimeFrame) (state : RuntimeState) :
    (RowState.ofFrame frame state).frame = frame := rfl

/-- The expanded spelling of `frame_ofFrame`.  Generated postconditions
project the row and registries separately, so retaining this equation keeps
their finalization frame recognizable without unfolding either abstraction. -/
@[simp] theorem RowState.rowFrame_ofFrame (frame : RuntimeFrame) (state : RuntimeState) :
    rowFrame (RowState.ofFrame frame state).row
      (RowState.ofFrame frame state).registries = frame := rfl

@[simp] theorem RowState.state_ofFrame (frame : RuntimeFrame) (state : RuntimeState) :
    (RowState.ofFrame frame state).state = state := rfl

@[simp] theorem RowState.ofFrame_frame (s : RowState) :
    RowState.ofFrame s.frame s.state = s := rfl

theorem RowState.ofFrame_injective {frame frame' : RuntimeFrame}
    {state state' : RuntimeState}
    (eq : RowState.ofFrame frame state = RowState.ofFrame frame' state') :
    frame = frame' ∧ state = state' := by
  have frames := congrArg RowState.frame eq
  have states := congrArg RowState.state eq
  simp only [RowState.frame_ofFrame, RowState.state_ofFrame] at frames states
  exact ⟨frames, states⟩

/-- A body as a computation over the row state.  The failure vocabulary
is never used: a throw is a control, and the function boundary turns it
into the function's failure. -/
abbrev RowSpec (α : Type) := Spec RowState Failure α

namespace RowSpec

/-- A step the relation does not admit: no execution. -/
def stuck : RowSpec α := Spec.bottom

/-! ## Monadic primitives, and their weakest preconditions -/

def get : RowSpec RowState := Spec.get

def set (s : RowState) : RowSpec Unit := Spec.set s

@[simp, lir_wp_norm] theorem wp_get (post : RowState → RowState → Prop)
    (aborts : Failure → Prop) (s : RowState) :
    wp get post aborts s ↔ post s s := by
  simp [wp, get, Spec.get]

@[simp, lir_wp_norm] theorem wp_set (t : RowState) (post : Unit → RowState → Prop)
    (aborts : Failure → Prop) (s : RowState) :
    wp (set t) post aborts s ↔ post () t := by
  simp only [wp, set, Spec.set]
  constructor
  · rintro ⟨normal, -, -⟩
    exact normal () t ⟨trivial, rfl⟩
  · rintro established
    refine ⟨?_, fun _ h => h.elim, fun h => h⟩
    rintro result final ⟨-, rfl⟩
    exact established

@[simp, lir_wp_norm] theorem wp_stuck (post : α → RowState → Prop)
    (aborts : Failure → Prop) (s : RowState) :
    wp (stuck : RowSpec α) post aborts s ↔ True := by
  simp [wp, stuck, Spec.bottom]

/-- The monad instance's operations are the primitives, for the rewriter. -/
@[simp, lir_wp_norm] theorem bind_def (action : RowSpec α) (next : α → RowSpec β) :
    (action >>= next) = Spec.bind action next := rfl

@[simp, lir_wp_norm] theorem pure_def (a : α) : (pure a : RowSpec α) = Spec.pure a := rfl

/-- A branch on a decided Boolean normalizes to its two implications. -/
@[lir_wp_norm] theorem wp_ite (b : Bool) (t e : RowSpec α)
    (post : α → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (if b then t else e) post aborts s ↔
      (b = true → wp t post aborts s) ∧ (b = false → wp e post aborts s) := by
  cases b <;> simp

/-! ## The combinators -/

/-- Operand rows: the values, or the first abrupt control. -/
abbrev Values := Except Control (List RuntimeValue)

/-- Statement rows: done, or the abrupt control that stopped them. -/
abbrev Statements := Option Control

def value (runtimeValue : RuntimeValue) : RowSpec Control :=
  pure (.value runtimeValue)

def localVar (localId : LocalId) : RowSpec Control := do
  let s ← get
  match readLocal? s.frame localId with
  | some runtimeValue => pure (.value runtimeValue)
  | none => stuck

def valuesNil : RowSpec Values := pure (.ok [])

def valuesCons (head : RowSpec Control) (tail : RowSpec Values) : RowSpec Values := do
  match ← head with
  | .value runtimeValue =>
      match ← tail with
      | .ok values => pure (.ok (runtimeValue :: values))
      | .error control => pure (.error control)
  | control => pure (.error control)

def statementsNil : RowSpec Statements := pure none

def statementsCons (head : RowSpec Control) (tail : RowSpec Statements) :
    RowSpec Statements := do
  match ← head with
  | .value _ => tail
  | control => pure (some control)

def blockUnit (statements : RowSpec Statements) : RowSpec Control := do
  match ← statements with
  | none => pure (.value .unit)
  | some control => pure control

def blockResult (statements : RowSpec Statements) (result : RowSpec Control) :
    RowSpec Control := do
  match ← statements with
  | none => result
  | some control => pure control

/-- Run a closed evaluator on the operands' values from the current state. -/
def evaluate (evaluator : NativeEvaluator) (values : Array RuntimeValue) :
    RowSpec Control := do
  let s ← get
  match evaluator values s.frame s.state with
  | some (.value frame state runtimeValue) =>
      set (RowState.ofFrame frame state)
      pure (.value runtimeValue)
  | some (.throw_ frame state kind thrown) =>
      set (RowState.ofFrame frame state)
      pure (.throw_ kind thrown)
  | none => stuck

def operation (evaluator : NativeEvaluator) (operands : RowSpec Values) :
    RowSpec Control := do
  match ← operands with
  | .error control => pure control
  | .ok values => evaluate evaluator values.toArray

/-- Resume the caller after an invocation. Only the callee's newly exported
write-backs are applied; the caller's pending prefix and unrelated locals
remain its own. Returned references are registered before the continuation. -/
def resumeCall (lexical : Option Nat) (initial : RowState)
    (final : RuntimeState) (outcome : Outcome) : RowState :=
  let resumed := applyPendingFrom initial.state.pending initial.frame final
  RowState.ofFrame (callFrame lexical outcome resumed.1) resumed.2

/-- One call boundary, retaining the callee's exact relation (including the
state of a throw). Verification consumes a proved contract at this boundary;
it never needs to unfold the callee body. -/
def invoke (lexical : Option Nat) (callee : FunctionDenotation)
    (arguments : Array RuntimeValue) : RowSpec Control where
  ok := fun initial control final =>
    ∃ calleeFinal outcome, callee initial.state arguments calleeFinal outcome ∧
      control = callControl outcome ∧ final = resumeCall lexical initial calleeFinal outcome
  aborts := fun _ _ => False

def callAt (lexical : Option Nat)
    (callee : Array (TypeId × TypeId) → FunctionDenotation)
    (operands : RowSpec Values) : RowSpec Control := do
  match ← operands with
  | .error control => pure control
  | .ok values =>
      let s ← get
      invoke lexical (callee s.registries.typeInstantiation) values.toArray

def call (lexical : Option Nat) (callee : FunctionDenotation)
    (operands : RowSpec Values) : RowSpec Control :=
  callAt lexical (fun _ => callee) operands

/-- Normalization stops at the modular boundary. The continuation receives
the reconciled caller state, independently of its local-row layout. -/
@[lir_wp_norm] theorem wp_invoke (lexical : Option Nat)
    (callee : FunctionDenotation) (arguments : Array RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (invoke lexical callee arguments) post aborts s ↔
      wpFunction callee s.state arguments (fun final outcome =>
        post (callControl outcome) (resumeCall lexical s final outcome)) := by
  simp only [wp, invoke, wpFunction]
  constructor
  · rintro ⟨normal, -, -⟩ final outcome step
    exact normal _ _ ⟨final, outcome, step, rfl, rfl⟩
  · intro normal
    refine ⟨?_, fun _ h => h.elim, fun h => h.elim⟩
    rintro control final ⟨calleeFinal, outcome, step, rfl, rfl⟩
    exact normal calleeFinal outcome step

def branch (condition thenBranch : RowSpec Control)
    (elseBranch : Option (RowSpec Control)) : RowSpec Control := do
  match ← condition with
  | .value (.bool decided) =>
      if decided then thenBranch else
        match elseBranch with
        | some elseBranch => elseBranch
        | none => pure (.value .unit)
  | .value _ => stuck
  | control => pure control

def throw_ (kind : ThrowKind) (arguments : RowSpec Values) : RowSpec Control := do
  match ← arguments with
  | .ok values => pure (.throw_ kind values.toArray)
  | .error control => pure control

def letValue (binder : NativePatternBinder) (initializer body : RowSpec Control) :
    RowSpec Control := do
  match ← initializer with
  | .value runtimeValue =>
      let s ← get
      match binder.bind s.frame runtimeValue with
      | some frame =>
          set (RowState.ofFrame frame s.state)
          body
      | none => stuck
  | control => pure control

/-- Assign through a local aggregate's dynamically checked local index. -/
def assignLocalIndex (base index : LocalId) (value : RowSpec Control) :
    RowSpec Control := do
  match ← value with
  | .value runtimeValue =>
      let s ← get
      match resolveLocalIndex? base index s.frame with
      | none => stuck
      | some resolved =>
          match writeRuntimePlace? s.frame s.state resolved runtimeValue with
          | none => stuck
          | some (frame, state) =>
              set (RowState.ofFrame frame state)
              pure (.value .unit)
  | control => pure control

/-! ## Agreement with the relational combinators -/

/-- The computation admits exactly the relation's steps. -/
def ExprAgrees (computation : RowSpec Control) (relation : ExprDenotation) : Prop :=
  ∀ s control frame state,
    computation.ok s control (RowState.ofFrame frame state) ↔
      relation s.frame s.state frame state control

def ValuesAgrees (computation : RowSpec Values) (relation : ValuesDenotation) : Prop :=
  ∀ s frame state,
    (∀ values, computation.ok s (.ok values) (RowState.ofFrame frame state) ↔
      relation s.frame s.state (.values state frame values)) ∧
    (∀ control, computation.ok s (.error control) (RowState.ofFrame frame state) ↔
      relation s.frame s.state (.control state frame control))

def StatementsAgrees (computation : RowSpec Statements)
    (relation : StatementsDenotation) : Prop :=
  ∀ s frame state,
    (computation.ok s none (RowState.ofFrame frame state) ↔
      relation s.frame s.state (.done state frame)) ∧
    (∀ control, computation.ok s (some control) (RowState.ofFrame frame state) ↔
      relation s.frame s.state (.control state frame control))

/-- A computation that never fails or owes a proof: every body does. -/
structure Total (computation : RowSpec α) : Prop where
  aborts : ∀ s failure, ¬ computation.aborts s failure
  undefined : ∀ s, ¬ computation.undefined s

theorem total_pure (a : α) : Total (pure a : RowSpec α) :=
  ⟨fun _ _ => id, fun _ => id⟩

theorem total_stuck : Total (stuck : RowSpec α) :=
  ⟨fun _ _ => id, fun _ => id⟩

theorem total_get : Total get := ⟨fun _ _ => id, fun _ => id⟩

theorem total_set (t : RowState) : Total (set t) := ⟨fun _ _ => id, fun _ => id⟩

theorem total_bind {action : RowSpec α} {next : α → RowSpec β}
    (action_total : Total action) (next_total : ∀ a, Total (next a)) :
    Total (action >>= next) := by
  refine ⟨?_, ?_⟩
  · rintro s failure (fail | ⟨a, middle, -, fail⟩)
    · exact action_total.aborts s failure fail
    · exact (next_total a).aborts middle failure fail
  · rintro s (owed | ⟨a, middle, -, owed⟩)
    · exact action_total.undefined s owed
    · exact (next_total a).undefined middle owed

/-- The `ok` relation of a bind, by definition. -/
theorem bind_ok {action : RowSpec α} {next : α → RowSpec β} {s t : RowState} {b : β} :
    (action >>= next).ok s b t ↔ ∃ a middle, action.ok s a middle ∧ (next a).ok middle b t :=
  Iff.rfl

theorem pure_ok {a a' : α} {s t : RowState} :
    (pure a : RowSpec α).ok s a' t ↔ a' = a ∧ t = s :=
  Iff.rfl

theorem get_ok {s t u : RowState} : get.ok s t u ↔ t = s ∧ u = s := Iff.rfl

theorem set_ok {s t u : RowState} {v : Unit} : (set t).ok s v u ↔ v = () ∧ u = t := Iff.rfl

theorem stuck_ok {s t : RowState} {a : α} : (stuck : RowSpec α).ok s a t ↔ False := Iff.rfl

theorem ofFrame_eq_iff {frame : RuntimeFrame} {state : RuntimeState} {s : RowState} :
    RowState.ofFrame frame state = s ↔ frame = s.frame ∧ state = s.state := by
  constructor
  · intro eq
    exact RowState.ofFrame_injective (eq.trans (RowState.ofFrame_frame s).symm)
  · rintro ⟨rfl, rfl⟩
    rfl

/-! The `ok` relations of the primitives, as one simp set: a combinator's
`ok` unfolds to existentials over the intermediate row states. -/
section Agreement

attribute [local simp] bind_ok pure_ok get_ok set_ok stuck_ok ofFrame_eq_iff

theorem value_agrees (runtimeValue : RuntimeValue) :
    ExprAgrees (value runtimeValue) (Denotation.value runtimeValue) := by
  intro s control frame state
  simp [value, Denotation.value]
  constructor
  · rintro ⟨rfl, rfl, rfl⟩; exact ⟨rfl, rfl, rfl⟩
  · rintro ⟨rfl, rfl, rfl⟩; exact ⟨rfl, rfl, rfl⟩

theorem localVar_agrees (localId : LocalId) :
    ExprAgrees (localVar localId) (Denotation.localVar localId) := by
  intro s control frame state
  simp only [localVar, Denotation.localVar, bind_ok, get_ok]
  constructor
  · rintro ⟨_, _, ⟨rfl, rfl⟩, step⟩
    split at step
    · rename_i runtimeValue read
      obtain ⟨rfl, eq⟩ := step
      obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq
      exact ⟨runtimeValue, read, rfl, rfl, rfl⟩
    · exact step.elim
  · rintro ⟨runtimeValue, read, rfl, rfl, rfl⟩
    refine ⟨s, s, ⟨rfl, rfl⟩, ?_⟩
    simp [read]

theorem valuesNil_agrees : ValuesAgrees valuesNil Denotation.valuesNil := by
  intro s frame state
  simp only [valuesNil, Denotation.valuesNil, pure_ok]
  constructor
  · intro values
    constructor
    · rintro ⟨eq, eq'⟩
      cases eq
      obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
      rfl
    · intro eq
      cases eq
      exact ⟨rfl, rfl⟩
  · intro control
    constructor
    · rintro ⟨eq, -⟩; cases eq
    · intro eq; cases eq

/-- The abrupt controls: everything but a value. -/
theorem abrupt_iff {control : Control} : Abrupt control ↔ ∀ v, control ≠ .value v := by
  constructor
  · rintro (_ | _ | _ | _) v <;> exact Control.noConfusion
  · intro ne
    cases control with
    | value v => exact absurd rfl (ne v)
    | break_ n v => exact .break_ n v
    | continue_ n => exact .continue_ n
    | return_ vs => exact .return_ vs
    | throw_ k a => exact .throw_ k a

theorem valuesCons_agrees {head : RowSpec Control} {tail : RowSpec Values}
    {headRelation : ExprDenotation} {tailRelation : ValuesDenotation}
    (head_agrees : ExprAgrees head headRelation)
    (tail_agrees : ValuesAgrees tail tailRelation) :
    ValuesAgrees (valuesCons head tail) (Denotation.valuesCons headRelation tailRelation) := by
  intro s frame state
  simp only [valuesCons, Denotation.valuesCons, bind_ok]
  constructor
  · intro values
    constructor
    · rintro ⟨control, middle, headStep, step⟩
      cases control with
      | value runtimeValue =>
          obtain ⟨tailResult, after, tailStep, step⟩ := step
          cases tailResult with
          | ok tailValues =>
              obtain ⟨eq, eq'⟩ := step
              cases eq
              obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
              refine Or.inr (Or.inl ⟨middle.frame, middle.state, runtimeValue, after.frame,
                after.state, tailValues, ?_, ?_, rfl⟩)
              · exact (head_agrees s _ _ _).mp (by simpa using headStep)
              · exact ((tail_agrees middle after.frame after.state).1 tailValues).mp
                  (by simpa using tailStep)
          | error control => obtain ⟨eq, -⟩ := step; cases eq
      | break_ n v => obtain ⟨eq, -⟩ := step; cases eq
      | continue_ n => obtain ⟨eq, -⟩ := step; cases eq
      | return_ vs => obtain ⟨eq, -⟩ := step; cases eq
      | throw_ k a => obtain ⟨eq, -⟩ := step; cases eq
    · rintro (⟨finalFrame, finalState, control, headStep, abrupt, eq⟩ |
        ⟨headFrame, headState, runtimeValue, finalFrame, finalState, tailValues,
          headStep, tailStep, eq⟩ |
        ⟨headFrame, headState, runtimeValue, finalFrame, finalState, control,
          headStep, tailStep, eq⟩)
      · cases eq
      · injection eq with hs hf hv
        subst hs hf hv
        refine ⟨.value runtimeValue, RowState.ofFrame headFrame headState,
          (head_agrees s _ _ _).mpr headStep, .ok tailValues, RowState.ofFrame frame state,
          ?_, rfl, rfl⟩
        exact ((tail_agrees (RowState.ofFrame headFrame headState) frame state).1 tailValues).mpr
          (by simpa using tailStep)
      · cases eq
  · intro control
    constructor
    · rintro ⟨headControl, middle, headStep, step⟩
      cases headControl with
      | value runtimeValue =>
          obtain ⟨tailResult, after, tailStep, step⟩ := step
          cases tailResult with
          | ok tailValues => obtain ⟨eq, -⟩ := step; cases eq
          | error control' =>
              obtain ⟨eq, eq'⟩ := step
              cases eq
              obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
              refine Or.inr (Or.inr ⟨middle.frame, middle.state, runtimeValue, after.frame,
                after.state, control, ?_, ?_, rfl⟩)
              · exact (head_agrees s _ _ _).mp (by simpa using headStep)
              · exact ((tail_agrees middle after.frame after.state).2 control).mp
                  (by simpa using tailStep)
      | break_ n v =>
          obtain ⟨eq, eq'⟩ := step
          cases eq
          obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
          exact Or.inl ⟨middle.frame, middle.state, _,
            (head_agrees s _ _ _).mp (by simpa using headStep), .break_ n v, rfl⟩
      | continue_ n =>
          obtain ⟨eq, eq'⟩ := step
          cases eq
          obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
          exact Or.inl ⟨middle.frame, middle.state, _,
            (head_agrees s _ _ _).mp (by simpa using headStep), .continue_ n, rfl⟩
      | return_ vs =>
          obtain ⟨eq, eq'⟩ := step
          cases eq
          obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
          exact Or.inl ⟨middle.frame, middle.state, _,
            (head_agrees s _ _ _).mp (by simpa using headStep), .return_ vs, rfl⟩
      | throw_ k a =>
          obtain ⟨eq, eq'⟩ := step
          cases eq
          obtain ⟨rfl, rfl⟩ := ofFrame_eq_iff.mp eq'
          exact Or.inl ⟨middle.frame, middle.state, _,
            (head_agrees s _ _ _).mp (by simpa using headStep), .throw_ k a, rfl⟩
    · rintro (⟨finalFrame, finalState, control', headStep, abrupt, eq⟩ |
        ⟨headFrame, headState, runtimeValue, finalFrame, finalState, tailValues,
          headStep, tailStep, eq⟩ |
        ⟨headFrame, headState, runtimeValue, finalFrame, finalState, control',
          headStep, tailStep, eq⟩)
      · injection eq with hs hf hc
        subst hs hf hc
        refine ⟨control, RowState.ofFrame frame state, (head_agrees s _ _ _).mpr headStep, ?_⟩
        cases abrupt <;> exact ⟨rfl, rfl⟩
      · cases eq
      · injection eq with hs hf hc
        subst hs hf hc
        refine ⟨.value runtimeValue, RowState.ofFrame headFrame headState,
          (head_agrees s _ _ _).mpr headStep, .error control, RowState.ofFrame frame state,
          ?_, rfl, rfl⟩
        exact ((tail_agrees (RowState.ofFrame headFrame headState) frame state).2 control).mpr
          (by simpa using tailStep)

/-- Close a `pure` step against a relational conjunction of equalities. -/
private theorem pure_step {a a' : α} {s frame : _} {state : RuntimeState}
    (step : (pure a : RowSpec α).ok s a' (RowState.ofFrame frame state)) :
    a' = a ∧ frame = s.frame ∧ state = s.state := by
  obtain ⟨rfl, eq⟩ := step
  exact ⟨rfl, ofFrame_eq_iff.mp eq⟩

theorem statementsNil_agrees : StatementsAgrees statementsNil Denotation.statementsNil := by
  intro s frame state
  simp only [statementsNil, Denotation.statementsNil]
  constructor
  · constructor
    · intro step
      obtain ⟨-, rfl, rfl⟩ := pure_step step
      rfl
    · intro eq
      cases eq
      exact ⟨rfl, rfl⟩
  · intro control
    constructor
    · intro step
      obtain ⟨eq, -⟩ := pure_step step
      cases eq
    · intro eq
      cases eq

theorem statementsCons_agrees {head : RowSpec Control} {tail : RowSpec Statements}
    {headRelation : ExprDenotation} {tailRelation : StatementsDenotation}
    (head_agrees : ExprAgrees head headRelation)
    (tail_agrees : StatementsAgrees tail tailRelation) :
    StatementsAgrees (statementsCons head tail)
      (Denotation.statementsCons headRelation tailRelation) := by
  intro s frame state
  simp only [statementsCons, Denotation.statementsCons, bind_ok]
  have abruptHead : ∀ (control : Control) middle,
      head.ok s control middle →
      (∀ v, control ≠ .value v) →
      (∀ result, (pure (some control) : RowSpec Statements).ok middle result
          (RowState.ofFrame frame state) →
        (∃ finalFrame finalState control', headRelation s.frame s.state finalFrame finalState
            control' ∧ Abrupt control' ∧
            (.control state frame result.get! : StatementsResult) =
              .control finalState finalFrame control') ∨ False) := by
    intro control middle headStep ne result step
    obtain ⟨rfl, rfl, rfl⟩ := pure_step step
    exact Or.inl ⟨_, _, _, (head_agrees s _ _ _).mp (by simpa using headStep),
      abrupt_iff.mpr ne, rfl⟩
  constructor
  · constructor
    · rintro ⟨control, middle, headStep, step⟩
      cases control with
      | value runtimeValue =>
          exact Or.inr ⟨middle.frame, middle.state, runtimeValue,
            (head_agrees s _ _ _).mp (by simpa using headStep),
            ((tail_agrees middle frame state).1).mp step⟩
      | break_ n v => obtain ⟨eq, -⟩ := pure_step step; cases eq
      | continue_ n => obtain ⟨eq, -⟩ := pure_step step; cases eq
      | return_ vs => obtain ⟨eq, -⟩ := pure_step step; cases eq
      | throw_ k a => obtain ⟨eq, -⟩ := pure_step step; cases eq
    · rintro (⟨finalFrame, finalState, control, headStep, abrupt, eq⟩ |
        ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩)
      · cases eq
      · exact ⟨.value runtimeValue, RowState.ofFrame headFrame headState,
          (head_agrees s _ _ _).mpr headStep,
          ((tail_agrees (RowState.ofFrame headFrame headState) frame state).1).mpr
            (by simpa using tailStep)⟩
  · intro control
    constructor
    · rintro ⟨headControl, middle, headStep, step⟩
      cases headControl with
      | value runtimeValue =>
          exact Or.inr ⟨middle.frame, middle.state, runtimeValue,
            (head_agrees s _ _ _).mp (by simpa using headStep),
            ((tail_agrees middle frame state).2 control).mp step⟩
      | break_ n v =>
          obtain ⟨eq, rfl, rfl⟩ := pure_step step
          cases eq
          exact Or.inl ⟨_, _, _, (head_agrees s _ _ _).mp (by simpa using headStep),
            .break_ n v, rfl⟩
      | continue_ n =>
          obtain ⟨eq, rfl, rfl⟩ := pure_step step
          cases eq
          exact Or.inl ⟨_, _, _, (head_agrees s _ _ _).mp (by simpa using headStep),
            .continue_ n, rfl⟩
      | return_ vs =>
          obtain ⟨eq, rfl, rfl⟩ := pure_step step
          cases eq
          exact Or.inl ⟨_, _, _, (head_agrees s _ _ _).mp (by simpa using headStep),
            .return_ vs, rfl⟩
      | throw_ k a =>
          obtain ⟨eq, rfl, rfl⟩ := pure_step step
          cases eq
          exact Or.inl ⟨_, _, _, (head_agrees s _ _ _).mp (by simpa using headStep),
            .throw_ k a, rfl⟩
    · rintro (⟨finalFrame, finalState, control', headStep, abrupt, eq⟩ |
        ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩)
      · injection eq with hs hf hc
        subst hs hf hc
        refine ⟨control, RowState.ofFrame frame state, (head_agrees s _ _ _).mpr headStep, ?_⟩
        cases abrupt <;> exact ⟨rfl, rfl⟩
      · exact ⟨.value runtimeValue, RowState.ofFrame headFrame headState,
          (head_agrees s _ _ _).mpr headStep,
          ((tail_agrees (RowState.ofFrame headFrame headState) frame state).2 control).mpr
            (by simpa using tailStep)⟩

theorem blockUnit_agrees {statements : RowSpec Statements}
    {relation : StatementsDenotation} (agrees : StatementsAgrees statements relation) :
    ExprAgrees (blockUnit statements) (Denotation.blockUnit relation) := by
  intro s control frame state
  simp only [blockUnit, Denotation.blockUnit, bind_ok]
  constructor
  · rintro ⟨result, middle, step, step'⟩
    cases result with
    | none =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step'
        exact Or.inr ⟨((agrees s _ _).1).mp (by simpa using step), rfl⟩
    | some control' =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step'
        exact Or.inl (((agrees s _ _).2 control).mp (by simpa using step))
  · rintro (step | ⟨step, rfl⟩)
    · exact ⟨some control, RowState.ofFrame frame state, ((agrees s _ _).2 control).mpr step,
        rfl, rfl⟩
    · exact ⟨none, RowState.ofFrame frame state, ((agrees s _ _).1).mpr step, rfl, rfl⟩

theorem blockResult_agrees {statements : RowSpec Statements} {result : RowSpec Control}
    {relation : StatementsDenotation} {resultRelation : ExprDenotation}
    (agrees : StatementsAgrees statements relation)
    (result_agrees : ExprAgrees result resultRelation) :
    ExprAgrees (blockResult statements result)
      (Denotation.blockResult relation resultRelation) := by
  intro s control frame state
  simp only [blockResult, Denotation.blockResult, bind_ok]
  constructor
  · rintro ⟨statementsResult, middle, step, step'⟩
    cases statementsResult with
    | none =>
        exact Or.inr ⟨middle.frame, middle.state, ((agrees s _ _).1).mp (by simpa using step),
          (result_agrees middle _ _ _).mp step'⟩
    | some control' =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step'
        exact Or.inl (((agrees s _ _).2 control).mp (by simpa using step))
  · rintro (step | ⟨statementFrame, statementState, step, resultStep⟩)
    · exact ⟨some control, RowState.ofFrame frame state, ((agrees s _ _).2 control).mpr step,
        rfl, rfl⟩
    · exact ⟨none, RowState.ofFrame statementFrame statementState, ((agrees s _ _).1).mpr step,
        (result_agrees _ _ _ _).mpr (by simpa using resultStep)⟩

theorem evaluate_agrees (evaluator : NativeEvaluator) (values : Array RuntimeValue)
    (s : RowState) (control : Control) (frame : RuntimeFrame) (state : RuntimeState) :
    (evaluate evaluator values).ok s control (RowState.ofFrame frame state) ↔
      (∃ runtimeValue, evaluator values s.frame s.state = some (.value frame state runtimeValue) ∧
        control = .value runtimeValue) ∨
      (∃ kind thrown, evaluator values s.frame s.state = some (.throw_ frame state kind thrown) ∧
        control = .throw_ kind thrown) := by
  simp only [evaluate, bind_ok, get_ok]
  constructor
  · rintro ⟨_, _, ⟨rfl, rfl⟩, step⟩
    split at step
    · rename_i frame' state' runtimeValue evaluated
      obtain ⟨_, _, ⟨rfl, rfl⟩, step⟩ := step
      obtain ⟨rfl, eq⟩ := pure_step step
      simp only [RowState.frame_ofFrame, RowState.state_ofFrame] at eq
      obtain ⟨rfl, rfl⟩ := eq
      exact Or.inl ⟨runtimeValue, evaluated, rfl⟩
    · rename_i frame' state' kind thrown evaluated
      obtain ⟨_, _, ⟨rfl, rfl⟩, step⟩ := step
      obtain ⟨rfl, eq⟩ := pure_step step
      simp only [RowState.frame_ofFrame, RowState.state_ofFrame] at eq
      obtain ⟨rfl, rfl⟩ := eq
      exact Or.inr ⟨kind, thrown, evaluated, rfl⟩
    · exact step.elim
  · rintro (⟨runtimeValue, evaluated, rfl⟩ | ⟨kind, thrown, evaluated, rfl⟩)
    · refine ⟨s, s, ⟨rfl, rfl⟩, ?_⟩
      simp only [evaluated]
      exact ⟨(), RowState.ofFrame frame state, ⟨rfl, rfl⟩, rfl, rfl⟩
    · refine ⟨s, s, ⟨rfl, rfl⟩, ?_⟩
      simp only [evaluated]
      exact ⟨(), RowState.ofFrame frame state, ⟨rfl, rfl⟩, rfl, rfl⟩

theorem operation_agrees {evaluator : NativeEvaluator} {operands : RowSpec Values}
    {relation : ValuesDenotation} (agrees : ValuesAgrees operands relation) :
    ExprAgrees (operation evaluator operands) (nativeOperation evaluator relation) := by
  intro s control frame state
  simp only [operation, nativeOperation, bind_ok]
  constructor
  · rintro ⟨result, middle, step, step'⟩
    cases result with
    | error control' =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step'
        exact Or.inl ⟨_, _, _, ((agrees s _ _).2 control).mp (by simpa using step), rfl, rfl, rfl⟩
    | ok values =>
        rcases (evaluate_agrees _ _ _ _ _ _).mp step' with
          ⟨runtimeValue, evaluated, rfl⟩ | ⟨kind, thrown, evaluated, rfl⟩
        · exact Or.inr ⟨middle.frame, middle.state, values,
            ((agrees s _ _).1 values).mp (by simpa using step),
            Or.inl ⟨runtimeValue, evaluated, rfl⟩⟩
        · exact Or.inr ⟨middle.frame, middle.state, values,
            ((agrees s _ _).1 values).mp (by simpa using step),
            Or.inr ⟨kind, thrown, evaluated, rfl⟩⟩
  · rintro (⟨operandFrame, operandState, propagated, step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, step,
        ⟨runtimeValue, evaluated, rfl⟩ | ⟨kind, thrown, evaluated, rfl⟩⟩)
    · exact ⟨.error control, RowState.ofFrame frame state,
        ((agrees s _ _).2 control).mpr step, rfl, rfl⟩
    · refine ⟨.ok values, RowState.ofFrame operandFrame operandState,
        ((agrees s _ _).1 values).mpr step, ?_⟩
      exact (evaluate_agrees _ _ _ _ _ _).mpr (Or.inl ⟨runtimeValue, by simpa using evaluated, rfl⟩)
    · refine ⟨.ok values, RowState.ofFrame operandFrame operandState,
        ((agrees s _ _).1 values).mpr step, ?_⟩
      exact (evaluate_agrees _ _ _ _ _ _).mpr (Or.inr ⟨kind, thrown, by simpa using evaluated, rfl⟩)

theorem callAt_agrees {lexical : Option Nat}
    {callee : Array (TypeId × TypeId) → FunctionDenotation}
    {operands : RowSpec Values} {relation : ValuesDenotation}
    (agrees : ValuesAgrees operands relation) :
    ExprAgrees (callAt lexical callee operands)
      (Denotation.callAt lexical callee relation) := by
  intro s control frame state
  simp only [callAt, Denotation.callAt, bind_ok]
  constructor
  · rintro ⟨result, middle, step, next⟩
    cases result with
    | error propagated =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step next
        exact .inl ⟨_, _, control,
          ((agrees s _ _).2 control).mp (by simpa using step), rfl, rfl, rfl⟩
    | ok values =>
        obtain ⟨observed, unchanged, ⟨rfl, rfl⟩, calleeFinal, outcome,
          called, rfl, resumed⟩ := next
        have eq := RowState.ofFrame_injective resumed
        exact .inr ⟨_, _, values, calleeFinal, outcome,
          ((agrees s _ _).1 values).mp (by simpa using step), called,
          eq.1, eq.2, rfl⟩
  · rintro (⟨operandFrame, operandState, propagated, step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, calleeFinal, outcome, step,
        called, rfl, rfl, rfl⟩)
    · exact ⟨.error control, RowState.ofFrame frame state,
        ((agrees s _ _).2 control).mpr step, rfl, rfl⟩
    · refine ⟨.ok values, RowState.ofFrame operandFrame operandState,
        ((agrees s _ _).1 values).mpr step, ?_⟩
      exact ⟨_, _, ⟨rfl, rfl⟩, calleeFinal, outcome, called, rfl, rfl⟩

theorem call_agrees {lexical : Option Nat} {callee : FunctionDenotation}
    {operands : RowSpec Values} {relation : ValuesDenotation}
    (agrees : ValuesAgrees operands relation) :
    ExprAgrees (call lexical callee operands) (Denotation.call lexical callee relation) :=
  callAt_agrees agrees

theorem throw_agrees {kind : ThrowKind} {arguments : RowSpec Values}
    {relation : ValuesDenotation} (agrees : ValuesAgrees arguments relation) :
    ExprAgrees (throw_ kind arguments) (nativeThrow kind relation) := by
  intro s control frame state
  simp only [throw_, nativeThrow, bind_ok]
  constructor
  · rintro ⟨result, middle, step, step'⟩
    cases result with
    | ok values =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step'
        exact Or.inl ⟨values, ((agrees s _ _).1 values).mp (by simpa using step), rfl⟩
    | error control' =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step'
        exact Or.inr (((agrees s _ _).2 control).mp (by simpa using step))
  · rintro (⟨values, step, rfl⟩ | step)
    · exact ⟨.ok values, RowState.ofFrame frame state, ((agrees s _ _).1 values).mpr step,
        rfl, rfl⟩
    · exact ⟨.error control, RowState.ofFrame frame state, ((agrees s _ _).2 control).mpr step,
        rfl, rfl⟩

theorem branch_agrees {condition thenBranch : RowSpec Control}
    {elseBranch : Option (RowSpec Control)}
    {conditionRelation thenRelation : ExprDenotation}
    {elseRelation : Option ExprDenotation}
    (condition_agrees : ExprAgrees condition conditionRelation)
    (then_agrees : ExprAgrees thenBranch thenRelation)
    (else_agrees : ∀ e, elseBranch = some e → ∃ r, elseRelation = some r ∧ ExprAgrees e r)
    (else_none : elseBranch = none ↔ elseRelation = none) :
    ExprAgrees (branch condition thenBranch elseBranch)
      (nativeBranch conditionRelation thenRelation elseRelation) := by
  intro s control frame state
  simp only [branch, nativeBranch, bind_ok]
  constructor
  · rintro ⟨conditionControl, middle, conditionStep, step⟩
    have conditionRel := (condition_agrees s _ middle.frame middle.state).mp
      (by simpa using conditionStep)
    cases conditionControl with
    | value runtimeValue =>
        cases runtimeValue with
        | bool decided =>
            cases decided with
            | true =>
                exact Or.inr (Or.inl ⟨middle.frame, middle.state, conditionRel,
                  (then_agrees middle _ _ _).mp step⟩)
            | false =>
                refine Or.inr (Or.inr ⟨middle.frame, middle.state, conditionRel, ?_⟩)
                cases elseBranch with
                | some e =>
                    obtain ⟨r, rfl, e_agrees⟩ := else_agrees e rfl
                    exact (e_agrees middle _ _ _).mp step
                | none =>
                    rw [else_none.mp rfl]
                    obtain ⟨rfl, rfl, rfl⟩ := pure_step step
                    exact ⟨rfl, rfl, rfl⟩
        | _ => exact step.elim
    | break_ n v =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨conditionRel, .break_ n v⟩
    | continue_ n =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨conditionRel, .continue_ n⟩
    | return_ vs =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨conditionRel, .return_ vs⟩
    | throw_ k a =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨conditionRel, .throw_ k a⟩
  · rintro (⟨conditionStep, abrupt⟩ |
      ⟨conditionFrame, conditionState, conditionStep, thenStep⟩ |
      ⟨conditionFrame, conditionState, conditionStep, elseStep⟩)
    · refine ⟨control, RowState.ofFrame frame state, (condition_agrees s _ _ _).mpr conditionStep, ?_⟩
      cases abrupt <;> exact ⟨rfl, rfl⟩
    · exact ⟨.value (.bool true), RowState.ofFrame conditionFrame conditionState,
        (condition_agrees s _ _ _).mpr conditionStep,
        (then_agrees _ _ _ _).mpr (by simpa using thenStep)⟩
    · refine ⟨.value (.bool false), RowState.ofFrame conditionFrame conditionState,
        (condition_agrees s _ _ _).mpr conditionStep, ?_⟩
      cases elseBranch with
      | some e =>
          obtain ⟨r, req, e_agrees⟩ := else_agrees e rfl
          rw [req] at elseStep
          exact (e_agrees _ _ _ _).mpr (by simpa using elseStep)
      | none =>
          rw [else_none.mp rfl] at elseStep
          obtain ⟨rfl, rfl, rfl⟩ := elseStep
          exact ⟨rfl, rfl⟩

theorem branchNone_agrees {condition thenBranch : RowSpec Control}
    {conditionRelation thenRelation : ExprDenotation}
    (condition_agrees : ExprAgrees condition conditionRelation)
    (then_agrees : ExprAgrees thenBranch thenRelation) :
    ExprAgrees (branch condition thenBranch none)
      (nativeBranch conditionRelation thenRelation none) :=
  branch_agrees condition_agrees then_agrees (fun _ h => nomatch h)
    ⟨fun _ => rfl, fun _ => rfl⟩

theorem branchSome_agrees {condition thenBranch elseBranch : RowSpec Control}
    {conditionRelation thenRelation elseRelation : ExprDenotation}
    (condition_agrees : ExprAgrees condition conditionRelation)
    (then_agrees : ExprAgrees thenBranch thenRelation)
    (else_agrees : ExprAgrees elseBranch elseRelation) :
    ExprAgrees (branch condition thenBranch (some elseBranch))
      (nativeBranch conditionRelation thenRelation (some elseRelation)) :=
  branch_agrees condition_agrees then_agrees
    (fun _ h => ⟨elseRelation, rfl, Option.some.inj h ▸ else_agrees⟩)
    ⟨(fun h => nomatch h), (fun h => nomatch h)⟩

theorem letValue_agrees {binder : NativePatternBinder} {initializer body : RowSpec Control}
    {initializerRelation bodyRelation : ExprDenotation}
    (initializer_agrees : ExprAgrees initializer initializerRelation)
    (body_agrees : ExprAgrees body bodyRelation) :
    ExprAgrees (letValue binder initializer body)
      (letNativeValue binder initializerRelation bodyRelation) := by
  intro s control frame state
  simp only [letValue, letNativeValue, bind_ok]
  constructor
  · rintro ⟨initializerControl, middle, initializerStep, step⟩
    have initializerRel := (initializer_agrees s _ middle.frame middle.state).mp
      (by simpa using initializerStep)
    cases initializerControl with
    | value runtimeValue =>
        obtain ⟨_, _, ⟨rfl, rfl⟩, step⟩ := step
        dsimp only at step
        split at step
        · rename_i boundFrame bound
          obtain ⟨_, _, ⟨rfl, rfl⟩, step⟩ := step
          exact Or.inr ⟨_, _, runtimeValue, boundFrame, initializerRel,
            bound, (body_agrees _ _ _ _).mp step⟩
        · exact step.elim
    | break_ n v =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨initializerRel, .break_ n v⟩
    | continue_ n =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨initializerRel, .continue_ n⟩
    | return_ vs =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨initializerRel, .return_ vs⟩
    | throw_ k a =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact Or.inl ⟨initializerRel, .throw_ k a⟩
  · rintro (⟨initializerStep, abrupt⟩ |
      ⟨initializedFrame, initializedState, runtimeValue, boundFrame, initializerStep, bound,
        bodyStep⟩)
    · refine ⟨control, RowState.ofFrame frame state,
        (initializer_agrees s _ _ _).mpr initializerStep, ?_⟩
      cases abrupt <;> exact ⟨rfl, rfl⟩
    · refine ⟨.value runtimeValue, RowState.ofFrame initializedFrame initializedState,
        (initializer_agrees s _ _ _).mpr initializerStep, _, _, ⟨rfl, rfl⟩, ?_⟩
      simp only [RowState.frame_ofFrame, bound]
      exact ⟨(), RowState.ofFrame boundFrame initializedState, ⟨rfl, rfl⟩,
        (body_agrees _ _ _ _).mpr (by simpa using bodyStep)⟩

theorem assignLocalIndex_agrees {base index : LocalId} {value : RowSpec Control}
    {valueRelation : ExprDenotation}
    (value_agrees : ExprAgrees value valueRelation) :
    ExprAgrees (assignLocalIndex base index value)
      (nativeAssignLocalIndex base index valueRelation) := by
  intro s control frame state
  simp only [assignLocalIndex, nativeAssignLocalIndex, bind_ok]
  constructor
  · rintro ⟨valueControl, middle, valueStep, step⟩
    have valueRel := (value_agrees s _ middle.frame middle.state).mp
      (by simpa using valueStep)
    cases valueControl with
    | value runtimeValue =>
        obtain ⟨_, _, ⟨rfl, rfl⟩, step⟩ := step
        dsimp only at step
        split at step
        · exact step.elim
        · rename_i resolved resolve_eq
          split at step
          · exact step.elim
          · rename_i result write_eq
            obtain ⟨finalFrame', finalState'⟩ := result
            obtain ⟨_, _, setStep, pureStep⟩ := step
            obtain ⟨rfl, rfl⟩ := set_ok.mp setStep
            obtain ⟨rfl, final_eq⟩ := pure_step pureStep
            obtain ⟨rfl, rfl⟩ := final_eq
            exact .inr ⟨_, _, runtimeValue, resolved,
              valueRel, resolve_eq, write_eq, rfl⟩
    | break_ n v =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact .inl ⟨valueRel, .break_ n v⟩
    | continue_ n =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact .inl ⟨valueRel, .continue_ n⟩
    | return_ values =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact .inl ⟨valueRel, .return_ values⟩
    | throw_ kind arguments =>
        obtain ⟨rfl, rfl, rfl⟩ := pure_step step
        exact .inl ⟨valueRel, .throw_ kind arguments⟩
  · rintro (⟨valueStep, abrupt⟩ |
      ⟨valueFrame, valueState, runtimeValue, resolved, valueStep,
        resolve_eq, write_eq, rfl⟩)
    · refine ⟨control, RowState.ofFrame frame state,
        (value_agrees s _ _ _).mpr valueStep, ?_⟩
      cases abrupt <;> exact ⟨rfl, rfl⟩
    · refine ⟨.value runtimeValue, RowState.ofFrame valueFrame valueState,
        (value_agrees s _ _ _).mpr valueStep, _, _, ⟨rfl, rfl⟩, ?_⟩
      simp only [RowState.frame_ofFrame, RowState.state_ofFrame, resolve_eq]
      simp only [write_eq]
      exact ⟨(), RowState.ofFrame frame state, ⟨rfl, rfl⟩, rfl, rfl⟩

/-! ## Totality: a body never fails or owes a proof -/

theorem total_value (v : RuntimeValue) : Total (value v) := total_pure _
theorem total_valuesNil : Total valuesNil := total_pure _
theorem total_statementsNil : Total statementsNil := total_pure _

private theorem total_match_control {k : Control → RowSpec α}
    (h : ∀ c, Total (k c)) (c : Control) : Total (k c) := h c

theorem total_localVar (i : LocalId) : Total (localVar i) := by
  refine total_bind total_get fun s => ?_
  split <;> first | exact total_pure _ | exact total_stuck

theorem total_valuesCons {head : RowSpec Control} {tail : RowSpec Values}
    (h : Total head) (t : Total tail) : Total (valuesCons head tail) := by
  refine total_bind h fun c => ?_
  cases c <;> first
    | exact total_pure _
    | (refine total_bind t fun r => ?_; cases r <;> exact total_pure _)

theorem total_statementsCons {head : RowSpec Control} {tail : RowSpec Statements}
    (h : Total head) (t : Total tail) : Total (statementsCons head tail) := by
  refine total_bind h fun c => ?_
  cases c <;> first | exact t | exact total_pure _

theorem total_blockUnit {statements : RowSpec Statements} (h : Total statements) :
    Total (blockUnit statements) := by
  refine total_bind h fun r => ?_
  cases r <;> exact total_pure _

theorem total_blockResult {statements : RowSpec Statements} {result : RowSpec Control}
    (h : Total statements) (r : Total result) : Total (blockResult statements result) := by
  refine total_bind h fun x => ?_
  cases x <;> first | exact r | exact total_pure _

theorem total_evaluate (evaluator : NativeEvaluator) (values : Array RuntimeValue) :
    Total (evaluate evaluator values) := by
  refine total_bind total_get fun s => ?_
  split <;> first
    | exact total_stuck
    | exact total_bind (total_set _) fun _ => total_pure _

theorem total_operation {evaluator : NativeEvaluator} {operands : RowSpec Values}
    (h : Total operands) : Total (operation evaluator operands) := by
  refine total_bind h fun r => ?_
  cases r <;> first | exact total_pure _ | exact total_evaluate _ _

theorem total_invoke (lexical : Option Nat) (callee : FunctionDenotation)
    (arguments : Array RuntimeValue) : Total (invoke lexical callee arguments) :=
  ⟨fun _ _ => id, fun _ => id⟩

theorem total_callAt {lexical : Option Nat}
    {callee : Array (TypeId × TypeId) → FunctionDenotation}
    {operands : RowSpec Values} (h : Total operands) :
    Total (callAt lexical callee operands) := by
  refine total_bind h fun result => ?_
  cases result with
  | error control => exact total_pure _
  | ok values => exact total_bind total_get fun _ => total_invoke _ _ _

theorem total_call {lexical : Option Nat} {callee : FunctionDenotation}
    {operands : RowSpec Values} (h : Total operands) :
    Total (call lexical callee operands) := total_callAt h

theorem total_throw {kind : ThrowKind} {arguments : RowSpec Values} (h : Total arguments) :
    Total (throw_ kind arguments) := by
  refine total_bind h fun r => ?_
  cases r <;> exact total_pure _

theorem total_branch {condition thenBranch : RowSpec Control}
    {elseBranch : Option (RowSpec Control)}
    (c : Total condition) (t : Total thenBranch)
    (e : ∀ b, elseBranch = some b → Total b) : Total (branch condition thenBranch elseBranch) := by
  refine total_bind c fun x => ?_
  cases x with
  | value v =>
      cases v with
      | bool decided =>
          cases decided
          · cases elseBranch with
            | some b => exact e b rfl
            | none => exact total_pure _
          · exact t
      | _ => exact total_stuck
  | _ => exact total_pure _

theorem total_branchNone {condition thenBranch : RowSpec Control}
    (c : Total condition) (t : Total thenBranch) : Total (branch condition thenBranch none) :=
  total_branch c t fun _ h => nomatch h

theorem total_branchSome {condition thenBranch elseBranch : RowSpec Control}
    (c : Total condition) (t : Total thenBranch) (e : Total elseBranch) :
    Total (branch condition thenBranch (some elseBranch)) :=
  total_branch c t fun _ h => Option.some.inj h ▸ e

theorem total_letValue {binder : NativePatternBinder} {initializer body : RowSpec Control}
    (i : Total initializer) (b : Total body) : Total (letValue binder initializer body) := by
  refine total_bind i fun x => ?_
  cases x with
  | value v =>
      refine total_bind total_get fun s => ?_
      split
      · exact total_bind (total_set _) fun _ => b
      · exact total_stuck
  | _ => exact total_pure _

theorem total_assignLocalIndex {base index : LocalId} {value : RowSpec Control}
    (v : Total value) : Total (assignLocalIndex base index value) := by
  refine total_bind v fun control => ?_
  cases control with
  | value runtimeValue =>
      refine total_bind total_get fun s => ?_
      cases resolve_eq : resolveLocalIndex? base index s.frame with
      | none => exact total_stuck
      | some resolved =>
          simp only
          cases write_eq : writeRuntimePlace? s.frame s.state resolved runtimeValue with
          | none => simp only; exact total_stuck
          | some result =>
              simp only
              exact total_bind (total_set _) fun _ => total_pure _
  | _ => exact total_pure _

end Agreement

/-! ## The function boundary -/

/-- The row state a call starts in. -/
def entryStateAt (shape : FunctionShape) (arguments : Array RuntimeValue)
    (typeInstantiation : Array (TypeId × TypeId))
    (initial : RuntimeState) : RowState :=
  ⟨initialLocals shape.localCount arguments,
    { activeLoans := #[], loanLocations := parameterLoanLocations arguments,
      typeInstantiation }, initial⟩

def entryState (shape : FunctionShape) (arguments : Array RuntimeValue)
    (initial : RuntimeState) : RowState :=
  entryStateAt shape arguments #[] initial

/-- The public `Spec` of a body computation over a lowered shape and an
explicit invocation type substitution. -/
def rowFunctionAt (unit : ExecutableUnit) (shape : FunctionShape)
    (typeInstantiation : Array (TypeId × TypeId))
    (body : RowSpec Control) (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) where
  ok := fun initial results final =>
    arguments.size = shape.parameterCount ∧ arguments.size ≤ shape.localCount ∧
    ∃ final' control,
      body.ok (entryStateAt shape arguments typeInstantiation initial) control final' ∧
      finishControl? shape.resultCount control = some (.returned results) ∧
      finalizeFunctionState unit shape.profile initial final'.state final'.frame
          (.returned results) = final
  aborts := fun initial failure =>
    arguments.size = shape.parameterCount ∧ arguments.size ≤ shape.localCount ∧
    ∃ final' control final,
      body.ok (entryStateAt shape arguments typeInstantiation initial) control final' ∧
      finishControl? shape.resultCount control = some (.threw failure.1 failure.2) ∧
      finalizeFunctionState unit shape.profile initial final'.state final'.frame
          (.threw failure.1 failure.2) = final

/-- The ordinary row computation has the identity type substitution. -/
def rowFunction (unit : ExecutableUnit) (shape : FunctionShape)
    (body : RowSpec Control) (arguments : Array RuntimeValue) :
    Spec RuntimeState Failure (Array RuntimeValue) :=
  rowFunctionAt unit shape #[] body arguments

theorem entryStateAt_frame (shape : FunctionShape) (arguments : Array RuntimeValue)
    (typeInstantiation : Array (TypeId × TypeId)) (initial : RuntimeState)
    (arity : arguments.size = shape.parameterCount)
    (declared : arguments.size ≤ shape.localCount) :
    nativeInitialFrame? shape arguments typeInstantiation =
      some (entryStateAt shape arguments typeInstantiation initial).frame :=
  nativeInitialFrame?_rowFrameAt shape arguments typeInstantiation arity declared

theorem entryState_frame (shape : FunctionShape) (arguments : Array RuntimeValue)
    (initial : RuntimeState)
    (arity : arguments.size = shape.parameterCount)
    (declared : arguments.size ≤ shape.localCount) :
    nativeInitialFrame? shape arguments = some (entryState shape arguments initial).frame :=
  nativeInitialFrame?_rowFrame shape arguments arity declared

theorem nativeInitialFrameAt?_none (shape : FunctionShape) (arguments : Array RuntimeValue)
    (typeInstantiation : Array (TypeId × TypeId))
    (bad : ¬ (arguments.size = shape.parameterCount ∧ arguments.size ≤ shape.localCount)) :
    nativeInitialFrame? shape arguments typeInstantiation = none := by
  simp only [nativeInitialFrame?]
  by_cases arity : arguments.size = shape.parameterCount
  · have lt : shape.localCount < shape.parameterCount :=
      arity ▸ Nat.lt_of_not_le (fun declared => bad ⟨arity, declared⟩)
    simp [arity, lt]
  · simp [arity]

theorem nativeInitialFrame?_none (shape : FunctionShape) (arguments : Array RuntimeValue)
    (bad : ¬ (arguments.size = shape.parameterCount ∧ arguments.size ≤ shape.localCount)) :
    nativeInitialFrame? shape arguments = none :=
  nativeInitialFrameAt?_none shape arguments #[] bad

/-- The computation's boundary is the relation's. -/
theorem rowFunctionAt_equiv {unit : ExecutableUnit} {shape : FunctionShape}
    {typeInstantiation : Array (TypeId × TypeId)}
    {body : RowSpec Control} {relation : ExprDenotation}
    (agrees : ExprAgrees body relation) (arguments : Array RuntimeValue) :
    Spec.Equiv (rowFunctionAt unit shape typeInstantiation body arguments)
      (nativeFunctionAt unit shape typeInstantiation relation arguments) := by
  refine ⟨?_, ?_, fun _ => Iff.rfl⟩
  · intro initial results final
    simp only [rowFunctionAt, nativeFunctionAt]
    constructor
    · rintro ⟨arity, declared, final', control, step, finish, rfl⟩
      exact ⟨_, final'.frame, final'.state, control,
        entryStateAt_frame shape arguments typeInstantiation initial arity declared,
        (agrees _ _ _ _).mp (by simpa using step), finish, rfl⟩
    · rintro ⟨frame, finalFrame, evaluatedState, control, entry, step, finish, rfl⟩
      by_cases wellFormed : arguments.size = shape.parameterCount ∧
          arguments.size ≤ shape.localCount
      · obtain ⟨arity, declared⟩ := wellFormed
        rw [entryStateAt_frame shape arguments typeInstantiation initial arity declared] at entry
        cases Option.some.inj entry
        exact ⟨arity, declared, RowState.ofFrame finalFrame evaluatedState, control,
          (agrees _ _ _ _).mpr step, finish, rfl⟩
      · rw [nativeInitialFrameAt?_none shape arguments typeInstantiation wellFormed] at entry
        cases entry
  · intro initial failure
    simp only [rowFunctionAt, nativeFunctionAt]
    constructor
    · rintro ⟨arity, declared, final', control, final, step, finish, rfl⟩
      exact ⟨_, final'.frame, final'.state, control, _,
        entryStateAt_frame shape arguments typeInstantiation initial arity declared,
        (agrees _ _ _ _).mp (by simpa using step), finish, rfl⟩
    · rintro ⟨frame, finalFrame, evaluatedState, control, final, entry, step, finish, rfl⟩
      by_cases wellFormed : arguments.size = shape.parameterCount ∧
          arguments.size ≤ shape.localCount
      · obtain ⟨arity, declared⟩ := wellFormed
        rw [entryStateAt_frame shape arguments typeInstantiation initial arity declared] at entry
        cases Option.some.inj entry
        exact ⟨arity, declared, RowState.ofFrame finalFrame evaluatedState, control, _,
          (agrees _ _ _ _).mpr step, finish, rfl⟩
      · rw [nativeInitialFrameAt?_none shape arguments typeInstantiation wellFormed] at entry
        cases entry

theorem rowFunction_equiv {unit : ExecutableUnit} {shape : FunctionShape}
    {body : RowSpec Control} {relation : ExprDenotation}
    (agrees : ExprAgrees body relation) (arguments : Array RuntimeValue) :
    Spec.Equiv (rowFunction unit shape body arguments)
      (nativeFunction unit shape relation arguments) := by
  simpa only [rowFunction, nativeFunction] using
    (rowFunctionAt_equiv (typeInstantiation := #[]) agrees arguments)

/-- Weakest preconditions agree across an equivalence of computations. -/
theorem wp_congr_equiv {a b : Spec σ ε α} (equiv : Spec.Equiv a b)
    (ensures : α → σ → Prop) (aborts : ε → Prop) (initial : σ) :
    wp a ensures aborts initial ↔ wp b ensures aborts initial := by
  simp only [wp]
  rw [(equiv.undefined initial)]
  constructor
  · rintro ⟨normal, failing, defined⟩
    exact ⟨fun r f step => normal r f ((equiv.ok _ _ _).mpr step),
      fun e step => failing e ((equiv.aborts _ _).mpr step), defined⟩
  · rintro ⟨normal, failing, defined⟩
    exact ⟨fun r f step => normal r f ((equiv.ok _ _ _).mp step),
      fun e step => failing e ((equiv.aborts _ _).mp step), defined⟩

/-- The boundary's weakest precondition: the body's, from the entry row
state, with the outcome finished and finalized.  A total body pays no
failure obligation. -/
theorem wp_rowFunctionAt {unit : ExecutableUnit} {shape : FunctionShape}
    {typeInstantiation : Array (TypeId × TypeId)}
    {body : RowSpec Control} (total : Total body) {arguments : Array RuntimeValue}
    {initial : RuntimeState} {ensures : Array RuntimeValue → RuntimeState → Prop}
    {aborts : Failure → Prop} :
    wp (rowFunctionAt unit shape typeInstantiation body arguments) ensures aborts initial ↔
      (arguments.size = shape.parameterCount → arguments.size ≤ shape.localCount →
        wp body
          (fun control final' =>
            ∀ outcome, finishControl? shape.resultCount control = some outcome →
              match outcome with
              | .returned results =>
                  ensures results
                    (finalizeFunctionState unit shape.profile initial final'.state
                      final'.frame outcome)
              | .threw kind thrown => aborts (kind, thrown))
          (fun _ => False)
          (entryStateAt shape arguments typeInstantiation initial)) := by
  constructor
  · rintro ⟨normal, failing, -⟩ arity declared
    refine ⟨?_, fun failure fail => total.aborts _ failure fail, total.undefined _⟩
    intro control final' step outcome finish
    cases outcome with
    | returned results =>
        exact normal results _ ⟨arity, declared, final', control, step, finish, rfl⟩
    | threw kind thrown =>
        exact failing (kind, thrown) ⟨arity, declared, final', control, _, step, finish, rfl⟩
  · intro established
    refine ⟨?_, ?_, fun owed => owed⟩
    · rintro results final ⟨arity, declared, final', control, step, finish, rfl⟩
      exact (established arity declared).1 control final' step (.returned results) finish
    · rintro failure ⟨arity, declared, final', control, final, step, finish, -⟩
      exact (established arity declared).1 control final' step (.threw failure.1 failure.2) finish

theorem wp_rowFunction {unit : ExecutableUnit} {shape : FunctionShape}
    {body : RowSpec Control} (total : Total body) {arguments : Array RuntimeValue}
    {initial : RuntimeState} {ensures : Array RuntimeValue → RuntimeState → Prop}
    {aborts : Failure → Prop} :
    wp (rowFunction unit shape body arguments) ensures aborts initial ↔
      (arguments.size = shape.parameterCount → arguments.size ≤ shape.localCount →
        wp body
          (fun control final' =>
            ∀ outcome, finishControl? shape.resultCount control = some outcome →
              match outcome with
              | .returned results =>
                  ensures results
                    (finalizeFunctionState unit shape.profile initial final'.state
                      final'.frame outcome)
              | .threw kind thrown => aborts (kind, thrown))
          (fun _ => False)
          (entryState shape arguments initial)) := by
  simpa only [rowFunction, entryState] using
    (wp_rowFunctionAt (typeInstantiation := #[]) total)

end RowSpec

end LeanerIR.Proofs.Denotation
