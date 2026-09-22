-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeLoop
import LeanerIR.Proofs.NativeControlAgreement

namespace LeanerIR.Proofs.ComputationAgreement

open LeanerIR.BigStep Denotation

/-- A loop repeats after an ordinary value or an unlabeled continue. -/
inductive Repeats : Control → Prop where
  | value (value : RuntimeValue) : Repeats (.value value)
  | continue_ : Repeats (.continue_ 0)

/-- The typed iteration boundary. Different normal and continue executions
may have the same typed successor: their distinction is irrelevant to the
loop, but no outer control or return is silently discarded. -/
structure LoopIteration (body : ExprDenotation)
    (entry : Locals → RuntimeFrame) (exit : Result → RuntimeFrame)
    (iteration : Locals → Spec RuntimeState Failure (NativeLoop.Step Locals Result)) : Prop where
  repeats : ∀ locals initial finalFrame finalState,
    (∃ control, Repeats control ∧ body (entry locals) initial finalFrame finalState control) ↔
      ∃ next, (iteration locals).ok initial (.next next) finalState ∧ finalFrame = entry next
  done : ∀ locals initial finalFrame finalState,
    body (entry locals) initial finalFrame finalState (.break_ 0 none) ↔
      ∃ result, (iteration locals).ok initial (.done result) finalState ∧ finalFrame = exit result
  aborts : ∀ locals initial error,
    (∃ finalFrame finalState,
      body (entry locals) initial finalFrame finalState (.throw_ error.1 error.2)) ↔
      (iteration locals).aborts initial error
  control : ∀ locals initial finalFrame finalState flow,
    body (entry locals) initial finalFrame finalState flow →
      Repeats flow ∨ flow = .break_ 0 none ∨ ∃ kind values, flow = .throw_ kind values
  defined : ∀ locals initial, ¬(iteration locals).undefined initial

variable {Locals Result : Type} {body : ExprDenotation}
  {entry : Locals → RuntimeFrame} {exit : Result → RuntimeFrame}
  {iteration : Locals → Spec RuntimeState Failure (NativeLoop.Step Locals Result)}
  {frame finalFrame : RuntimeFrame} {initial finalState : RuntimeState} {control : Control}

private theorem loop_sound
    (agreement : LoopIteration body entry exit iteration)
    (executed : Denotation.NativeLoop body frame initial finalFrame finalState control) :
    ∀ locals, frame = entry locals →
      (∃ result, NativeLoop.Runs iteration locals initial result finalState ∧
        finalFrame = exit result ∧ control = .value .unit) ∨
      (∃ error, NativeLoop.Fails iteration locals initial error ∧
        control = .throw_ error.1 error.2) := by
  induction executed with
  | repeatValue value _ step _ ih =>
    intro locals same
    subst_vars
    obtain ⟨next, ran, equal⟩ := (agreement.repeats _ _ _ _).mp
      ⟨.value value, .value value, step⟩
    rcases ih next equal with ⟨result, rest, same, control⟩ | ⟨error, rest, control⟩
    · exact Or.inl ⟨result, .next ran rest, same, control⟩
    · exact Or.inr ⟨error, .next ran rest, control⟩
  | repeatContinue _ step _ ih =>
    intro locals same
    subst_vars
    obtain ⟨next, ran, equal⟩ := (agreement.repeats _ _ _ _).mp
      ⟨.continue_ 0, .continue_, step⟩
    rcases ih next equal with ⟨result, rest, same, control⟩ | ⟨error, rest, control⟩
    · exact Or.inl ⟨result, .next ran rest, same, control⟩
    · exact Or.inr ⟨error, .next ran rest, control⟩
  | break_ value step =>
    intro locals same
    subst_vars
    rcases agreement.control _ _ _ _ _ step with repeats | equal | ⟨_, _, equal⟩
    · cases repeats
    · cases equal
      obtain ⟨result, ran, same⟩ := (agreement.done _ _ _ _).mp step
      exact Or.inl ⟨result, .done ran, same, rfl⟩
    · cases equal
  | throw_ kind values step =>
    intro locals same
    subst_vars
    exact Or.inr ⟨(kind, values), .here ((agreement.aborts _ _ _).mp ⟨_, _, step⟩), rfl⟩
  | outerContinue nest step =>
    intro locals same
    subst_vars
    rcases agreement.control _ _ _ _ _ step with repeats | equal | ⟨_, _, equal⟩
    · cases repeats
    · cases equal
    · cases equal
  | outerBreak nest value step =>
    intro locals same
    subst_vars
    rcases agreement.control _ _ _ _ _ step with repeats | equal | ⟨_, _, equal⟩
    · cases repeats
    · cases equal
    · cases equal
  | return_ values step =>
    intro locals same
    subst_vars
    rcases agreement.control _ _ _ _ _ step with repeats | equal | ⟨_, _, equal⟩
    · cases repeats
    · cases equal
    · cases equal

