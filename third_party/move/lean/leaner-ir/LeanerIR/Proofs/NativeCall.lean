-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Native

/-!
# Modular calls over the frame-free row

The frame route resolves a call by unfolding the callee's relation —
symbolic re-execution at every call site.  Here the caller consumes the
callee's proved contract instead, over the row representation.

A call statement is not `RowStable`: on the abort path the callee's final
state is unconstrained by its contract, so the caller frame reconciled
against it is arbitrary.  It does not need to be — the contract's abort
clause reads only the entry state and the failure, so the caller's
obligation on a thrown outcome is uniform in whatever frame and state the
abort path produced.  The spine below therefore carries two
postconditions: a row postcondition for value-and-return steps, which
carries the reached row itself — a call statement has no structural
stability to lean on, only its contract makes the value path a row again —
and a frame-free, state-free postcondition for throwing steps, row-framed
or not.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR.SemanticOperations

/-! ## The throw-aware weakest precondition -/

/-- Weakest precondition over the locals, with throwing steps carried by a
postcondition that reads neither the frame nor the state they left. -/
def wpRowThrow (denotation : ExprDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop) : Prop :=
  ∀ finalFrame finalState control,
    denotation (rowFrame row registries) state finalFrame finalState
        control →
      match control with
      | .throw_ kind thrown => postThrow kind thrown
      | _ =>
          ∃ finalRow finalRegistries,
            finalFrame = rowFrame finalRow finalRegistries ∧
            postValue finalRow finalRegistries finalState control

/-- A `RowStable` segment embeds: its throwing steps are row-framed, and
`wpRow` already covers them. -/
theorem wpRowThrow_of_wpRow {denotation : ExprDenotation} {row : Row}
    {registries : Registries} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (stable : RowStable denotation)
    (rowWp : wpRow denotation row registries state
      fun finalRow finalState control =>
        match control with
        | .throw_ kind thrown => postThrow kind thrown
        | _ => postValue finalRow registries finalState control) :
    wpRowThrow denotation row registries state postValue postThrow := by
  intro finalFrame finalState control step
  obtain ⟨finalRow, rfl⟩ :=
    stable row registries state finalFrame finalState _ step
  cases control with
  | throw_ kind thrown => exact rowWp finalRow finalState _ step
  | value runtimeValue =>
      exact ⟨finalRow, registries, rfl, rowWp finalRow finalState _ step⟩
  | return_ values =>
      exact ⟨finalRow, registries, rfl, rowWp finalRow finalState _ step⟩
  | break_ label =>
      exact ⟨finalRow, registries, rfl, rowWp finalRow finalState _ step⟩
  | continue_ label =>
      exact ⟨finalRow, registries, rfl, rowWp finalRow finalState _ step⟩

/-! ## The throw-aware statement spine -/

