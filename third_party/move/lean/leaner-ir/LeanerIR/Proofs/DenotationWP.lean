-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denotation
import LeanerIR.Proofs.Typed
import LeanerIR.Proofs.WP

/-!
# Weakest-precondition rules for native denotations

These rules calculate over the generated combinator tree.  Unlike the deep
`wpExpr` rules, none of their left-hand sides contains an expression arena or
an `ExprId`; the identifiers retained by operation combinators are only the
runtime sites required by the executable semantics.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR.Validation
open LeanerIR.BigStep
open LeanerIR.SemanticOperations

/-- Predicate transformer of one native expression relation. -/
def wpExpr (denotation : ExprDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  ∀ finalFrame finalState control,
    denotation frame state finalFrame finalState control →
      post finalFrame finalState control

/-- Predicate transformer of a native operand row. -/
def wpValues (denotation : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState) (post : ValuesResult → Prop) : Prop :=
  ∀ result, denotation frame state result → post result

/-- Predicate transformer of a native statement row. -/
def wpStatements (denotation : StatementsDenotation) (frame : RuntimeFrame)
    (state : RuntimeState) (post : StatementsResult → Prop) : Prop :=
  ∀ result, denotation frame state result → post result

/-- Predicate transformer of a state-retaining native function relation. -/
def wpFunction (denotation : FunctionDenotation) (state : RuntimeState)
    (arguments : Array RuntimeValue)
    (post : RuntimeState → Outcome → Prop) : Prop :=
  ∀ final outcome, denotation state arguments final outcome →
    post final outcome

theorem wpExpr_post_congr {denotation : ExprDenotation}
    {frame : RuntimeFrame} {state : RuntimeState}
    {post post' : RuntimeFrame → RuntimeState → Control → Prop}
    (equivalent : ∀ finalFrame finalState control,
      post finalFrame finalState control ↔
        post' finalFrame finalState control) :
    wpExpr denotation frame state post ↔
      wpExpr denotation frame state post' := by
  constructor
  · intro h finalFrame finalState control step
    exact (equivalent finalFrame finalState control).mp
      (h finalFrame finalState control step)
  · intro h finalFrame finalState control step
    exact (equivalent finalFrame finalState control).mpr
      (h finalFrame finalState control step)

theorem wpValues_post_congr {denotation : ValuesDenotation}
    {frame : RuntimeFrame} {state : RuntimeState}
    {post post' : ValuesResult → Prop}
    (equivalent : ∀ result, post result ↔ post' result) :
    wpValues denotation frame state post ↔
      wpValues denotation frame state post' := by
  constructor
  · intro h result step
    exact (equivalent result).mp (h result step)
  · intro h result step
    exact (equivalent result).mpr (h result step)

/-! ## Function boundary -/

theorem wpFunction_functionRelation
    {unit : ExecutableUnit} {declaration : FunctionDecl FunctionBody}
    {body : ExprDenotation} {arguments : Array RuntimeValue}
    {initial : RuntimeState} {post : RuntimeState → Outcome → Prop} :
    wpFunction (functionRelation unit declaration body) initial arguments post ↔
      ∀ frame, initialFrame? declaration arguments = some frame →
        wpExpr body frame initial fun finalFrame evaluatedState control =>
          ∀ outcome,
            finishControl? declaration.signature.results.size control =
                some outcome →
              post
                (finalizeFunctionState unit declaration.profile initial
                  evaluatedState finalFrame outcome)
                outcome := by
  constructor
  · intro h frame frame_eq finalFrame evaluatedState control body_step
      outcome outcome_eq
    exact h _ outcome
      ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
        outcome_eq, rfl⟩
  · rintro h final outcome
      ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
        outcome_eq, finalize_eq⟩
    subst final
    exact h frame frame_eq finalFrame evaluatedState control body_step
      outcome outcome_eq

theorem wpFunction_nativeFunctionRelation
    {unit : ExecutableUnit} {shape : FunctionShape}
    {body : ExprDenotation} {arguments : Array RuntimeValue}
    {initial : RuntimeState} {post : RuntimeState → Outcome → Prop} :
    wpFunction (nativeFunctionRelation unit shape body) initial arguments post ↔
      ∀ frame, nativeInitialFrame? shape arguments = some frame →
        wpExpr body frame initial fun finalFrame evaluatedState control =>
          ∀ outcome,
            finishControl? shape.resultCount control = some outcome →
              post
                (finalizeFunctionState unit shape.profile initial
                  evaluatedState finalFrame outcome)
                outcome := by
  constructor
  · intro h frame frame_eq finalFrame evaluatedState control body_step
      outcome outcome_eq
    exact h _ outcome
      ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
        outcome_eq, rfl⟩
  · rintro h final outcome
      ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
        outcome_eq, finalize_eq⟩
    subst final
    exact h frame frame_eq finalFrame evaluatedState control body_step
      outcome outcome_eq

theorem wp_function {unit : ExecutableUnit}
    {declaration : FunctionDecl FunctionBody} {body : ExprDenotation}
    {arguments : Array RuntimeValue} {initial : RuntimeState}
    {ensures : Array RuntimeValue → RuntimeState → Prop}
    {aborts : Failure → Prop} :
    wp (function unit declaration body arguments) ensures aborts initial ↔
      ∀ frame, initialFrame? declaration arguments = some frame →
        wpExpr body frame initial fun finalFrame evaluatedState control =>
          ∀ outcome,
            finishControl? declaration.signature.results.size control =
                some outcome →
              match outcome with
              | .returned results =>
                  ensures results
                    (finalizeFunctionState unit declaration.profile initial
                      evaluatedState finalFrame outcome)
              | .threw kind thrown => aborts (kind, thrown) := by
  constructor
  · rintro ⟨normal, failing, -⟩ frame frame_eq finalFrame evaluatedState
      control body_step outcome outcome_eq
    cases outcome with
    | returned results =>
        apply normal results
          (finalizeFunctionState unit declaration.profile initial
            evaluatedState finalFrame (.returned results))
        exact ⟨frame, finalFrame, evaluatedState, control, frame_eq,
          body_step, outcome_eq, rfl⟩
    | threw kind thrown =>
        apply failing (kind, thrown)
        exact ⟨frame, finalFrame, evaluatedState, control,
          finalizeFunctionState unit declaration.profile initial
            evaluatedState finalFrame (.threw kind thrown),
          frame_eq, body_step, outcome_eq, rfl⟩
  · intro calculated
    refine ⟨?_, ?_, fun impossible => impossible⟩
    · rintro results final
        ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
          outcome_eq, finalize_eq⟩
      subst final
      exact calculated frame frame_eq finalFrame evaluatedState control
        body_step (.returned results) outcome_eq
    · rintro ⟨kind, thrown⟩
        ⟨frame, finalFrame, evaluatedState, control, final, frame_eq,
          body_step, outcome_eq, finalize_eq⟩
      exact calculated frame frame_eq finalFrame evaluatedState control
        body_step (.threw kind thrown) outcome_eq

theorem wp_nativeFunction {unit : ExecutableUnit}
    {shape : FunctionShape} {body : ExprDenotation}
    {arguments : Array RuntimeValue} {initial : RuntimeState}
    {ensures : Array RuntimeValue → RuntimeState → Prop}
    {aborts : Failure → Prop} :
    wp (nativeFunction unit shape body arguments) ensures aborts initial ↔
      ∀ frame, nativeInitialFrame? shape arguments = some frame →
        wpExpr body frame initial fun finalFrame evaluatedState control =>
          ∀ outcome,
            finishControl? shape.resultCount control = some outcome →
              match outcome with
              | .returned results =>
                  ensures results
                    (finalizeFunctionState unit shape.profile initial
                      evaluatedState finalFrame outcome)
              | .threw kind thrown => aborts (kind, thrown) := by
  constructor
  · rintro ⟨normal, failing, -⟩ frame frame_eq finalFrame evaluatedState
      control body_step outcome outcome_eq
    cases outcome with
    | returned results =>
        apply normal results
          (finalizeFunctionState unit shape.profile initial
            evaluatedState finalFrame (.returned results))
        exact ⟨frame, finalFrame, evaluatedState, control, frame_eq,
          body_step, outcome_eq, rfl⟩
    | threw kind thrown =>
        apply failing (kind, thrown)
        exact ⟨frame, finalFrame, evaluatedState, control,
          finalizeFunctionState unit shape.profile initial
            evaluatedState finalFrame (.threw kind thrown),
          frame_eq, body_step, outcome_eq, rfl⟩
  · intro calculated
    refine ⟨?_, ?_, fun impossible => impossible⟩
    · rintro results final
        ⟨frame, finalFrame, evaluatedState, control, frame_eq, body_step,
          outcome_eq, finalize_eq⟩
      subst final
      exact calculated frame frame_eq finalFrame evaluatedState control
        body_step (.returned results) outcome_eq
    · rintro ⟨kind, thrown⟩
        ⟨frame, finalFrame, evaluatedState, control, final, frame_eq,
          body_step, outcome_eq, finalize_eq⟩
      exact calculated frame frame_eq finalFrame evaluatedState control
        body_step (.threw kind thrown) outcome_eq

/-! ## Leaves and sequencing -/

theorem wpExpr_value (runtimeValue : RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (value runtimeValue) frame state post ↔
      post frame state (.value runtimeValue) := by
  constructor
  · intro h
    exact h frame state (.value runtimeValue) ⟨rfl, rfl, rfl⟩
  · intro h finalFrame finalState control step
    rcases step with ⟨rfl, rfl, rfl⟩
    exact h

theorem wpExpr_localVar_equations (localId : LocalId)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (localVar localId) frame state post ↔
      ∀ runtimeValue, readLocal? frame localId = some runtimeValue →
        post frame state (.value runtimeValue) := by
  constructor
  · intro h runtimeValue local_eq
    exact h frame state (.value runtimeValue)
      ⟨runtimeValue, local_eq, rfl, rfl, rfl⟩
  · intro h finalFrame finalState control step
    rcases step with ⟨runtimeValue, local_eq, rfl, rfl, rfl⟩
    exact h runtimeValue local_eq

theorem wpExpr_localVar (localId : LocalId)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (localVar localId) frame state post ↔
      match readLocal? frame localId with
      | none => True
      | some runtimeValue => post frame state (.value runtimeValue) := by
  rw [wpExpr_localVar_equations]
  cases local_eq : readLocal? frame localId <;> simp

theorem wpValues_nil (frame : RuntimeFrame)
    (state : RuntimeState) (post : ValuesResult → Prop) :
    wpValues valuesNil frame state post ↔ post (.values state frame []) := by
  simp [wpValues, valuesNil]

theorem wpValues_cons (head : ExprDenotation)
    (tail : ValuesDenotation) (frame : RuntimeFrame) (state : RuntimeState)
    (post : ValuesResult → Prop) :
    wpValues (valuesCons head tail) frame state post ↔
      wpExpr head frame state
        (valueOr (fun headFrame headState control =>
            post (.control headState headFrame control))
          fun headFrame headState runtimeValue =>
            wpValues tail headFrame headState (consValues runtimeValue post)) := by
  constructor
  · intro h headFrame headState control head_step
    cases control with
    | value runtimeValue =>
        intro result tail_step
        cases result with
        | values finalState finalFrame values =>
            exact h _ <| .inr <| .inl
              ⟨headFrame, headState, runtimeValue, finalFrame, finalState,
                values, head_step, tail_step, rfl⟩
        | control finalState finalFrame propagated =>
            exact h _ <| .inr <| .inr
              ⟨headFrame, headState, runtimeValue, finalFrame, finalState,
                propagated, head_step, tail_step, rfl⟩
    | break_ nest runtimeValue =>
        exact h _ <| .inl
          ⟨headFrame, headState, .break_ nest runtimeValue, head_step,
            by simp, rfl⟩
    | continue_ nest =>
        exact h _ <| .inl
          ⟨headFrame, headState, .continue_ nest, head_step, by simp, rfl⟩
    | return_ values =>
        exact h _ <| .inl
          ⟨headFrame, headState, .return_ values, head_step, by simp, rfl⟩
    | throw_ kind thrown =>
        exact h _ <| .inl
          ⟨headFrame, headState, .throw_ kind thrown, head_step, by simp, rfl⟩
  · intro h result step
    rcases step with
      ⟨finalFrame, finalState, control, head_step, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, finalFrame, finalState, values,
        head_step, tail_step, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, finalFrame, finalState, control,
        head_step, tail_step, rfl⟩
    · have propagated := h finalFrame finalState control head_step
      cases abrupt <;> exact propagated
    · exact h headFrame headState (.value runtimeValue) head_step
        (.values finalState finalFrame values) tail_step
    · exact h headFrame headState (.value runtimeValue) head_step
        (.control finalState finalFrame control) tail_step

theorem wpStatements_nil (frame : RuntimeFrame)
    (state : RuntimeState) (post : StatementsResult → Prop) :
    wpStatements statementsNil frame state post ↔ post (.done state frame) := by
  simp [wpStatements, statementsNil]

theorem wpStatements_cons (head : ExprDenotation)
    (tail : StatementsDenotation) (frame : RuntimeFrame)
    (state : RuntimeState) (post : StatementsResult → Prop) :
    wpStatements (statementsCons head tail) frame state post ↔
      wpExpr head frame state
        (valueOr (fun headFrame headState control =>
            post (.control headState headFrame control))
          fun headFrame headState _ =>
            wpStatements tail headFrame headState post) := by
  constructor
  · intro h headFrame headState control head_step
    cases control with
    | value runtimeValue =>
        intro result tail_step
        exact h result <| .inr
          ⟨headFrame, headState, runtimeValue, head_step, tail_step⟩
    | break_ nest runtimeValue =>
        exact h _ <| .inl
          ⟨headFrame, headState, .break_ nest runtimeValue, head_step,
            by simp, rfl⟩
    | continue_ nest =>
        exact h _ <| .inl
          ⟨headFrame, headState, .continue_ nest, head_step, by simp, rfl⟩
    | return_ values =>
        exact h _ <| .inl
          ⟨headFrame, headState, .return_ values, head_step, by simp, rfl⟩
    | throw_ kind thrown =>
        exact h _ <| .inl
          ⟨headFrame, headState, .throw_ kind thrown, head_step, by simp, rfl⟩
  · intro h result step
    rcases step with
      ⟨finalFrame, finalState, control, head_step, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, head_step, tail_step⟩
    · have propagated := h finalFrame finalState control head_step
      cases abrupt <;> exact propagated
    · exact h headFrame headState (.value runtimeValue) head_step result tail_step

/-! ## Operations -/

theorem wpExpr_nativeOperation_equations (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeOperation evaluate operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          (∀ finalFrame finalState runtimeValue,
            evaluate values.toArray operandFrame operandState =
                some (.value finalFrame finalState runtimeValue) →
              post finalFrame finalState (.value runtimeValue)) ∧
          (∀ finalFrame finalState throwKind thrown,
            evaluate values.toArray operandFrame operandState =
                some (.throw_ finalFrame finalState throwKind thrown) →
              post finalFrame finalState (.throw_ throwKind thrown))) := by
  constructor
  · intro h result operands_step
    cases result with
    | control operandState operandFrame propagated =>
        exact h operandFrame operandState propagated <| .inl
          ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩
    | values operandState operandFrame values =>
        refine ⟨?_, ?_⟩
        · intro finalFrame finalState runtimeValue evaluate_eq
          exact h finalFrame finalState (.value runtimeValue) <| .inr
            ⟨operandFrame, operandState, values, operands_step, .inl
              ⟨runtimeValue, evaluate_eq, rfl⟩⟩
        · intro finalFrame finalState throwKind thrown evaluate_eq
          exact h finalFrame finalState (.throw_ throwKind thrown) <| .inr
            ⟨operandFrame, operandState, values, operands_step, .inr
              ⟨throwKind, thrown, evaluate_eq, rfl⟩⟩
  · intro h finalFrame finalState control step
    rcases step with
      ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, operands_step,
        ⟨runtimeValue, evaluate_eq, rfl⟩ |
          ⟨throwKind, thrown, evaluate_eq, rfl⟩⟩
    · exact h (.control finalState finalFrame control) operands_step
    · exact (h (.values operandState operandFrame values) operands_step).1
        finalFrame finalState runtimeValue evaluate_eq
    · exact (h (.values operandState operandFrame values) operands_step).2
        finalFrame finalState throwKind thrown evaluate_eq

theorem wpExpr_nativeOperation (evaluate : NativeEvaluator)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeOperation evaluate operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match evaluate values.toArray operandFrame operandState with
          | none => True
          | some (.value finalFrame finalState runtimeValue) =>
              post finalFrame finalState (.value runtimeValue)
          | some (.throw_ finalFrame finalState throwKind thrown) =>
              post finalFrame finalState (.throw_ throwKind thrown)) := by
  rw [wpExpr_nativeOperation_equations]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases evaluate_eq : evaluate values.toArray operandFrame operandState with
      | none => simp
      | some evaluated =>
          cases evaluated with
          | value finalFrame finalState runtimeValue => simp
          | throw_ finalFrame finalState throwKind thrown =>
              simp only [Option.some.injEq,
                GlobalOperationResult.throw_.injEq]
              constructor
              · intro h
                exact h.2 finalFrame finalState throwKind thrown
                  ⟨rfl, rfl, rfl, rfl⟩
              · intro h
                constructor
                · intro targetFrame targetState runtimeValue impossible
                  cases impossible
                · intro targetFrame targetState targetKind targetThrown equal
                  rcases equal with ⟨rfl, rfl, rfl, rfl⟩
                  exact h

theorem wpExpr_nativeLocalOperation (operation : LocalLocationOperation)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeLocalOperation operation operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match operation.evaluate? values.toArray operandFrame operandState with
          | none => True
          | some (.value finalFrame finalState runtimeValue) =>
              post finalFrame finalState (.value runtimeValue)
          | some (.throw_ finalFrame finalState throwKind thrown) =>
              post finalFrame finalState (.throw_ throwKind thrown)) := by
  simpa [nativeLocalOperation] using
    (wpExpr_nativeOperation operation.evaluate? operands frame state post)

/-- Direct WP entry point for a lowering-resolved reborrow through a local
reference slot.  The place-arena walk and result-type lookup have already
been discharged by the descriptor's agreement certificate. -/
theorem wpExpr_nativeDerefLocalBorrowOperation
    (operation : DerefLocalBorrowOperation)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeDerefLocalBorrowOperation operation operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match operation.evaluate? values.toArray operandFrame operandState with
          | none => True
          | some (.value finalFrame finalState runtimeValue) =>
              post finalFrame finalState (.value runtimeValue)
          | some (.throw_ finalFrame finalState throwKind thrown) =>
              post finalFrame finalState (.throw_ throwKind thrown)) := by
  simpa [nativeDerefLocalBorrowOperation] using
    (wpExpr_nativeOperation operation.evaluate? operands frame state post)

/-- Relational elimination of one partial storage lookup.  This exposes the
single dynamic presence/absence choice without leaving an `Option` matcher
in the proof term. -/
def storageOptionPost (entry : Option RuntimeValue) (onMissing : Prop)
    (onPresent : RuntimeValue → Prop) : Prop :=
  (entry = none → onMissing) ∧
    (∀ value, entry = some value → onPresent value)

/-- Postcondition for a resolved global location.  Resource identity,
reference kind, and lexical loan identity are lowering-time data.  Only the
selected slot's presence and contents remain dynamic. -/
def globalOperationPost (operation : GlobalLocationOperation)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  match operation with
  | .contains resource =>
      match arguments.toList with
      | [key] =>
          match key.storageKey? with
          | none => True
          | some _ =>
              storageOptionPost
                (state.globals.lookup
                  (globalKey resource.namespaceId resource.typeId key))
                (post frame state (.value (.bool false)))
                (fun _ => post frame state (.value (.bool true)))
      | _ => True
  | .borrow site =>
      match arguments.toList with
      | [key] =>
          match key.storageKey? with
          | none => True
          | some _ =>
              storageOptionPost
                (state.globals.lookup
                  (globalKey site.resource.namespaceId site.resource.typeId key))
                (post frame state (.throw_ .abort #[]))
                fun current =>
                  match site.kind with
                  | .immutable =>
                      if site.referenceType.kind != .shared then True
                      else post frame state (.value current)
                  | .mutable =>
                      if site.referenceType.kind != .mutable then True
                      else
                        let loan := state.nextLoan
                        let global := globalKey site.resource.namespaceId
                          site.resource.typeId key
                        post
                          { frame with
                            activeLoans :=
                              (frame.activeLoans.filter
                                (·.1 != ⟨site.lexicalLoan⟩)).push
                                (⟨site.lexicalLoan⟩, loan)
                            loanLocations :=
                              frame.loanLocations.push
                                (loan, { root := .global global }) }
                          { state with
                            globals := state.globals.insert global (.loanHole loan)
                            globalLoans := (loan, global) :: state.globalLoans
                            nextLoan := loan + 1 }
                          (.value (.borrow loan current))
                  | .profile _ => True
      | _ => True
  | .take resource =>
      match arguments.toList with
      | [key] =>
          match key.storageKey? with
          | none => True
          | some _ =>
              let global := globalKey resource.namespaceId resource.typeId key
              storageOptionPost
                (state.globals.lookup global)
                (post frame state (.throw_ .abort #[]))
                (fun value => post frame
                  { state with globals := state.globals.erase global }
                  (.value value))
      | _ => True
  | .publish resource =>
      match arguments.toList with
      | [key, value] =>
          match key.storageKey? with
          | none => True
          | some _ =>
              let global := globalKey resource.namespaceId resource.typeId key
              storageOptionPost
                (state.globals.lookup global)
                (post frame
                  { state with globals := state.globals.insert global value }
                  (.value .unit))
                (fun _ => post frame state (.throw_ .abort #[]))
      | _ => True

private theorem storageOptionPost_iff_match (entry : Option RuntimeValue)
    (onMissing : Prop) (onPresent : RuntimeValue → Prop) :
    (match entry with
      | none => onMissing
      | some value => onPresent value) ↔
      storageOptionPost entry onMissing onPresent := by
  cases entry <;> simp [storageOptionPost]

private theorem globalOperation_post_iff
    (operation : GlobalLocationOperation) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match operation.evaluate? arguments frame state with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)) ↔
      globalOperationPost operation arguments frame state post := by
  cases operation with
  | contains resource =>
      generalize args_eq : arguments.toList = args
      cases args with
      | nil =>
          simp [globalOperationPost, GlobalLocationOperation.evaluate?,
            containsGlobalAt?, args_eq]
      | cons key tail =>
          cases tail with
          | nil =>
              generalize shape_eq : key.storageKey? = shape
              cases shape with
              | none =>
                  simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                    containsGlobalAt?, args_eq, shape_eq]
              | some storageKey =>
                  generalize lookup_eq : state.globals.lookup
                    (globalKey resource.namespaceId resource.typeId key) = entry
                  cases entry <;>
                    simp [globalOperationPost, storageOptionPost, globalValue?,
                      GlobalLocationOperation.evaluate?, containsGlobalAt?,
                      globalExists, args_eq, shape_eq, lookup_eq]
          | cons second rest =>
              simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                containsGlobalAt?, args_eq]
  | borrow site =>
      generalize args_eq : arguments.toList = args
      cases args with
      | nil =>
          simp [globalOperationPost, GlobalLocationOperation.evaluate?,
            borrowGlobalAt?, borrowGlobalUsing?_unfold, args_eq]
      | cons key tail =>
          cases tail with
          | nil =>
              generalize shape_eq : key.storageKey? = shape
              cases shape with
              | none =>
                  simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                    borrowGlobalAt?, borrowGlobalUsing?_unfold, args_eq,
                    shape_eq]
              | some storageKey =>
                  generalize lookup_eq : state.globals.lookup
                    (globalKey site.resource.namespaceId
                      site.resource.typeId key) = entry
                  cases entry with
                  | none =>
                      simp [globalOperationPost, storageOptionPost, globalValue?,
                        GlobalLocationOperation.evaluate?, borrowGlobalAt?,
                        borrowGlobalUsing?_unfold, args_eq, shape_eq, lookup_eq]
                  | some current =>
                      have raw_lookup_eq : state.globals.lookup
                          (globalKey site.resource.namespaceId
                            site.resource.typeId key) = some current := by
                        exact lookup_eq
                      cases kind_eq : site.kind with
                      | immutable =>
                          cases reference_eq : site.referenceType.kind with
                          | shared =>
                              have same_kind :
                                  (ReferenceKind.shared !=
                                    ReferenceKind.shared) = false := by rfl
                              have borrow_eq :=
                                borrowRuntimePlaceAt?_global_immutable_of_lookup
                                  site.lexicalLoan site.referenceType frame state
                                  (globalKey site.resource.namespaceId
                                    site.resource.typeId key) current
                                  reference_eq raw_lookup_eq
                              simp [globalOperationPost, storageOptionPost, globalValue?,
                                GlobalLocationOperation.evaluate?, borrowGlobalAt?,
                                borrowGlobalUsing?_unfold, args_eq, shape_eq,
                                lookup_eq, kind_eq, reference_eq, same_kind,
                                borrow_eq]
                          | mutable =>
                              have different_kind :
                                  (ReferenceKind.mutable !=
                                    ReferenceKind.shared) = true := by rfl
                              simp [globalOperationPost, storageOptionPost, globalValue?,
                                GlobalLocationOperation.evaluate?, borrowGlobalAt?,
                                borrowGlobalUsing?_unfold, args_eq, shape_eq,
                                lookup_eq, kind_eq, reference_eq,
                                different_kind, borrowRuntimePlaceAt?]
                      | mutable =>
                          cases reference_eq : site.referenceType.kind with
                          | shared =>
                              have different_kind :
                                  (ReferenceKind.shared !=
                                    ReferenceKind.mutable) = true := by rfl
                              simp [globalOperationPost, storageOptionPost, globalValue?,
                                GlobalLocationOperation.evaluate?, borrowGlobalAt?,
                                borrowGlobalUsing?_unfold, args_eq, shape_eq,
                                lookup_eq, kind_eq, reference_eq,
                                different_kind, borrowRuntimePlaceAt?]
                          | mutable =>
                              have same_kind :
                                  (ReferenceKind.mutable !=
                                    ReferenceKind.mutable) = false := by rfl
                              have borrow_eq :=
                                borrowRuntimePlaceAt?_global_mutable_of_lookup
                                  site.lexicalLoan site.referenceType frame state
                                  (globalKey site.resource.namespaceId
                                    site.resource.typeId key) current
                                  reference_eq raw_lookup_eq
                              simp [globalOperationPost, storageOptionPost, globalValue?,
                                GlobalLocationOperation.evaluate?, borrowGlobalAt?,
                                borrowGlobalUsing?_unfold, args_eq, shape_eq,
                                lookup_eq, kind_eq, reference_eq, same_kind,
                                borrow_eq]
                      | profile profile =>
                          simp [globalOperationPost, storageOptionPost, globalValue?,
                            GlobalLocationOperation.evaluate?, borrowGlobalAt?,
                            borrowGlobalUsing?_unfold, args_eq, shape_eq,
                            lookup_eq, kind_eq, borrowRuntimePlaceAt?]
          | cons second rest =>
              simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                borrowGlobalAt?, borrowGlobalUsing?_unfold, args_eq]
  | take resource =>
      generalize args_eq : arguments.toList = args
      cases args with
      | nil =>
          simp [globalOperationPost, GlobalLocationOperation.evaluate?,
            takeGlobalAt?, args_eq]
      | cons key tail =>
          cases tail with
          | nil =>
              generalize shape_eq : key.storageKey? = shape
              cases shape with
              | none =>
                  simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                    takeGlobalAt?, args_eq, shape_eq]
              | some storageKey =>
                  generalize lookup_eq : state.globals.lookup
                    (globalKey resource.namespaceId resource.typeId key) = entry
                  cases entry <;>
                    simp [globalOperationPost, storageOptionPost, globalValue?,
                      GlobalLocationOperation.evaluate?, takeGlobalAt?, args_eq,
                      shape_eq, lookup_eq]
          | cons second rest =>
              simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                takeGlobalAt?, args_eq]
  | publish resource =>
      generalize args_eq : arguments.toList = args
      cases args with
      | nil =>
          simp [globalOperationPost, GlobalLocationOperation.evaluate?,
            publishGlobalAt?, args_eq]
      | cons key tail =>
          cases tail with
          | nil =>
              simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                publishGlobalAt?, args_eq]
          | cons value rest =>
              cases rest with
              | nil =>
                  generalize shape_eq : key.storageKey? = shape
                  cases shape with
                  | none =>
                      simp [globalOperationPost,
                        GlobalLocationOperation.evaluate?, publishGlobalAt?,
                        args_eq, shape_eq]
                  | some storageKey =>
                      generalize lookup_eq : state.globals.lookup
                        (globalKey resource.namespaceId resource.typeId key) = entry
                      cases entry <;>
                        simp [globalOperationPost, storageOptionPost, globalValue?,
                          GlobalLocationOperation.evaluate?, publishGlobalAt?,
                          args_eq, shape_eq, lookup_eq]
              | cons third rest =>
                  simp [globalOperationPost, GlobalLocationOperation.evaluate?,
                    publishGlobalAt?, args_eq]

