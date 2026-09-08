-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.DenotationWP

/-!
# Frame-free weakest preconditions

A verification goal should not carry a `RuntimeFrame`.  The frame is three
arrays — the locals, and two loan registries the *interpreter* needs so
execution knows where to write back — and carrying it through every step is
what makes a goal grow with the machinery rather than with the mathematics.
This module states the weakest precondition over the locals alone, as a row
of values with no `Option` wrapper and no registries, which is the shape v0
verified against.

The row is not a second semantics.  `wpRow` is the existing `wpExpr` at a
frame built from the row, so a theorem proved through these rules is a
theorem about the same relation, and the connection is definitional rather
than an agreement obligation.  What the rules add is that the row *stays* a
row: each combinator in this subset is proved to leave the registries empty
and the locals total, so its rule can hand the next step a row instead of
a frame.

Scope: the registry-free subset — values, locals, operand and statement
rows, checked primitive operations, assignment, and return.  A construct
that mints a loan or touches global storage is not here; those are the
prophecy and family-store milestones in
[`certifying-execution.md`](../../designs/certifying-execution.md), and
until they land such a construct keeps the frame-based rules.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR.Validation
open LeanerIR.SemanticOperations
open LeanerIR.BigStep

/-- A body's locals.  A slot is `none` before its first assignment, which
is why the row is the locals array itself rather than a row of values: a
body declares more locals than it takes parameters, and the entry rule has
to describe all of them. -/
abbrev Row := Array (Option RuntimeValue)

/-- The loan bookkeeping a frame carries for the interpreter.

It is a parameter of this layer, never a literal it reduces: a step that
neither mints nor ends a loan threads it unchanged, so it stays one opaque
variable in the goal instead of two arrays rewritten at every program
point.  A step that does touch it is outside this layer, and is the
prophecy work rather than bookkeeping to carry. -/
structure Registries where
  activeLoans : Array (ExprId × Nat)
  loanLocations : Array (Nat × RuntimePlace)

/-- The frame a row denotes, over registries this layer does not read. -/
def rowFrame (row : Row) (registries : Registries) : RuntimeFrame :=
  { locals := row
    activeLoans := registries.activeLoans
    loanLocations := registries.loanLocations }

@[simp] theorem rowFrame_locals (row : Row) (registries : Registries) :
    (rowFrame row registries).locals = row := rfl

@[simp] theorem rowFrame_activeLoans (row : Row) (registries : Registries) :
    (rowFrame row registries).activeLoans = registries.activeLoans := rfl

@[simp] theorem rowFrame_loanLocations (row : Row) (registries : Registries) :
    (rowFrame row registries).loanLocations = registries.loanLocations := rfl