private theorem loop_runs
    (agreement : LoopIteration body entry exit iteration)
    (executed : NativeLoop.Runs iteration locals initial result final) :
    Denotation.NativeLoop body (entry locals) initial (exit result) final (.value .unit) := by
  induction executed with
  | done step =>
    exact .break_ none ((agreement.done _ _ _ _).mpr ⟨_, step, rfl⟩)
  | next step _ ih =>
    obtain ⟨control, repeats, ran⟩ := (agreement.repeats _ _ _ _).mpr ⟨_, step, rfl⟩
    cases repeats with
    | value value => exact .repeatValue value _ ran ih
    | continue_ => exact .repeatContinue _ ran ih

private theorem loop_fails
    (agreement : LoopIteration body entry exit iteration)
    (executed : NativeLoop.Fails iteration locals initial error) :
    ∃ finalFrame finalState, Denotation.NativeLoop body (entry locals) initial
      finalFrame finalState (.throw_ error.1 error.2) := by
  induction executed with
  | here step =>
    obtain ⟨frame, state, ran⟩ := (agreement.aborts _ _ _).mpr step
    exact ⟨frame, state, .throw_ _ _ ran⟩
  | next step _ ih =>
    obtain ⟨control, repeats, ran⟩ := (agreement.repeats _ _ _ _).mpr ⟨_, step, rfl⟩
    obtain ⟨frame, state, rest⟩ := ih
    cases repeats with
    | value value => exact ⟨frame, state, .repeatValue value _ ran rest⟩
    | continue_ => exact ⟨frame, state, .repeatContinue _ ran rest⟩

theorem loop_normal (agreement : LoopIteration body entry exit iteration)
    (site : ExprId) (locals : Locals) (invariant : Locals → RuntimeState → Prop)
    (initial finalState : RuntimeState) (finalFrame : RuntimeFrame) (value : RuntimeValue) :
    nativeLoop site body (entry locals) initial finalFrame finalState (.value value) ↔
      ∃ result, (NativeLoop.run iteration locals invariant).ok initial result finalState ∧
        finalFrame = exit result ∧ value = .unit := by
  constructor
  · intro executed
    rcases loop_sound agreement executed locals rfl with
      ⟨result, ran, same, equal⟩ | ⟨_, _, equal⟩
    · cases equal
      exact ⟨result, (NativeLoop.run_ok _ _ _ _ _ _).mpr ran, same, rfl⟩
    · cases equal
  · rintro ⟨result, ran, rfl, rfl⟩
    exact loop_runs agreement ((NativeLoop.run_ok _ _ _ _ _ _).mp ran)

theorem loop_aborts (agreement : LoopIteration body entry exit iteration)
    (site : ExprId) (locals : Locals) (invariant : Locals → RuntimeState → Prop)
    (initial : RuntimeState) (error : Failure) :
    (∃ finalFrame finalState,
      nativeLoop site body (entry locals) initial finalFrame finalState (.throw_ error.1 error.2)) ↔
      (NativeLoop.run iteration locals invariant).aborts initial error := by
  constructor
  · rintro ⟨frame, state, executed⟩
    rcases loop_sound agreement executed locals rfl with
      ⟨_, _, _, equal⟩ | ⟨actual, ran, equal⟩
    · cases equal
    · cases error
      cases actual
      cases equal
      exact (NativeLoop.run_aborts _ _ _ _ _).mpr ran
  · intro aborted
    exact loop_fails agreement ((NativeLoop.run_aborts _ _ _ _ _).mp aborted)

theorem loop_abrupt (agreement : LoopIteration body entry exit iteration)
    (site : ExprId) (locals : Locals)
    (executed : nativeLoop site body (entry locals) initial finalFrame finalState flow)
    (abrupt : Abrupt flow) : ∃ kind values, flow = .throw_ kind values := by
  rcases loop_sound agreement executed locals rfl with
    ⟨_, _, _, equal⟩ | ⟨error, _, equal⟩
  · subst flow; cases abrupt
  · exact ⟨error.1, error.2, equal⟩