/-- Weakest precondition of a statement row whose throwing results are
carried frame-free and state-free. -/
def wpStatementsRowThrow (denotation : StatementsDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (postDone : Row → Registries → RuntimeState → Prop)
    (postControl : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop) : Prop :=
  ∀ result, denotation (rowFrame row registries) state result →
    match result with
    | .control _ _ (.throw_ kind thrown) => postThrow kind thrown
    | .control resultState resultFrame control =>
        ∃ resultRow resultRegistries,
          resultFrame = rowFrame resultRow resultRegistries ∧
          postControl resultRow resultRegistries resultState control
    | .done resultState resultFrame =>
        ∃ resultRow resultRegistries,
          resultFrame = rowFrame resultRow resultRegistries ∧
          postDone resultRow resultRegistries resultState

theorem wpStatementsRowThrow_nil (row : Row) (registries : Registries)
    (state : RuntimeState) (postDone : Row → Registries → RuntimeState → Prop)
    (postControl : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (done : postDone row registries state) :
    wpStatementsRowThrow statementsNil row registries state postDone
      postControl postThrow := by
  rintro result rfl
  exact ⟨row, registries, rfl, done⟩

theorem wpStatementsRowThrow_cons (head : ExprDenotation)
    (tail : StatementsDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState) (postDone : Row → Registries → RuntimeState → Prop)
    (postControl : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (stepped : wpRowThrow head row registries state
      (fun headRow headRegistries headState control =>
        match control with
        | .value _ =>
            wpStatementsRowThrow tail headRow headRegistries headState
              postDone postControl postThrow
        | _ => postControl headRow headRegistries headState control)
      postThrow) :
    wpStatementsRowThrow (statementsCons head tail) row registries state
      postDone postControl postThrow := by
  rintro result step
  rcases step with ⟨finalFrame, finalState, raised, headStep, abrupt, rfl⟩ |
    ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩
  · have applied := stepped finalFrame finalState raised headStep
    cases raised with
    | throw_ kind thrown => exact applied
    | value runtimeValue => exact absurd abrupt (by rintro ⟨⟩)
    | return_ values => exact applied
    | break_ label => exact applied
    | continue_ label => exact applied
  · obtain ⟨headRow, headRegistries, rfl, continuation⟩ :=
      stepped headFrame headState (.value runtimeValue) headStep
    exact continuation result tailStep

/-! ## Block and entry bridges -/

/-- A unit block over the throw-aware spine. -/
theorem wpRowThrow_blockUnit (statements : StatementsDenotation) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (stepped : wpStatementsRowThrow statements row registries state
      (fun doneRow doneRegistries doneState =>
        postValue doneRow doneRegistries doneState (.value .unit))
      (fun controlRow controlRegistries controlState control =>
        postValue controlRow controlRegistries controlState control)
      postThrow) :
    wpRowThrow (blockUnit statements) row registries state postValue
      postThrow := by
  intro finalFrame finalState control step
  rcases step with abrupt | ⟨done, rfl⟩
  · have applied := stepped (.control finalState finalFrame control) abrupt
    cases control with
    | throw_ kind thrown => exact applied
    | value runtimeValue => exact applied
    | return_ values => exact applied
    | break_ label => exact applied
    | continue_ label => exact applied
  · exact stepped (.done finalState finalFrame) done

/-- A result block over the throw-aware spine: the statements run first,
then the result expression from the row they left.  The result segment
is stated over the throw-aware form too, so a `RowStable` scalar segment
embeds through `wpRowThrow_of_wpRow` and continues with the ordinary
row drive. -/
theorem wpRowThrow_blockResult (statements : StatementsDenotation)
    (result : ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (stepped : wpStatementsRowThrow statements row registries state
      (fun doneRow doneRegistries doneState =>
        wpRowThrow result doneRow doneRegistries doneState postValue
          postThrow)
      (fun controlRow controlRegistries controlState control =>
        postValue controlRow controlRegistries controlState control)
      postThrow) :
    wpRowThrow (blockResult statements result) row registries state
      postValue postThrow := by
  intro finalFrame finalState control step
  rcases step with abrupt | ⟨statementFrame, statementState, done, resultStep⟩
  · have applied := stepped (.control finalState finalFrame control) abrupt
    cases control with
    | throw_ kind thrown => exact applied
    | value runtimeValue => exact applied
    | return_ values => exact applied
    | break_ label => exact applied
    | continue_ label => exact applied
  · obtain ⟨doneRow, doneRegistries, rfl, continuation⟩ :=
      stepped (.done statementState statementFrame) done
    exact continuation finalFrame finalState control resultStep

/-- The entry rule for a throw-aware body.  The throw obligation
quantifies over the frame and state the abort path left, which is sound to
demand of the generated caller post because a contract's abort clause
reads only the entry state and the failure. -/
theorem nativeEntryThrow_rowFrame {shape : FunctionShape}
    {body : ExprDenotation} {arguments : Array RuntimeValue}
    {initial : RuntimeState}
    {post : RuntimeFrame → RuntimeState → Control → Prop}
    (arity : arguments.size = shape.parameterCount)
    (declared : arguments.size ≤ shape.localCount)
    (rowWp : wpRowThrow body (initialLocals shape.localCount arguments)
        { activeLoans := #[]
          loanLocations := parameterLoanLocations arguments }
        initial
        (fun finalRow finalRegistries finalState control =>
          post (rowFrame finalRow finalRegistries) finalState control)
        (fun kind thrown =>
          ∀ finalFrame finalState,
            post finalFrame finalState (.throw_ kind thrown))) :
    ∀ frame, nativeInitialFrame? shape arguments = some frame →
      wpExpr body frame initial post := by
  intro frame entry
  rw [nativeInitialFrame?_rowFrame shape arguments arity declared] at entry
  cases Option.some.inj entry
  unfold wpExpr
  intro finalFrame finalState control step
  have applied := rowWp finalFrame finalState control step
  cases control with
  | throw_ kind thrown => exact applied finalFrame finalState
  | value runtimeValue =>
      obtain ⟨finalRow, finalRegistries, rfl, established⟩ := applied
      exact established
  | return_ values =>
      obtain ⟨finalRow, finalRegistries, rfl, established⟩ := applied
      exact established
  | break_ label =>
      obtain ⟨finalRow, finalRegistries, rfl, established⟩ := applied
      exact established
  | continue_ label =>
      obtain ⟨finalRow, finalRegistries, rfl, established⟩ := applied
      exact established

/-! ## The reborrow operand, computed -/

/-- The reborrow evaluator on the single-borrow row: read the referent,
mint the dynamic loan, leave the hole in the lender, register the loan and
its location.  Everything here is what the *interpreter* needs; the
composed call law below is the only consumer, and it retires all of it
before its continuation sees a row again. -/
theorem derefLocalBorrow_evaluate_singleBorrow
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (lex outer : Nat) (current : RuntimeValue) (state : RuntimeState)
    (rest : List (Option RuntimeValue)) :
    (({ location := ⟨⟨0⟩⟩, fields := [], referenceType,
        kind := .mutable, lexicalLoan := lex } :
      DerefLocalBorrowOperation)).evaluate? #[]
      (rowFrame (some (.borrow outer current) :: rest).toArray
        { activeLoans := #[]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }) state =
    some (.value
      { locals := (some (.borrow outer (.loanHole state.nextLoan)) :: rest).toArray
        activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan current)) := by
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    DerefLocalBorrowOperation.resolve?, resolveDerefLocalFieldPath?,
    resolveNominalFieldSteps?, readLocal?, borrowRuntimePlaceAt?,
    readRuntimePlace?, readRoot?, readProjections?, writeRuntimePlace?,
    writeRoot?, writeProjections?, rowFrame, mutableKind, Array.filter,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-- The write-back a returning callee exported through the reborrow's
loan, applied to the caller's frame. -/
theorem applyPendingWriteBack_derefLocalZero (state : RuntimeState)
    (outer loan : Nat) (replacement : RuntimeValue)
    (activeLoans : Array (ExprId × Nat))
    (rest : List (Option RuntimeValue)) :
    applyPendingWriteBack
      { locals := (some (.borrow outer (.loanHole loan)) :: rest).toArray
        activeLoans
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
          (loan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
      state loan replacement =
    ({ locals := (some (.borrow outer replacement) :: rest).toArray
       activeLoans := transferActiveLoan activeLoans loan replacement
       loanLocations := transferLoanLocation
         #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
           (loan, ⟨.local ⟨0⟩, #[.deref], true⟩)] loan replacement },
     state) := by
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?]

/-- The lexical death marker after a scalar write-back already retired the
dynamic loan: both transfer rows collapse and the marker passes its single
argument through. -/
theorem endLoan_evaluate_retiredDerefLocalZero
    (lexical outer loan : Nat) (value : Int)
    (locals : Array (Option RuntimeValue)) (argument : RuntimeValue)
    (state : RuntimeState) (separate : outer ≠ loan) :
    (ReferenceLocationOperation.endLoan #[⟨lexical⟩]).evaluate? #[argument]
      { locals
        activeLoans := transferActiveLoan #[(⟨lexical⟩, loan)] loan
          (.integer value)
        loanLocations := transferLoanLocation
          #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨0⟩, #[.deref], true⟩)] loan (.integer value) }
      state =
    some (.value
      { locals
        activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state argument) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, endLoans?,
    transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    Array.filter, separate]

/-! ## The composed call statement

`core.call f(&mut *slot)` is one statement: the reborrow mints a loan and
registers it, the callee's export lands on `pending`, the write-back fills
it through the registered location, and the death marker retires the
lexical row.  Registries and `pending` return to their entry values and
the row moves `borrow loan cur → borrow loan new` — the prophetic thesis
at the call boundary, packaged as a single fixed-registries law. -/

/-- A returned reborrow of the sole parameter, as a whole body: the loan is
minted, its hole rests in the parameter, and the borrow is the value.  The
exit is the script's, at the row and registries the mint leaves. -/
theorem wpRowThrow_returnedReborrow (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex outer : Nat)
    (current : RuntimeValue) (state : RuntimeState)
    (rest : List (Option RuntimeValue))
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (exit : postValue
      (some (.borrow outer (.loanHole state.nextLoan)) :: rest).toArray
      { activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan current))) :
    wpRowThrow
      (nativeDerefLocalBorrowOperation
        { location := ⟨⟨0⟩⟩, fields := [], referenceType, kind := .mutable,
          lexicalLoan := lex }
        valuesNil)
      (some (.borrow outer current) :: rest).toArray
      { activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with h1 h2 h3
    subst oS oF values
    rcases evaluated with ⟨rv, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [derefLocalBorrow_evaluate_singleBorrow referenceType mutableKind lex
        outer current state rest] at evaluation <;>
      cases Option.some.inj evaluation
    exact ⟨_, ⟨#[(⟨lex⟩, state.nextLoan)],
      #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
        (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)], #[]⟩, rfl, exit⟩

/-- Every step of the call statement, characterized: the callee threw and
the caller's throw obligation is discharged, or it returned, the loan
retired, and the caller stands at the moved row with its continuation
established.  Both the wp form and the statement-spine rule read off
this; nothing weaker sequences, because only the callee's contract — not
any structural stability — makes the value path a row again. -/
theorem callReborrowStatement_step
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    {outer : Nat} {cur : Int} {state : RuntimeState}
    {rest : List (Option RuntimeValue)}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (priorLoan : outer < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan (.integer cur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ newValue : Int,
              calleeFinal.pending =
                state.pending.push (state.nextLoan, .integer newValue) ∧
              postValue (some (.borrow outer (.integer newValue)) :: rest).toArray
                { activeLoans := #[]
                  loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (packResults results))
        | .threw kind thrown => postThrow kind thrown)) :
    ∀ {finalFrame : RuntimeFrame} {finalState : RuntimeState} {control : Control},
      (nativeReferenceOperation (.endLoan #[⟨lex⟩])
        (valuesCons
          (nativeCall handle none
            (nativeFunctionRelation calleeUnit calleeShape calleeBody)
            (valuesCons
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨0⟩⟩, fields := []
                  referenceType, kind := .mutable, lexicalLoan := lex }
                valuesNil)
              valuesNil))
          valuesNil))
        (rowFrame (some (.borrow outer (.integer cur)) :: rest).toArray
          { activeLoans := #[]
            loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] })
        state finalFrame finalState control →
      (∃ kind thrown, control = .throw_ kind thrown ∧ postThrow kind thrown) ∨
      ∃ (results : Array RuntimeValue) (newValue : Int),
        control = .value (packResults results) ∧
        finalFrame = rowFrame (some (.borrow outer (.integer newValue)) :: rest).toArray
          { activeLoans := #[]
            loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] } ∧
        postValue (some (.borrow outer (.integer newValue)) :: rest).toArray
          { activeLoans := #[]
            loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
          finalState (.value (packResults results)) := by
  have separate : outer ≠ state.nextLoan := Nat.ne_of_lt priorLoan
  unfold wpFunction at calleeWp
  intro finalFrame finalState control step
  /- The reborrow operand is deterministic; compute it once. -/
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨0⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lex }
          valuesNil)
          (rowFrame (some (.borrow outer (.integer cur)) :: rest).toArray
            { activeLoans := #[]
              loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] })
          state rF rS c →
        rF = { locals := (some (.borrow outer (.loanHole state.nextLoan)) :: rest).toArray
               activeLoans := #[(⟨lex⟩, state.nextLoan)]
               loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
                 (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] } ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan (.integer cur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_singleBorrow referenceType
          mutableKind lex outer (.integer cur) state rest] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  /- The call operand: either the callee threw and the control propagates,
  or it returned and the write-back plus death marker retire the loan. -/
  have callRun :
      ∀ {cF : RuntimeFrame} {cS : RuntimeState} {c : Control},
        (nativeCall handle none
          (nativeFunctionRelation calleeUnit calleeShape calleeBody)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨0⟩⟩, fields := []
                referenceType, kind := .mutable, lexicalLoan := lex }
              valuesNil)
            valuesNil))
          (rowFrame (some (.borrow outer (.integer cur)) :: rest).toArray
            { activeLoans := #[]
              loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] })
          state cF cS c →
        (∃ kind thrown, c = .throw_ kind thrown ∧ postThrow kind thrown) ∨
        ∃ (results : Array RuntimeValue) (newValue : Int)
            (calleeFinal : RuntimeState),
          c = .value (packResults results) ∧
          cF = { locals := (some (.borrow outer (.integer newValue)) :: rest).toArray
                 activeLoans := transferActiveLoan
                   #[(⟨lex⟩, state.nextLoan)] state.nextLoan
                   (.integer newValue)
                 loanLocations := transferLoanLocation
                   #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
                     (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)]
                   state.nextLoan (.integer newValue) } ∧
          cS = { globals := calleeFinal.globals
                 globalLoans := calleeFinal.globalLoans
                 nextLoan := calleeFinal.nextLoan
                 pending := state.pending } ∧
          postValue (some (.borrow outer (.integer newValue)) :: rest).toArray
            { activeLoans := #[]
              loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
            { globals := calleeFinal.globals
              globalLoans := calleeFinal.globalLoans
              nextLoan := calleeFinal.nextLoan
              pending := state.pending }
            (.value (packResults results)) := by
    rintro cF cS c
      (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
        ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
          rfl, rfl, rfl⟩)
    · /- The reborrow cannot raise abrupt control. -/
      rcases operandControl with
        ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
      · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
      · simp [valuesNil] at nilStep
        obtain ⟨rfl, rfl, rfl⟩ := nilStep
        simp at resultEq
      · simp [valuesNil] at nilStep
    · /- Operand row evaluated: the reborrow's value fed the callee. -/
      rcases operandValues with
        ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
      · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
      · obtain ⟨rfl, rfl, veq⟩ := reborrowRun reborrowStep
        injection veq with veq
        subst veq
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        have applied := calleeWp calleeState outcome (by
          simpa using calleeStep)
        cases outcome with
        | threw kind thrown =>
            exact .inl ⟨kind, thrown, rfl, applied⟩
        | returned results =>
            obtain ⟨newValue, pendingShape, continuation⟩ := applied
            refine .inr ⟨results, newValue, calleeState, rfl, ?_, ?_,
              continuation⟩
            · rw [callFrame_returned]
              simp only [registerReturnedLoan]
              rw [applyPendingFrom_single (inherited := state.pending)
                (loan := state.nextLoan)
                (current := .integer newValue) pendingShape]
              rw [applyPendingWriteBack_derefLocalZero]
            · rw [applyPendingFrom_single (inherited := state.pending)
                (loan := state.nextLoan)
                (current := .integer newValue) pendingShape]
              rw [applyPendingWriteBack_derefLocalZero]
      · obtain ⟨rfl, rfl, -⟩ := reborrowRun reborrowStep
        simp [valuesNil] at nilStep
  /- The whole statement: endLoan over the call's operand row. -/
  rcases step with
    ⟨oF, oS, propagated, operandsStep, rfl, rfl, rfl⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · /- Abrupt operand: only the callee's throw. -/
    rcases operandsStep with
      ⟨cF, cS, cc, callStep, abrupt, resultEq⟩ |
      ⟨cF, cS, cv, f, s, vs, callStep, nilStep, resultEq⟩ |
      ⟨cF, cS, cv, f, s, cc, callStep, nilStep, resultEq⟩
    · rcases callRun callStep with ⟨kind, thrown, rfl, thrown_post⟩ |
        ⟨results, newValue, calleeFinal, rfl, -, -, -⟩
      · injection resultEq with h1 h2 h3
        subst h1 h2 h3
        exact .inl ⟨kind, thrown, rfl, thrown_post⟩
      · cases abrupt
    · simp [valuesNil] at nilStep
      obtain ⟨rfl, rfl, rfl⟩ := nilStep
      simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨cF, cS, cc, callStep, abrupt, resultEq⟩ |
      ⟨cF, cS, cv, f, s, vs, callStep, nilStep, resultEq⟩ |
      ⟨cF, cS, cv, f, s, cc, callStep, nilStep, resultEq⟩
    · simp at resultEq
    · rcases callRun callStep with ⟨kind, thrown, absurdEq, -⟩ |
        ⟨results, newValue, calleeFinal, ceq, feq, seq, continuation⟩
      · cases absurdEq
      · injection ceq with ceq
        subst ceq
        subst feq seq
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (ReferenceLocationOperation.endLoan
              #[(⟨lex⟩ : LoanId)]).evaluate?
              [packResults results].toArray _ _ = _ from
            endLoan_evaluate_retiredDerefLocalZero lex outer state.nextLoan
              newValue (some (.borrow outer (.integer newValue)) :: rest).toArray
              (packResults results) _ separate] at evaluation <;>
          cases Option.some.inj evaluation
        exact .inr ⟨results, newValue, rfl, rfl, continuation⟩
    · simp [valuesNil] at nilStep

/-- The statement-spine rule for a call statement: the callee's contract
carries the spine forward.  On return the tail continues from the moved
row; on abort the caller's throw obligation is already discharged. -/
theorem wpStatementsRowThrow_consCallReborrow
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    {outer : Nat} {cur : Int} {state : RuntimeState}
    {rest : List (Option RuntimeValue)}
    (tail : StatementsDenotation)
    {postDone : Row → Registries → RuntimeState → Prop}
    {postControl : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (priorLoan : outer < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan (.integer cur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned _ =>
            ∃ newValue : Int,
              calleeFinal.pending =
                state.pending.push (state.nextLoan, .integer newValue) ∧
              wpStatementsRowThrow tail
                (some (.borrow outer (.integer newValue)) :: rest).toArray
                { activeLoans := #[]
                  loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                postDone postControl postThrow
        | .threw kind thrown => postThrow kind thrown)) :
    wpStatementsRowThrow
      (statementsCons
        (nativeReferenceOperation (.endLoan #[⟨lex⟩])
          (valuesCons
            (nativeCall handle none
              (nativeFunctionRelation calleeUnit calleeShape calleeBody)
              (valuesCons
                (nativeDerefLocalBorrowOperation
                  { location := ⟨⟨0⟩⟩, fields := []
                    referenceType, kind := .mutable, lexicalLoan := lex }
                  valuesNil)
                valuesNil))
            valuesNil))
        tail)
      (some (.borrow outer (.integer cur)) :: rest).toArray
      { activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postDone postControl postThrow := by
  rintro result
    (⟨finalFrame, finalState, raised, headStep, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩)
  · rcases callReborrowStatement_step
        (postValue := fun continuationRow continuationRegistries
            continuationState _ =>
          wpStatementsRowThrow tail continuationRow continuationRegistries
            continuationState postDone postControl postThrow)
        handle lex referenceType mutableKind priorLoan calleeWp headStep with
      ⟨kind, thrown, rfl, thrown_post⟩ |
      ⟨results, newValue, rfl, -, -⟩
    · exact thrown_post
    · cases abrupt
  · rcases callReborrowStatement_step
        (postValue := fun continuationRow continuationRegistries
            continuationState _ =>
          wpStatementsRowThrow tail continuationRow continuationRegistries
            continuationState postDone postControl postThrow)
        handle lex referenceType mutableKind priorLoan calleeWp headStep with
      ⟨kind, thrown, absurdEq, -⟩ |
      ⟨results, newValue, valueEq, rfl, continuation⟩
    · cases absurdEq
    · exact continuation result tailStep

/-- A reborrow call statement in result position — the last expression of
an arm block — read off the expression itself: the callee's throw is the
caller's, and the value path continues from the row the callee's exported
value left, with the call's packed results as the value. -/
theorem wpRowThrow_callReborrowStatement
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    {outer : Nat} {cur : Int} {state : RuntimeState}
    {rest : List (Option RuntimeValue)}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (priorLoan : outer < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan (.integer cur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ newValue : Int,
              calleeFinal.pending =
                state.pending.push (state.nextLoan, .integer newValue) ∧
              postValue (some (.borrow outer (.integer newValue)) :: rest).toArray
                { activeLoans := #[]
                  loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (packResults results))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeReferenceOperation (.endLoan #[⟨lex⟩])
        (valuesCons
          (nativeCall handle none
            (nativeFunctionRelation calleeUnit calleeShape calleeBody)
            (valuesCons
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨0⟩⟩, fields := []
                  referenceType, kind := .mutable, lexicalLoan := lex }
                valuesNil)
              valuesNil))
          valuesNil))
      (some (.borrow outer (.integer cur)) :: rest).toArray
      { activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  rcases callReborrowStatement_step handle lex referenceType mutableKind priorLoan
      calleeWp step with
    ⟨kind, thrown, rfl, thrown_post⟩ | ⟨results, newValue, rfl, rfl, continuation⟩
  · exact thrown_post
  · exact ⟨_, _, rfl, continuation⟩

/-- A place evaluator never throws. -/
theorem liftPlaceEvaluator_ne_throw
    (evaluate : Array RuntimeValue → RuntimeFrame → RuntimeState →
      Option (RuntimeFrame × RuntimeState × RuntimeValue))
    (arguments : Array RuntimeValue) (frame finalFrame : RuntimeFrame)
    (state finalState : RuntimeState) (kind : ThrowKind)
    (thrown : Array RuntimeValue) :
    liftPlaceEvaluator evaluate arguments frame state ≠
      some (.throw_ finalFrame finalState kind thrown) := by
  unfold liftPlaceEvaluator
  cases evaluate arguments frame state with
  | none => simp
  | some result => simp

/-! ## Control on the throw-aware spine

A storage body's inner block may retire its loans and throw from a failing
arm: the throw leaves the bracket's registries, which `RowStable` cannot
name.  The rules below are the segment's control rules with their throwing
steps carried frame-free, so the arm needs no stability. -/

/-- A branch whose arms are throw-aware: the condition is stable and its
boolean selects the arm. -/
theorem wpRowThrow_nativeBranch (condition thenBranch : ExprDenotation)
    (elseBranch : Option ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (conditionStable : RowStable condition)
    (stepped : wpRow condition row registries state
      fun conditionRow conditionState control =>
        match control with
        | .value (.bool true) =>
            wpRowThrow thenBranch conditionRow registries conditionState
              postValue postThrow
        | .value (.bool false) =>
            match elseBranch with
            | some branch =>
                wpRowThrow branch conditionRow registries conditionState
                  postValue postThrow
            | none => postValue conditionRow registries conditionState (.value .unit)
        | .value _ => True
        | .throw_ kind thrown => postThrow kind thrown
        | _ => postValue conditionRow registries conditionState control) :
    wpRowThrow (nativeBranch condition thenBranch elseBranch) row registries
      state postValue postThrow := by
  rintro finalFrame finalState control step
  rcases step with ⟨conditionStep, abrupt⟩ |
    ⟨conditionFrame, conditionState, conditionStep, armStep⟩ |
    ⟨conditionFrame, conditionState, conditionStep, elseStep⟩
  · obtain ⟨finalRow, rfl⟩ :=
      conditionStable _ _ _ _ _ _ conditionStep
    have applied := stepped finalRow finalState control conditionStep
    cases control with
    | throw_ kind thrown => exact applied
    | value produced => cases abrupt
    | return_ values => exact ⟨finalRow, registries, rfl, applied⟩
    | break_ label => exact ⟨finalRow, registries, rfl, applied⟩
    | continue_ label => exact ⟨finalRow, registries, rfl, applied⟩
  · obtain ⟨conditionRow, rfl⟩ :=
      conditionStable _ _ _ _ _ _ conditionStep
    exact stepped conditionRow conditionState _ conditionStep
      finalFrame finalState control armStep
  · obtain ⟨conditionRow, rfl⟩ :=
      conditionStable _ _ _ _ _ _ conditionStep
    have applied := stepped conditionRow conditionState _ conditionStep
    cases elseBranch with
    | some branch => exact applied finalFrame finalState control elseStep
    | none =>
        obtain ⟨rfl, rfl, rfl⟩ := elseStep
        exact ⟨conditionRow, registries, rfl, applied⟩

/-- A `let` whose body is throw-aware: the initializer is stable, the
binding is left as a step over the row, the body runs on the spine. -/
theorem wpRowThrow_letNativeValue (binder : NativePatternBinder)
    (initializer body : ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (initializerStable : RowStable initializer)
    (stepped : wpRow initializer row registries state
      fun initializedRow initializedState control =>
        match control with
        | .value runtimeValue =>
            ∀ boundFrame,
              binder.bind (rowFrame initializedRow registries) runtimeValue =
                  some boundFrame →
                ∃ boundRow, boundFrame = rowFrame boundRow registries ∧
                  wpRowThrow body boundRow registries initializedState
                    postValue postThrow
        | .throw_ kind thrown => postThrow kind thrown
        | _ => postValue initializedRow registries initializedState control) :
    wpRowThrow (letNativeValue binder initializer body) row registries state
      postValue postThrow := by
  rintro finalFrame finalState control step
  rcases step with ⟨initStep, abrupt⟩ |
    ⟨initializedFrame, initializedState, runtimeValue, boundFrame, initStep,
      bindEq, bodyStep⟩
  · obtain ⟨finalRow, rfl⟩ := initializerStable _ _ _ _ _ _ initStep
    have applied := stepped finalRow finalState control initStep
    cases control with
    | throw_ kind thrown => exact applied
    | value produced => cases abrupt
    | return_ values => exact ⟨finalRow, registries, rfl, applied⟩
    | break_ label => exact ⟨finalRow, registries, rfl, applied⟩
    | continue_ label => exact ⟨finalRow, registries, rfl, applied⟩
  · obtain ⟨initializedRow, rfl⟩ := initializerStable _ _ _ _ _ _ initStep
    obtain ⟨boundRow, rfl, bodyWp⟩ :=
      stepped initializedRow initializedState _ initStep boundFrame bindEq
    exact bodyWp finalFrame finalState control bodyStep

/-- A throw whose one argument is a death marker over a literal: the
marker's exit row is left as a step, and the throw carries its result. -/
theorem wpRowThrow_nativeThrowEndLoan (kind : ThrowKind) (loans : Array LoanId)
    (thrown : RuntimeValue) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : ∀ finalFrame finalState retired,
      (ReferenceLocationOperation.endLoan loans).evaluate? #[thrown]
          (rowFrame row registries) state =
        some (.value finalFrame finalState retired) →
      postThrow kind #[retired]) :
    wpRowThrow
      (nativeThrow kind
        (valuesCons
          (nativeReferenceOperation (ReferenceLocationOperation.endLoan loans)
            (valuesCons (value thrown) valuesNil))
          valuesNil))
      row registries state postValue postThrow := by
  /- The marker over its literal operand: the only step is the closed
  evaluation, which never throws. -/
  have markerRun :
      ∀ {mF : RuntimeFrame} {mS : RuntimeState} {c : Control},
        (nativeReferenceOperation (ReferenceLocationOperation.endLoan loans)
          (valuesCons (value thrown) valuesNil))
          (rowFrame row registries) state mF mS c →
        ∃ retired,
          (ReferenceLocationOperation.endLoan loans).evaluate? #[thrown]
              (rowFrame row registries) state = some (.value mF mS retired) ∧
          c = .value retired := by
    rintro mF mS c
      (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
        ⟨oF, oS, values, operandStep, evaluated⟩)
    · rcases operandStep with
        ⟨rF, rS, rc, valueStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, valueStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, valueStep, nilStep, resultEq⟩
      · obtain ⟨rfl, rfl, rfl⟩ := valueStep
        cases abrupt
      · simp at resultEq
      · simp [valuesNil] at nilStep
    · rcases operandStep with
        ⟨rF, rS, rc, valueStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, valueStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, valueStep, nilStep, resultEq⟩
      · obtain ⟨rfl, rfl, rfl⟩ := valueStep
        cases abrupt
      · obtain ⟨rfl, rfl, valueEq⟩ := valueStep
        injection valueEq with valueEq
        subst rv
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind', thrown', evaluation, rfl⟩
        · exact ⟨rv, evaluation, rfl⟩
        · exact absurd evaluation
            (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)
      · simp [valuesNil] at nilStep
  rintro finalFrame finalState control
    (⟨values, valuesStep, rfl⟩ | controlStep)
  · rcases valuesStep with
      ⟨mF, mS, mc, markerStep, abrupt, resultEq⟩ |
      ⟨mF, mS, mv, f, s, vs, markerStep, nilStep, resultEq⟩ |
      ⟨mF, mS, mv, f, s, mc, markerStep, nilStep, resultEq⟩
    · simp at resultEq
    · obtain ⟨retired, evaluation, valueEq⟩ := markerRun markerStep
      injection valueEq with valueEq
      subst mv
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst finalState finalFrame values
      exact exit _ _ _ evaluation
    · simp [valuesNil] at nilStep
  · rcases controlStep with
      ⟨mF, mS, mc, markerStep, abrupt, resultEq⟩ |
      ⟨mF, mS, mv, f, s, vs, markerStep, nilStep, resultEq⟩ |
      ⟨mF, mS, mv, f, s, mc, markerStep, nilStep, resultEq⟩
    · obtain ⟨retired, -, rfl⟩ := markerRun markerStep
      cases abrupt
    · simp at resultEq
    · simp [valuesNil] at nilStep

/-! ## A callee returning a reborrow

The three caller shapes of a reference-returning callee.  The callee's
summary now states the transfer — its export is the returned loan's hole —
so the caller's write-back fills its parameter's hole with that hole and
retargets its lexical loan to the returned one; nothing about the callee's
body is consulted. -/

/-- Retargeting a lexical row to the loan a hole names. -/
theorem transferActiveLoan_hole (lex loan returned : Nat) :
    transferActiveLoan #[(⟨lex⟩, loan)] loan (.loanHole returned) =
      #[(⟨lex⟩, returned)] := by
  simp [transferActiveLoan, transferredLoan?, findFirst, Array.filter]

/-- Retargeting a registered location to the loan a hole names. -/
theorem transferLoanLocation_hole (outer loan returned : Nat)
    (place projected : RuntimePlace) (separate : outer ≠ loan) :
    transferLoanLocation #[(outer, place), (loan, projected)] loan
        (.loanHole returned) =
      #[(outer, place), (returned, projected)] := by
  simp [transferLoanLocation, transferredLoan?, findFirst, Array.filter,
    separate, Array.findRev?]

/-- The value of a call whose callee returns a reborrow of the argument
reborrow: the callee's export is the returned loan's hole, which the
caller's write-back fills into its parameter, retargeting the lexical loan
to the returned one. -/
theorem wpRowThrow_callReturnedReborrow
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    {outer : Nat} {cur : Int} {state : RuntimeState}
    {rest : List (Option RuntimeValue)}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (priorLoan : outer < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan (.integer cur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ (returned : Nat) (value : Int),
              results = #[.borrow returned (.integer value)] ∧
              calleeFinal.pending =
                state.pending.push (state.nextLoan, .loanHole returned) ∧
              postValue (some (.borrow outer (.loanHole returned)) :: rest).toArray
                { activeLoans := #[(⟨lex⟩, returned)]
                  loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
                    (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (.borrow returned (.integer value)))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeCall handle none
        (nativeFunctionRelation calleeUnit calleeShape calleeBody)
        (valuesCons
          (nativeDerefLocalBorrowOperation
            { location := ⟨⟨0⟩⟩, fields := []
              referenceType, kind := .mutable, lexicalLoan := lex }
            valuesNil)
          valuesNil))
      (some (.borrow outer (.integer cur)) :: rest).toArray
      { activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postValue postThrow := by
  have separate : outer ≠ state.nextLoan := Nat.ne_of_lt priorLoan
  unfold wpFunction at calleeWp
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨0⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lex }
          valuesNil)
          (rowFrame (some (.borrow outer (.integer cur)) :: rest).toArray
            { activeLoans := #[]
              loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] })
          state rF rS c →
        rF = { locals := (some (.borrow outer (.loanHole state.nextLoan)) :: rest).toArray
               activeLoans := #[(⟨lex⟩, state.nextLoan)]
               loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
                 (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] } ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan (.integer cur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_singleBorrow referenceType
          mutableKind lex outer (.integer cur) state rest] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
      ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
        rfl, rfl, rfl⟩)
  · rcases operandControl with
      ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
    · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
      cases abrupt
    · simp [valuesNil] at nilStep
      obtain ⟨rfl, rfl, rfl⟩ := nilStep
      simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandValues with
      ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
    · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
      cases abrupt
    · obtain ⟨rfl, rfl, veq⟩ := reborrowRun reborrowStep
      injection veq with veq
      subst veq
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      have applied := calleeWp calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown => exact applied
      | returned results =>
          obtain ⟨returned, value, rfl, pendingShape, continuation⟩ := applied
          have packOne : packResults #[.borrow returned (.integer value)] =
              .borrow returned (.integer value) := rfl
          simp only [callControl, packOne, callFrame_returned,
            registerReturnedLoan]
          rw [applyPendingFrom_single (inherited := state.pending)
            (loan := state.nextLoan) (current := .loanHole returned) pendingShape]
          rw [applyPendingWriteBack_derefLocalZero, transferActiveLoan_hole,
            transferLoanLocation_hole _ _ _ _ _ separate]
          exact ⟨_, ⟨#[(⟨lex⟩, returned)],
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)], #[]⟩, rfl, continuation⟩
    · obtain ⟨rfl, rfl, -⟩ := reborrowRun reborrowStep
      simp [valuesNil] at nilStep

/-- A `let` on the throw-aware spine whose initializer is itself
throw-aware — a call — so no stability is asked of it: the binding is a
step at the row and registries the initializer reached. -/
theorem wpRowThrow_letNativeValueThrow (binder : NativePatternBinder)
    (initializer body : ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (stepped : wpRowThrow initializer row registries state
      (fun initializedRow initializedRegistries initializedState control =>
        match control with
        | .value runtimeValue =>
            ∀ boundFrame,
              binder.bind (rowFrame initializedRow initializedRegistries)
                  runtimeValue = some boundFrame →
                ∃ boundRow, boundFrame = rowFrame boundRow initializedRegistries ∧
                  wpRowThrow body boundRow initializedRegistries
                    initializedState postValue postThrow
        | _ => postValue initializedRow initializedRegistries initializedState
            control)
      postThrow) :
    wpRowThrow (letNativeValue binder initializer body) row registries state
      postValue postThrow := by
  rintro finalFrame finalState control
    (⟨initStep, abrupt⟩ |
      ⟨initializedFrame, initializedState, runtimeValue, boundFrame, initStep,
        bindEq, bodyStep⟩)
  · have applied := stepped finalFrame finalState control initStep
    cases control with
    | throw_ kind thrown => exact applied
    | value produced => cases abrupt
    | return_ values => exact applied
    | break_ label => exact applied
    | continue_ label => exact applied
  · obtain ⟨initializedRow, initializedRegistries, rfl, bindWp⟩ :=
      stepped initializedFrame initializedState _ initStep
    obtain ⟨boundRow, rfl, bodyWp⟩ := bindWp boundFrame bindEq
    exact bodyWp finalFrame finalState control bodyStep

/-- A death marker with no operand, as a statement: its exit row is left as
a step. -/
theorem wpRowThrow_endLoanStatement (loans : Array LoanId) (row : Row)
    (registries : Registries) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : ∀ finalFrame finalState retired,
      (ReferenceLocationOperation.endLoan loans).evaluate? #[]
          (rowFrame row registries) state =
        some (.value finalFrame finalState retired) →
      ∃ finalRow finalRegistries,
        finalFrame = rowFrame finalRow finalRegistries ∧
        postValue finalRow finalRegistries finalState (.value retired)) :
    wpRowThrow (nativeReferenceOperation (.endLoan loans) valuesNil) row
      registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with h1 h2 h3
    subst oS oF values
    rcases evaluated with ⟨rv, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩
    · exact exit _ _ _ evaluation
    · exact absurd evaluation (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)

/-- A death marker over one operand: the operand runs on the spine, and
the marker's exit row over what it reached is left as a step. -/
theorem wpRowThrow_endLoanOver (loans : Array LoanId) (operand : ExprDenotation)
    (row : Row) (registries : Registries) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (inner : wpRowThrow operand row registries state
      (fun innerRow innerRegistries innerState control =>
        match control with
        | .value produced =>
            ∀ finalFrame finalState retired,
              (ReferenceLocationOperation.endLoan loans).evaluate? #[produced]
                  (rowFrame innerRow innerRegistries) innerState =
                some (.value finalFrame finalState retired) →
              ∃ finalRow finalRegistries,
                finalFrame = rowFrame finalRow finalRegistries ∧
                postValue finalRow finalRegistries finalState (.value retired)
        | _ => postValue innerRow innerRegistries innerState control)
      postThrow) :
    wpRowThrow
      (nativeReferenceOperation (.endLoan loans) (valuesCons operand valuesNil))
      row registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandsStep, frameEq, stateEq, controlEq⟩ |
      ⟨oF, oS, values, operandsStep, evaluated⟩)
  · subst finalFrame finalState control
    rcases operandsStep with
      ⟨lF, lS, lc, operandStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, operandStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, operandStep, nilStep, resultEq⟩
    · injection resultEq with h1 h2 h3
      subst oS oF propagated
      have applied := inner lF lS lc operandStep
      cases lc with
      | throw_ kind thrown => exact applied
      | value produced => cases abrupt
      | return_ values => exact applied
      | break_ label => exact applied
      | continue_ label => exact applied
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨lF, lS, lc, operandStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, operandStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, operandStep, nilStep, resultEq⟩
    · simp at resultEq
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      obtain ⟨innerRow, innerRegistries, rfl, exitWp⟩ :=
        inner lF lS (.value lv) operandStep
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩
      · exact exitWp finalFrame finalState rv evaluation
      · exact absurd evaluation (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)
    · simp [valuesNil] at nilStep

/-- A write through the returned reborrow resting in local 1, while the
parameter in local 0 carries its hole: the loan's registered location is
the parameter's dereference, which is the hole, so the write lands on the
resting borrow. -/
theorem mutate_evaluate_returnedReborrowLocal1 (state : RuntimeState)
    (outer returned : Nat) (current written : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) (separate : outer ≠ returned) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow returned current, written]
      (rowFrame #[some (.borrow outer (.loanHole returned)),
          some (.borrow returned current)]
        { activeLoans
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)] })
      state =
    some (.value
      (rowFrame #[some (.borrow outer (.loanHole returned)),
          some (.borrow returned written)]
        { activeLoans
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)] })
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, rewriteFirst, writeRuntimePlace?, writeRoot?, separate]