theorem rowFrame_injective {row row' : Row} {registries : Registries}
    (equal : rowFrame row registries = rowFrame row' registries) :
    row = row' :=
  congrArg RuntimeFrame.locals equal

/-- Reading a local of a row is reading the row. -/
@[simp] theorem readLocal?_rowFrame (row : Row) (registries : Registries) (localId : LocalId) :
    readLocal? (rowFrame row registries) localId = row[localId.index]?.join := by
  rfl

/-- A relation stays inside the subset: from a row it reaches a row, and
the state it reaches carries no new write-backs.  Every rule below needs
this of the combinator it steps over, because that is what lets it hand the
next step a row. -/
def RowStable (denotation : ExprDenotation) : Prop :=
  ∀ row registries state finalFrame finalState control,
    denotation (rowFrame row registries) state finalFrame finalState control →
      ∃ finalRow, finalFrame = rowFrame finalRow registries

/-- Weakest precondition over the locals alone.

This is `wpExpr` at the row's frame, with the postcondition phrased over
the row the step reaches.  `stable` is what makes the two agree: without
it a final frame outside the subset would be dropped, and the resulting
predicate would be weaker than the one it replaces. -/
def wpRow (denotation : ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop) : Prop :=
  ∀ finalRow finalState control,
    denotation (rowFrame row registries) state (rowFrame finalRow registries)
        finalState control →
      post finalRow finalState control

theorem wpExpr_of_wpRow {denotation : ExprDenotation} {row : Row}
    {registries : Registries}
    {state : RuntimeState} {post : RuntimeFrame → RuntimeState → Control → Prop}
    (stable : RowStable denotation)
    (rowWp : wpRow denotation row registries state
      fun finalRow finalState control =>
        post (rowFrame finalRow registries) finalState control) :
    wpExpr denotation (rowFrame row registries) state post := by
  unfold wpExpr
  intro finalFrame finalState control step
  obtain ⟨finalRow, rfl⟩ :=
    stable row registries state finalFrame finalState control step
  exact rowWp finalRow finalState control step

theorem wpRow_of_wpExpr {denotation : ExprDenotation} {row : Row}
    {registries : Registries}
    {state : RuntimeState} {post : Row → RuntimeState → Control → Prop}
    (frameWp : wpExpr denotation (rowFrame row registries) state
      fun finalFrame finalState control =>
        ∀ finalRow, finalFrame = rowFrame finalRow registries →
          post finalRow finalState control) :
    wpRow denotation row registries state post := by
  intro finalRow finalState control step
  have expanded := frameWp
  unfold wpExpr at expanded
  exact expanded (rowFrame finalRow registries) finalState control step finalRow rfl

/-! ## Stability of the registry-free combinators -/

theorem rowStable_value (runtimeValue : RuntimeValue) :
    RowStable (value runtimeValue) := by
  rintro row registries state finalFrame finalState control ⟨rfl, _, _⟩
  exact ⟨row, rfl⟩

theorem rowStable_localVar (localId : LocalId) :
    RowStable (localVar localId) := by
  rintro row registries state finalFrame finalState control ⟨_, _, rfl, _, _⟩
  exact ⟨row, rfl⟩

/-- Operand rows stay in the subset too, and their result carries the row
the last operand reached. -/
def RowStableValues (denotation : ValuesDenotation) : Prop :=
  ∀ row registries state result,
    denotation (rowFrame row registries) state result →
    ∃ resultRow,
      (∃ resultState values,
        result = .values resultState (rowFrame resultRow registries) values) ∨
      (∃ resultState control,
        result = .control resultState (rowFrame resultRow registries) control)

theorem rowStableValues_nil : RowStableValues valuesNil := by
  rintro row registries state result rfl
  exact ⟨row, .inl ⟨state, [], rfl⟩⟩

theorem rowStableValues_cons {head : ExprDenotation} {tail : ValuesDenotation}
    (headStable : RowStable head) (tailStable : RowStableValues tail) :
    RowStableValues (valuesCons head tail) := by
  rintro row registries state result step
  rcases step with ⟨finalFrame, finalState, control, headStep, _, rfl⟩ |
    ⟨headFrame, headState, runtimeValue, finalFrame, finalState, values,
      headStep, tailStep, rfl⟩ |
    ⟨headFrame, headState, runtimeValue, finalFrame, finalState, control,
      headStep, tailStep, rfl⟩
  · obtain ⟨finalRow, rfl⟩ := headStable row registries state finalFrame finalState control headStep
    exact ⟨finalRow, .inr ⟨finalState, control, rfl⟩⟩
  · obtain ⟨headRow, rfl⟩ :=
      headStable row registries state headFrame headState (.value runtimeValue) headStep
    obtain ⟨resultRow, result⟩ := tailStable headRow registries headState _ tailStep
    rcases result with ⟨resultState, values', equal⟩ | ⟨resultState, control, equal⟩
    · cases equal
      exact ⟨resultRow, .inl ⟨finalState, _, rfl⟩⟩
    · exact absurd equal (by simp)
  · obtain ⟨headRow, rfl⟩ :=
      headStable row registries state headFrame headState (.value runtimeValue) headStep
    obtain ⟨resultRow, result⟩ := tailStable headRow registries headState _ tailStep
    rcases result with ⟨resultState, values', equal⟩ | ⟨resultState, control', equal⟩
    · exact absurd equal (by simp)
    · cases equal
      exact ⟨resultRow, .inr ⟨finalState, control, rfl⟩⟩

/-- A checked primitive operation stays in the subset: it computes on
values, so its evaluator returns the frame it was given.  This is proved of
`liftPrimitiveEvaluator` rather than assumed of an arbitrary evaluator —
the property is exactly what "primitive" means at this boundary. -/
theorem rowStable_nativeLiftedPrimitive
    {evaluate : Array RuntimeValue →
      Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue)}
    {operands : ValuesDenotation} (stable : RowStableValues operands) :
    RowStable (nativeOperation (liftPrimitiveEvaluator evaluate) operands) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨operandFrame, operandState, propagated, operandStep, rfl, _, _⟩ |
    ⟨operandFrame, operandState, values, operandStep, evaluation⟩
  · obtain ⟨resultRow, result⟩ := stable row registries state _ operandStep
    rcases result with ⟨_, _, equal⟩ | ⟨_, _, equal⟩
    · exact absurd equal (by simp)
    · cases equal
      exact ⟨resultRow, rfl⟩
  · obtain ⟨resultRow, result⟩ := stable row registries state _ operandStep
    rcases result with ⟨resultState, values', equal⟩ | ⟨_, _, equal⟩
    · cases equal
      /- Both evaluator outcomes carry the frame they were given. -/
      have frameKept : finalFrame = rowFrame resultRow registries := by
        rcases evaluation with ⟨runtimeValue, evaluated, _⟩ |
          ⟨throwKind, thrown, evaluated, _⟩ <;>
        · unfold liftPrimitiveEvaluator at evaluated
          cases evaluated' : evaluate values.toArray with
          | none => rw [evaluated'] at evaluated; simp at evaluated
          | some outcome =>
              rw [evaluated'] at evaluated
              cases outcome <;> simp_all
      exact ⟨resultRow, frameKept⟩
    · exact absurd equal (by simp)

/-! ## Entry, without a shape inventory

The frame a call starts in is the declared locals with the arguments in
front and the mutable ones registered.  Over a row that is one law for
every arity and every mix of parameters, where the frame formulation needed
a closed constructor equation per shape. -/

theorem nativeInitialFrame?_rowFrame (shape : FunctionShape)
    (arguments : Array RuntimeValue)
    (arity : arguments.size = shape.parameterCount)
    (declared : arguments.size ≤ shape.localCount) :
    nativeInitialFrame? shape arguments
      = some (rowFrame (initialLocals shape.localCount arguments)
          { activeLoans := #[]
            loanLocations := parameterLoanLocations arguments }) := by
  simp only [nativeInitialFrame?, rowFrame, arity, bne_self_eq_false,
    Bool.false_eq_true, if_false]
  rw [if_neg (Nat.not_lt.mpr (arity ▸ declared))]

/-- The whole call boundary, over a row.

Entry, body, and exit compose into one rule: the call starts in the
declared locals with its arguments in front, the body runs frame-free, and
what the caller sees is the finalized state.  A `&mut` argument enters as
the borrow it is and leaves as whatever its slot then holds — the two
halves of the prophecy pair — with no step in between registering
anything on the verifier's behalf. -/
theorem wpFunction_rowFrame {unit : ExecutableUnit} {shape : FunctionShape}
    {body : ExprDenotation} {arguments : Array RuntimeValue}
    {initial : RuntimeState} {post : RuntimeState → Outcome → Prop}
    (arity : arguments.size = shape.parameterCount)
    (declared : arguments.size ≤ shape.localCount)
    (stable : RowStable body)
    (rowWp : wpRow body (initialLocals shape.localCount arguments)
        { activeLoans := #[], loanLocations := parameterLoanLocations arguments }
        initial
        fun finalRow finalState control =>
          ∀ outcome, finishControl? shape.resultCount control = some outcome →
            post (finalizeFunctionState unit shape.profile initial finalState
              (rowFrame finalRow
                { activeLoans := #[]
                  loanLocations := parameterLoanLocations arguments })
              outcome) outcome) :
    wpFunction (nativeFunctionRelation unit shape body) initial arguments post := by
  rw [wpFunction_nativeFunctionRelation]
  intro frame entry
  rw [nativeInitialFrame?_rowFrame shape arguments arity declared] at entry
  cases Option.some.inj entry
  exact wpExpr_of_wpRow stable rowWp

/-- The entry rule at the shape the generated script reaches.

After `wp_nativeFunction` the goal quantifies over the initial frame; this
turns it into a row obligation, with the arity side conditions computing on
the literal argument row and the loan table appearing once, opaquely, as
the registries every later rule threads unchanged. -/
theorem nativeEntry_rowFrame {shape : FunctionShape} {body : ExprDenotation}
    {arguments : Array RuntimeValue} {initial : RuntimeState}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (arity : arguments.size = shape.parameterCount)
    (declared : arguments.size ≤ shape.localCount)
    (stable : RowStable body)
    (rowWp : wpRow body (initialLocals shape.localCount arguments)
        { activeLoans := #[], loanLocations := parameterLoanLocations arguments }
        initial
        fun finalRow finalState control =>
          post (rowFrame finalRow
              { activeLoans := #[]
                loanLocations := parameterLoanLocations arguments })
            finalState control) :
    ∀ frame, nativeInitialFrame? shape arguments = some frame →
      wpExpr body frame initial post := by
  intro frame entry
  rw [nativeInitialFrame?_rowFrame shape arguments arity declared] at entry
  cases Option.some.inj entry
  exact wpExpr_of_wpRow stable rowWp

/-! ## Exit, without a shape inventory

The borrows a frame carries are the borrows its locals carry, and over a
row that is one law rather than one lemma per slot layout.  The frame-based
inventory needed a closed row per shape because it had to recognise the
literal frame; a row states the same fact generically. -/

/-- A row of plain values carries no borrows, at any number of locals.
The frame formulation needed a closed row per slot layout to see this; over
a row it is one induction. -/
theorem frameBorrows_rowFrame_borrowFree (row : Row) (registries : Registries)
    (borrowFree : ∀ slot ∈ row, ∀ value, slot = some value →
      outermostBorrows value = #[]) :
    frameBorrows (rowFrame row registries) = #[] := by
  simp only [frameBorrows, rowFrame]
  exact Array.foldl_induction (motive := fun _ borrows => borrows = #[]) rfl
    (fun index borrows collected => by
      subst collected
      cases slot : row[index] with
      | none => rfl
      | some value =>
          simp only [Array.empty_append]
          exact borrowFree row[index] (Array.getElem_mem index.isLt) value slot)

/-- A row of plain values exports nothing: the exit rule for every function
whose locals hold no reference, at any number of locals. -/
theorem exportFrameLoans_rowFrame_borrowFree (row : Row) (registries : Registries)
    (state : RuntimeState)
    (borrowFree : ∀ slot ∈ row, ∀ value, slot = some value →
      outermostBorrows value = #[]) :
    exportFrameLoans (rowFrame row registries) state = state :=
  exportFrameLoans_borrowFree _ _ (frameBorrows_rowFrame_borrowFree row registries borrowFree)

/-- A row that holds no prophecy hole exports each of its borrows directly.

This is the general shape of finalization: no case analysis on how many
locals there are or where the borrow sits, only the fact that a hole would
have had to be settled first.  Prophecy is why the fold is the whole story
— the write-back has nowhere else to go, so a borrow's current value *is*
the value its parameter is seen to have at the loan's death. -/
theorem exportFrameLoans_rowFrame_holeFree (row : Row) (registries : Registries)
    (state : RuntimeState)
    (holeFree : ∀ loan, holeInFrame (rowFrame row registries) loan = false) :
    exportFrameLoans (rowFrame row registries) state
      = (frameBorrows (rowFrame row registries)).foldl (init := state)
          fun state entry =>
            (applyWriteBack { locals := #[] } state entry.1 entry.2).2 := by
  unfold exportFrameLoans
  rw [Array.find?_eq_none.mpr (fun entry _ => by simp [holeFree entry.1])]
  simp only [exportSettledLoans]
  refine Array.foldl_congr rfl ?_ rfl rfl rfl
  funext accumulated entry
  simp [holeFree entry.1]

/-- What a returning call leaves the caller.

Composed with the boundary rule this is the prophecy pair in full: the
mutable argument entered as `borrow loan current`, and the value the caller
sees at the loan's death is whatever its slot holds now.  Nothing tracked
that; it is read off the row. -/
theorem finalizeFunctionState_returned_rowFrame {unit : ExecutableUnit}
    {profile : Profile} {initial evaluated : RuntimeState}
    (row : Row) (registries : Registries) (results : Array RuntimeValue)
    (holeFree : ∀ loan, holeInFrame (rowFrame row registries) loan = false) :
    finalizeFunctionState unit profile initial evaluated
        (rowFrame row registries) (.returned results)
      = (frameBorrows (rowFrame row registries)).foldl (init := evaluated)
          fun state entry =>
            (applyWriteBack { locals := #[] } state entry.1 entry.2).2 :=
  exportFrameLoans_rowFrame_holeFree row registries evaluated holeFree

/-! ## Operand rows, frame-free

An operand row's result carries the frame the last operand reached; over a
row it carries the row.  `RowValuesResult` is that result with the frame
replaced, and `embed` is how it reads back — the same relation, once
again, rather than a parallel one. -/

/-- An operand row's result, over a row. -/
inductive RowValuesResult where
  | values (state : RuntimeState) (row : Row) (values : List RuntimeValue)
  | control (state : RuntimeState) (row : Row) (control : Control)

/-- The frame-carrying result a row result denotes. -/
def RowValuesResult.embed (registries : Registries) :
    RowValuesResult → ValuesResult
  | .values state row operands =>
      ValuesResult.values state (rowFrame row registries) operands
  | .control state row propagated =>
      ValuesResult.control state (rowFrame row registries) propagated

/-- Weakest precondition of an operand row, over the locals alone. -/
def wpValuesRow (denotation : ValuesDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : RowValuesResult → Prop) : Prop :=
  ∀ result, denotation (rowFrame row registries) state (result.embed registries) →
    post result

theorem wpValuesRow_nil (row : Row) (registries : Registries)
    (state : RuntimeState) (post : RowValuesResult → Prop) :
    wpValuesRow valuesNil row registries state post ↔
      post (.values state row []) := by
  constructor
  · intro h
    exact h (.values state row []) (by simp [valuesNil, RowValuesResult.embed])
  · intro h result step
    simp only [valuesNil] at step
    cases result with
    | values resultState resultRow values =>
        simp only [RowValuesResult.embed] at step
        obtain ⟨rfl, equal, rfl⟩ : resultState = state ∧
            rowFrame resultRow registries = rowFrame row registries ∧ values = [] := by
          simpa using step
        cases rowFrame_injective equal
        exact h
    | control resultState resultRow control =>
        simp only [RowValuesResult.embed] at step
        exact absurd step (by simp)

theorem wpValuesRow_cons (head : ExprDenotation) (tail : ValuesDenotation)
    (row : Row) (registries : Registries) (state : RuntimeState)
    (post : RowValuesResult → Prop)
    (headStable : RowStable head) (tailStable : RowStableValues tail)
    (stepped : wpRow head row registries state
      fun headRow headState control =>
        match control with
        | .value runtimeValue =>
            wpValuesRow tail headRow registries headState fun result =>
              match result with
              | .values finalState finalRow values =>
                  post (.values finalState finalRow (runtimeValue :: values))
              | .control finalState finalRow control =>
                  post (.control finalState finalRow control)
        | _ => post (.control headState headRow control)) :
    wpValuesRow (valuesCons head tail) row registries state post := by
  rintro result step
  rcases step with ⟨finalFrame, finalState, raised, headStep, abrupt, embedded⟩ |
    ⟨headFrame, headState, runtimeValue, finalFrame, finalState, operands,
      headStep, tailStep, embedded⟩ |
    ⟨headFrame, headState, runtimeValue, finalFrame, finalState, control,
      headStep, tailStep, embedded⟩
  · obtain ⟨finalRow, rfl⟩ :=
      headStable row registries state finalFrame finalState raised headStep
    have applied := stepped finalRow finalState raised headStep
    cases result with
    | values _ _ _ => exact absurd embedded (by simp [RowValuesResult.embed])
    | control resultState resultRow propagated =>
        simp only [RowValuesResult.embed] at embedded
        obtain ⟨rfl, equal, rfl⟩ : resultState = finalState ∧
            rowFrame resultRow registries = rowFrame finalRow registries ∧
            propagated = raised := by simpa using embedded
        cases rowFrame_injective equal
        cases propagated <;> simp_all
  · obtain ⟨headRow, rfl⟩ :=
      headStable row registries state headFrame headState (.value runtimeValue) headStep
    obtain ⟨finalRow, shape⟩ := tailStable headRow registries headState
      (ValuesResult.values finalState finalFrame operands) tailStep
    rcases shape with ⟨resultState, operands', equal⟩ | ⟨_, _, equal⟩
    · cases equal
      have applied := stepped headRow headState (.value runtimeValue) headStep
      simp only [] at applied
      have tailApplied :=
        applied (RowValuesResult.values finalState finalRow operands) tailStep
      cases result with
      | values _ _ _ =>
          simp only [RowValuesResult.embed] at embedded
          obtain ⟨rfl, equal, rfl⟩ := by simpa using embedded
          cases rowFrame_injective equal
          exact tailApplied
      | control _ _ _ => exact absurd embedded (by simp [RowValuesResult.embed])
    · exact absurd equal (by simp)
  · obtain ⟨headRow, rfl⟩ :=
      headStable row registries state headFrame headState (.value runtimeValue) headStep
    obtain ⟨finalRow, shape⟩ := tailStable headRow registries headState
      (ValuesResult.control finalState finalFrame control) tailStep
    rcases shape with ⟨_, _, equal⟩ | ⟨resultState, control', equal⟩
    · exact absurd equal (by simp)
    · cases equal
      have applied := stepped headRow headState (.value runtimeValue) headStep
      simp only [] at applied
      have tailApplied :=
        applied (RowValuesResult.control finalState finalRow control) tailStep
      cases result with
      | values _ _ _ => exact absurd embedded (by simp [RowValuesResult.embed])
      | control _ _ _ =>
          simp only [RowValuesResult.embed] at embedded
          obtain ⟨rfl, equal, rfl⟩ := by simpa using embedded
          cases rowFrame_injective equal
          exact tailApplied

/-! ## Statement rows and blocks, frame-free -/

/-- A statement row's result, over a row. -/
inductive RowStatementsResult where
  | done (state : RuntimeState) (row : Row)
  | control (state : RuntimeState) (row : Row) (control : Control)

/-- The frame-carrying result a row result denotes. -/
def RowStatementsResult.embed (registries : Registries) :
    RowStatementsResult → StatementsResult
  | .done state row => StatementsResult.done state (rowFrame row registries)
  | .control state row propagated =>
      StatementsResult.control state (rowFrame row registries) propagated

def RowStableStatements (denotation : StatementsDenotation) : Prop :=
  ∀ row registries state result,
    denotation (rowFrame row registries) state result →
    ∃ resultRow,
      (∃ resultState, result = .done resultState (rowFrame resultRow registries)) ∨
      (∃ resultState propagated,
        result = .control resultState (rowFrame resultRow registries) propagated)

theorem rowStableStatements_nil : RowStableStatements statementsNil := by
  rintro row registries state result rfl
  exact ⟨row, .inl ⟨state, rfl⟩⟩

theorem rowStableStatements_cons {head : ExprDenotation}
    {tail : StatementsDenotation} (headStable : RowStable head)
    (tailStable : RowStableStatements tail) :
    RowStableStatements (statementsCons head tail) := by
  rintro row registries state result step
  rcases step with ⟨finalFrame, finalState, raised, headStep, _, rfl⟩ |
    ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩
  · obtain ⟨finalRow, rfl⟩ :=
      headStable row registries state finalFrame finalState raised headStep
    exact ⟨finalRow, .inr ⟨finalState, raised, rfl⟩⟩
  · obtain ⟨headRow, rfl⟩ :=
      headStable row registries state headFrame headState (.value runtimeValue) headStep
    exact tailStable headRow registries headState result tailStep

/-- Weakest precondition of a statement row, over the locals alone. -/
def wpStatementsRow (denotation : StatementsDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : RowStatementsResult → Prop) : Prop :=
  ∀ result, denotation (rowFrame row registries) state (result.embed registries) →
    post result

theorem wpStatementsRow_nil (row : Row) (registries : Registries)
    (state : RuntimeState) (post : RowStatementsResult → Prop) :
    wpStatementsRow statementsNil row registries state post ↔
      post (.done state row) := by
  constructor
  · intro h
    exact h (.done state row) (by simp [statementsNil, RowStatementsResult.embed])
  · intro h result step
    simp only [statementsNil] at step
    cases result with
    | done resultState resultRow =>
        simp only [RowStatementsResult.embed] at step
        obtain ⟨rfl, equal⟩ : resultState = state ∧
            rowFrame resultRow registries = rowFrame row registries := by simpa using step
        cases rowFrame_injective equal
        exact h
    | control _ _ _ =>
        simp only [RowStatementsResult.embed] at step
        exact absurd step (by simp)

theorem wpStatementsRow_cons (head : ExprDenotation) (tail : StatementsDenotation)
    (row : Row) (registries : Registries) (state : RuntimeState)
    (post : RowStatementsResult → Prop) (headStable : RowStable head)
    (stepped : wpRow head row registries state
      fun headRow headState control =>
        match control with
        | .value _ => wpStatementsRow tail headRow registries headState post
        | _ => post (.control headState headRow control)) :
    wpStatementsRow (statementsCons head tail) row registries state post := by
  rintro result step
  rcases step with ⟨finalFrame, finalState, raised, headStep, abrupt, embedded⟩ |
    ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩
  · obtain ⟨finalRow, rfl⟩ :=
      headStable row registries state finalFrame finalState raised headStep
    have applied := stepped finalRow finalState raised headStep
    cases result with
    | done _ _ => exact absurd embedded (by simp [RowStatementsResult.embed])
    | control resultState resultRow propagated =>
        simp only [RowStatementsResult.embed] at embedded
        obtain ⟨rfl, equal, rfl⟩ : resultState = finalState ∧
            rowFrame resultRow registries = rowFrame finalRow registries ∧
            propagated = raised := by simpa using embedded
        cases rowFrame_injective equal
        cases propagated <;> simp_all
  · obtain ⟨headRow, rfl⟩ :=
      headStable row registries state headFrame headState (.value runtimeValue) headStep
    have applied := stepped headRow headState (.value runtimeValue) headStep
    simp only [] at applied
    exact applied result tailStep

/-- A block whose statements run for effect. -/
theorem rowStable_blockUnit {statements : StatementsDenotation}
    (stable : RowStableStatements statements) : RowStable (blockUnit statements) := by
  rintro row registries state finalFrame finalState control step
  rcases step with abrupt | ⟨done, _⟩
  · obtain ⟨resultRow, shape⟩ := stable row registries state _ abrupt
    rcases shape with ⟨_, equal⟩ | ⟨_, _, equal⟩
    · exact absurd equal (by simp)
    · exact ⟨resultRow, by simpa using (StatementsResult.control.inj equal).2.1⟩
  · obtain ⟨resultRow, shape⟩ := stable row registries state _ done
    rcases shape with ⟨_, equal⟩ | ⟨_, _, equal⟩
    · exact ⟨resultRow, by simpa using (StatementsResult.done.inj equal).2⟩
    · exact absurd equal (by simp)

theorem wpRow_blockUnit (statements : StatementsDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop)
    (stepped : wpStatementsRow statements row registries state fun result =>
      match result with
      | .done finalState finalRow => post finalRow finalState (.value .unit)
      | .control finalState finalRow control => post finalRow finalState control) :
    wpRow (blockUnit statements) row registries state post := by
  rintro finalRow finalState control step
  rcases step with abrupt | ⟨done, rfl⟩
  · exact stepped (.control finalState finalRow control) abrupt
  · exact stepped (.done finalState finalRow) done

/-! ## Operations, frame-free

Every lowered operation — primitive, local, deref-local borrow, and
reference — is `nativeOperation` at a different evaluator, so one rule
serves all four.  What varies is whether the evaluator moves the frame: a
primitive computes on values and returns the frame it was given, while a
write through a reference updates the referent's slot.  That difference is
the evaluator's own row law, stated once here and discharged per
evaluator. -/

/-- The frame an evaluator result carries. -/
def resultFrame : GlobalOperationResult → RuntimeFrame
  | .value frame _ _ => frame
  | .throw_ frame _ _ _ => frame

/-- An evaluator keeps the locals a row: it may write a slot, but it does
not introduce a shape the layer cannot name, and it leaves the loan
bookkeeping to the interpreter. -/
def EvaluatorRowStable (evaluate : NativeEvaluator) : Prop :=
  ∀ operands row registries state result,
    evaluate operands (rowFrame row registries) state = some result →
      ∃ resultRow, resultFrame result = rowFrame resultRow registries

/-- A resolved constructor returns the frame it was given. -/
theorem evaluatorRowStable_nominalConstructor (constructor : NominalConstructor) :
    EvaluatorRowStable constructor.evaluate? := by
  intro operands row registries state result evaluated
  simp only [NominalConstructor.evaluate?] at evaluated
  split at evaluated
  · cases evaluated
  · cases Option.some.inj evaluated
    exact ⟨row, rfl⟩

theorem rowStable_nativeOperation {evaluate : NativeEvaluator}
    {operands : ValuesDenotation} (operandsStable : RowStableValues operands)
    (evaluatorStable : EvaluatorRowStable evaluate) :
    RowStable (nativeOperation evaluate operands) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨operandFrame, operandState, propagated, operandStep, rfl, _, _⟩ |
    ⟨operandFrame, operandState, values, operandStep, evaluation⟩
  · obtain ⟨resultRow, shape⟩ := operandsStable row registries state _ operandStep
    rcases shape with ⟨_, _, equal⟩ | ⟨_, _, equal⟩
    · exact absurd equal (by simp)
    · cases equal
      exact ⟨resultRow, rfl⟩
  · obtain ⟨resultRow, shape⟩ := operandsStable row registries state _ operandStep
    rcases shape with ⟨resultState, values', equal⟩ | ⟨_, _, equal⟩
    · cases equal
      rcases evaluation with ⟨runtimeValue, evaluated, _⟩ |
        ⟨throwKind, thrown, evaluated, _⟩
      · obtain ⟨finalRow, shaped⟩ :=
          evaluatorStable _ resultRow registries _ _ evaluated
        exact ⟨finalRow, shaped⟩
      · obtain ⟨finalRow, shaped⟩ :=
          evaluatorStable _ resultRow registries _ _ evaluated
        exact ⟨finalRow, shaped⟩
    · exact absurd equal (by simp)

/-- The weakest precondition of any lowered operation, over a row: evaluate
the operands, then read the evaluator's two outcomes off the row it
reaches. -/
theorem wpRow_nativeOperation (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState) (post : Row → RuntimeState → Control → Prop)
    (operandsStable : RowStableValues operands)
    (stepped : wpValuesRow operands row registries state fun result =>
      match result with
      | .control operandState operandRow propagated =>
          post operandRow operandState propagated
      | .values operandState operandRow values =>
          (∀ finalRow finalState runtimeValue,
            evaluate values.toArray (rowFrame operandRow registries) operandState =
                some (.value (rowFrame finalRow registries) finalState runtimeValue) →
              post finalRow finalState (.value runtimeValue)) ∧
          (∀ finalRow finalState throwKind thrown,
            evaluate values.toArray (rowFrame operandRow registries) operandState =
                some (.throw_ (rowFrame finalRow registries) finalState throwKind
                  thrown) →
              post finalRow finalState (.throw_ throwKind thrown))) :
    wpRow (nativeOperation evaluate operands) row registries state post := by
  rintro finalRow finalState control step
  rcases step with ⟨operandFrame, operandState, propagated, operandStep, equal, rfl, rfl⟩ |
    ⟨operandFrame, operandState, values, operandStep, evaluation⟩
  · obtain ⟨operandRow, shape⟩ := operandsStable row registries state _ operandStep
    rcases shape with ⟨_, _, absurdEqual⟩ | ⟨resultState, propagated', shaped⟩
    · exact absurd absurdEqual (by simp)
    · obtain ⟨rfl, frameEqual, rfl⟩ :
          finalState = resultState ∧ operandFrame = rowFrame operandRow registries ∧
            control = propagated' := by
        simpa [eq_comm] using shaped
      cases rowFrame_injective (equal.trans frameEqual)
      exact stepped (.control finalState finalRow control)
        (by simpa [RowValuesResult.embed, frameEqual] using operandStep)
  · obtain ⟨operandRow, shape⟩ := operandsStable row registries state _ operandStep
    rcases shape with ⟨resultState, values', shaped⟩ | ⟨_, _, absurdEqual⟩
    · obtain ⟨rfl, frameEqual, rfl⟩ :
          operandState = resultState ∧ operandFrame = rowFrame operandRow registries ∧
            values = values' := by
        simpa [eq_comm] using shaped
      have rowStep : operands (rowFrame row registries) state
          ((RowValuesResult.values operandState operandRow values).embed registries) := by
        simpa [RowValuesResult.embed, frameEqual] using operandStep
      rcases evaluation with ⟨runtimeValue, evaluated, rfl⟩ |
        ⟨throwKind, thrown, evaluated, rfl⟩
      · exact (stepped _ rowStep).1 finalRow finalState runtimeValue
          (by simpa [frameEqual] using evaluated)
      · exact (stepped _ rowStep).2 finalRow finalState throwKind thrown
          (by simpa [frameEqual] using evaluated)
    · exact absurd absurdEqual (by simp)

/-! ### The writers keep the bookkeeping

Every frame a reference evaluator produces is either the frame it was given
or that frame with new locals.  Stated as preservation of the two registry
fields, each step is one case split. -/

theorem writeRoot?_registries {frame : RuntimeFrame} {state : RuntimeState}
    {root : RuntimePlaceRoot} {value : RuntimeValue}
    {written : RuntimeFrame} {writtenState : RuntimeState}
    (write : writeRoot? frame state root value = some (written, writtenState)) :
    written.activeLoans = frame.activeLoans ∧
      written.loanLocations = frame.loanLocations := by
  unfold writeRoot? at write
  split at write
  · split at write
    · simp at write
    · obtain ⟨rfl, _⟩ := Prod.mk.inj (Option.some.inj write)
      exact ⟨rfl, rfl⟩
  · cases lookup : state.globals.lookup _ with
    | none => rw [lookup] at write; simp at write
    | some _ =>
        rw [lookup] at write
        obtain ⟨rfl, _⟩ := Prod.mk.inj (Option.some.inj write)
        exact ⟨rfl, rfl⟩

theorem writeRuntimePlace?_registries {frame : RuntimeFrame} {state : RuntimeState}
    {place : RuntimePlace} {value : RuntimeValue}
    {written : RuntimeFrame} {writtenState : RuntimeState}
    (write : writeRuntimePlace? frame state place value
      = some (written, writtenState)) :
    written.activeLoans = frame.activeLoans ∧
      written.loanLocations = frame.loanLocations := by
  unfold writeRuntimePlace? at write
  split at write
  · simp at write
  · split at write
    · exact writeRoot?_registries write
    · have spelled : (readRoot? frame state place.root).bind
          (fun root => (writeProjections? root place.projections.toList value).bind
            (fun updated => writeRoot? frame state place.root updated))
          = some (written, writtenState) := write
      rw [Option.bind_eq_some_iff] at spelled
      obtain ⟨_, _, rest⟩ := spelled
      rw [Option.bind_eq_some_iff] at rest
      obtain ⟨_, _, final⟩ := rest
      exact writeRoot?_registries final

theorem fillVisibleHole_registries (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (replacement : RuntimeValue) :
    (fillVisibleHole frame state loan replacement).1.activeLoans
        = frame.activeLoans ∧
      (fillVisibleHole frame state loan replacement).1.loanLocations
        = frame.loanLocations := by
  unfold fillVisibleHole
  repeat' split
  all_goals first
    | exact ⟨rfl, rfl⟩
    | simp

theorem applyWriteBack_registries (frame : RuntimeFrame) (state : RuntimeState)
    (loan : Nat) (current : RuntimeValue) :
    (applyWriteBack frame state loan current).1.activeLoans = frame.activeLoans ∧
      (applyWriteBack frame state loan current).1.loanLocations
        = frame.loanLocations := by
  have preserved := fillVisibleHole_registries frame state loan current
  unfold applyWriteBack
  rcases filled : fillVisibleHole frame state loan current with ⟨written, writtenState, found⟩
  rw [filled] at preserved
  cases found <;> exact preserved

theorem updateLocalBorrowValue?_registries {frame : RuntimeFrame}
    {state : RuntimeState} {loan : Nat} {replacement : RuntimeValue}
    {updated : RuntimeFrame} {updatedState : RuntimeState}
    (update : updateLocalBorrowValue? frame state loan replacement
      = some (updated, updatedState)) :
    updated.activeLoans = frame.activeLoans ∧
      updated.loanLocations = frame.loanLocations := by
  unfold updateLocalBorrowValue? at update
  have spelled : (localLoanPlace? frame loan).bind (fun place =>
      (readRuntimePlace? frame state place).bind (fun value =>
        (rewriteFirst (fun candidate =>
            match candidate with
            | .borrow instance_ _ =>
                if instance_ == loan then some (.borrow instance_ replacement)
                else none
            | _ => none) value).bind (fun updatedValue =>
          writeRuntimePlace? frame state place updatedValue)))
      = some (updated, updatedState) := update
  rw [Option.bind_eq_some_iff] at spelled
  obtain ⟨_, _, rest⟩ := spelled
  rw [Option.bind_eq_some_iff] at rest
  obtain ⟨_, _, deeper⟩ := rest
  rw [Option.bind_eq_some_iff] at deeper
  obtain ⟨_, _, final⟩ := deeper
  exact writeRuntimePlace?_registries final

theorem updateBorrowValue?_registries {frame : RuntimeFrame} {state : RuntimeState}
    {loan : Nat} {replacement : RuntimeValue}
    {updated : RuntimeFrame} {updatedState : RuntimeState}
    (update : updateBorrowValue? frame state loan replacement
      = some (updated, updatedState)) :
    updated.activeLoans = frame.activeLoans ∧
      updated.loanLocations = frame.loanLocations := by
  unfold updateBorrowValue? at update
  dsimp only at update
  cases localUpdate : updateLocalBorrowValue? frame state loan replacement with
  | some pair =>
      rw [localUpdate] at update
      cases Option.some.inj update
      exact updateLocalBorrowValue?_registries localUpdate
  | none =>
      rw [localUpdate] at update
      dsimp only at update
      split at update
      · rw [Option.map_eq_some_iff] at update
        obtain ⟨_, _, built⟩ := update
        obtain ⟨rfl, _⟩ := Prod.mk.inj built
        exact ⟨rfl, rfl⟩
      · /- The remaining search rewrites a global entry and leaves the frame
        exactly as it was. -/
        split at update
        · rw [Option.bind_eq_some_iff] at update
          obtain ⟨_, _, rest⟩ := update
          rw [Option.map_eq_some_iff] at rest
          obtain ⟨_, _, built⟩ := rest
          obtain ⟨rfl, _⟩ := Prod.mk.inj built
          exact ⟨rfl, rfl⟩
        · exact absurd update (by simp)

theorem mutateBorrow?_registries {operands : Array RuntimeValue}
    {frame : RuntimeFrame} {state : RuntimeState}
    {written : RuntimeFrame} {writtenState : RuntimeState} {result : RuntimeValue}
    (mutation : mutateBorrow? operands frame state
      = some (written, writtenState, result)) :
    written.activeLoans = frame.activeLoans ∧
      written.loanLocations = frame.loanLocations := by
  unfold mutateBorrow? at mutation
  split at mutation
  · split at mutation
    · rename_i pair borrowUpdate
      obtain ⟨rfl, _, _⟩ := Prod.mk.inj (Option.some.inj mutation) |>.imp id Prod.mk.inj
      exact updateBorrowValue?_registries borrowUpdate
    · obtain ⟨rfl, _⟩ := Prod.mk.inj (Option.some.inj mutation)
      exact applyWriteBack_registries frame state _ _
  · exact absurd mutation (by simp)

/-- An evaluator that only ever rewrites locals keeps the row.

This is the shape every reference evaluator has — `updateBorrowValue?`,
`applyWriteBack`, and the reads beside them all produce
`{ frame with locals := … }` — so the row law is proved once from that
observation rather than from each evaluator's internals. -/
theorem evaluatorRowStable_of_localsOnly {evaluate : NativeEvaluator}
    (localsOnly : ∀ operands frame state result,
      evaluate operands frame state = some result →
        resultFrame result
          = { frame with locals := (resultFrame result).locals }) :
    EvaluatorRowStable evaluate := by
  intro operands row registries state result evaluated
  refine ⟨(resultFrame result).locals, ?_⟩
  rw [localsOnly operands (rowFrame row registries) state result evaluated]
  rfl

/-- Reading through a reference does not move the frame at all. -/
theorem evaluatorRowStable_dereference :
    EvaluatorRowStable (ReferenceLocationOperation.dereference.evaluate?) := by
  refine evaluatorRowStable_of_localsOnly ?_
  intro operands frame state result evaluated
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    dereferenceBorrow?] at evaluated
  split at evaluated
  · have reduced : some (GlobalOperationResult.value frame state _) = some result :=
      evaluated
    cases Option.some.inj reduced
    rfl
  · simp_all

/-- A checked primitive computes on values, so it keeps the row. -/
theorem evaluatorRowStable_liftPrimitive
    (evaluate : Array RuntimeValue →
      Option (Except (ThrowKind × Array RuntimeValue) RuntimeValue)) :
    EvaluatorRowStable (liftPrimitiveEvaluator evaluate) := by
  refine evaluatorRowStable_of_localsOnly ?_
  intro operands frame state result evaluated
  unfold liftPrimitiveEvaluator at evaluated
  have spelled : (evaluate operands).bind (fun outcome =>
      match outcome with
      | .ok value => some (GlobalOperationResult.value frame state value)
      | .error (kind, thrown) =>
          some (GlobalOperationResult.throw_ frame state kind thrown))
      = some result := evaluated
  rw [Option.bind_eq_some_iff] at spelled
  obtain ⟨outcome, _, built⟩ := spelled
  cases outcome with
  | ok value => cases Option.some.inj built; rfl
  | error pair =>
      obtain ⟨kind, thrown⟩ := pair
      cases Option.some.inj built
      rfl

/-- Writing through a reference keeps the row: it rewrites a local, or a
global entry, and leaves the loan bookkeeping to the interpreter.  With
this the whole of a mutation through a reference is frame-free. -/
theorem evaluatorRowStable_mutate :
    EvaluatorRowStable (ReferenceLocationOperation.mutate.evaluate?) := by
  refine evaluatorRowStable_of_localsOnly ?_
  intro operands frame state result evaluated
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator] at evaluated
  have spelled : (mutateBorrow? operands frame state).bind
      (fun triple =>
        some (GlobalOperationResult.value triple.1 triple.2.1 triple.2.2))
      = some result := evaluated
  rw [Option.bind_eq_some_iff] at spelled
  obtain ⟨triple, mutation, built⟩ := spelled
  obtain ⟨written, writtenState, produced⟩ := triple
  obtain ⟨registriesActive, registriesLocations⟩ := mutateBorrow?_registries mutation
  cases Option.some.inj built
  simp only [resultFrame]
  rw [← registriesActive, ← registriesLocations]

/-! ## Frame-free rules

Each rule is the existing frame rule with the frame read off the row.  The
right-hand sides mention only values, the state, and the row. -/

@[simp] theorem wpRow_value (runtimeValue : RuntimeValue) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop) :
    wpRow (value runtimeValue) row registries state post ↔
      post row state (.value runtimeValue) := by
  constructor
  · intro h
    exact h row state (.value runtimeValue) ⟨rfl, rfl, rfl⟩
  · rintro h finalRow finalState control ⟨equal, rfl, rfl⟩
    cases rowFrame_injective equal
    exact h

@[simp] theorem wpRow_localVar (localId : LocalId) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop) :
    wpRow (localVar localId) row registries state post ↔
      ∀ runtimeValue, row[localId.index]?.join = some runtimeValue →
        post row state (.value runtimeValue) := by
  constructor
  · intro h runtimeValue read
    exact h row state (.value runtimeValue) ⟨runtimeValue, read, rfl, rfl, rfl⟩
  · rintro h finalRow finalState control ⟨runtimeValue, read, equal, rfl, rfl⟩
    cases rowFrame_injective equal
    exact h runtimeValue read

/-- Assignment updates the row.  This is the case that shows the layer
carries state without a frame: the successor is `row.set! index value`,
an array of values, where the frame formulation rebuilds a record whose
other two fields have to be carried along and later reconciled. -/
theorem rowStable_nativeAssignLocal {localId : LocalId} {value : ExprDenotation}
    (stable : RowStable value) : RowStable (nativeAssignLocal localId value) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨valueStep, _⟩ | ⟨valueFrame, valueState, runtimeValue,
    valueStep, bound, rfl, _⟩
  · exact stable row registries state finalFrame finalState control valueStep
  · obtain ⟨valueRow, rfl⟩ :=
      stable row registries state valueFrame valueState (.value runtimeValue) valueStep
    refine ⟨valueRow.set! localId.index (some runtimeValue), ?_⟩
    simp [rowFrame]

theorem wpRow_nativeAssignLocal (localId : LocalId) (value : ExprDenotation)
    (row : Row) (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop)
    (stable : RowStable value)
    (assigned : wpRow value row registries state
      fun valueRow valueState control =>
      match control with
      | .value runtimeValue =>
          localId.index < valueRow.size →
            post (valueRow.set! localId.index (some runtimeValue)) valueState
              (.value .unit)
      | _ => post valueRow valueState control) :
    wpRow (nativeAssignLocal localId value) row registries state post := by
  rintro finalRow finalState control step
  rcases step with ⟨valueStep, abrupt⟩ | ⟨valueFrame, valueState, runtimeValue,
    valueStep, bound, equal, stateEqual, controlEqual⟩
  · have := assigned finalRow finalState control valueStep
    cases control <;> simp_all
  · obtain ⟨valueRow, rfl⟩ :=
      stable row registries state valueFrame valueState (.value runtimeValue) valueStep
    have applied := assigned valueRow valueState (.value runtimeValue) valueStep
    simp only [] at applied
    have size : localId.index < valueRow.size := by simpa using bound
    have updated : finalRow = valueRow.set! localId.index (some runtimeValue) := by
      apply rowFrame_injective
      rw [equal]
      simp [rowFrame]
    subst updated stateEqual controlEqual
    exact applied size

/-- Reading through a reference, closed: the borrow's current, any frame.
Stated at the operation head so resolution matches it without unfolding
the evaluator — an unfolded evaluator is exactly what the closed mutate
lemma below can no longer match. -/
theorem dereference_evaluate (loan : Nat) (current : RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    ReferenceLocationOperation.dereference.evaluate? #[.borrow loan current]
        frame state
      = some (.value frame state current) := rfl

/-- Mutating the sole scalar mutable parameter, at row level: the borrow's
current is replaced and everything else — including the registries — is
untouched.  Proved from the frame-level closed row; stated with `rowFrame`
folded so a goal never sees the record. -/
theorem mutate_evaluate_rowFrame_singleBorrow (loan : Nat)
    (current replacement : RuntimeValue) (state : RuntimeState)
    (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate?
        #[.borrow loan current, replacement]
        (rowFrame #[some (.borrow loan current)]
          { activeLoans
            loanLocations := #[(loan, ⟨.local ⟨0⟩, #[], true⟩)] })
        state
      = some (.value
          (rowFrame #[some (.borrow loan replacement)]
            { activeLoans
              loanLocations := #[(loan, ⟨.local ⟨0⟩, #[], true⟩)] })
          state .unit) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame]
  rw [updateBorrowValue?_localZero state loan current replacement _ _
    (by simp) (by simp)]
  rfl

/-- Mutating the borrow in local 0 of any row: the borrow's current is
replaced and the other locals, like the registries, are untouched. -/
theorem mutate_evaluate_rowFrame_borrowFirst (loan : Nat)
    (current replacement : RuntimeValue) (rest : List (Option RuntimeValue))
    (state : RuntimeState) (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate?
        #[.borrow loan current, replacement]
        (rowFrame (some (.borrow loan current) :: rest).toArray
          { activeLoans
            loanLocations := #[(loan, ⟨.local ⟨0⟩, #[], true⟩)] })
        state
      = some (.value
          (rowFrame (some (.borrow loan replacement) :: rest).toArray
            { activeLoans
              loanLocations := #[(loan, ⟨.local ⟨0⟩, #[], true⟩)] })
          state .unit) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame]
  rw [updateBorrowValue?_localZero state loan current replacement _ _
    (by simp) (by simp)]
  rfl

/-- Writing through the first of two mutable parameters, on their row. -/
theorem mutate_evaluate_rowFrame_twoBorrows_left (leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent replacement : RuntimeValue)
    (state : RuntimeState) (activeLoans : Array (ExprId × Nat))
    (separate : leftLoan ≠ rightLoan) :
    ReferenceLocationOperation.mutate.evaluate?
        #[.borrow leftLoan leftCurrent, replacement]
        (rowFrame #[some (.borrow leftLoan leftCurrent), some (.borrow rightLoan rightCurrent)]
          { activeLoans
            loanLocations := #[(leftLoan, ⟨.local ⟨0⟩, #[], true⟩),
              (rightLoan, ⟨.local ⟨1⟩, #[], true⟩)] })
        state
      = some (.value
          (rowFrame #[some (.borrow leftLoan replacement), some (.borrow rightLoan rightCurrent)]
            { activeLoans
              loanLocations := #[(leftLoan, ⟨.local ⟨0⟩, #[], true⟩),
                (rightLoan, ⟨.local ⟨1⟩, #[], true⟩)] })
          state .unit) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame]
  rw [updateBorrowValue?_twoLocals_left state leftLoan rightLoan leftCurrent rightCurrent
    replacement activeLoans separate]
  rfl

/-- Writing through the second of two mutable parameters, on their row. -/
theorem mutate_evaluate_rowFrame_twoBorrows_right (leftLoan rightLoan : Nat)
    (leftCurrent rightCurrent replacement : RuntimeValue)
    (state : RuntimeState) (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate?
        #[.borrow rightLoan rightCurrent, replacement]
        (rowFrame #[some (.borrow leftLoan leftCurrent), some (.borrow rightLoan rightCurrent)]
          { activeLoans
            loanLocations := #[(leftLoan, ⟨.local ⟨0⟩, #[], true⟩),
              (rightLoan, ⟨.local ⟨1⟩, #[], true⟩)] })
        state
      = some (.value
          (rowFrame #[some (.borrow leftLoan leftCurrent), some (.borrow rightLoan replacement)]
            { activeLoans
              loanLocations := #[(leftLoan, ⟨.local ⟨0⟩, #[], true⟩),
                (rightLoan, ⟨.local ⟨1⟩, #[], true⟩)] })
          state .unit) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame]
  rw [updateBorrowValue?_twoLocals_right state leftLoan rightLoan leftCurrent rightCurrent
    replacement activeLoans]
  rfl

/-- Finalizing two scalar mutable parameters, at row level: each exports
its current as its pending write-back, in local order. -/
theorem exportFrameLoans_rowFrame_twoIntegers (state : RuntimeState)
    (leftLoan rightLoan : Nat) (left right : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noLeftGlobal : globalLoanKeyIn? state.globalLoans leftLoan = none)
    (noRightGlobal : globalLoanKeyIn? state.globalLoans rightLoan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow leftLoan (.integer left)),
          some (.borrow rightLoan (.integer right))]
          { activeLoans, loanLocations })
        state
      = { state with
          pending :=
            (state.pending.push (leftLoan, .integer left)).push (rightLoan, .integer right) } :=
  exportFrameLoans_twoIntegers_state state leftLoan rightLoan left right activeLoans
    loanLocations noLeftGlobal noRightGlobal

/-- Finalizing the mutable parameter beside two plain integer locals. -/
theorem exportFrameLoans_rowFrame_borrowTwoIntegers (state : RuntimeState)
    (loan : Nat) (value first second : Int) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan (.integer value)), some (.integer first),
          some (.integer second)]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (loan, .integer value) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, applyWriteBack_empty_export, globalLoanKey?, noGlobal]

/-- Finalizing a parameter whose reborrow was returned, at row level: the
lender exports the returned loan's hole as its prophecy. -/
theorem exportFrameLoans_rowFrame_returnedReborrow (state : RuntimeState)
    (outer returned : Nat) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outer ≠ returned)
    (noGlobal : globalLoanKeyIn? state.globalLoans outer = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow outer (.loanHole returned))]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (outer, .loanHole returned) } :=
  exportFrameLoans_returnedReborrow_state state outer returned activeLoans
    loanLocations separate noGlobal

/-- Finalizing the single scalar mutable parameter, at row level: the
borrow's current is exported as the pending write-back, which is the
prophecy pair's second half made concrete. -/
theorem exportFrameLoans_rowFrame_singleInteger (state : RuntimeState)
    (loan : Nat) (value : Int) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan (.integer value))]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (loan, .integer value) } :=
  exportFrameLoans_singleInteger_state state loan value activeLoans
    loanLocations noGlobal

/- A returning callee's exported write-back, read by the caller's contract
through the results it returned; scalar results carry no borrow. -/
attribute [lir_data_norm high] resolveReturnedBorrows_empty
  resolveReturnedBorrows_integer

/-- The exit row of a borrow parameter beside a plain integer local: only
the borrow exports, as a pending write-back of its current value. -/
theorem exportFrameLoans_rowFrame_borrowInteger (state : RuntimeState)
    (loan : Nat) (value plain : Int) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan (.integer value)), some (.integer plain)]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (loan, .integer value) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, applyWriteBack_empty_export, globalLoanKey?, noGlobal]

/-- The `Bool` twin of `exportFrameLoans_rowFrame_borrowInteger`. -/
theorem exportFrameLoans_rowFrame_borrowBool (state : RuntimeState)
    (loan : Nat) (value : Int) (plain : Bool) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan (.integer value)), some (.bool plain)]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (loan, .integer value) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, applyWriteBack_empty_export, globalLoanKey?, noGlobal]

/-- Result blocks, mirroring `blockUnit`. -/
theorem rowStable_blockResult {statements : StatementsDenotation}
    {result : ExprDenotation} (stable : RowStableStatements statements)
    (resultStable : RowStable result) : RowStable (blockResult statements result) := by
  rintro row registries state finalFrame finalState control step
  rcases step with abrupt | ⟨statementFrame, statementState, done, resultStep⟩
  · obtain ⟨resultRow, shape⟩ := stable row registries state _ abrupt
    rcases shape with ⟨_, equal⟩ | ⟨_, _, equal⟩
    · exact absurd equal (by simp)
    · exact ⟨resultRow, by simpa using (StatementsResult.control.inj equal).2.1⟩
  · obtain ⟨statementRow, shape⟩ := stable row registries state _ done
    rcases shape with ⟨_, equal⟩ | ⟨_, _, equal⟩
    · obtain ⟨rfl, frameEq⟩ := StatementsResult.done.inj equal
      exact resultStable statementRow registries _ finalFrame finalState control
        (frameEq ▸ resultStep)
    · exact absurd equal (by simp)

theorem wpRow_blockResult (statements : StatementsDenotation)
    (result : ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState) (post : Row → RuntimeState → Control → Prop)
    (stable : RowStableStatements statements)
    (stepped : wpStatementsRow statements row registries state fun outcome =>
      match outcome with
      | .done doneState doneRow => wpRow result doneRow registries doneState post
      | .control finalState finalRow control => post finalRow finalState control) :
    wpRow (blockResult statements result) row registries state post := by
  rintro finalRow finalState control step
  rcases step with abrupt | ⟨statementFrame, statementState, done, resultStep⟩
  · exact stepped (.control finalState finalRow control) abrupt
  · obtain ⟨statementRow, shape⟩ := stable row registries state _ done
    rcases shape with ⟨_, equal⟩ | ⟨_, _, equal⟩
    · obtain ⟨rfl, frameEq⟩ := StatementsResult.done.inj equal
      have continued := stepped (.done statementState statementRow)
        (by simpa [RowStatementsResult.embed, frameEq] using done)
      exact continued finalRow finalState control (frameEq ▸ resultStep)
    · exact absurd equal (by simp)

/-- The two-parameter entry row, closed. -/
theorem initialLocals_two (first second : RuntimeValue) :
    initialLocals 2 #[first, second] = #[some first, some second] := by
  simp [initialLocals]
  rfl

/-- The one-parameter entry row, closed: the argument in front, no other
declared locals.  One lemma per arity is cheap and bounded — the arity is
the function's, not the corpus's. -/
theorem initialLocals_one (argument : RuntimeValue) :
    initialLocals 1 #[argument] = #[some argument] := by
  simp [initialLocals]
  rfl

/-- The `u64` bounds, closed.  `Ty.integerBounds?`'s own equations expose a
width test that `simp only` will not evaluate, and an unevaluated width
test is what a later split wastes its fuel on; a closed equation per width
in use keeps the range test the only conditional left. -/
theorem integerBounds_u64 :
    (Ty.integer (IntWidth.bits 64) false).integerBounds? = some (0, 2 ^ 64 - 1) := rfl

theorem integerBounds_u8 :
    (Ty.integer (IntWidth.bits 8) false).integerBounds? = some (0, 2 ^ 8 - 1) := rfl

/-- A `do`-bind over a known `some` reduces; the evaluator equations
produce exactly this shape and no core lemma spells it at `Bind.bind`. -/
theorem bind_some_eval {α β : Type} (value : α) (continuation : α → Option β) :
    (some value >>= continuation) = continuation value := rfl

/- The closed evaluations a resolution step computes with.  Definitions
whose equations compute on literal operands, plus the row-level closed
lemmas above; `rewriteFirst`-style value recursion is deliberately absent —
it does not unfold, which is why the mutate lemma exists. -/
attribute [lir_row_eval]
  List.getElem?_toArray
  List.getElem?_cons_zero
  List.getElem?_cons_succ
  Option.join_some
  PrimitiveLocationOperation.evaluate?
  liftPrimitiveEvaluator
  checkedBinaryInteger
  checkedInteger
  integerBounds_u64
  integerBounds_u8
  dereference_evaluate
  mutate_evaluate_rowFrame_singleBorrow
  finishControl?
  unpackFallthrough
  Option.map_eq_map
  Option.map_some
  Option.some.injEq
  bind_some_eval

/-! ## Driving a body frame-free

The rules above are structural: each names one combinator, so walking a
body is applying them in the order the body is built.  There is no search
here and no inventory to match against — the body's own shape selects the
rule, which is the point of the whole layer. -/


/-! ## Control inside a segment: branches, throws, lets

A storage body's inner block branches on a comparison, throws from the
failing arm, and binds a read.  Each is one rule over the row, with any
side computation (the branch's condition, the let's binding) left as a
step the script closes at the literal row. -/

theorem rowStable_nativeBranch {condition thenBranch : ExprDenotation}
    {elseBranch : Option ExprDenotation}
    (conditionStable : RowStable condition) (thenStable : RowStable thenBranch)
    (elseStable : ∀ branch, elseBranch = some branch → RowStable branch) :
    RowStable (nativeBranch condition thenBranch elseBranch) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨conditionStep, -⟩ |
    ⟨conditionFrame, conditionState, conditionStep, thenStep⟩ |
    ⟨conditionFrame, conditionState, conditionStep, elseStep⟩
  · exact conditionStable _ _ _ _ _ _ conditionStep
  · obtain ⟨conditionRow, rfl⟩ := conditionStable _ _ _ _ _ _ conditionStep
    exact thenStable _ _ _ _ _ _ thenStep
  · obtain ⟨conditionRow, rfl⟩ := conditionStable _ _ _ _ _ _ conditionStep
    cases elseBranch with
    | some branch => exact elseStable branch rfl _ _ _ _ _ _ elseStep
    | none =>
        obtain ⟨rfl, -, -⟩ := elseStep
        exact ⟨conditionRow, rfl⟩

theorem rowStable_nativeThrow {kind : ThrowKind} {arguments : ValuesDenotation}
    (argumentsStable : RowStableValues arguments) :
    RowStable (nativeThrow kind arguments) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨values, valuesStep, -⟩ | controlStep
  · obtain ⟨resultRow, shape⟩ := argumentsStable row registries state _ valuesStep
    rcases shape with ⟨_, _, equal⟩ | ⟨_, _, equal⟩
    · exact ⟨resultRow, by simpa using (ValuesResult.values.inj equal).2.1⟩
    · exact absurd equal (by simp)
  · obtain ⟨resultRow, shape⟩ := argumentsStable row registries state _ controlStep
    rcases shape with ⟨_, _, equal⟩ | ⟨_, _, equal⟩
    · exact absurd equal (by simp)
    · exact ⟨resultRow, by simpa using (ValuesResult.control.inj equal).2.1⟩

/-- The weakest precondition of a branch: the condition's boolean selects
the arm; an absent else arm is unit. -/
theorem wpRow_nativeBranch (condition thenBranch : ExprDenotation)
    (elseBranch : Option ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState) (post : Row → RuntimeState → Control → Prop)
    (conditionStable : RowStable condition)
    (stepped : wpRow condition row registries state
      fun conditionRow conditionState control =>
        match control with
        | .value (.bool true) =>
            wpRow thenBranch conditionRow registries conditionState post
        | .value (.bool false) =>
            match elseBranch with
            | some branch => wpRow branch conditionRow registries conditionState post
            | none => post conditionRow conditionState (.value .unit)
        | .value _ => True
        | _ => post conditionRow conditionState control) :
    wpRow (nativeBranch condition thenBranch elseBranch) row registries state
      post := by
  rintro finalRow finalState control step
  rcases step with ⟨conditionStep, abrupt⟩ |
    ⟨conditionFrame, conditionState, conditionStep, thenStep⟩ |
    ⟨conditionFrame, conditionState, conditionStep, elseStep⟩
  · have applied := stepped finalRow finalState control conditionStep
    cases control with
    | value runtimeValue => cases abrupt
    | return_ values => exact applied
    | break_ label value => exact applied
    | continue_ label => exact applied
    | throw_ kind thrown => exact applied
  · obtain ⟨conditionRow, rfl⟩ := conditionStable _ _ _ _ _ _ conditionStep
    have applied := stepped conditionRow conditionState _ conditionStep
    exact applied finalRow finalState control thenStep
  · obtain ⟨conditionRow, rfl⟩ := conditionStable _ _ _ _ _ _ conditionStep
    have applied := stepped conditionRow conditionState _ conditionStep
    cases elseBranch with
    | some branch => exact applied finalRow finalState control elseStep
    | none =>
        obtain ⟨frameEq, rfl, rfl⟩ := elseStep
        cases rowFrame_injective frameEq
        exact applied

/-- The weakest precondition of a throw: its argument row, then the
throw control. -/
theorem wpRow_nativeThrow (kind : ThrowKind) (arguments : ValuesDenotation)
    (row : Row) (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop)
    (argumentsStable : RowStableValues arguments)
    (stepped : wpValuesRow arguments row registries state fun result =>
      match result with
      | .control resultState resultRow propagated =>
          post resultRow resultState propagated
      | .values resultState resultRow values =>
          post resultRow resultState (.throw_ kind values.toArray)) :
    wpRow (nativeThrow kind arguments) row registries state post := by
  rintro finalRow finalState control step
  rcases step with ⟨values, valuesStep, rfl⟩ | controlStep
  · obtain ⟨resultRow, shape⟩ := argumentsStable row registries state _ valuesStep
    rcases shape with ⟨resultState, values', equal⟩ | ⟨_, _, equal⟩
    · obtain ⟨rfl, frameEq, rfl⟩ : finalState = resultState ∧
          rowFrame finalRow registries = rowFrame resultRow registries ∧
          values = values' := by simpa using equal
      cases rowFrame_injective frameEq
      exact stepped (.values finalState finalRow values)
        (by simpa [RowValuesResult.embed] using valuesStep)
    · exact absurd equal (by simp)
  · obtain ⟨resultRow, shape⟩ := argumentsStable row registries state _ controlStep
    rcases shape with ⟨_, _, equal⟩ | ⟨resultState, propagated, equal⟩
    · exact absurd equal (by simp)
    · obtain ⟨rfl, frameEq, rfl⟩ : finalState = resultState ∧
          rowFrame finalRow registries = rowFrame resultRow registries ∧
          control = propagated := by simpa using equal
      cases rowFrame_injective frameEq
      exact stepped (.control finalState finalRow control)
        (by simpa [RowValuesResult.embed] using controlStep)

/-- Binding a `let`'s variable pattern writes its local. -/
theorem bindVariable_rowFrame (fuel : Nat) (localId : LocalId) (row : Row)
    (registries : Registries) (value : RuntimeValue)
    (inBounds : localId.index < row.size) :
    NativePatternBinder.bind ⟨fuel + 1, .variable localId⟩
      (rowFrame row registries) value =
    some (rowFrame (row.set! localId.index (some value)) registries) := by
  simp [NativePatternBinder.bind, bindNativePatternFuel, rowFrame, inBounds]

/-- The weakest precondition of a `let`: the initializer, then the binding
as a step the script closes at the literal row, then the body. -/
theorem wpRow_letNativeValue (binder : NativePatternBinder)
    (initializer body : ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState) (post : Row → RuntimeState → Control → Prop)
    (initializerStable : RowStable initializer)
    (stepped : wpRow initializer row registries state
      fun initializedRow initializedState control =>
        match control with
        | .value runtimeValue =>
            ∀ boundFrame,
              binder.bind (rowFrame initializedRow registries) runtimeValue =
                  some boundFrame →
                ∃ boundRow, boundFrame = rowFrame boundRow registries ∧
                  wpRow body boundRow registries initializedState post
        | _ => post initializedRow initializedState control) :
    wpRow (letNativeValue binder initializer body) row registries state
      post := by
  rintro finalRow finalState control step
  rcases step with ⟨initializerStep, abrupt⟩ |
    ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
      initializerStep, bindEq, bodyStep⟩
  · have applied := stepped finalRow finalState control initializerStep
    cases control with
    | value runtimeValue => cases abrupt
    | return_ values => exact applied
    | break_ label value => exact applied
    | continue_ label => exact applied
    | throw_ kind thrown => exact applied
  · obtain ⟨initializedRow, rfl⟩ :=
      initializerStable _ _ _ _ _ _ initializerStep
    have applied := stepped initializedRow initializedState _ initializerStep
    obtain ⟨boundRow, rfl, continuation⟩ := applied boundFrame bindEq
    exact continuation finalRow finalState control bodyStep

theorem rowStable_letNativeValue (fuel : Nat) (localId : LocalId)
    {initializer body : ExprDenotation}
    (initializerStable : RowStable initializer) (bodyStable : RowStable body) :
    RowStable (letNativeValue ⟨fuel + 1, .variable localId⟩ initializer body) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨initializerStep, -⟩ |
    ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
      initializerStep, bindEq, bodyStep⟩
  · exact initializerStable _ _ _ _ _ _ initializerStep
  · obtain ⟨initializedRow, rfl⟩ :=
      initializerStable _ _ _ _ _ _ initializerStep
    by_cases inBounds : localId.index < initializedRow.size
    · rw [bindVariable_rowFrame fuel localId _ _ _ inBounds] at bindEq
      cases Option.some.inj bindEq
      exact bodyStable _ _ _ _ _ _ bodyStep
    · simp [NativePatternBinder.bind, bindNativePatternFuel, rowFrame,
        inBounds] at bindEq

/-- The comparison a branch guards on, computed. -/
theorem less_evaluate (resultType : Ty) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.less resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.bool (decide (left < right)))) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    compareOrdered]

/-- Structural equality of two integers is the decision of their
equality. -/
theorem equal_evaluate (resultType : Ty) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.equal resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.bool (decide (left = right)))) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    SemanticOperations.equalValues?]

theorem notEqual_evaluate (resultType : Ty) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.notEqual resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.bool (decide (left ≠ right)))) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    SemanticOperations.notEqualValues?, bne]

theorem lessEqual_evaluate (resultType : Ty) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.lessEqual resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.bool (decide (left ≤ right)))) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    compareOrdered]