/-- Direct WP for a descriptor-preserving global operation. -/
theorem wpExpr_nativeGlobalOperation
    (operation : GlobalLocationOperation)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeGlobalOperation operation operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          globalOperationPost operation values.toArray
            operandFrame operandState post) := by
  simp only [nativeGlobalOperation]
  rw [wpExpr_nativeOperation]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      exact globalOperation_post_iff operation values.toArray
        operandFrame operandState post

/-- The checked-integer branch as a proposition rather than a residual
`Decidable.rec`.  This is the dynamic part that a VC should retain. -/
def checkedIntegerPost (failure : ThrowKind) (resultType : Ty) (value : Int)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  match resultType.integerBounds? with
  | none => post frame state (.throw_ failure #[])
  | some (lower, upper) =>
      ((lower ≤ value ∧ value ≤ upper) →
        post frame state (.value (.integer value))) ∧
      (¬(lower ≤ value ∧ value ≤ upper) →
        post frame state (.throw_ failure #[.integer value]))

/-- Primitive postcondition after lowering has selected the operation and
result type.  Checked arithmetic exposes only its range split; the remaining
primitive subset is total for validated, constructor-shaped operands and can
reduce directly. -/
def primitiveOperationPost (operation : PrimitiveLocationOperation)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  match operation with
  | .checkedAdd failure resultType =>
      match arguments.toList with
      | [.integer left, .integer right] =>
          checkedIntegerPost failure resultType (left + right) frame state post
      | _ => True
  | .checkedSubtract failure resultType =>
      match arguments.toList with
      | [.integer left, .integer right] =>
          checkedIntegerPost failure resultType (left - right) frame state post
      | _ => True
  | .checkedMultiply failure resultType =>
      match arguments.toList with
      | [.integer left, .integer right] =>
          checkedIntegerPost failure resultType (left * right) frame state post
      | _ => True
  | operation =>
      match operation.evaluate? arguments frame state with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState throwKind thrown) =>
          post finalFrame finalState (.throw_ throwKind thrown)

private theorem checkedInteger_post_iff (failure : ThrowKind)
    (resultType : Ty) (value : Int) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match checkedInteger failure resultType value with
      | .ok runtimeValue => post frame state (.value runtimeValue)
      | .error (kind, thrown) => post frame state (.throw_ kind thrown)) ↔
    checkedIntegerPost failure resultType value frame state post := by
  cases bounds_eq : resultType.integerBounds? with
  | none => simp [checkedIntegerPost, checkedInteger, bounds_eq]
  | some bounds =>
      rcases bounds with ⟨lower, upper⟩
      by_cases in_range : lower ≤ value ∧ value ≤ upper
      · simp [checkedIntegerPost, checkedInteger, bounds_eq, in_range]
      · simp [checkedIntegerPost, checkedInteger, bounds_eq, in_range]

private theorem liftedCheckedInteger_post_iff (failure : ThrowKind)
    (resultType : Ty) (value : Int) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match (match checkedInteger failure resultType value with
      | .ok runtimeValue =>
          some (GlobalOperationResult.value frame state runtimeValue)
      | .error (kind, thrown) =>
          some (GlobalOperationResult.throw_ frame state kind thrown)) with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)) ↔
    checkedIntegerPost failure resultType value frame state post := by
  rw [← checkedInteger_post_iff]
  cases checkedInteger failure resultType value with
  | ok runtimeValue => rfl
  | error failure => cases failure; rfl