/-- The caller's own death marker settling the returned loan: the resting
current fills the parameter's hole, and the returned slot is cleared. -/
theorem endLoan_evaluate_returnedReborrow_nil (state : RuntimeState)
    (outer returned : Nat) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace)) (separate : outer ≠ returned) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[]
      (rowFrame #[some (.borrow outer (.loanHole returned)),
          some (.borrow returned current)]
        { activeLoans := #[(⟨0⟩, returned)], loanLocations })
      state =
    some (.value
      (rowFrame #[some (.borrow outer current), some .unit]
        { activeLoans := #[], loanLocations })
      state .unit) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  rw [endLoans?_returnedReborrow_zero _ _ _ _ _ separate]
  rfl

/-- The same marker wrapping a value: the value passes through. -/
theorem endLoan_evaluate_returnedReborrow_value (state : RuntimeState)
    (outer returned : Nat) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace)) (separate : outer ≠ returned) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[argument]
      (rowFrame #[some (.borrow outer (.loanHole returned)),
          some (.borrow returned current)]
        { activeLoans := #[(⟨0⟩, returned)], loanLocations })
      state =
    some (.value
      (rowFrame #[some (.borrow outer current), some .unit]
        { activeLoans := #[], loanLocations })
      state argument) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  rw [endLoans?_returnedReborrow_zero_value _ _ _ _ _ _ separate]
  rfl

/-- One returned value packs as itself. -/
theorem packResults_singleton (value : RuntimeValue) :
    SemanticOperations.packResults #[value] = value := rfl

/-- Finalizing the parameter beside a cleared returned slot. -/
theorem exportFrameLoans_rowFrame_borrowUnit (state : RuntimeState)
    (loan : Nat) (value : Int) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan (.integer value)), some .unit]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (loan, .integer value) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, applyWriteBack_empty_export, globalLoanKey?, noGlobal]

/-! ## A callee taking a value

A call passing one plain value read from local 0.  The callee exports no
write-back — its summary says so — so the caller's row, registries, and
pending are untouched, and only the callee's globals and loan counter
reach the caller. -/

theorem wpRowThrow_callValue
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) {argument : RuntimeValue}
    {rest : List (Option RuntimeValue)} {registries : Registries}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      state #[argument]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            calleeFinal.pending = state.pending ∧
            postValue (some argument :: rest).toArray registries
              { globals := calleeFinal.globals
                globalLoans := calleeFinal.globalLoans
                nextLoan := calleeFinal.nextLoan
                pending := state.pending }
              (.value (packResults results))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeCall handle none
        (nativeFunctionRelation calleeUnit calleeShape calleeBody)
        (valuesCons (localVar ⟨0⟩) valuesNil))
      (some argument :: rest).toArray registries state postValue postThrow := by
  unfold wpFunction at calleeWp
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
      ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
        rfl, rfl, rfl⟩)
  · rcases operandControl with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandValues with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
      subst rF rS
      simp only [readLocal?_rowFrame, List.getElem?_toArray,
        List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
        at readEq
      subst readEq
      injection valueEq with valueEq
      subst rv
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      have applied := calleeWp calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown => exact applied
      | returned results =>
          obtain ⟨pendingEq, continuation⟩ := applied
          simp only [callControl, callFrame_returned, registerReturnedLoan]
          rw [applyPendingFrom_none pendingEq]
          exact ⟨_, registries, rfl, continuation⟩
    · simp [valuesNil] at nilStep

/-! ## An invocation-aware call over an arbitrary stable operand row

Generic callees are selected by the type substitution carried by the frame
reached after evaluating their operands.  The operands used by lowered direct
calls are row-stable, so their frame has the caller's registry component and
therefore its already-computed substitution.  This arity-independent bridge
keeps the modular call path constant-size for one, two, and larger value rows.
-/

theorem wpRowThrow_callAt
    (handle : FunctionHandle)
    (callee : Array (TypeId × TypeId) → FunctionDenotation)
    (operands : ValuesDenotation)
    {row : Row} {registries : Registries} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (stable : RowStableValues operands)
    (stepped : wpValuesRow operands row registries state fun result =>
      match result with
      | .control operandState operandRow control =>
          match control with
          | .throw_ kind thrown => postThrow kind thrown
          | _ => postValue operandRow registries operandState control
      | .values operandState operandRow values =>
          wpFunction (callee registries.typeInstantiation)
            operandState values.toArray fun calleeFinal outcome =>
              match outcome with
              | .returned results =>
                  calleeFinal.pending = operandState.pending ∧
                  postValue operandRow registries
                    { globals := calleeFinal.globals
                      globalLoans := calleeFinal.globalLoans
                      nextLoan := calleeFinal.nextLoan
                      pending := operandState.pending }
                    (.value (packResults results))
              | .threw kind thrown => postThrow kind thrown) :
    wpRowThrow (nativeCallAt handle none callee operands)
      row registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨operandFrame, operandState, propagated, operandStep, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, calleeState, outcome, operandStep,
        calleeStep, rfl, rfl, rfl⟩)
  · obtain ⟨operandRow, shape⟩ := stable row registries state _ operandStep
    rcases shape with ⟨resultState, resultValues, impossible⟩ |
      ⟨resultState, resultControl, equal⟩
    · cases impossible
    · cases equal
      have applied := stepped (.control finalState operandRow control)
        (by simpa [RowValuesResult.embed] using operandStep)
      cases control with
      | throw_ kind thrown => exact applied
      | value runtimeValue => exact ⟨operandRow, registries, rfl, applied⟩
      | return_ results => exact ⟨operandRow, registries, rfl, applied⟩
      | break_ label => exact ⟨operandRow, registries, rfl, applied⟩
      | continue_ label => exact ⟨operandRow, registries, rfl, applied⟩
  · obtain ⟨operandRow, shape⟩ := stable row registries state _ operandStep
    rcases shape with ⟨resultState, resultValues, equal⟩ |
      ⟨resultState, resultControl, impossible⟩
    · cases equal
      have opened := stepped (.values operandState operandRow values)
        (by simpa [RowValuesResult.embed] using operandStep)
      unfold wpFunction at opened
      have applied := opened calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown => exact applied
      | returned results =>
          obtain ⟨pendingEq, continuation⟩ := applied
          simp only [callControl, callFrame_returned, registerReturnedLoan]
          rw [applyPendingFrom_none pendingEq]
          exact ⟨operandRow, registries, rfl, continuation⟩
    · cases impossible