theorem greater_evaluate (resultType : Ty) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.greater resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.bool (decide (left > right)))) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    compareOrdered]

theorem greaterEqual_evaluate (resultType : Ty) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.greaterEqual resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.bool (decide (left ≥ right)))) := by
  simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    compareOrdered]

/-- Checked division by a nonzero divisor: the quotient's range check, as
any checked result. -/
theorem checkedDivide_evaluate (failure : ThrowKind) (resultType : Ty)
    (left right : Int) (frame : RuntimeFrame) (state : RuntimeState)
    (nonzero : right ≠ 0) :
    (PrimitiveLocationOperation.checkedDivide failure resultType).evaluate?
        #[.integer left, .integer right] frame state =
      some (match resultType.integerBounds? with
        | some (lower, upper) =>
            if lower <= left.tdiv right && left.tdiv right <= upper then
              .value frame state (.integer (left.tdiv right))
            else .throw_ frame state failure #[.integer (left.tdiv right)]
        | none => .throw_ frame state failure #[]) := by
  simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator]
  unfold checkedDivideIntegers?
  simp only []
  rw [truncatingQuotient?_eq, if_neg nonzero]
  simp only [bind, Option.bind, pure, checkedInteger]
  cases resultType.integerBounds? with
  | none => rfl
  | some bounds =>
      obtain ⟨lower, upper⟩ := bounds
      by_cases h : (lower <= left.tdiv right && left.tdiv right <= upper) = true
      · simp only [h, ↓reduceIte]
      · simp only [h, Bool.false_eq_true, ↓reduceIte]