/-- A completed typed-local loop passes its exit locals and actual exit store
to one continuation. Iterations need not preserve the store. The frame is
only an execution-agreement parameter. -/
theorem fromFrame_loop (unit : Validation.ExecutableUnit) (shape : SemanticOperations.FunctionShape)
    (site : ExprId) (body result : ExprDenotation) (tail : StatementsDenotation)
    (entry : Locals → RuntimeFrame) (exit : Exit → RuntimeFrame)
    (iteration : Locals → Spec RuntimeState Failure (NativeLoop.Step Locals Exit))
    (locals : Locals) (invariant : Locals → RuntimeState → Prop)
    (next : Exit → Spec RuntimeState Failure Result) (codec : Codec Result (Array RuntimeValue))
    (agreement : LoopIteration body entry exit iteration)
    (continuation : ∀ value, Spec.Equiv (fromFrame unit shape (blockResult tail result) (exit value))
      (encodeSpec codec (next value))) :
    Spec.Equiv (fromFrame unit shape
      (blockResult (statementsCons (nativeLoop site body) tail) result) (entry locals))
      (encodeSpec codec (Spec.bind (NativeLoop.run iteration locals invariant) next)) := by
  rw [block_discard]
  constructor
  · intro initial result final
    constructor
    · rintro ⟨finalFrame, state, control, (⟨ran, abrupt⟩ |
        ⟨loopFrame, loopState, value, bound, ran, binds, continued⟩), ended, exported⟩
      · obtain ⟨kind, values, rfl⟩ := loop_abrupt agreement site locals ran abrupt
        cases ended
      · obtain ⟨value, step, same, rfl⟩ :=
          (loop_normal agreement site locals invariant _ _ _ _).mp ran
        cases binds
        subst loopFrame
        obtain ⟨native, executed, encoded⟩ := (continuation value).ok _ _ _ |>.mp
          ⟨finalFrame, state, control, continued, ended, exported⟩
        exact ⟨native, ⟨value, loopState, step, executed⟩, encoded⟩
    · rintro ⟨native, ⟨value, middle, step, executed⟩, encoded⟩
      obtain ⟨finalFrame, state, control, continued, ended, exported⟩ :=
        (continuation value).ok _ _ _ |>.mpr ⟨native, executed, encoded⟩
      exact ⟨finalFrame, state, control,
        Or.inr ⟨exit value, middle, .unit, exit value,
          (loop_normal agreement site locals invariant _ _ _ _).mpr ⟨value, step, rfl, rfl⟩,
          rfl, continued⟩, ended, exported⟩
  · intro initial error
    constructor
    · rintro ⟨finalFrame, state, control, (⟨ran, abrupt⟩ |
        ⟨loopFrame, loopState, value, bound, ran, binds, continued⟩), ended⟩
      · obtain ⟨kind, values, rfl⟩ := loop_abrupt agreement site locals ran abrupt
        cases error with | mk actualKind actualValues =>
          cases ended
          exact Or.inl ((loop_aborts agreement site locals invariant _ _).mp ⟨_, _, ran⟩)
      · obtain ⟨value, step, same, rfl⟩ :=
          (loop_normal agreement site locals invariant _ _ _ _).mp ran
        cases binds
        subst loopFrame
        exact Or.inr ⟨value, loopState, step,
          (continuation value).aborts _ _ |>.mp ⟨finalFrame, state, control, continued, ended⟩⟩
    · rintro (aborted | ⟨value, middle, step, aborted⟩)
      · obtain ⟨frame, state, ran⟩ := (loop_aborts agreement site locals invariant _ _).mpr aborted
        exact ⟨frame, state, .throw_ error.1 error.2, Or.inl ⟨ran, .throw_ _ _⟩, rfl⟩
      · obtain ⟨frame, state, control, continued, ended⟩ :=
          (continuation value).aborts _ _ |>.mpr aborted
        exact ⟨frame, state, control,
          Or.inr ⟨exit value, middle, .unit, exit value,
            (loop_normal agreement site locals invariant _ _ _ _).mpr ⟨value, step, rfl, rfl⟩,
            rfl, continued⟩, ended⟩
  · intro initial
    constructor
    · exact False.elim
    · rintro (undefined | ⟨value, state, _, undefined⟩)
      · exact NativeLoop.defined _ _ _ agreement.defined _ undefined
      · exact (continuation value).undefined state |>.mpr undefined

end LeanerIR.Proofs.ComputationAgreement