/-! ## Two reborrows, one call

`core.call f(&mut *left, &mut *right)`: the first reborrow mints a loan on
the first parameter, the second on the second, the callee exports both
write-backs in that order, and the death marker retires both lexical rows. -/

/-- The second reborrow, after the first has minted its loan. -/
theorem derefLocalBorrow_evaluate_secondOfTwo
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (lexFirst lexSecond leftOuter rightOuter first : Nat)
    (distinctLexical : lexFirst ≠ lexSecond)
    (current : RuntimeValue) (state : RuntimeState) :
    (({ location := ⟨⟨1⟩⟩, fields := [], referenceType,
        kind := .mutable, lexicalLoan := lexSecond } :
      DerefLocalBorrowOperation)).evaluate? #[]
      { locals := #[some (.borrow leftOuter (.loanHole first)),
          some (.borrow rightOuter current)]
        activeLoans := #[(⟨lexFirst⟩, first)]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
          (first, ⟨.local ⟨0⟩, #[.deref], true⟩)] } state =
    some (.value
      { locals := #[some (.borrow leftOuter (.loanHole first)),
          some (.borrow rightOuter (.loanHole state.nextLoan))]
        activeLoans := #[(⟨lexFirst⟩, first), (⟨lexSecond⟩, state.nextLoan)]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
          (first, ⟨.local ⟨0⟩, #[.deref], true⟩),
          (state.nextLoan, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan current)) := by
  have distinct : ((⟨lexFirst⟩ : ExprId) != ⟨lexSecond⟩) = true := by
    simp [bne, show ((⟨lexFirst⟩ : ExprId) == ⟨lexSecond⟩) = (lexFirst == lexSecond) from rfl,
      distinctLexical]
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    DerefLocalBorrowOperation.resolve?, resolveDerefLocalFieldPath?,
    resolveNominalFieldSteps?, readLocal?, borrowRuntimePlaceAt?,
    readRuntimePlace?, readRoot?, readProjections?, writeRuntimePlace?,
    writeRoot?, writeProjections?, mutableKind, Array.filter, distinct,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-- The first reborrow of two parameters, on their exact row. -/
theorem derefLocalBorrow_evaluate_firstOfTwo
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (lex leftOuter rightOuter : Nat)
    (leftCurrent rightCurrent : RuntimeValue) (state : RuntimeState) :
    (({ location := ⟨⟨0⟩⟩, fields := [], referenceType,
        kind := .mutable, lexicalLoan := lex } :
      DerefLocalBorrowOperation)).evaluate? #[]
      (rowFrame #[some (.borrow leftOuter leftCurrent), some (.borrow rightOuter rightCurrent)]
        { activeLoans := #[]
          loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
            (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }) state =
    some (.value
      { locals := #[some (.borrow leftOuter (.loanHole state.nextLoan)),
          some (.borrow rightOuter rightCurrent)]
        activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan leftCurrent)) := by
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    DerefLocalBorrowOperation.resolve?, resolveDerefLocalFieldPath?,
    resolveNominalFieldSteps?, readLocal?, borrowRuntimePlaceAt?,
    readRuntimePlace?, readRoot?, readProjections?, writeRuntimePlace?,
    writeRoot?, writeProjections?, rowFrame, mutableKind, Array.filter,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-- The death marker of both lexical rows after both write-backs retired
their loans: nothing is left to end, and the argument passes through. -/
theorem endLoan_evaluate_retiredTwo (lexFirst lexSecond leftOuter rightOuter : Nat)
    (locals : Array (Option RuntimeValue)) (argument : RuntimeValue)
    (state : RuntimeState) :
    (ReferenceLocationOperation.endLoan #[⟨lexFirst⟩, ⟨lexSecond⟩]).evaluate? #[argument]
      { locals
        activeLoans := #[]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
      state =
    some (.value
      { locals
        activeLoans := #[]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
      state argument) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, endLoans?,
    Array.filter]

/-- Both write-backs of two dereference loans, applied to the caller's
frame, whatever the lexical rows are called. -/
theorem applyPendingFrom_twoDerefLocals_lexical
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan lexFirst lexSecond leftOuter rightOuter firstLoan secondLoan : Nat)
    (leftReplacement rightReplacement : RuntimeValue)
    (noLeftTransfer : transferredLoan? leftReplacement = none)
    (noRightTransfer : transferredLoan? rightReplacement = none)
    (freshSeparate : firstLoan ≠ secondLoan)
    (leftFirst : leftOuter ≠ firstLoan)
    (leftSecond : leftOuter ≠ secondLoan)
    (rightFirst : rightOuter ≠ firstLoan)
    (rightSecond : rightOuter ≠ secondLoan) :
    applyPendingFrom inherited
        { locals := #[some (.borrow leftOuter (.loanHole firstLoan)),
            some (.borrow rightOuter (.loanHole secondLoan))]
          activeLoans := #[(⟨lexFirst⟩, firstLoan), (⟨lexSecond⟩, secondLoan)]
          loanLocations :=
            #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
              (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
              (firstLoan, ⟨.local ⟨0⟩, #[.deref], true⟩),
              (secondLoan, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := inherited.push (firstLoan, leftReplacement)
            |>.push (secondLoan, rightReplacement) } =
      ({ locals := #[some (.borrow leftOuter leftReplacement),
            some (.borrow rightOuter rightReplacement)]
         activeLoans := #[]
         loanLocations :=
           #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
             (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  have secondFirst : secondLoan ≠ firstLoan := Ne.symm freshSeparate
  rw [applyPendingFrom_two_push]
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?,
    transferActiveLoan, transferLoanLocation, noLeftTransfer, noRightTransfer,
    Array.filter, secondFirst, leftFirst, leftSecond, rightFirst, rightSecond]

/-- `core.call f(&mut *left, &mut *right)` as one statement, composed. -/
theorem callTwoReborrowsStatement_step
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lexFirst lexSecond : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (distinctLexical : lexFirst ≠ lexSecond)
    {leftOuter rightOuter : Nat} {leftCur rightCur : Int} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (leftPrior : leftOuter < state.nextLoan)
    (rightPrior : rightOuter < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 + 1 }
      #[.borrow state.nextLoan (.integer leftCur),
        .borrow (state.nextLoan + 1) (.integer rightCur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ leftNew rightNew : Int,
              calleeFinal.pending =
                (state.pending.push (state.nextLoan, .integer leftNew)).push
                  (state.nextLoan + 1, .integer rightNew) ∧
              postValue
                #[some (.borrow leftOuter (.integer leftNew)),
                  some (.borrow rightOuter (.integer rightNew))]
                { activeLoans := #[]
                  loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                    (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (packResults results))
        | .threw kind thrown => postThrow kind thrown)) :
    ∀ {finalFrame : RuntimeFrame} {finalState : RuntimeState} {control : Control},
      (nativeReferenceOperation (.endLoan #[⟨lexFirst⟩, ⟨lexSecond⟩])
        (valuesCons
          (nativeCall handle none
            (nativeFunctionRelation calleeUnit calleeShape calleeBody)
            (valuesCons
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨0⟩⟩, fields := []
                  referenceType, kind := .mutable, lexicalLoan := lexFirst }
                valuesNil)
              (valuesCons
                (nativeDerefLocalBorrowOperation
                  { location := ⟨⟨1⟩⟩, fields := []
                    referenceType, kind := .mutable, lexicalLoan := lexSecond }
                  valuesNil)
                valuesNil)))
          valuesNil))
        (rowFrame #[some (.borrow leftOuter (.integer leftCur)),
            some (.borrow rightOuter (.integer rightCur))]
          { activeLoans := #[]
            loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
              (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
        state finalFrame finalState control →
      (∃ kind thrown, control = .throw_ kind thrown ∧ postThrow kind thrown) ∨
      ∃ (results : Array RuntimeValue) (leftNew rightNew : Int),
        control = .value (packResults results) ∧
        finalFrame = rowFrame
          #[some (.borrow leftOuter (.integer leftNew)),
            some (.borrow rightOuter (.integer rightNew))]
          { activeLoans := #[]
            loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
              (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] } ∧
        postValue
          #[some (.borrow leftOuter (.integer leftNew)),
            some (.borrow rightOuter (.integer rightNew))]
          { activeLoans := #[]
            loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
              (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
          finalState (.value (packResults results)) := by
  have leftFirst : leftOuter ≠ state.nextLoan := Nat.ne_of_lt leftPrior
  have leftSecond : leftOuter ≠ state.nextLoan + 1 := by omega
  have rightFirst : rightOuter ≠ state.nextLoan := Nat.ne_of_lt rightPrior
  have rightSecond : rightOuter ≠ state.nextLoan + 1 := by omega
  have freshSeparate : state.nextLoan ≠ state.nextLoan + 1 := by omega
  unfold wpFunction at calleeWp
  intro finalFrame finalState control step
  /- The frames the two mints leave. -/
  let mintedOne : RuntimeFrame :=
    { locals := #[some (.borrow leftOuter (.loanHole state.nextLoan)),
        some (.borrow rightOuter (.integer rightCur))]
      activeLoans := #[(⟨lexFirst⟩, state.nextLoan)]
      loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
        (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
        (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
  let mintedTwo : RuntimeFrame :=
    { locals := #[some (.borrow leftOuter (.loanHole state.nextLoan)),
        some (.borrow rightOuter (.loanHole (state.nextLoan + 1)))]
      activeLoans := #[(⟨lexFirst⟩, state.nextLoan), (⟨lexSecond⟩, state.nextLoan + 1)]
      loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
        (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
        (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩),
        (state.nextLoan + 1, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
  have firstRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨0⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lexFirst }
          valuesNil)
          (rowFrame #[some (.borrow leftOuter (.integer leftCur)),
              some (.borrow rightOuter (.integer rightCur))]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
          state rF rS c →
        rF = mintedOne ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan (.integer leftCur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_firstOfTwo referenceType mutableKind lexFirst
          leftOuter rightOuter (.integer leftCur) (.integer rightCur) state] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  have secondRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨1⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lexSecond }
          valuesNil)
          mintedOne { state with nextLoan := state.nextLoan + 1 } rF rS c →
        rF = mintedTwo ∧
        rS = { state with nextLoan := state.nextLoan + 1 + 1 } ∧
        c = .value (.borrow (state.nextLoan + 1) (.integer rightCur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [show mintedOne = _ from rfl,
          derefLocalBorrow_evaluate_secondOfTwo referenceType mutableKind lexFirst
            lexSecond leftOuter rightOuter state.nextLoan distinctLexical
            (.integer rightCur) { state with nextLoan := state.nextLoan + 1 }] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  /- The operand row: both mints, in order; neither raises control. -/
  have operandsRun :
      ∀ {result : BigStep.ValuesResult},
        (valuesCons
          (nativeDerefLocalBorrowOperation
            { location := ⟨⟨0⟩⟩, fields := []
              referenceType, kind := .mutable, lexicalLoan := lexFirst }
            valuesNil)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨1⟩⟩, fields := []
                referenceType, kind := .mutable, lexicalLoan := lexSecond }
              valuesNil)
            valuesNil))
          (rowFrame #[some (.borrow leftOuter (.integer leftCur)),
              some (.borrow rightOuter (.integer rightCur))]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
          state result →
        result = .values { state with nextLoan := state.nextLoan + 1 + 1 } mintedTwo
          [.borrow state.nextLoan (.integer leftCur),
            .borrow (state.nextLoan + 1) (.integer rightCur)] := by
    rintro result
      (⟨rF, rS, rc, firstStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, firstStep, tailStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, firstStep, tailStep, resultEq⟩)
    · obtain ⟨-, -, rfl⟩ := firstRun firstStep
      cases abrupt
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · obtain ⟨rfl, rfl, veq'⟩ := secondRun secondStep
        injection veq' with veq'
        subst veq'
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s' f' vs'
        injection tailEq with h4 h5 h6
        subst s f vs
        exact resultEq
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · simp at tailEq
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep
  /- The call operand: the callee threw, or returned and both write-backs
  landed. -/
  have callRun :
      ∀ {cF : RuntimeFrame} {cS : RuntimeState} {c : Control},
        (nativeCall handle none
          (nativeFunctionRelation calleeUnit calleeShape calleeBody)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨0⟩⟩, fields := []
                referenceType, kind := .mutable, lexicalLoan := lexFirst }
              valuesNil)
            (valuesCons
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨1⟩⟩, fields := []
                  referenceType, kind := .mutable, lexicalLoan := lexSecond }
                valuesNil)
              valuesNil)))
          (rowFrame #[some (.borrow leftOuter (.integer leftCur)),
              some (.borrow rightOuter (.integer rightCur))]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
          state cF cS c →
        (∃ kind thrown, c = .throw_ kind thrown ∧ postThrow kind thrown) ∨
        ∃ (results : Array RuntimeValue) (leftNew rightNew : Int)
            (calleeFinal : RuntimeState),
          c = .value (packResults results) ∧
          cF = { locals := #[some (.borrow leftOuter (.integer leftNew)),
                   some (.borrow rightOuter (.integer rightNew))]
                 activeLoans := #[]
                 loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                   (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] } ∧
          cS = { globals := calleeFinal.globals
                 globalLoans := calleeFinal.globalLoans
                 nextLoan := calleeFinal.nextLoan
                 pending := state.pending } ∧
          postValue
            #[some (.borrow leftOuter (.integer leftNew)),
              some (.borrow rightOuter (.integer rightNew))]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
            { globals := calleeFinal.globals
              globalLoans := calleeFinal.globalLoans
              nextLoan := calleeFinal.nextLoan
              pending := state.pending }
            (.value (packResults results)) := by
    rintro cF cS c
      (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
        ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
          rfl, rfl, rfl⟩)
    · cases operandsRun operandControl
    · injection operandsRun operandValues with h1 h2 h3
      subst oS oF values
      have applied := calleeWp calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown =>
          exact .inl ⟨kind, thrown, rfl, applied⟩
      | returned results =>
          obtain ⟨leftNew, rightNew, pendingShape, continuation⟩ := applied
          have noLeftTransfer : transferredLoan? (.integer leftNew) = none := by
            simp [transferredLoan?, findFirst]
          have noRightTransfer : transferredLoan? (.integer rightNew) = none := by
            simp [transferredLoan?, findFirst]
          refine .inr ⟨results, leftNew, rightNew, calleeState, rfl, ?_, ?_, continuation⟩
          · rw [callFrame_returned]
            simp only [registerReturnedLoan]
            rcases calleeState with ⟨globals, globalLoans, nextLoan, pending⟩
            simp only at pendingShape
            subst pendingShape
            rw [show mintedTwo = _ from rfl,
              applyPendingFrom_twoDerefLocals_lexical state.pending globals globalLoans
                nextLoan lexFirst lexSecond leftOuter rightOuter state.nextLoan
                (state.nextLoan + 1) (.integer leftNew) (.integer rightNew) noLeftTransfer
                noRightTransfer freshSeparate leftFirst leftSecond rightFirst rightSecond]
          · rcases calleeState with ⟨globals, globalLoans, nextLoan, pending⟩
            simp only at pendingShape
            subst pendingShape
            rw [show mintedTwo = _ from rfl,
              applyPendingFrom_twoDerefLocals_lexical state.pending globals globalLoans
                nextLoan lexFirst lexSecond leftOuter rightOuter state.nextLoan
                (state.nextLoan + 1) (.integer leftNew) (.integer rightNew) noLeftTransfer
                noRightTransfer freshSeparate leftFirst leftSecond rightFirst rightSecond]
  /- The whole statement: the death marker over the call's operand row. -/
  rcases step with
    ⟨oF, oS, propagated, operandsStep, rfl, rfl, rfl⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · rcases operandsStep with
      ⟨cF, cS, cc, callStep, abrupt, resultEq⟩ |
      ⟨cF, cS, cv, f, s, vs, callStep, nilStep, resultEq⟩ |
      ⟨cF, cS, cv, f, s, cc, callStep, nilStep, resultEq⟩
    · rcases callRun callStep with ⟨kind, thrown, rfl, thrown_post⟩ |
        ⟨results, leftNew, rightNew, calleeFinal, rfl, -, -, -⟩
      · injection resultEq with h1 h2 h3
        subst h1 h2 h3
        exact .inl ⟨kind, thrown, rfl, thrown_post⟩
      · cases abrupt
    · simp [valuesNil] at nilStep
      obtain ⟨rfl, rfl, rfl⟩ := nilStep
      simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨cF, cS, cc, callStep, abrupt, resultEq⟩ |
      ⟨cF, cS, cv, f, s, vs, callStep, nilStep, resultEq⟩ |
      ⟨cF, cS, cv, f, s, cc, callStep, nilStep, resultEq⟩
    · simp at resultEq
    · rcases callRun callStep with ⟨kind, thrown, absurdEq, -⟩ |
        ⟨results, leftNew, rightNew, calleeFinal, ceq, feq, seq, continuation⟩
      · cases absurdEq
      · injection ceq with ceq
        subst ceq
        subst feq seq
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (ReferenceLocationOperation.endLoan
              #[(⟨lexFirst⟩ : LoanId), ⟨lexSecond⟩]).evaluate?
              [packResults results].toArray _ _ = _ from
            endLoan_evaluate_retiredTwo lexFirst lexSecond leftOuter rightOuter
              #[some (.borrow leftOuter (.integer leftNew)),
                some (.borrow rightOuter (.integer rightNew))]
              (packResults results) _] at evaluation <;>
          cases Option.some.inj evaluation
        exact .inr ⟨results, leftNew, rightNew, rfl, rfl, continuation⟩
    · simp [valuesNil] at nilStep