private theorem checkedBinaryEvaluator_post_iff (failure : ThrowKind)
    (resultType : Ty) (arguments : Array RuntimeValue)
    (operation : Int → Int → Int) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match (do
        let value ← checkedBinaryInteger failure resultType arguments operation
        match value with
        | .ok value => some (GlobalOperationResult.value frame state value)
        | .error (kind, thrown) =>
            some (GlobalOperationResult.throw_ frame state kind thrown)) with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)) ↔
    match arguments.toList with
    | [.integer left, .integer right] =>
        checkedIntegerPost failure resultType (operation left right)
          frame state post
    | _ => True := by
  generalize args_eq : arguments.toList = args
  cases args with
  | nil => simp [checkedBinaryInteger, args_eq]
  | cons first tail =>
      cases tail with
      | nil => simp [checkedBinaryInteger, args_eq]
      | cons second tail =>
          cases tail with
          | nil =>
              cases first <;> cases second <;>
                simp [checkedBinaryInteger, args_eq] <;>
                try apply liftedCheckedInteger_post_iff
          | cons third tail => simp [checkedBinaryInteger, args_eq]

/-- Direct WP for a descriptor-preserving primitive.  Checked arithmetic is
lowered to a bounds proposition and an exact success/throw continuation;
there is no existential evaluator equation for the proof driver to solve. -/
theorem wpExpr_nativePrimitiveOperation
    (operation : PrimitiveLocationOperation)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativePrimitiveOperation operation operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          primitiveOperationPost operation values.toArray
            operandFrame operandState post) := by
  simp only [nativePrimitiveOperation]
  rw [wpExpr_nativeOperation]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases operation with
      | tuple => rfl
      | copyValue resultType => rfl
      | moveValue resultType => rfl
      | add resultType => rfl
      | subtract resultType => rfl
      | multiply resultType => rfl
      | less resultType => rfl
      | greater resultType => rfl
      | lessEqual resultType => rfl
      | greaterEqual resultType => rfl
      | equal resultType => rfl
      | notEqual resultType => rfl
      | divide resultType => rfl
      | checkedDivide failure resultType => rfl
      | modulo resultType => rfl
      | checkedModulo failure resultType => rfl
      | checkedAdd failure resultType =>
          change
            (match (do
                let value ← checkedBinaryInteger failure resultType
                  values.toArray (fun left right => left + right)
                match value with
                | .ok value =>
                    some (GlobalOperationResult.value operandFrame operandState value)
                | .error (kind, thrown) =>
                    some (GlobalOperationResult.throw_ operandFrame operandState
                      kind thrown)) with
              | none => True
              | some (.value finalFrame finalState runtimeValue) =>
                  post finalFrame finalState (.value runtimeValue)
              | some (.throw_ finalFrame finalState kind thrown) =>
                  post finalFrame finalState (.throw_ kind thrown)) ↔
            match values.toArray.toList with
            | [.integer left, .integer right] =>
                checkedIntegerPost failure resultType (left + right)
                  operandFrame operandState post
            | _ => True
          exact checkedBinaryEvaluator_post_iff failure resultType values.toArray
            (fun left right => left + right) operandFrame operandState post
      | checkedSubtract failure resultType =>
          change
            (match (do
                let value ← checkedBinaryInteger failure resultType
                  values.toArray (fun left right => left - right)
                match value with
                | .ok value =>
                    some (GlobalOperationResult.value operandFrame operandState value)
                | .error (kind, thrown) =>
                    some (GlobalOperationResult.throw_ operandFrame operandState
                      kind thrown)) with
              | none => True
              | some (.value finalFrame finalState runtimeValue) =>
                  post finalFrame finalState (.value runtimeValue)
              | some (.throw_ finalFrame finalState kind thrown) =>
                  post finalFrame finalState (.throw_ kind thrown)) ↔
            match values.toArray.toList with
            | [.integer left, .integer right] =>
                checkedIntegerPost failure resultType (left - right)
                  operandFrame operandState post
            | _ => True
          exact checkedBinaryEvaluator_post_iff failure resultType values.toArray
            (fun left right => left - right) operandFrame operandState post
      | checkedMultiply failure resultType =>
          change
            (match (do
                let value ← checkedBinaryInteger failure resultType
                  values.toArray (fun left right => left * right)
                match value with
                | .ok value =>
                    some (GlobalOperationResult.value operandFrame operandState value)
                | .error (kind, thrown) =>
                    some (GlobalOperationResult.throw_ operandFrame operandState
                      kind thrown)) with
              | none => True
              | some (.value finalFrame finalState runtimeValue) =>
                  post finalFrame finalState (.value runtimeValue)
              | some (.throw_ finalFrame finalState kind thrown) =>
                  post finalFrame finalState (.throw_ kind thrown)) ↔
            match values.toArray.toList with
            | [.integer left, .integer right] =>
                checkedIntegerPost failure resultType (left * right)
                  operandFrame operandState post
            | _ => True
          exact checkedBinaryEvaluator_post_iff failure resultType values.toArray
            (fun left right => left * right) operandFrame operandState post