/-- Reduce the target's head — beta and iota only, never delta — so a rule
whose continuation is a `match` on the produced control exposes the next
combinator without traversing the rest of the goal.  The whole-goal `dsimp`
this replaces walked the full continuation at every program point, which is
the same per-step tax the frame drive paid before its rule set was
registered. -/
elab "leaner_row_head" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← goal.withContext do
    Lean.instantiateMVars (← goal.getType)
  let reduced ← goal.withContext do Lean.Meta.whnfCore target
  if reduced == target then
    Lean.Meta.throwTacticEx `leaner_row_head goal "the head does not reduce"
  Lean.Elab.Tactic.replaceMainGoal [← goal.change reduced (checkDefEq := false)]

/-- A shared global borrow keeps the frame it was given. -/
theorem evaluatorRowStable_globalBorrowShared (namespaceId : NamespaceId)
    (typeId : TypeId) (referenceType : ReferenceType) (lex : Nat)
    (sharedKind : referenceType.kind = .shared) :
    EvaluatorRowStable
      (GlobalLocationOperation.borrow
        { resource := ⟨namespaceId, typeId⟩, referenceType, kind := .immutable,
          lexicalLoan := lex }).evaluate? := by
  intro operands row registries state result evaluated
  simp only [GlobalLocationOperation.evaluate?, borrowGlobalAt?,
    borrowGlobalUsing?] at evaluated
  cases operandsEq : operands.toList with
  | nil => simp [operandsEq] at evaluated
  | cons key rest =>
    cases rest with
    | cons _ _ => simp [operandsEq] at evaluated
    | nil =>
      simp only [operandsEq] at evaluated
      cases keyEq : key.storageKey? with
      | none => simp [keyEq] at evaluated
      | some storageKey =>
        simp only [keyEq, bind, Option.bind] at evaluated
        cases lookupEq : globalValue? state namespaceId typeId key with
        | none =>
            simp only [lookupEq] at evaluated
            cases Option.some.inj evaluated
            exact ⟨row, rfl⟩
        | some value =>
            simp only [lookupEq] at evaluated
            rw [borrowRuntimePlaceAt?_global_immutable_of_lookup lex referenceType
              _ _ _ value sharedKind lookupEq] at evaluated
            simp only [bind, Option.bind] at evaluated
            cases Option.some.inj evaluated
            exact ⟨row, rfl⟩

/-- Taking a resource out of storage returns the frame it was given. -/
theorem evaluatorRowStable_takeGlobal (namespaceId : NamespaceId)
    (typeId : TypeId) :
    EvaluatorRowStable (GlobalLocationOperation.take ⟨namespaceId, typeId⟩).evaluate? := by
  intro operands row registries state result evaluated
  simp only [GlobalLocationOperation.evaluate?, takeGlobalAt?] at evaluated
  cases operandsEq : operands.toList with
  | nil => simp [operandsEq] at evaluated
  | cons key rest =>
    cases rest with
    | cons _ _ => simp [operandsEq] at evaluated
    | nil =>
      simp only [operandsEq] at evaluated
      cases keyEq : key.storageKey? with
      | none => simp [keyEq] at evaluated
      | some storageKey =>
        simp only [keyEq, bind, Option.bind] at evaluated
        cases lookupEq : globalValue? state namespaceId typeId key with
        | none =>
          simp only [lookupEq] at evaluated
          cases Option.some.inj evaluated
          exact ⟨row, rfl⟩
        | some value =>
          simp only [lookupEq] at evaluated
          cases Option.some.inj evaluated
          exact ⟨row, rfl⟩

/-- The existence test keeps the frame. -/
theorem evaluatorRowStable_containsGlobal (namespaceId : NamespaceId)
    (typeId : TypeId) :
    EvaluatorRowStable
      (GlobalLocationOperation.contains ⟨namespaceId, typeId⟩).evaluate? := by
  intro operands row registries state result evaluated
  simp only [GlobalLocationOperation.evaluate?, containsGlobalAt?] at evaluated
  cases operandsEq : operands.toList with
  | nil => simp [operandsEq] at evaluated
  | cons key rest =>
    cases rest with
    | cons _ _ => simp [operandsEq] at evaluated
    | nil =>
      simp only [operandsEq] at evaluated
      cases keyEq : key.storageKey? with
      | none => simp [keyEq] at evaluated
      | some storageKey =>
        simp only [keyEq, bind, Option.bind] at evaluated
        cases Option.some.inj evaluated
        exact ⟨row, rfl⟩

/-- A `let` destructuring a one-field struct into a local keeps the row
shape: the binding sets one local or fails. -/
theorem rowStable_letConstructorOne (fuel : Nat) (source : StructHandle)
    (localId : LocalId) {initializer body : ExprDenotation}
    (initializerStable : RowStable initializer) (bodyStable : RowStable body) :
    RowStable (letNativeValue ⟨fuel + 2, .constructor source none [.variable localId]⟩
      initializer body) := by
  rintro row registries state finalFrame finalState control step
  rcases step with ⟨initializerStep, -⟩ |
    ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
      initializerStep, bindEq, bodyStep⟩
  · exact initializerStable _ _ _ _ _ _ initializerStep
  · obtain ⟨initializedRow, rfl⟩ :=
      initializerStable _ _ _ _ _ _ initializerStep
    suffices bound : ∃ boundRow, boundFrame = rowFrame boundRow registries by
      obtain ⟨boundRow, rfl⟩ := bound
      exact bodyStable _ _ _ _ _ _ bodyStep
    simp only [NativePatternBinder.bind, bindNativePatternFuel] at bindEq
    split at bindEq
    · split at bindEq
      · exact absurd bindEq (by simp)
      · rename_i values _
        rcases valuesEq : values.toList with _ | ⟨value, _ | ⟨_, _⟩⟩
        · simp [valuesEq, bindNativePatternRow] at bindEq
        · simp only [valuesEq, bindNativePatternRow, rowFrame, bind, Option.bind] at bindEq
          split at bindEq <;> rename_i heq
          all_goals first
            | (simp at bindEq; done)
            | (simp only [Option.some.injEq] at bindEq
               subst bindEq
               split at heq
               · first
                 | exact ⟨_, (Option.some.inj heq).symm⟩
                 | exact ⟨_, Option.some.inj heq⟩
               · simp at heq)
        · simp [valuesEq, bindNativePatternRow] at bindEq
    · exact absurd bindEq (by simp)

/-- A copy keeps the frame. -/
theorem evaluatorRowStable_copyValue (resultType : Ty) :
    EvaluatorRowStable (PrimitiveLocationOperation.copyValue resultType).evaluate? := by
  intro operands row registries state result evaluated
  simp only [PrimitiveLocationOperation.evaluate?] at evaluated
  cases operandsEq : operands.toList with
  | nil => simp [operandsEq] at evaluated
  | cons value rest =>
    cases rest with
    | cons _ _ => simp [operandsEq] at evaluated
    | nil =>
      simp only [operandsEq] at evaluated
      cases Option.some.inj evaluated
      exact ⟨row, rfl⟩

/-- A constructor-lifted evaluator keeps the frame. -/
theorem evaluatorRowStable_liftConstructor
    (evaluate : Array RuntimeValue → Option RuntimeValue) :
    EvaluatorRowStable (liftConstructorEvaluator evaluate) := by
  intro operands row registries state result evaluated
  simp only [liftConstructorEvaluator, bind, Option.bind] at evaluated
  split at evaluated
  · cases evaluated
  · cases Option.some.inj evaluated
    exact ⟨row, rfl⟩

/-! ## Modular arithmetic rows -/

/-- The unsigned modular integer at a positive width: the residue the
runtime normalizes twice is the Euclidean remainder. -/
theorem modularInteger_unsigned (width : Nat) (value : Int) (widthPos : 0 < width) :
    modularInteger (.integer (.bits width) false) value =
      some (.integer (value % (2 : Int) ^ width)) := by
  have widthNonzero : (width == 0) = false := by
    simp [Nat.pos_iff_ne_zero.mp widthPos]
  unfold modularInteger
  simp only [widthNonzero, Bool.false_eq_true, ↓reduceIte, Bool.false_and]
  rw [Int.emod_add_emod, Int.add_emod, Int.emod_self, Int.add_zero, Int.emod_emod]

/-- The unsigned modular `+`: a closed row. -/
theorem add_evaluate_unsigned (width : Nat) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) (widthPos : 0 < width) :
    (PrimitiveLocationOperation.add (.integer (.bits width) false)).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.integer ((left + right) % (2 : Int) ^ width))) := by
  simp only [PrimitiveLocationOperation.evaluate?, modularBinaryInteger,
    modularInteger_unsigned _ _ widthPos]
  rfl

/-- The unsigned modular `-`: a closed row. -/
theorem subtract_evaluate_unsigned (width : Nat) (left right : Int)
    (frame : RuntimeFrame) (state : RuntimeState) (widthPos : 0 < width) :
    (PrimitiveLocationOperation.subtract (.integer (.bits width) false)).evaluate?
        #[.integer left, .integer right] frame state =
      some (.value frame state (.integer ((left - right) % (2 : Int) ^ width))) := by
  simp only [PrimitiveLocationOperation.evaluate?, modularBinaryInteger,
    modularInteger_unsigned _ _ widthPos]
  rfl

/-! ## Loops on the row -/

theorem rowStable_nativeSpec : RowStable nativeSpec := rowStable_value _

theorem wpRow_nativeSpec (row : Row) (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop) :
    wpRow nativeSpec row registries state post ↔ post row state (.value .unit) :=
  wpRow_value _ _ _ _ _

theorem rowStable_nativeBreakUnit (nest : Nat) : RowStable (nativeBreak nest none) := by
  rintro row registries state finalFrame finalState control ⟨rfl, _, _⟩
  exact ⟨row, rfl⟩

theorem wpRow_nativeBreakUnit (nest : Nat) (row : Row) (registries : Registries)
    (state : RuntimeState) (post : Row → RuntimeState → Control → Prop) :
    wpRow (nativeBreak nest none) row registries state post ↔
      post row state (.break_ nest none) := by
  constructor
  · intro h
    exact h row state (.break_ nest none) ⟨rfl, rfl, rfl⟩
  · rintro h finalRow finalState control ⟨equal, rfl, rfl⟩
    cases rowFrame_injective equal
    exact h

theorem rowStable_nativeContinue (nest : Nat) : RowStable (nativeContinue nest) := by
  rintro row registries state finalFrame finalState control ⟨rfl, _, _⟩
  exact ⟨row, rfl⟩

theorem wpRow_nativeContinue (nest : Nat) (row : Row) (registries : Registries)
    (state : RuntimeState) (post : Row → RuntimeState → Control → Prop) :
    wpRow (nativeContinue nest) row registries state post ↔
      post row state (.continue_ nest) := by
  constructor
  · intro h
    exact h row state (.continue_ nest) ⟨rfl, rfl, rfl⟩
  · rintro h finalRow finalState control ⟨equal, rfl, rfl⟩
    cases rowFrame_injective equal
    exact h

/-- A loop whose body keeps the row keeps the row: every iteration's frame
is a row by the body's stability. -/
theorem rowStable_nativeLoop {site : ExprId} {body : ExprDenotation}
    (bodyStable : RowStable body) : RowStable (nativeLoop site body) := by
  intro row registries state finalFrame finalState control step
  change NativeLoop body (rowFrame row registries) state finalFrame finalState control at step
  suffices h : ∀ frame, NativeLoop body frame state finalFrame finalState control →
      ∀ row, frame = rowFrame row registries → ∃ finalRow, finalFrame = rowFrame finalRow registries from
    h _ step row rfl
  intro frame loopStep
  clear step
  induction loopStep with
  | repeatValue value control bodyStep repeatStep ih =>
      rintro row rfl
      obtain ⟨bodyRow, rfl⟩ := bodyStable _ _ _ _ _ _ bodyStep
      exact ih bodyRow rfl
  | repeatContinue control bodyStep repeatStep ih =>
      rintro row rfl
      obtain ⟨bodyRow, rfl⟩ := bodyStable _ _ _ _ _ _ bodyStep
      exact ih bodyRow rfl
  | outerContinue nest bodyStep => rintro row rfl; exact bodyStable _ _ _ _ _ _ bodyStep
  | break_ breakValue bodyStep => rintro row rfl; exact bodyStable _ _ _ _ _ _ bodyStep
  | outerBreak nest breakValue bodyStep => rintro row rfl; exact bodyStable _ _ _ _ _ _ bodyStep
  | return_ values bodyStep => rintro row rfl; exact bodyStable _ _ _ _ _ _ bodyStep
  | throw_ kind arguments bodyStep => rintro row rfl; exact bodyStable _ _ _ _ _ _ bodyStep

/-- A loop's weakest precondition, sealed: the drive stops here — nothing
reduces or opens an irreducible constant — and the script resolves it by
the loop's invariant. -/
@[irreducible] def wpRowLoop (site : ExprId) (body : ExprDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop) : Prop :=
  wpRow (nativeLoop site body) row registries state post

theorem wpRow_nativeLoop_sealed (site : ExprId) (body : ExprDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop)
    (sealed : wpRowLoop site body row registries state post) :
    wpRow (nativeLoop site body) row registries state post := by
  unfold wpRowLoop at sealed
  exact sealed

/-- A loop on the row by an invariant over rows at the loop's registries:
the recursion is over the finite `NativeLoop` derivation, never an
unrolling.  The invariant is phrased over frames, as the contract
generator states it, and read at the row's frame.  One iteration
establishes the invariant where the loop repeats and the loop's own
postcondition where it leaves; the match is spelled out so the drive's
head reduction exposes it. -/
theorem wpRow_nativeLoop_of_invariant (site : ExprId) (body : ExprDenotation)
    (invariant : RuntimeFrame → RuntimeState → RuntimeFrame → RuntimeState → Prop)
    (row : Row) (registries : Registries) (state : RuntimeState)
    (post : Row → RuntimeState → Control → Prop)
    (bodyStable : RowStable body)
    (entry : invariant (rowFrame row registries) state (rowFrame row registries) state)
    (preserved : ∀ bodyRow bodyState,
      invariant (rowFrame row registries) state (rowFrame bodyRow registries) bodyState →
        wpRow body bodyRow registries bodyState
          fun iterationRow iterationState control =>
            match control with
            | .value _ =>
                invariant (rowFrame row registries) state
                  (rowFrame iterationRow registries) iterationState
            | .continue_ 0 =>
                invariant (rowFrame row registries) state
                  (rowFrame iterationRow registries) iterationState
            | .continue_ (nest + 1) => post iterationRow iterationState (.continue_ nest)
            | .break_ 0 value => post iterationRow iterationState (.value (value.getD .unit))
            | .break_ (nest + 1) value => post iterationRow iterationState (.break_ nest value)
            | .return_ values => post iterationRow iterationState (.return_ values)
            | .throw_ kind arguments => post iterationRow iterationState (.throw_ kind arguments)) :
    wpRowLoop site body row registries state post := by
  unfold wpRowLoop
  intro finalRow finalState control step
  change NativeLoop body (rowFrame row registries) state (rowFrame finalRow registries)
    finalState control at step
  suffices h : ∀ frame iterationState finalFrame,
      NativeLoop body frame iterationState finalFrame finalState control →
      ∀ iterationRow, frame = rowFrame iterationRow registries →
        invariant (rowFrame row registries) state (rowFrame iterationRow registries)
          iterationState →
        finalFrame = rowFrame finalRow registries →
        post finalRow finalState control from
    h _ _ _ step row rfl entry rfl
  intro frame iterationState finalFrame loopStep
  clear step
  induction loopStep with
  | repeatValue value control bodyStep repeatStep ih =>
      rintro iterationRow rfl inv rfl
      obtain ⟨bodyRow, rfl⟩ := bodyStable _ _ _ _ _ _ bodyStep
      exact ih bodyRow rfl (preserved _ _ inv _ _ _ bodyStep) rfl
  | repeatContinue control bodyStep repeatStep ih =>
      rintro iterationRow rfl inv rfl
      obtain ⟨bodyRow, rfl⟩ := bodyStable _ _ _ _ _ _ bodyStep
      exact ih bodyRow rfl (preserved _ _ inv _ _ _ bodyStep) rfl
  | outerContinue nest bodyStep =>
      rintro iterationRow rfl inv rfl
      exact preserved _ _ inv _ _ _ bodyStep
  | break_ breakValue bodyStep =>
      rintro iterationRow rfl inv rfl
      exact preserved _ _ inv _ _ _ bodyStep
  | outerBreak nest breakValue bodyStep =>
      rintro iterationRow rfl inv rfl
      exact preserved _ _ inv _ _ _ bodyStep
  | return_ values bodyStep =>
      rintro iterationRow rfl inv rfl
      exact preserved _ _ inv _ _ _ bodyStep
  | throw_ kind arguments bodyStep =>
      rintro iterationRow rfl inv rfl
      exact preserved _ _ inv _ _ _ bodyStep

/-- One stability step, chosen by the goal's head and the denotation's
head: a stability side condition names its shape syntactically, and a
chain of alternatives would pay a failed unification per rule at every
program point. -/
elab "leaner_row_stable_step" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← goal.withContext do Lean.instantiateMVars (← goal.getType)
  let rule ←
    if target.isForall then
      `(tactic| rintro branch ⟨⟩)
    else
      let some head := target.getAppFn.constName?
        | throwError "leaner_row_stable_step: not a stability goal"
      let argument := target.appArg!
      let some shape := argument.getAppFn.constName?
        | throwError "leaner_row_stable_step: no rule for this denotation"
      if head == ``RowStable then
        if shape == ``value then `(tactic| exact rowStable_value _)
        else if shape == ``localVar then `(tactic| exact rowStable_localVar _)
        else if shape == ``blockUnit then `(tactic| apply rowStable_blockUnit)
        else if shape == ``blockResult then `(tactic| apply rowStable_blockResult)
        else if shape == ``nativeOperation || shape == ``nativeReferenceOperation ||
            shape == ``nativePrimitiveOperation || shape == ``nativeLocalOperation ||
            shape == ``nativeDerefLocalBorrowOperation ||
            shape == ``nativeGlobalOperation then
          `(tactic| apply rowStable_nativeOperation)
        else if shape == ``nativeAssignLocal then `(tactic| apply rowStable_nativeAssignLocal)
        else if shape == ``nativeBranch then `(tactic| apply rowStable_nativeBranch)
        else if shape == ``nativeThrow then `(tactic| apply rowStable_nativeThrow)
        else if shape == ``nativeSpec then `(tactic| exact rowStable_nativeSpec)
        else if shape == ``nativeBreak then `(tactic| exact rowStable_nativeBreakUnit _)
        else if shape == ``nativeContinue then `(tactic| exact rowStable_nativeContinue _)
        else if shape == ``nativeLoop then `(tactic| apply rowStable_nativeLoop)
        else if shape == ``letNativeValue then
          `(tactic| first
            | apply rowStable_letNativeValue
            | apply rowStable_letConstructorOne)
        else throwError "leaner_row_stable_step: no rule for {shape}"
      else if head == ``RowStableValues then
        if shape == ``valuesNil then `(tactic| exact rowStableValues_nil)
        else if shape == ``valuesCons then `(tactic| apply rowStableValues_cons)
        else throwError "leaner_row_stable_step: no rule for {shape}"
      else if head == ``RowStableStatements then
        if shape == ``statementsNil then `(tactic| exact rowStableStatements_nil)
        else if shape == ``statementsCons then `(tactic| apply rowStableStatements_cons)
        else throwError "leaner_row_stable_step: no rule for {shape}"
      else if head == ``EvaluatorRowStable then
        if shape == ``ReferenceLocationOperation.evaluate? then
          `(tactic| first
            | exact evaluatorRowStable_dereference
            | exact evaluatorRowStable_mutate)
        else if shape == ``liftPrimitiveEvaluator then
          `(tactic| apply evaluatorRowStable_liftPrimitive)
        else if shape == ``NominalConstructor.evaluate? then
          `(tactic| exact evaluatorRowStable_nominalConstructor _)
        else if shape == ``liftConstructorEvaluator ||
            shape == ``NominalFieldLocation.evaluateSelect? then
          `(tactic| exact evaluatorRowStable_liftConstructor _)
        else if shape == ``PrimitiveLocationOperation.evaluate? then
          /- The resolved primitives lift a pure evaluator; the copy is its
          own rule. -/
          `(tactic| first
            | apply evaluatorRowStable_liftPrimitive
            | exact evaluatorRowStable_copyValue _)
        else if shape == ``GlobalLocationOperation.evaluate? then
          /- The operation's constructor picks the law: a failed `exact`
          unfolds both evaluators before it fails. -/
          let some operation := argument.appArg!.getAppFn.constName?
            | throwError "leaner_row_stable_step: no rule for this global operation"
          if operation == ``GlobalLocationOperation.contains then
            `(tactic| exact evaluatorRowStable_containsGlobal _ _)
          else if operation == ``GlobalLocationOperation.take then
            `(tactic| exact evaluatorRowStable_takeGlobal _ _)
          else if operation == ``GlobalLocationOperation.borrow then
            `(tactic| (apply evaluatorRowStable_globalBorrowShared; rfl))
          else throwError "leaner_row_stable_step: no rule for {operation}"
        else throwError "leaner_row_stable_step: no rule for {shape}"
      else throwError "leaner_row_stable_step: not a stability goal"
  Lean.Elab.Tactic.evalTactic rule

syntax "leaner_row_stable" : tactic

macro_rules
  | `(tactic| leaner_row_stable) => `(tactic| repeat' leaner_row_stable_step)

/-- Whether this goal is a stability side condition. -/
private def stabilityHead : Lean.Name → Bool
  | ``RowStable | ``RowStableValues | ``RowStableStatements
  | ``EvaluatorRowStable => true
  | _ => false

/-- Close a stability side condition, deciding syntactically first that the
goal is one — the closer is a search over a dozen lemmas, and paying it at
every program point for goals it cannot close is the drive tax again. -/
elab "leaner_row_stable_guarded" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← goal.withContext do
    Lean.instantiateMVars (← goal.getType)
  let isStability := match target.getAppFn.constName? with
    | some name => stabilityHead name
    | none => false
  unless isStability do
    Lean.Meta.throwTacticEx `leaner_row_stable_guarded goal
      "not a stability side condition"
  Lean.Elab.Tactic.evalTactic (← `(tactic| leaner_row_stable))

/-- Apply the one rule the goal's head names.  A `first` chain over the
rules paid a failed unification per rule per program point — unfolding
`wpRow` against a pi and comparing bodies with metavariables — where the
answer is syntactic: the combinator at the head of the denotation.  Every
rule is applied with `refine`, never `apply` or `rw`: a rewrite searches
the whole goal for its pattern, and `apply` unfolds `wpRow` to count
binders and then unifies inside it, which against a concrete continuation
is a higher-order pattern that fails or misassigns. -/
elab "leaner_row_dispatch" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← goal.withContext do Lean.instantiateMVars (← goal.getType)
  let denotation ←
    if target.isAppOfArity ``wpRow 5 then pure (target.getArg! 0)
    else if target.isAppOfArity ``wpStatementsRow 5 then pure (target.getArg! 0)
    else if target.isAppOfArity ``wpValuesRow 5 then pure (target.getArg! 0)
    else throwError "leaner_row_dispatch: not a row goal"
  let some head := denotation.getAppFn.constName?
    | throwError "leaner_row_dispatch: no rule for this goal"
  let rule ←
    if head == ``blockUnit then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_blockUnit _ _ _ _ _ ?_)
    else if head == ``blockResult then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_blockResult _ _ _ _ _ _ ?_ ?_)
    else if head == ``statementsCons then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpStatementsRow_cons _ _ _ _ _ _ ?_ ?_)
    else if head == ``statementsNil then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpStatementsRow_nil _ _ _ _).mpr ?_)
    else if head == ``nativeOperation || head == ``nativeReferenceOperation ||
        head == ``nativePrimitiveOperation || head == ``nativeLocalOperation ||
        head == ``nativeDerefLocalBorrowOperation || head == ``nativeGlobalOperation then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_nativeOperation _ _ _ _ _ _ ?_ ?_)
    else if head == ``valuesCons then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpValuesRow_cons _ _ _ _ _ _ ?_ ?_ ?_)
    else if head == ``valuesNil then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpValuesRow_nil _ _ _ _).mpr ?_)
    else if head == ``value then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpRow_value _ _ _ _ _).mpr ?_)
    else if head == ``localVar then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpRow_localVar _ _ _ _ _).mpr ?_)
    else if head == ``nativeAssignLocal then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_nativeAssignLocal _ _ _ _ _ _ ?_ ?_)
    else if head == ``nativeBranch then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_nativeBranch _ _ _ _ _ _ _ ?_ ?_)
    else if head == ``nativeThrow then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_nativeThrow _ _ _ _ _ _ ?_ ?_)
    else if head == ``nativeSpec then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpRow_nativeSpec _ _ _ _).mpr ?_)
    else if head == ``nativeBreak then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpRow_nativeBreakUnit _ _ _ _ _).mpr ?_)
    else if head == ``nativeContinue then
      `(tactic| refine (LeanerIR.Proofs.Denotation.wpRow_nativeContinue _ _ _ _ _).mpr ?_)
    else if head == ``nativeLoop then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_nativeLoop_sealed _ _ _ _ _ _ ?_)
    else if head == ``letNativeValue then
      `(tactic| refine LeanerIR.Proofs.Denotation.wpRow_letNativeValue _ _ _ _ _ _ _ ?_ ?_)
    else throwError "leaner_row_dispatch: no rule for {head}"
  Lean.Elab.Tactic.evalTactic rule

/-- An assignment's side condition: the written slot is in bounds at the
literal row, and the written row normalizes.  Decided syntactically — the
condition is an arrow whose domain compares an index with a row's size —
so no other binder is opened here. -/
elab "leaner_row_assign" : tactic => do
  let goal ← Lean.Elab.Tactic.getMainGoal
  let target ← goal.withContext do Lean.instantiateMVars (← goal.getType)
  let isBounds := target.isArrow &&
    target.bindingDomain!.isAppOfArity ``LT.lt 4 &&
    (target.bindingDomain!.getArg! 3).isAppOfArity ``Array.size 2
  unless isBounds do
    Lean.Meta.throwTacticEx `leaner_row_assign goal "not an assignment's bounds"
  Lean.Elab.Tactic.evalTactic (← `(tactic| (intro _; simp only [Array.set!_eq_setIfInBounds,
    List.setIfInBounds_toArray, List.set_cons_succ, List.set_cons_zero])))

/-- Walk a body's weakest precondition frame-free. -/
syntax "leaner_row_drive" : tactic

macro_rules
  | `(tactic| leaner_row_drive) =>
      `(tactic|
        repeat' first
          | leaner_row_dispatch
          /- The rules hand their continuation a `match` on the control the
          step produced; reducing it is what exposes the next combinator. -/
          | leaner_row_head
          | (leaner_row_stable_guarded; done)
          | leaner_row_assign
          /- A local read hands its value through a binder; opening it is
          what exposes the next combinator. -/
          | intro _ _)

/-! ## Modular call boundary

A call in a caller's body carries the callee as a relation, and the frame
route resolves it by unfolding that relation — re-executing the callee
symbolically at every call site.  The verified callee already proved its
contract once; this law lets the caller consume that theorem instead: what
the callee's relation can step to is bounded by what its contract permits.
The `Spec` boundary `nativeFunction` and the relation
`nativeFunctionRelation` are the same tuple read at two types, so the
callee's `Satisfies` fact applies to the relation definitionally. -/

/-- What a verified callee's relation can reach, read off its contract. -/
theorem wpFunction_of_satisfies {unit : ExecutableUnit}
    {shape : SemanticOperations.FunctionShape} {body : ExprDenotation}
    {contract : FunctionContract}
    (verified : Satisfies (nativeFunction unit shape body) contract)
    {arguments : Array RuntimeValue} {state : RuntimeState}
    (permitted : contract.requires arguments state)
    {post : RuntimeState → Outcome → Prop}
    (onReturn : ∀ results final,
      (¬contract.mayAbort arguments state →
        contract.ensures arguments state results final) →
      contract.frame arguments state final →
      ¬contract.mustAbort arguments state →
      post final (.returned results))
    (onThrow : ∀ kind thrown final,
      contract.aborts arguments state (kind, thrown) →
      post final (.threw kind thrown)) :
    wpFunction (nativeFunctionRelation unit shape body) state arguments
      post := by
  unfold wpFunction
  intro final outcome step
  obtain ⟨normal, failing, _⟩ := verified arguments state permitted
  cases outcome with
  | returned results =>
      have ok : (nativeFunction unit shape body arguments).ok state results
          final := step
      exact onReturn results final (normal results final ok).1
        (normal results final ok).2.1 (normal results final ok).2.2
  | threw kind thrown =>
      obtain ⟨frame, finalFrame, evaluatedState, control, entry, run, finish,
        finalize⟩ := step
      exact onThrow kind thrown final (failing (kind, thrown)
        ⟨frame, finalFrame, evaluatedState, control, final, entry, run, finish,
          finalize⟩)

end LeanerIR.Proofs.Denotation