/-- No result packs as the unit. -/
theorem packResults_nil : packResults #[] = .unit := rfl

/-- The two-reborrow call under its death marker as an expression — a
body that is the call itself — rather than a statement: the same step,
its value the packed results. -/
theorem wpRowThrow_callTwoReborrows
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lexFirst lexSecond : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (distinctLexical : lexFirst ≠ lexSecond)
    {leftOuter rightOuter : Nat} {leftCur rightCur : Int} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (leftPrior : leftOuter < state.nextLoan)
    (rightPrior : rightOuter < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 + 1 }
      #[.borrow state.nextLoan (.integer leftCur),
        .borrow (state.nextLoan + 1) (.integer rightCur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ leftNew rightNew : Int,
              calleeFinal.pending =
                (state.pending.push (state.nextLoan, .integer leftNew)).push
                  (state.nextLoan + 1, .integer rightNew) ∧
              postValue
                #[some (.borrow leftOuter (.integer leftNew)),
                  some (.borrow rightOuter (.integer rightNew))]
                { activeLoans := #[]
                  loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                    (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (packResults results))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeReferenceOperation (.endLoan #[⟨lexFirst⟩, ⟨lexSecond⟩])
        (valuesCons
          (nativeCall handle none
            (nativeFunctionRelation calleeUnit calleeShape calleeBody)
            (valuesCons
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨0⟩⟩, fields := []
                  referenceType, kind := .mutable, lexicalLoan := lexFirst }
                valuesNil)
              (valuesCons
                (nativeDerefLocalBorrowOperation
                  { location := ⟨⟨1⟩⟩, fields := []
                    referenceType, kind := .mutable, lexicalLoan := lexSecond }
                  valuesNil)
                valuesNil)))
          valuesNil))
      #[some (.borrow leftOuter (.integer leftCur)),
        some (.borrow rightOuter (.integer rightCur))]
      { activeLoans := #[]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  rcases callTwoReborrowsStatement_step handle lexFirst lexSecond referenceType
      mutableKind distinctLexical leftPrior rightPrior calleeWp step with
    ⟨kind, thrown, rfl, thrownPost⟩ | ⟨results, leftNew, rightNew, rfl, rfl, post⟩
  · exact thrownPost
  · exact ⟨_, _, rfl, post⟩

/-- The statement-spine rule for a two-reborrow call statement. -/
theorem wpStatementsRowThrow_consCallTwoReborrows
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lexFirst lexSecond : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (distinctLexical : lexFirst ≠ lexSecond)
    {leftOuter rightOuter : Nat} {leftCur rightCur : Int} {state : RuntimeState}
    (tail : StatementsDenotation)
    {postDone : Row → Registries → RuntimeState → Prop}
    {postControl : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (leftPrior : leftOuter < state.nextLoan)
    (rightPrior : rightOuter < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 + 1 }
      #[.borrow state.nextLoan (.integer leftCur),
        .borrow (state.nextLoan + 1) (.integer rightCur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned _ =>
            ∃ leftNew rightNew : Int,
              calleeFinal.pending =
                (state.pending.push (state.nextLoan, .integer leftNew)).push
                  (state.nextLoan + 1, .integer rightNew) ∧
              wpStatementsRowThrow tail
                #[some (.borrow leftOuter (.integer leftNew)),
                  some (.borrow rightOuter (.integer rightNew))]
                { activeLoans := #[]
                  loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                    (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                postDone postControl postThrow
        | .threw kind thrown => postThrow kind thrown)) :
    wpStatementsRowThrow
      (statementsCons
        (nativeReferenceOperation (.endLoan #[⟨lexFirst⟩, ⟨lexSecond⟩])
          (valuesCons
            (nativeCall handle none
              (nativeFunctionRelation calleeUnit calleeShape calleeBody)
              (valuesCons
                (nativeDerefLocalBorrowOperation
                  { location := ⟨⟨0⟩⟩, fields := []
                    referenceType, kind := .mutable, lexicalLoan := lexFirst }
                  valuesNil)
                (valuesCons
                  (nativeDerefLocalBorrowOperation
                    { location := ⟨⟨1⟩⟩, fields := []
                      referenceType, kind := .mutable, lexicalLoan := lexSecond }
                    valuesNil)
                  valuesNil)))
            valuesNil))
        tail)
      #[some (.borrow leftOuter (.integer leftCur)),
        some (.borrow rightOuter (.integer rightCur))]
      { activeLoans := #[]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
      state postDone postControl postThrow := by
  rintro result
    (⟨finalFrame, finalState, raised, headStep, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩)
  · rcases callTwoReborrowsStatement_step
        (postValue := fun continuationRow continuationRegistries
            continuationState _ =>
          wpStatementsRowThrow tail continuationRow continuationRegistries
            continuationState postDone postControl postThrow)
        handle lexFirst lexSecond referenceType mutableKind distinctLexical
        leftPrior rightPrior calleeWp headStep with
      ⟨kind, thrown, rfl, thrown_post⟩ |
      ⟨results, leftNew, rightNew, rfl, -, -⟩
    · exact thrown_post
    · cases abrupt
  · rcases callTwoReborrowsStatement_step
        (postValue := fun continuationRow continuationRegistries
            continuationState _ =>
          wpStatementsRowThrow tail continuationRow continuationRegistries
            continuationState postDone postControl postThrow)
        handle lexFirst lexSecond referenceType mutableKind distinctLexical
        leftPrior rightPrior calleeWp headStep with
      ⟨kind, thrown, absurdEq, -⟩ |
      ⟨results, leftNew, rightNew, valueEq, rfl, continuation⟩
    · cases absurdEq
    · exact continuation result tailStep

/-! ## A call whose operand is itself throw-aware

`wpRowThrow_callValue` reads its one argument from local 0.  An argument
that is an expression — a nested call, say — is evaluated first on the
spine: its value is the callee's argument, its own throw is the caller's
throw, and any other control it raises is the caller's.  The callee is any
function denotation; an unspecified one is opened by
`wpFunction_nativeFunctionRelation` and run inline. -/

theorem wpRowThrow_callValueOperand (handle : FunctionHandle)
    (callee : FunctionDenotation) (operand : ExprDenotation)
    {row : Row} {registries : Registries} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (stepped : wpRowThrow operand row registries state
      (fun operandRow operandRegistries operandState control =>
        match control with
        | .value argument =>
            wpFunction callee operandState #[argument]
              (fun calleeFinal outcome =>
                match outcome with
                | .returned results =>
                    calleeFinal.pending = operandState.pending ∧
                    postValue operandRow operandRegistries
                      { globals := calleeFinal.globals
                        globalLoans := calleeFinal.globalLoans
                        nextLoan := calleeFinal.nextLoan
                        pending := operandState.pending }
                      (.value (packResults results))
                | .threw kind thrown => postThrow kind thrown)
        | _ => postValue operandRow operandRegistries operandState control)
      postThrow) :
    wpRowThrow (nativeCall handle none callee (valuesCons operand valuesNil))
      row registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
      ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
        rfl, rfl, rfl⟩)
  · rcases operandControl with
      ⟨rF, rS, rc, headStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, headStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, headStep, nilStep, resultEq⟩
    · have applied := stepped rF rS rc headStep
      cases rc with
      | throw_ kind thrown => injection resultEq with h1 h2 h3; subst_vars; exact applied
      | value produced => cases abrupt
      | return_ values => injection resultEq with h1 h2 h3; subst_vars; exact applied
      | break_ label => injection resultEq with h1 h2 h3; subst_vars; exact applied
      | continue_ label => injection resultEq with h1 h2 h3; subst_vars; exact applied
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandValues with
      ⟨rF, rS, rc, headStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, headStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, headStep, nilStep, resultEq⟩
    · simp at resultEq
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      obtain ⟨operandRow, operandRegistries, rfl, opened⟩ :=
        stepped rF rS (.value rv) headStep
      have opened' : wpFunction callee rS #[rv] _ := opened
      unfold wpFunction at opened'
      have applied := opened' calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown => exact applied
      | returned results =>
          obtain ⟨pendingEq, continuation⟩ := applied
          simp only [callControl, callFrame_returned, registerReturnedLoan]
          rw [applyPendingFrom_none pendingEq]
          exact ⟨_, operandRegistries, rfl, continuation⟩
    · simp [valuesNil] at nilStep

/-- A branch whose condition is throw-aware — a call, say: the condition
runs on the spine, and the boolean it produced selects the arm.  The
boolean is a computed term, so each arm receives the equation selecting
it as a hypothesis rather than a pattern. -/
theorem wpRowThrow_nativeBranchThrow (condition thenBranch : ExprDenotation)
    (elseBranch : Option ExprDenotation) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (stepped : wpRowThrow condition row registries state
      (fun conditionRow conditionRegistries conditionState control =>
        match control with
        | .value (.bool decided) =>
            (decided = true →
              wpRowThrow thenBranch conditionRow conditionRegistries conditionState
                postValue postThrow) ∧
            (decided = false →
              match elseBranch with
              | some branch =>
                  wpRowThrow branch conditionRow conditionRegistries conditionState
                    postValue postThrow
              | none => postValue conditionRow conditionRegistries conditionState (.value .unit))
        | .value _ => True
        | _ => postValue conditionRow conditionRegistries conditionState control)
      postThrow) :
    wpRowThrow (nativeBranch condition thenBranch elseBranch) row registries
      state postValue postThrow := by
  rintro finalFrame finalState control step
  rcases step with ⟨conditionStep, abrupt⟩ |
    ⟨conditionFrame, conditionState, conditionStep, armStep⟩ |
    ⟨conditionFrame, conditionState, conditionStep, elseStep⟩
  · have applied := stepped finalFrame finalState control conditionStep
    cases control with
    | throw_ kind thrown => exact applied
    | value produced => cases abrupt
    | return_ values => exact applied
    | break_ label => exact applied
    | continue_ label => exact applied
  · obtain ⟨conditionRow, conditionRegistries, rfl, opened⟩ :=
      stepped conditionFrame conditionState _ conditionStep
    exact opened.1 rfl finalFrame finalState control armStep
  · obtain ⟨conditionRow, conditionRegistries, rfl, opened⟩ :=
      stepped conditionFrame conditionState _ conditionStep
    cases elseBranch with
    | some branch => exact opened.2 rfl finalFrame finalState control elseStep
    | none =>
        obtain ⟨rfl, rfl, rfl⟩ := elseStep
        exact ⟨conditionRow, conditionRegistries, rfl, opened.2 rfl⟩

/-! ## Moving the sole local out

A generic body that returns its parameter moves it out of local 0: the
slot empties and the value is the control.  The value is abstract — a
generic parameter's encoding — and so are the registries, since an
abstract value may be a borrow. -/

/-- The move evaluator on a row whose first slot is filled. -/
theorem move_evaluate_rowFrame (value : RuntimeValue)
    (rest : List (Option RuntimeValue)) (registries : Registries)
    (state : RuntimeState) :
    (LocalLocationOperation.move ⟨⟨0⟩⟩).evaluate? #[]
      (rowFrame (some value :: rest).toArray registries) state =
    some (.value (rowFrame (none :: rest).toArray registries) state value) := by
  simp [LocalLocationOperation.evaluate?, rowFrame, readRuntimePlace?, readRoot?,
    readLocal?, readProjections?]

/-- Moving local 0 out, as a whole body. -/
theorem wpRowThrow_moveLocal0 (value : RuntimeValue)
    (rest : List (Option RuntimeValue)) (registries : Registries)
    (state : RuntimeState)
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (exit : postValue (none :: rest).toArray registries state (.value value)) :
    wpRowThrow (nativeLocalOperation (.move ⟨⟨0⟩⟩) valuesNil)
      (some value :: rest).toArray registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with h1 h2 h3
    subst oS oF values
    rcases evaluated with ⟨rv, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [move_evaluate_rowFrame value rest registries state] at evaluation <;>
      cases Option.some.inj evaluation
    exact ⟨_, registries, rfl, exit⟩

end LeanerIR.Proofs.Denotation