/-- Native evaluator equation for an explicit death marker on a returned
reborrow.  Keeping this at the descriptor boundary lets equation reduction
select the closed semantic certificate before unfolding the generic
`endLoans?` fold. -/
theorem ReferenceLocationOperation.evaluate?_endLoan_returnedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ loan) :
    (ReferenceLocationOperation.endLoan #[(⟨0⟩ : LoanId)]).evaluate? #[]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state =
      some (.value
        { locals := #[some (.borrow outerLoan current), some .unit]
          activeLoans := #[]
          loanLocations }
        state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    endLoans?_returnedReborrow_zero, separate]

theorem ReferenceLocationOperation.evaluate?_endLoan_returnedReborrow_zero_value
    (state : RuntimeState) (outerLoan loan : Nat) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outerLoan ≠ loan) :
    (ReferenceLocationOperation.endLoan #[(⟨0⟩ : LoanId)]).evaluate?
        #[argument]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state =
      some (.value
        { locals := #[some (.borrow outerLoan current), some .unit]
          activeLoans := #[]
          loanLocations }
        state argument) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    endLoans?_returnedReborrow_zero_value, separate]

/-- Keep an explicit loan-death postcondition folded at the native WP
boundary. The native proof driver accepts only a closed constructor equation
for this head; an unsupported death shape remains as a visible obligation. -/
def endLoanOperationPost (loans : Array LoanId)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  match (ReferenceLocationOperation.endLoan loans).evaluate?
      arguments frame state with
  | none => True
  | some (.value finalFrame finalState runtimeValue) =>
      post finalFrame finalState (.value runtimeValue)
  | some (.throw_ finalFrame finalState kind thrown) =>
      post finalFrame finalState (.throw_ kind thrown)

attribute [irreducible] endLoanOperationPost

/-- Let targeted reconciliation simplify the concrete frame/state arguments
without unfolding the explicit-death post itself. -/
@[congr] theorem endLoanOperationPost_congr
    (loans : Array LoanId) (arguments : Array RuntimeValue)
    {frame nextFrame : RuntimeFrame} {state nextState : RuntimeState}
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (frame_eq : frame = nextFrame) (state_eq : state = nextState) :
    endLoanOperationPost loans arguments frame state post =
      endLoanOperationPost loans arguments nextFrame nextState post := by
  subst nextFrame
  subst nextState
  rfl

theorem endLoanOperationPost_returnedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (separate : outerLoan ≠ loan) :
    endLoanOperationPost #[(⟨0⟩ : LoanId)] #[]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state post =
      post
        { locals := #[some (.borrow outerLoan current), some .unit]
          activeLoans := #[]
          loanLocations }
        state (.value .unit) := by
  simp [endLoanOperationPost,
    ReferenceLocationOperation.evaluate?_endLoan_returnedReborrow_zero,
    separate]

theorem endLoanOperationPost_returnedReborrow_zero_value
    (state : RuntimeState) (outerLoan loan : Nat) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (separate : outerLoan ≠ loan) :
    endLoanOperationPost #[(⟨0⟩ : LoanId)] #[argument]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state post =
      post
        { locals := #[some (.borrow outerLoan current), some .unit]
          activeLoans := #[]
          loanLocations }
        state (.value argument) := by
  simp [endLoanOperationPost,
    ReferenceLocationOperation.evaluate?_endLoan_returnedReborrow_zero_value,
    separate]

theorem endLoanOperationPost_returnedProjectedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat) (name : StructHandle)
    (right : Int) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (separate : outerLoan ≠ loan) :
    endLoanOperationPost #[(⟨0⟩ : LoanId)] #[.unit]
        { locals := #[some (.borrow outerLoan
              (.nominal name none #[.loanHole loan, .integer right])),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state post =
      post
        { locals := #[some (.borrow outerLoan
              (.nominal name none #[current, .integer right])), some .unit]
          activeLoans := #[]
          loanLocations }
        state (.value .unit) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator,
    SemanticOperations.endLoans?_returnedProjectedReborrow_zero, separate]

/-- Close the explicit lifetime death of a projected reference returned from
global storage.  The dynamic loan is routed by the key transferred with its
prophecy hole; the returned reference itself carries no owner path. -/
theorem endLoanOperationPost_returnedGlobalProjection_zero
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (name : StructHandle)
    (right current : Int)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    endLoanOperationPost #[(⟨0⟩ : LoanId)] #[.unit]
        { locals := #[some (.address address),
            some (.borrow loan (.integer current))]
          activeLoans := #[(⟨0⟩, loan)] }
        { globals := globals.insert key
              (.nominal name none #[.loanHole loan, .integer right])
          globalLoans := (loan, key) :: rest
          nextLoan
          pending }
        post =
      post
        { locals := #[some (.address address), some .unit]
          activeLoans := #[] }
        { globals := (globals.insert key
              (.nominal name none #[.loanHole loan, .integer right])).insert key
              (.nominal name none #[.integer current, .integer right])
          globalLoans := rest
          nextLoan
          pending }
        (.value .unit) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator,
    SemanticOperations.endLoans?_returnedGlobalProjection_zero]

/-- Explicit function-exit death of a global borrow held in the third local.
Borrow analysis places this marker after the non-reference result; the keyed
global registry writes the prophetic current value back directly and the frame
keeps only a stale, semantically irrelevant location-cache row. -/
theorem endLoanOperationPost_globalBorrowThirdLocal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (argument : Int)
    (current : RuntimeValue)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (noTransfer : SemanticOperations.transferredLoan? current = none) :
    endLoanOperationPost #[(⟨0⟩ : LoanId)] #[.unit]
        { locals := #[some (.address address), some (.integer argument),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(loan, { root := .global key })] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending }
        post =
      post
        { locals := #[some (.address address), some (.integer argument),
            some .unit]
          activeLoans := #[]
          loanLocations := #[(loan, { root := .global key })] }
        { globals := (globals.insert key (.loanHole loan)).insert key
              current
          globalLoans := rest
          nextLoan
          pending }
        (.value .unit) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator,
    SemanticOperations.endLoans?_globalBorrowThirdLocal,
    SemanticOperations.transferGlobalLoan,
    SemanticOperations.removeGlobalLoan, noTransfer]

/-- Native WP certificate for the paired function-exit deaths of a focused
global field and its enclosing resource loan. -/
theorem endLoanOperationPost_focusedGlobalNominal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount value : Int)
    (outerName innerName : StructHandle) (argument : RuntimeValue)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    endLoanOperationPost #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)),
            some (.borrow loan
              (.nominal outerName none
                #[.nominal innerName none #[.loanHole (loan + 1)]]))]
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨3⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending }
        post =
      post
        { locals := #[some (.address address), some (.integer amount),
            some .unit, some .unit]
          activeLoans := #[]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨3⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := (globals.insert key (.loanHole loan)).insert key
              (.nominal outerName none
                #[.nominal innerName none #[.integer value]])
          globalLoans := rest
          nextLoan
          pending }
        (.value argument) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator, SemanticOperations.endLoans?_focusedGlobalNominal]

/-- Saved-value form of `endLoanOperationPost_focusedGlobalNominal`. -/
theorem endLoanOperationPost_focusedGlobalNominalSaved
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (amount saved value : Int)
    (outerName innerName : StructHandle) (argument : RuntimeValue)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    endLoanOperationPost #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)), some (.integer saved),
            some (.borrow loan
              (.nominal outerName none
                #[.nominal innerName none #[.loanHole (loan + 1)]]))]
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending }
        post =
      post
        { locals := #[some (.address address), some (.integer amount),
            some .unit, some (.integer saved), some .unit]
          activeLoans := #[]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := (globals.insert key (.loanHole loan)).insert key
              (.nominal outerName none
                #[.nominal innerName none #[.integer value]])
          globalLoans := rest
          nextLoan
          pending }
        (.value argument) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator,
    SemanticOperations.endLoans?_focusedGlobalNominalSaved]

/-- Abort-path spelling of the saved focused-global frame.  No evaluator
follows the guarded abort to normalize its prepared local writes, so this
certificate absorbs those closed constructor updates while leaving the
continuation opaque. -/
theorem endLoanOperationPost_focusedGlobalNominalSavedPrepared
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String)
    (amount : SpecInt (.bits 64) false) (saved : Int)
    (outerName innerName : StructHandle) (argument : RuntimeValue)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    endLoanOperationPost #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals :=
            (((#[some (Codec.address.encode address),
                  some ((Codec.specInt (.bits 64) false).encode amount),
                  none, none, none]
                  |>.setIfInBounds 4
                    (some (.borrow loan
                      (.nominal outerName none
                        #[.nominal innerName none #[.integer saved]]))))
                |>.setIfInBounds 4
                  (some (.borrow loan
                    (.nominal outerName none
                      #[.nominal innerName none #[.loanHole (loan + 1)]]))))
              |>.setIfInBounds 2
                (some (.borrow (loan + 1) #[.integer saved][0])))
            |>.setIfInBounds 3 (some #[.integer saved][0])
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations :=
            ((parameterLoanLocations #[Codec.address.encode address,
                (Codec.specInt (.bits 64) false).encode amount]).push
              (loan, { root := .global key })).push
              (loan + 1,
                { root := .local ⟨4⟩
                  projections := #[.deref, .field 0].push (.field 0) }) }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending }
        post =
      post
        { locals := #[some (Codec.address.encode address),
            some ((Codec.specInt (.bits 64) false).encode amount),
            some .unit, some (.integer saved), some .unit]
          activeLoans := #[]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0, .field 0], true⟩ :
                RuntimePlace))] }
        { globals := (globals.insert key (.loanHole loan)).insert key
              (.nominal outerName none
                #[.nominal innerName none #[.integer saved]])
          globalLoans := rest
          nextLoan
          pending }
        (.value argument) := by
  simp only [Codec.address, Codec.specInt,
    SemanticOperations.parameterLoanLocations_addressInteger,
    SemanticOperations.array_five_setIfInBounds_two,
    SemanticOperations.array_five_setIfInBounds_three,
    SemanticOperations.array_five_setIfInBounds_four,
    SemanticOperations.array_singleton_getElem_zero,
    SemanticOperations.array_empty_push,
    SemanticOperations.array_singleton_push,
    SemanticOperations.array_pair_push]
  rw [endLoanOperationPost_focusedGlobalNominalSaved]

/-- A call that fully reconciled an ordinary scalar reborrow has already
retired the marker's dynamic loan.  The following explicit marker is thus a
native no-op; its frame is normalized here rather than by unfolding the
generic loan fold. -/
theorem endLoanOperationPost_retiredDerefLocalZero_integer
    (state : RuntimeState) (lexical outerLoan loan : Nat) (value : Int)
    (locals : Array (Option RuntimeValue))
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (separate : outerLoan ≠ loan) :
    endLoanOperationPost #[(⟨lexical⟩ : LoanId)] #[packResults #[]]
        { locals
          activeLoans := transferActiveLoan #[(⟨lexical⟩, loan)] loan
            (.integer value)
          loanLocations := transferLoanLocation
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (loan,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))]
            loan (.integer value) }
        state post =
      post
        { locals
          activeLoans := #[]
          loanLocations :=
            #[(outerLoan,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        state (.value (packResults #[])) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator, endLoans?, transferActiveLoan, transferLoanLocation,
    transferredLoan?, findFirst, Array.filter, separate]

/-- Closed two-parameter call boundary followed by its explicit death
marker.  Both callee loans are reconciled into their lenders first; the
borrow-analysis marker then observes that both lexical rows are already
retired and passes the unit result through. -/
theorem endLoanOperationPost_twoRetiredDerefLocals_integer
    (initial : RuntimeState) (leftOuter rightOuter : Nat)
    (left right : Int)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (freshGlobal : FreshGlobalLoanIds initial)
    (leftPrior : leftOuter < initial.nextLoan)
    (rightPrior : rightOuter < initial.nextLoan) :
    endLoanOperationPost #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)]
        #[packResults #[]]
        (applyPendingFrom initial.pending
          { locals := #[some (.borrow leftOuter
                (.loanHole initial.nextLoan)),
              some (.borrow rightOuter
                (.loanHole (initial.nextLoan + 1)))]
            activeLoans := #[(⟨0⟩, initial.nextLoan),
              (⟨1⟩, initial.nextLoan + 1)]
            loanLocations :=
              (#[(leftOuter,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (rightOuter,
                    (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))].push
                (initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))).push
                (initial.nextLoan + 1,
                  (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)) }
          (exportFrameLoans
            { locals := #[some (.borrow initial.nextLoan (.integer left)),
                some (.borrow (initial.nextLoan + 1) (.integer right))]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
            { globals := initial.globals
              globalLoans := initial.globalLoans
              nextLoan := initial.nextLoan + 1 + 1
              pending := initial.pending })).fst
        (applyPendingFrom initial.pending
          { locals := #[some (.borrow leftOuter
                (.loanHole initial.nextLoan)),
              some (.borrow rightOuter
                (.loanHole (initial.nextLoan + 1)))]
            activeLoans := #[(⟨0⟩, initial.nextLoan),
              (⟨1⟩, initial.nextLoan + 1)]
            loanLocations :=
              (#[(leftOuter,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (rightOuter,
                    (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))].push
                (initial.nextLoan,
                  (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))).push
                (initial.nextLoan + 1,
                  (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)) }
          (exportFrameLoans
            { locals := #[some (.borrow initial.nextLoan (.integer left)),
                some (.borrow (initial.nextLoan + 1) (.integer right))]
              loanLocations :=
                #[(initial.nextLoan,
                    (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
                  (initial.nextLoan + 1,
                    (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
            { globals := initial.globals
              globalLoans := initial.globalLoans
              nextLoan := initial.nextLoan + 1 + 1
              pending := initial.pending })).snd
        post =
      post
        { locals := #[some (.borrow leftOuter (.integer left)),
            some (.borrow rightOuter (.integer right))]
          activeLoans := #[]
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace))] }
        { globals := initial.globals
          globalLoans := initial.globalLoans
          nextLoan := initial.nextLoan + 1 + 1
          pending := initial.pending }
        (.value (packResults #[])) := by
  have noLeftGlobal :
      globalLoanKey?
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := initial.nextLoan + 1 + 1
            pending := initial.pending }
          initial.nextLoan = none :=
    FreshGlobalLoanIds.lookup_next freshGlobal
  have noRightGlobal :
      globalLoanKey?
          { globals := initial.globals
            globalLoans := initial.globalLoans
            nextLoan := initial.nextLoan + 1 + 1
            pending := initial.pending }
          (initial.nextLoan + 1) = none :=
    FreshGlobalLoanIds.lookup_add freshGlobal 1
  rw [exportFrameLoans_twoIntegers_state _ _ _ _ _ _ _
    noLeftGlobal noRightGlobal]
  have noLeftTransfer : transferredLoan? (.integer left) = none := by
    simp [transferredLoan?, findFirst]
  have noRightTransfer : transferredLoan? (.integer right) = none := by
    simp [transferredLoan?, findFirst]
  simp only [array_pair_push, array_triple_push]
  rw [applyPendingFrom_twoDerefLocals initial.pending initial.globals
    initial.globalLoans (initial.nextLoan + 1 + 1) leftOuter rightOuter
    initial.nextLoan (initial.nextLoan + 1) (.integer left) (.integer right)
    noLeftTransfer noRightTransfer]
  all_goals try omega
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator, endLoans?]

/-- Explicit joint death for two returned mutable references.  Borrow
analysis names both lexical sites; the native certificate settles their
currents into the two parameter holes and clears the resting result locals. -/
theorem endLoanOperationPost_twoReturnedReborrows_integer
    (state : RuntimeState) (leftOuter rightOuter nextLoan : Nat)
    (left right : Int)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (leftPrior : leftOuter < nextLoan)
    (rightPrior : rightOuter < nextLoan) :
    endLoanOperationPost #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[.unit]
        { locals := #[some (.borrow leftOuter (.loanHole (nextLoan + 1 + 1))),
            some (.borrow rightOuter (.loanHole (nextLoan + 1 + 1 + 1))),
            some (.borrow (nextLoan + 1 + 1) (.integer left)),
            some (.borrow (nextLoan + 1 + 1 + 1) (.integer right))]
          activeLoans := #[(⟨0⟩, nextLoan + 1 + 1),
            (⟨1⟩, nextLoan + 1 + 1 + 1)]
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (nextLoan + 1 + 1,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (nextLoan + 1 + 1 + 1,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state post =
      post
        { locals := #[some (.borrow leftOuter (.integer left)),
            some (.borrow rightOuter (.integer right)),
            some .unit, some .unit]
          activeLoans := #[]
          loanLocations :=
            #[(leftOuter,
                (⟨.local (⟨0⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (rightOuter,
                (⟨.local (⟨1⟩ : LocalId), #[], true⟩ : RuntimePlace)),
              (nextLoan + 1 + 1,
                (⟨.local (⟨0⟩ : LocalId), #[.deref], true⟩ : RuntimePlace)),
              (nextLoan + 1 + 1 + 1,
                (⟨.local (⟨1⟩ : LocalId), #[.deref], true⟩ : RuntimePlace))] }
        state (.value .unit) := by
  simp [endLoanOperationPost, ReferenceLocationOperation.evaluate?,
    liftPlaceEvaluator, SemanticOperations.endLoans?_twoReturnedReborrows,
    leftPrior, rightPrior]

/-- Direct postcondition for a resolved reference operation.  Dereference
selects the carried value immediately.  Mutation retains only the genuine
dynamic choice between updating a resting borrow and writing back a consumed
one; evaluator-result reconstruction is gone. -/
def referenceOperationPost (operation : ReferenceLocationOperation)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) : Prop :=
  match operation with
  | .dereference =>
      match arguments.toList with
      | [.borrow _ current] => post frame state (.value current)
      | _ => True
  | .mutate =>
      match arguments.toList with
      | [.borrow loan _current, value] =>
          match updateBorrowValue? frame state loan value with
          | some (finalFrame, finalState) =>
              post finalFrame finalState (.value .unit)
          | none =>
              let (finalFrame, finalState) :=
                applyWriteBack frame state loan value
              post finalFrame finalState (.value .unit)
      | _ => True
  | .endLoan loans => endLoanOperationPost loans arguments frame state post
  | operation =>
      match operation.evaluate? arguments frame state with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)

/-- WP-facing form of the returned-reborrow death certificate.  It rewrites
the whole reference-operation post before head reduction can unfold the
generic loan fold into the continuation. -/
theorem referenceOperationPost_endLoan_returnedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat) (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (separate : outerLoan ≠ loan) :
    referenceOperationPost
        (.endLoan #[(⟨0⟩ : LoanId)]) #[]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state post =
      post
        { locals := #[some (.borrow outerLoan current), some .unit]
          activeLoans := #[]
          loanLocations }
        state (.value .unit) := by
  simp [referenceOperationPost, endLoanOperationPost,
    ReferenceLocationOperation.evaluate?_endLoan_returnedReborrow_zero,
    separate]

theorem referenceOperationPost_endLoan_returnedReborrow_zero_value
    (state : RuntimeState) (outerLoan loan : Nat) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (separate : outerLoan ≠ loan) :
    referenceOperationPost
        (.endLoan #[(⟨0⟩ : LoanId)]) #[argument]
        { locals := #[some (.borrow outerLoan (.loanHole loan)),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations }
        state post =
      post
        { locals := #[some (.borrow outerLoan current), some .unit]
          activeLoans := #[]
          loanLocations }
        state (.value argument) := by
  simp [referenceOperationPost, endLoanOperationPost,
    ReferenceLocationOperation.evaluate?_endLoan_returnedReborrow_zero_value,
    separate]

private theorem liftedMutation_post_iff
    (updated : Option (RuntimeFrame × RuntimeState))
    (fallback : RuntimeFrame × RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match (match updated with
      | some (frame, state) => some (frame, state, RuntimeValue.unit)
      | none => some (fallback.1, fallback.2, RuntimeValue.unit)).bind
        (fun result => some (GlobalOperationResult.value
          result.1 result.2.1 result.2.2)) with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)) ↔
    match updated with
    | some (finalFrame, finalState) =>
        post finalFrame finalState (.value .unit)
    | none => post fallback.1 fallback.2 (.value .unit) := by
  cases updated with
  | none => rfl
  | some result => cases result; rfl

private theorem referenceMutate_post_iff (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match ReferenceLocationOperation.mutate.evaluate? arguments frame state with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)) ↔
    referenceOperationPost .mutate arguments frame state post := by
  generalize args_eq : arguments.toList = args
  cases args with
  | nil =>
      simp [referenceOperationPost, ReferenceLocationOperation.evaluate?,
        liftPlaceEvaluator, mutateBorrow?, args_eq]
  | cons first tail =>
      cases tail with
      | nil =>
          simp [referenceOperationPost, ReferenceLocationOperation.evaluate?,
            liftPlaceEvaluator, mutateBorrow?, args_eq]
      | cons second tail =>
          cases tail with
          | nil =>
              cases first <;> cases second <;>
                simp [referenceOperationPost,
                  ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
                  mutateBorrow?, args_eq] <;>
                try apply liftedMutation_post_iff
          | cons third tail =>
              simp [referenceOperationPost,
                ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
                mutateBorrow?, args_eq]

private theorem referenceDereference_post_iff
    (arguments : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    (match ReferenceLocationOperation.dereference.evaluate?
        arguments frame state with
      | none => True
      | some (.value finalFrame finalState runtimeValue) =>
          post finalFrame finalState (.value runtimeValue)
      | some (.throw_ finalFrame finalState kind thrown) =>
          post finalFrame finalState (.throw_ kind thrown)) ↔
    referenceOperationPost .dereference arguments frame state post := by
  generalize args_eq : arguments.toList = args
  cases args with
  | nil =>
      simp [referenceOperationPost, ReferenceLocationOperation.evaluate?,
        liftPlaceEvaluator, dereferenceBorrow?, args_eq]
  | cons first tail =>
      cases tail with
      | nil =>
          cases first <;>
            simp [referenceOperationPost,
              ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
              dereferenceBorrow?, args_eq]
      | cons second tail =>
          simp [referenceOperationPost, ReferenceLocationOperation.evaluate?,
            liftPlaceEvaluator, dereferenceBorrow?, args_eq]

/-- Direct WP for a descriptor-preserving reference operation. -/
theorem wpExpr_nativeReferenceOperation
    (operation : ReferenceLocationOperation)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeReferenceOperation operation operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          referenceOperationPost operation values.toArray
            operandFrame operandState post) := by
  simp only [nativeReferenceOperation]
  rw [wpExpr_nativeOperation]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases operation with
      | dereference =>
          exact referenceDereference_post_iff values.toArray
            operandFrame operandState post
      | mutate =>
          exact referenceMutate_post_iff values.toArray
            operandFrame operandState post
      | freeze resultType => rfl
      | endLoan loans =>
          simp [referenceOperationPost, endLoanOperationPost]

theorem wpExpr_primitive_equations (ns : ValidatedNamespace)
    (resultType : TypeId) (operation : PrimitiveOperation)
    (operands : ValuesDenotation) (pointerWidth : Option Nat)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (primitive ns resultType operation operands pointerWidth)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          (∀ runtimeValue,
            evaluatePrimitiveOperation? ns resultType operation values.toArray
                pointerWidth = some (.ok runtimeValue) →
              post operandFrame operandState (.value runtimeValue)) ∧
          (∀ kind thrown,
            evaluatePrimitiveOperation? ns resultType operation values.toArray
                pointerWidth = some (.error (kind, thrown)) →
              post operandFrame operandState (.throw_ kind thrown))) := by
  constructor
  · intro h result operands_step
    cases result with
    | control operandState operandFrame propagated =>
        exact h operandFrame operandState propagated <| .inl
          ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩
    | values operandState operandFrame values =>
        refine ⟨?_, ?_⟩
        · intro runtimeValue evaluate_eq
          exact h operandFrame operandState (.value runtimeValue) <| .inr
            ⟨operandFrame, operandState, values, operands_step, .inl
              ⟨runtimeValue, evaluate_eq, rfl, rfl, rfl⟩⟩
        · intro kind thrown evaluate_eq
          exact h operandFrame operandState (.throw_ kind thrown) <| .inr
            ⟨operandFrame, operandState, values, operands_step, .inr
              ⟨kind, thrown, evaluate_eq, rfl, rfl, rfl⟩⟩
  · intro h finalFrame finalState control step
    rcases step with
      ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, operands_step,
        ⟨runtimeValue, evaluate_eq, rfl, rfl, rfl⟩ |
          ⟨kind, thrown, evaluate_eq, rfl, rfl, rfl⟩⟩
    · exact h (.control finalState finalFrame control) operands_step
    · exact (h (.values finalState finalFrame values) operands_step).1
        runtimeValue evaluate_eq
    · exact (h (.values finalState finalFrame values) operands_step).2
        kind thrown evaluate_eq

theorem wpExpr_primitive (ns : ValidatedNamespace)
    (resultType : TypeId) (operation : PrimitiveOperation)
    (operands : ValuesDenotation) (pointerWidth : Option Nat)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (primitive ns resultType operation operands pointerWidth)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match evaluatePrimitiveOperation? ns resultType operation
              values.toArray pointerWidth with
          | none => True
          | some (.ok runtimeValue) =>
              post operandFrame operandState (.value runtimeValue)
          | some (.error (kind, thrown)) =>
              post operandFrame operandState (.throw_ kind thrown)) := by
  rw [wpExpr_primitive_equations]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases evaluate_eq : evaluatePrimitiveOperation? ns resultType operation
          values.toArray pointerWidth with
      | none => simp
      | some evaluated =>
          cases evaluated with
          | ok runtimeValue => simp
          | error failure =>
              rcases failure with ⟨kind, thrown⟩
              simp

theorem wpExpr_global_equations (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (kind : GlobalKind) (instantiations : Array GenericArgument)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (global unit ns resultType site kind instantiations operands)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          (∀ finalFrame finalState runtimeValue,
            evaluateGlobalOperation? unit ns resultType site kind instantiations
                values.toArray operandFrame operandState =
                  some (.value finalFrame finalState runtimeValue) →
              post finalFrame finalState (.value runtimeValue)) ∧
          (∀ finalFrame finalState throwKind thrown,
            evaluateGlobalOperation? unit ns resultType site kind instantiations
                values.toArray operandFrame operandState =
                  some (.throw_ finalFrame finalState throwKind thrown) →
              post finalFrame finalState (.throw_ throwKind thrown))) := by
  constructor
  · intro h result operands_step
    cases result with
    | control operandState operandFrame propagated =>
        exact h operandFrame operandState propagated <| .inl
          ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩
    | values operandState operandFrame values =>
        refine ⟨?_, ?_⟩
        · intro finalFrame finalState runtimeValue evaluate_eq
          exact h finalFrame finalState (.value runtimeValue) <| .inr
            ⟨operandFrame, operandState, values, operands_step, .inl
              ⟨runtimeValue, evaluate_eq, rfl⟩⟩
        · intro finalFrame finalState throwKind thrown evaluate_eq
          exact h finalFrame finalState (.throw_ throwKind thrown) <| .inr
            ⟨operandFrame, operandState, values, operands_step, .inr
              ⟨throwKind, thrown, evaluate_eq, rfl⟩⟩
  · intro h finalFrame finalState control step
    rcases step with
      ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, operands_step,
        ⟨runtimeValue, evaluate_eq, rfl⟩ |
          ⟨throwKind, thrown, evaluate_eq, rfl⟩⟩
    · exact h (.control finalState finalFrame control) operands_step
    · exact (h (.values operandState operandFrame values) operands_step).1
        finalFrame finalState runtimeValue evaluate_eq
    · exact (h (.values operandState operandFrame values) operands_step).2
        finalFrame finalState throwKind thrown evaluate_eq

theorem wpExpr_global (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (kind : GlobalKind) (instantiations : Array GenericArgument)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (global unit ns resultType site kind instantiations operands)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match evaluateGlobalOperation? unit ns resultType site kind
              instantiations values.toArray operandFrame operandState with
          | none => True
          | some (.value finalFrame finalState runtimeValue) =>
              post finalFrame finalState (.value runtimeValue)
          | some (.throw_ finalFrame finalState throwKind thrown) =>
              post finalFrame finalState (.throw_ throwKind thrown)) := by
  rw [wpExpr_global_equations]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases evaluate_eq : evaluateGlobalOperation? unit ns resultType site kind
          instantiations values.toArray operandFrame operandState with
      | none => simp
      | some evaluated =>
          cases evaluated with
          | value finalFrame finalState runtimeValue => simp
          | throw_ finalFrame finalState throwKind thrown =>
              simp only [Option.some.injEq,
                GlobalOperationResult.throw_.injEq]
              constructor
              · intro h
                exact h.2 finalFrame finalState throwKind thrown
                  ⟨rfl, rfl, rfl, rfl⟩
              · intro h
                constructor
                · intro targetFrame targetState runtimeValue impossible
                  cases impossible
                · intro targetFrame targetState targetKind targetThrown equal
                  rcases equal with ⟨rfl, rfl, rfl, rfl⟩
                  exact h

theorem wpExpr_placeOperation_equations (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (operation : Operation) (operands : ValuesDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (placeOperation unit ns resultType site operation operands)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          ∀ finalFrame finalState runtimeValue,
            evaluatePlaceOperation? unit ns resultType site operation
                values.toArray operandFrame operandState =
                  some (finalFrame, finalState, runtimeValue) →
              post finalFrame finalState (.value runtimeValue)) := by
  constructor
  · intro h result operands_step
    cases result with
    | control operandState operandFrame propagated =>
        exact h operandFrame operandState propagated <| .inl
          ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩
    | values operandState operandFrame values =>
        intro finalFrame finalState runtimeValue evaluate_eq
        exact h finalFrame finalState (.value runtimeValue) <| .inr
          ⟨operandFrame, operandState, values, runtimeValue, operands_step,
            evaluate_eq, rfl⟩
  · intro h finalFrame finalState control step
    rcases step with
      ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, runtimeValue, operands_step,
        evaluate_eq, rfl⟩
    · exact h (.control finalState finalFrame control) operands_step
    · exact h (.values operandState operandFrame values) operands_step
        finalFrame finalState runtimeValue evaluate_eq

theorem wpExpr_placeOperation (unit : ValidatedUnit)
    (ns : ValidatedNamespace) (resultType : TypeId) (site : ExprId)
    (operation : Operation) (operands : ValuesDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (placeOperation unit ns resultType site operation operands)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match evaluatePlaceOperation? unit ns resultType site operation
              values.toArray operandFrame operandState with
          | none => True
          | some (finalFrame, finalState, runtimeValue) =>
              post finalFrame finalState (.value runtimeValue)) := by
  rw [wpExpr_placeOperation_equations]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases evaluate_eq : evaluatePlaceOperation? unit ns resultType site
          operation values.toArray operandFrame operandState with
      | none => simp
      | some evaluated =>
          rcases evaluated with ⟨finalFrame, finalState, runtimeValue⟩
          simp

theorem wpExpr_constructor_equations (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (reference : QualifiedRef)
    (variant : Option String) (operands : ValuesDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (constructor unit namespaceId reference variant operands)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          ∀ runtimeValue,
            constructNominal? unit namespaceId reference variant values.toArray =
                some runtimeValue →
              post operandFrame operandState (.value runtimeValue)) := by
  constructor
  · intro h result operands_step
    cases result with
    | control operandState operandFrame propagated =>
        exact h operandFrame operandState propagated <| .inl
          ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩
    | values operandState operandFrame values =>
        intro runtimeValue construct_eq
        exact h operandFrame operandState (.value runtimeValue) <| .inr
          ⟨operandFrame, operandState, values, runtimeValue, operands_step,
            construct_eq, rfl, rfl, rfl⟩
  · intro h finalFrame finalState control step
    rcases step with
      ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, runtimeValue, operands_step,
        construct_eq, rfl, rfl, rfl⟩
    · exact h (.control finalState finalFrame control) operands_step
    · exact h (.values finalState finalFrame values) operands_step
        runtimeValue construct_eq

theorem wpExpr_constructor (unit : ValidatedUnit)
    (namespaceId : NamespaceId) (reference : QualifiedRef)
    (variant : Option String) (operands : ValuesDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (constructor unit namespaceId reference variant operands)
        frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          match constructNominal? unit namespaceId reference variant
              values.toArray with
          | none => True
          | some runtimeValue =>
              post operandFrame operandState (.value runtimeValue)) := by
  rw [wpExpr_constructor_equations]
  apply wpValues_post_congr
  intro result
  cases result with
  | control => rfl
  | values operandState operandFrame values =>
      simp only [valuesPost]
      cases construct_eq : constructNominal? unit namespaceId reference variant
          values.toArray <;> simp

theorem wpExpr_call (lexical : Option Nat) (callee : FunctionDenotation)
    (operands : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (call lexical callee operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          wpFunction callee operandState values.toArray fun calleeState outcome =>
            post (callFrame lexical outcome
                (applyPendingFrom operandState.pending operandFrame calleeState).1)
              (applyPendingFrom operandState.pending operandFrame calleeState).2
              (callControl outcome)) := by
  constructor
  · intro h result operands_step
    cases result with
    | control operandState operandFrame propagated =>
        exact h operandFrame operandState propagated <| .inl
          ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩
    | values operandState operandFrame values =>
        intro calleeState outcome callee_step
        exact h (callFrame lexical outcome
            (applyPendingFrom operandState.pending operandFrame calleeState).1)
          (applyPendingFrom operandState.pending operandFrame calleeState).2
          (callControl outcome) <|
            .inr ⟨operandFrame, operandState, values, calleeState, outcome,
              operands_step, callee_step, rfl, rfl, rfl⟩
  · intro h finalFrame finalState control step
    rcases step with
      ⟨operandFrame, operandState, propagated, operands_step, rfl, rfl, rfl⟩ |
      ⟨operandFrame, operandState, values, calleeState, outcome,
        operands_step, callee_step, rfl, rfl, rfl⟩
    · exact h (.control finalState finalFrame control) operands_step
    · exact h (.values operandState operandFrame values) operands_step
        calleeState outcome callee_step

theorem wpExpr_nativeCall (handle : FunctionHandle) (lexical : Option Nat)
    (callee : FunctionDenotation) (operands : ValuesDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeCall handle lexical callee operands) frame state post ↔
      wpValues operands frame state
        (valuesPost post fun operandFrame operandState values =>
          wpFunction callee operandState values.toArray fun calleeState outcome =>
            post (callFrame lexical outcome
                (applyPendingFrom operandState.pending operandFrame calleeState).1)
              (applyPendingFrom operandState.pending operandFrame calleeState).2
              (callControl outcome)) := by
  simpa [nativeCall] using wpExpr_call lexical callee operands frame state post

/-! ## Function control -/

theorem wpExpr_nativeReturn
    (values : ValuesDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeReturn values) frame state post ↔
      wpValues values frame state
        (valuesPost post fun finalFrame finalState runtimeValues =>
          post finalFrame finalState (.return_ runtimeValues.toArray)) := by
  constructor
  · intro h result values_step
    cases result with
    | values finalState finalFrame runtimeValues =>
        exact h finalFrame finalState (.return_ runtimeValues.toArray) <|
          .inl ⟨runtimeValues, values_step, rfl⟩
    | control finalState finalFrame control =>
        exact h finalFrame finalState control (.inr values_step)
  · intro h finalFrame finalState control step
    rcases step with ⟨runtimeValues, values_step, rfl⟩ | values_step
    · exact h (.values finalState finalFrame runtimeValues) values_step
    · exact h (.control finalState finalFrame control) values_step

theorem wpExpr_nativeThrow
    (kind : ThrowKind) (arguments : ValuesDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeThrow kind arguments) frame state post ↔
      wpValues arguments frame state
        (valuesPost post fun finalFrame finalState runtimeValues =>
          post finalFrame finalState (.throw_ kind runtimeValues.toArray)) := by
  constructor
  · intro h result arguments_step
    cases result with
    | values finalState finalFrame runtimeValues =>
        exact h finalFrame finalState (.throw_ kind runtimeValues.toArray) <|
          .inl ⟨runtimeValues, arguments_step, rfl⟩
    | control finalState finalFrame control =>
        exact h finalFrame finalState control (.inr arguments_step)
  · intro h finalFrame finalState control step
    rcases step with ⟨runtimeValues, arguments_step, rfl⟩ | arguments_step
    · exact h (.values finalState finalFrame runtimeValues) arguments_step
    · exact h (.control finalState finalFrame control) arguments_step

/-! ## Native loops -/

/-- The one-iteration continuation expected from a loop body.  Ordinary
values and a level-zero `continue` re-establish the invariant.  A level-zero
`break` exits as a value, while outer control and function control propagate
with exactly the adjustment made by `NativeLoop`.

This predicate is a native Lean term: it contains neither an expression id
nor an interpreter/fuel argument. -/
def loopInvariantPost
    (invariant : RuntimeFrame → RuntimeState → Prop)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (frame : RuntimeFrame) (state : RuntimeState) : Control → Prop
  | .value _ => invariant frame state
  | .continue_ 0 => invariant frame state
  | .continue_ (nest + 1) => post frame state (.continue_ nest)
  | .break_ 0 value => post frame state (.value (value.getD .unit))
  | .break_ (nest + 1) value => post frame state (.break_ nest value)
  | .return_ values => post frame state (.return_ values)
  | .throw_ kind arguments => post frame state (.throw_ kind arguments)

/-- Verify a native loop by one elaborated Lean invariant proof.  The proof
recurses over the finite `NativeLoop` derivation, never by symbolic execution
or unrolling of the source loop. -/
theorem wpExpr_nativeLoop_of_invariant
    (site : ExprId) (body : ExprDenotation)
    (invariant : RuntimeFrame → RuntimeState → Prop)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (entry : invariant frame state)
    (preserved : ∀ bodyFrame bodyState,
      invariant bodyFrame bodyState →
        wpExpr body bodyFrame bodyState (loopInvariantPost invariant post)) :
    wpExpr (nativeLoop site body) frame state post := by
  intro finalFrame finalState control step
  change NativeLoop body frame state finalFrame finalState control at step
  revert entry
  induction step with
  | repeatValue value control body_step repeat_step ih =>
      intro entry
      apply ih
      exact preserved _ _ entry _ _ _ body_step
  | repeatContinue control body_step repeat_step ih =>
      intro entry
      apply ih
      exact preserved _ _ entry _ _ _ body_step
  | outerContinue nest body_step =>
      intro entry
      exact preserved _ _ entry _ _ _ body_step
  | break_ breakValue body_step =>
      intro entry
      exact preserved _ _ entry _ _ _ body_step
  | outerBreak nest breakValue body_step =>
      intro entry
      exact preserved _ _ entry _ _ _ body_step
  | return_ values body_step =>
      intro entry
      exact preserved _ _ entry _ _ _ body_step
  | throw_ kind arguments body_step =>
      intro entry
      exact preserved _ _ entry _ _ _ body_step

theorem wpExpr_nativeContinue
    (nest : Nat) (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeContinue nest) frame state post ↔
      post frame state (.continue_ nest) := by
  constructor
  · intro h
    exact h frame state (.continue_ nest) ⟨rfl, rfl, rfl⟩
  · rintro h finalFrame finalState control ⟨rfl, rfl, rfl⟩
    exact h

theorem wpExpr_nativeBreakNone
    (nest : Nat) (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeBreak nest none) frame state post ↔
      post frame state (.break_ nest none) := by
  constructor
  · intro h
    exact h frame state (.break_ nest none) ⟨rfl, rfl, rfl⟩
  · rintro h finalFrame finalState control ⟨rfl, rfl, rfl⟩
    exact h

theorem wpExpr_nativeSpec
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr nativeSpec frame state post ↔ post frame state (.value .unit) := by
  simpa [nativeSpec] using wpExpr_value (.unit) frame state post

/-- Postcondition selected after evaluating the right-hand side of a closed
local assignment.  An out-of-range local cannot take a native step and is
therefore a vacuous branch of the weakest precondition. -/
def assignLocalPost (localId : LocalId)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (frame : RuntimeFrame) (state : RuntimeState) : Control → Prop
  | .value value => localId.index < frame.locals.size →
      post { frame with locals := frame.locals.set! localId.index (some value) }
        state (.value .unit)
  | control => Abrupt control → post frame state control

theorem wpExpr_nativeAssignLocal
    (localId : LocalId) (value : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeAssignLocal localId value) frame state post ↔
      wpExpr value frame state (assignLocalPost localId post) := by
  constructor
  · intro h finalFrame finalState control value_step
    cases control with
    | value runtimeValue =>
        intro in_bounds
        exact h _ _ _ <| .inr
          ⟨finalFrame, finalState, runtimeValue, value_step, in_bounds,
            rfl, rfl, rfl⟩
    | break_ nest breakValue =>
        intro abrupt
        exact h _ _ _ (.inl ⟨value_step, abrupt⟩)
    | continue_ nest =>
        intro abrupt
        exact h _ _ _ (.inl ⟨value_step, abrupt⟩)
    | return_ values =>
        intro abrupt
        exact h _ _ _ (.inl ⟨value_step, abrupt⟩)
    | throw_ kind arguments =>
        intro abrupt
        exact h _ _ _ (.inl ⟨value_step, abrupt⟩)
  · intro h finalFrame finalState control step
    rcases step with ⟨value_step, abrupt⟩ |
      ⟨valueFrame, valueState, runtimeValue, value_step, in_bounds,
        rfl, rfl, rfl⟩
    · have post_step := h _ _ _ value_step
      cases abrupt <;> exact post_step (by constructor)
    · exact h _ _ _ value_step in_bounds

/-! ## Blocks and bindings -/

theorem wpExpr_blockUnit
    (statements : StatementsDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (blockUnit statements) frame state post ↔
      wpStatements statements frame state
        (blockPost post fun doneFrame doneState =>
          post doneFrame doneState (.value .unit)) := by
  constructor
  · intro h result statements_step
    cases result with
    | control finalState finalFrame control =>
        exact h finalFrame finalState control (.inl statements_step)
    | done finalState finalFrame =>
        exact h finalFrame finalState (.value .unit)
          (.inr ⟨statements_step, rfl⟩)
  · intro h finalFrame finalState control step
    rcases step with statements_step | ⟨statements_step, rfl⟩
    · exact h (.control finalState finalFrame control) statements_step
    · exact h (.done finalState finalFrame) statements_step

theorem wpExpr_blockResult
    (statements : StatementsDenotation) (result : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (blockResult statements result) frame state post ↔
      wpStatements statements frame state
        (blockPost post fun doneFrame doneState =>
          wpExpr result doneFrame doneState post) := by
  constructor
  · intro h statementsResult statements_step
    cases statementsResult with
    | control finalState finalFrame control =>
        exact h finalFrame finalState control (.inl statements_step)
    | done statementState statementFrame =>
        intro finalFrame finalState control result_step
        exact h finalFrame finalState control <| .inr
          ⟨statementFrame, statementState, statements_step, result_step⟩
  · intro h finalFrame finalState control step
    rcases step with statements_step |
      ⟨statementFrame, statementState, statements_step, result_step⟩
    · exact h (.control finalState finalFrame control) statements_step
    · exact h (.done statementState statementFrame) statements_step
        finalFrame finalState control result_step

/-- Direct postcondition of a branch: the value the condition produced
selects the arm, and abrupt control passes straight through.  A validated
condition is boolean, so nothing is claimed about any other value. -/
def branchPost (thenBranch : ExprDenotation) (elseBranch : Option ExprDenotation)
    (post : RuntimeFrame → RuntimeState → Control → Prop)
    (frame : RuntimeFrame) (state : RuntimeState) (control : Control) : Prop :=
  match control with
  | .value (.bool true) => wpExpr thenBranch frame state post
  | .value (.bool false) =>
      match elseBranch with
      | some elseBranch => wpExpr elseBranch frame state post
      | none => post frame state (.value .unit)
  | .value _ => True
  | control => post frame state control

/-! The branch postcondition reduces on the control the condition
produced.  These are the same shape as the block and operand-row
postconditions: one equation per constructor, so a step lands on the arm
the value selects without unfolding the transformer. -/

@[simp] theorem branchPost_true (thenBranch elseBranch post frame state) :
    branchPost thenBranch elseBranch post frame state (.value (.bool true))
      = wpExpr thenBranch frame state post := rfl

@[simp] theorem branchPost_falseElse (thenBranch elseBranch post frame state) :
    branchPost thenBranch (some elseBranch) post frame state
        (.value (.bool false))
      = wpExpr elseBranch frame state post := rfl

@[simp] theorem branchPost_falseUnit (thenBranch post frame state) :
    branchPost thenBranch none post frame state (.value (.bool false))
      = post frame state (.value .unit) := rfl

@[simp] theorem branchPost_throw (thenBranch elseBranch post frame state kind thrown) :
    branchPost thenBranch elseBranch post frame state (.throw_ kind thrown)
      = post frame state (.throw_ kind thrown) := rfl

@[simp] theorem branchPost_return (thenBranch elseBranch post frame state values) :
    branchPost thenBranch elseBranch post frame state (.return_ values)
      = post frame state (.return_ values) := rfl

@[simp] theorem branchPost_break (thenBranch elseBranch post frame state nest value) :
    branchPost thenBranch elseBranch post frame state (.break_ nest value)
      = post frame state (.break_ nest value) := rfl

@[simp] theorem branchPost_continue (thenBranch elseBranch post frame state nest) :
    branchPost thenBranch elseBranch post frame state (.continue_ nest)
      = post frame state (.continue_ nest) := rfl

attribute [lir_wp_norm]
  branchPost_true branchPost_falseElse branchPost_falseUnit
  branchPost_throw branchPost_return branchPost_break branchPost_continue

/-- Direct WP for a branch.  The condition is transformed once and its own
postcondition chooses the arm; no evaluator equation survives for the proof
driver to reconstruct. -/
theorem wpExpr_nativeBranch (condition thenBranch : ExprDenotation)
    (elseBranch : Option ExprDenotation) (frame : RuntimeFrame)
    (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (nativeBranch condition thenBranch elseBranch) frame state post ↔
      wpExpr condition frame state (branchPost thenBranch elseBranch post) := by
  constructor
  · intro h conditionFrame conditionState control conditionStep
    match control with
    | .value (.bool true) =>
        intro finalFrame finalState control thenStep
        exact h finalFrame finalState control
          (.inr (.inl ⟨conditionFrame, conditionState, conditionStep, thenStep⟩))
    | .value (.bool false) =>
        match elseBranch with
        | some elseBranch =>
            intro finalFrame finalState control elseStep
            exact h finalFrame finalState control
              (.inr (.inr ⟨conditionFrame, conditionState, conditionStep, elseStep⟩))
        | none =>
            exact h conditionFrame conditionState (.value .unit)
              (.inr (.inr ⟨conditionFrame, conditionState, conditionStep,
                rfl, rfl, rfl⟩))
    | .value .unit | .value (.integer _) | .value (.address _) |
      .value (.signer _) | .value (.string _) | .value (.character _) |
      .value (.bytes _) | .value (.vector _) | .value (.tuple _) |
      .value (.nominal ..) | .value (.closure ..) | .value (.borrow ..) |
      .value (.loanHole _) => trivial
    | .break_ .. | .continue_ .. | .return_ .. | .throw_ .. =>
        exact h conditionFrame conditionState _
          (.inl ⟨conditionStep, by constructor⟩)
  · intro h finalFrame finalState control step
    rcases step with ⟨conditionStep, abrupt⟩ |
      ⟨conditionFrame, conditionState, conditionStep, thenStep⟩ |
      ⟨conditionFrame, conditionState, conditionStep, elseStep⟩
    · have := h finalFrame finalState control conditionStep
      cases abrupt <;> exact this
    · exact h conditionFrame conditionState _ conditionStep
        finalFrame finalState control thenStep
    · have := h conditionFrame conditionState _ conditionStep
      match elseBranch with
      | some elseBranch => exact this finalFrame finalState control elseStep
      | none =>
          obtain ⟨frame_eq, state_eq, control_eq⟩ := elseStep
          subst frame_eq; subst state_eq; subst control_eq
          exact this

theorem wpExpr_letNoValue (body : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (letNoValue body) frame state post ↔ wpExpr body frame state post :=
  Iff.rfl

theorem wpExpr_letNativeValue_equations (binder : NativePatternBinder)
    (initializer body : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (letNativeValue binder initializer body) frame state post ↔
      wpExpr initializer frame state
        (valueOr post fun initializedFrame initializedState runtimeValue =>
          ∀ boundFrame,
            binder.bind initializedFrame runtimeValue = some boundFrame →
              wpExpr body boundFrame initializedState post) := by
  constructor
  · intro h initializedFrame initializedState control initializer_step
    cases control with
    | value runtimeValue =>
        intro boundFrame bind_eq finalFrame finalState control body_step
        exact h finalFrame finalState control <| .inr
          ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
            initializer_step, bind_eq, body_step⟩
    | break_ nest runtimeValue =>
        exact h initializedFrame initializedState (.break_ nest runtimeValue)
          (.inl ⟨initializer_step, by simp⟩)
    | continue_ nest =>
        exact h initializedFrame initializedState (.continue_ nest)
          (.inl ⟨initializer_step, by simp⟩)
    | return_ values =>
        exact h initializedFrame initializedState (.return_ values)
          (.inl ⟨initializer_step, by simp⟩)
    | throw_ kind thrown =>
        exact h initializedFrame initializedState (.throw_ kind thrown)
          (.inl ⟨initializer_step, by simp⟩)
  · intro h finalFrame finalState control step
    rcases step with ⟨initializer_step, abrupt⟩ |
      ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
        initializer_step, bind_eq, body_step⟩
    · have propagated := h finalFrame finalState control initializer_step
      cases abrupt <;> exact propagated
    · exact h initializedFrame initializedState (.value runtimeValue)
        initializer_step boundFrame bind_eq finalFrame finalState control body_step

theorem wpExpr_letNativeValue (binder : NativePatternBinder)
    (initializer body : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (letNativeValue binder initializer body) frame state post ↔
      wpExpr initializer frame state
        (valueOr post fun initializedFrame initializedState runtimeValue =>
          match binder.bind initializedFrame runtimeValue with
          | none => True
          | some boundFrame =>
              wpExpr body boundFrame initializedState post) := by
  rw [wpExpr_letNativeValue_equations]
  apply wpExpr_post_congr
  intro initializedFrame initializedState control
  cases control <;> try rfl
  case value runtimeValue =>
    simp only [valueOr]
    cases bind_eq : binder.bind initializedFrame runtimeValue <;>
      simp

theorem wpExpr_letValue_equations (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (pattern : PatternId) (initializer body : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (letValue unit ns pattern initializer body) frame state post ↔
      wpExpr initializer frame state
        (valueOr post fun initializedFrame initializedState runtimeValue =>
          ∀ boundFrame,
            bindPattern unit ns initializedFrame pattern runtimeValue =
                some boundFrame →
              wpExpr body boundFrame initializedState post) := by
  constructor
  · intro h initializedFrame initializedState control initializer_step
    cases control with
    | value runtimeValue =>
        intro boundFrame bind_eq finalFrame finalState control body_step
        exact h finalFrame finalState control <| .inr
          ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
            initializer_step, bind_eq, body_step⟩
    | break_ nest runtimeValue =>
        exact h initializedFrame initializedState (.break_ nest runtimeValue)
          (.inl ⟨initializer_step, by simp⟩)
    | continue_ nest =>
        exact h initializedFrame initializedState (.continue_ nest)
          (.inl ⟨initializer_step, by simp⟩)
    | return_ values =>
        exact h initializedFrame initializedState (.return_ values)
          (.inl ⟨initializer_step, by simp⟩)
    | throw_ kind thrown =>
        exact h initializedFrame initializedState (.throw_ kind thrown)
          (.inl ⟨initializer_step, by simp⟩)
  · intro h finalFrame finalState control step
    rcases step with ⟨initializer_step, abrupt⟩ |
      ⟨initializedFrame, initializedState, runtimeValue, boundFrame,
        initializer_step, bind_eq, body_step⟩
    · have propagated := h finalFrame finalState control initializer_step
      cases abrupt <;> exact propagated
    · exact h initializedFrame initializedState (.value runtimeValue)
        initializer_step boundFrame bind_eq finalFrame finalState control body_step

theorem wpExpr_letValue (unit : ValidatedUnit) (ns : ValidatedNamespace)
    (pattern : PatternId) (initializer body : ExprDenotation)
    (frame : RuntimeFrame) (state : RuntimeState)
    (post : RuntimeFrame → RuntimeState → Control → Prop) :
    wpExpr (letValue unit ns pattern initializer body) frame state post ↔
      wpExpr initializer frame state
        (valueOr post fun initializedFrame initializedState runtimeValue =>
          match bindPattern unit ns initializedFrame pattern runtimeValue with
          | none => True
          | some boundFrame =>
              wpExpr body boundFrame initializedState post) := by
  rw [wpExpr_letValue_equations]
  apply wpExpr_post_congr
  intro initializedFrame initializedState control
  cases control <;> try rfl
  case value runtimeValue =>
    simp only [valueOr]
    cases bind_eq : bindPattern unit ns initializedFrame pattern runtimeValue <;>
      simp

/-! ## Sealing, discharge, and the reconcile inventory

The native transformers are sealed: the row route steps them one
combinator at a time by their closed rules, and an unfolded transformer
would normalize a statement tail underneath the unresolved result of its
head operation.  What remains here is what the certified closing consumes:
the discharger for the side conditions its normalizations raise, and the
closed rows an equation over a semantic operation resolves through. -/

attribute [irreducible] wpExpr wpValues wpStatements wpFunction

syntax "leaner_denotation_discharge" : tactic

macro_rules
  | `(tactic| leaner_denotation_discharge) =>
      `(tactic|
        first
        | assumption
        | omega
        | apply SemanticOperations.FreshGlobalLoanIds.lookup_next <;>
            assumption
        | apply SemanticOperations.FreshGlobalLoanIds.lookup_add <;>
            assumption)

/- The closed rows the certified closing resolves an equation with.

They are registered once, as a set: a list inside a tactic is
re-elaborated into a discrimination tree on every application. -/
attribute [lir_reconcile]
  SemanticOperations.finalizeFunctionState
  SemanticOperations.nativeInitialFrame?_singleBorrow_oneLocal
  SemanticOperations.nativeInitialFrame?_singleBorrow_twoLocals
  SemanticOperations.nativeInitialFrame?_twoBorrows_twoLocals
  SemanticOperations.exportFrameLoans_singleInteger
  SemanticOperations.exportFrameLoans_plainInteger_state
  SemanticOperations.exportFrameLoans_clearedSingleton_state
  SemanticOperations.exportFrameLoans_plainBool_state
  SemanticOperations.exportFrameLoans_plainAddress_state
  SemanticOperations.exportFrameLoans_plainTwoIntegers_state
  SemanticOperations.exportFrameLoans_addressNominalInteger_state
  SemanticOperations.exportFrameLoans_globalNominalThirdLocal
  SemanticOperations.exportFrameLoans_globalNominalSingletonLocation
  SemanticOperations.exportFrameLoans_globalScalarNominalSingletonLocation
  SemanticOperations.exportFrameLoans_focusedGlobalNominal
  SemanticOperations.exportFrameLoans_focusedGlobalNominalSaved
  SemanticOperations.exportFrameLoans_borrowInteger_state
  SemanticOperations.exportFrameLoans_borrowBool_state
  SemanticOperations.exportFrameLoans_borrowTwoIntegers_state
  SemanticOperations.exportFrameLoans_twoIntegers_state
  SemanticOperations.globalLoanKey?_registry
  SemanticOperations.globalLoanKeyIn?_head
  SemanticOperations.parameterLoanLocations_singleBorrow
  SemanticOperations.parameterLoanLocations_twoBorrows
  SemanticOperations.parameterLoanLocations_borrowInteger
  SemanticOperations.parameterLoanLocations_borrowBool
  SemanticOperations.parameterLoanLocations_singleInteger
  SemanticOperations.localLoanPlace?_singleLocal
  SemanticOperations.localLoanPlace?_twoLocals_left
  SemanticOperations.localLoanPlace?_twoLocals_right
  SemanticOperations.updateBorrowValue?_localZero
  SemanticOperations.updateBorrowValue?_singleLocal
  SemanticOperations.updateBorrowValue?_singleLocal_pair
  SemanticOperations.updateBorrowValue?_returnedReborrow_pair
  SemanticOperations.updateBorrowValue?_twoLocals_left
  SemanticOperations.updateBorrowValue?_twoLocals_right
  SemanticOperations.updateBorrowValue?_globalBorrowThirdLocal
  SemanticOperations.updateBorrowValue?_globalBorrowSingletonLocation
  SemanticOperations.updateBorrowValue?_focusedBorrowPair
  SemanticOperations.updateBorrowValue?_focusedBorrowPairSaved
  SemanticOperations.array_mk_eq_toArray
  SemanticOperations.array_set!_eq_setIfInBounds
  SemanticOperations.array_pair_setIfInBounds_zero
  SemanticOperations.array_pair_setIfInBounds_one
  SemanticOperations.array_triple_setIfInBounds_zero
  SemanticOperations.array_triple_setIfInBounds_one
  SemanticOperations.array_triple_setIfInBounds_two
  SemanticOperations.array_singleton_setIfInBounds_zero
  SemanticOperations.array_pair_set_zero
  SemanticOperations.array_pair_set_one
  SemanticOperations.array_triple_set_zero
  SemanticOperations.array_triple_set_two
  SemanticOperations.array_filter_empty
  SemanticOperations.array_singleton_push
  SemanticOperations.array_empty_push
  SemanticOperations.activeLoans_singleton_retire
  SemanticOperations.activeLoans_zero_then_one
  SemanticOperations.activeLoans_zero_then_one_list
  SemanticOperations.applyPendingFrom_twoDerefLocals
  SemanticOperations.applyPendingFrom_returnedReborrow_derefLocalZero_pair
  SemanticOperations.applyPendingFrom_derefLocalZero_afterBorrow
  SemanticOperations.applyPendingFrom_derefLocalZero_pair
  SemanticOperations.applyPendingFrom_derefLocalZero
  SemanticOperations.applyPendingFrom_self
  SemanticOperations.FreshGlobalLoanIds.lookup_next
  SemanticOperations.FreshGlobalLoanIds.lookup_add
  SemanticOperations.priorLoan_ne_next
  SemanticOperations.nextLoan_ne_prior
  SemanticOperations.priorLoan_ne_future
  SemanticOperations.futureLoan_ne_prior

end LeanerIR.Proofs.Denotation
