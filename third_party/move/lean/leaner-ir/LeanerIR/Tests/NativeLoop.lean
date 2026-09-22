-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeLoopAgreement
import Lean.Util.CollectAxioms

set_option maxHeartbeats 1000

namespace LeanerIR.Tests.NativeLoop

open LeanerIR.Proofs

-- A store-changing loop, not an execution with a frozen loop-entry store.
def countdown : Nat → Spec Nat String (NativeLoop.Step Nat Unit)
  | 0 => Spec.pure (.done ())
  | n + 1 => Spec.bind (Spec.modify (· + 1)) (fun _ => Spec.pure (.next n))

theorem countdown_runs (count initial : Nat) :
    NativeLoop.Runs countdown count initial () (initial + count) := by
  induction count generalizing initial with
  | zero => exact .done ⟨rfl, rfl⟩
  | succ count ih =>
    apply NativeLoop.Runs.next (next := count) (middle := initial + 1)
    · exact ⟨(), initial + 1, ⟨rfl, rfl⟩, rfl, rfl⟩
    · simpa [Nat.add_assoc, Nat.add_comm, Nat.add_left_comm] using ih (initial + 1)

example (count : Nat) :
    (NativeLoop.run countdown count (fun locals store => locals + store = count)).ok
      0 () count := by
  apply (NativeLoop.run_ok _ _ _ _ _ _).mpr
  simpa using countdown_runs count 0

-- The entry and one-step invariant proof suffice for all iteration counts.
example (count : Nat) :
    wp (NativeLoop.run countdown count (fun locals store => locals + store = count))
      (fun _ final => final = count) (fun _ => False) 0 := by
  apply NativeLoop.wp_run
  · omega
  · intro locals store invariant
    cases locals with
    | zero =>
      rw [countdown, wp_pure]
      simpa using invariant
    | succ locals =>
      simp only [countdown, wp, Spec.bind, Spec.modify, Spec.pure]
      constructor
      · rintro _ _ ⟨_, _, ⟨_, rfl⟩, rfl, rfl⟩; omega
      · exact ⟨by simp, by simp⟩

-- A false invariant is not a termination assumption or a semantic filter.
example : (NativeLoop.run countdown 2 (fun _ _ => False)).ok 5 () 7 :=
  (NativeLoop.run_ok _ _ _ _ _ _).mpr (countdown_runs 2 5)

example : ¬wp (NativeLoop.run countdown 2 (fun _ _ => False))
    (fun _ _ => False) (fun _ => False) 5 := by
  intro verified
  exact verified.1 () 7 ((NativeLoop.run_ok _ _ _ _ _ _).mpr (countdown_runs 2 5))

-- A later abort observes the intermediate store, including its payload.
def failLater : Bool → Spec Nat Nat (NativeLoop.Step Bool Unit)
  | true => Spec.bind (Spec.modify (· + 3)) (fun _ => Spec.pure (.next false))
  | false => { ok := fun _ _ _ => False, aborts := fun state error => error = state }

example : (NativeLoop.run failLater true (fun _ _ => True)).aborts 7 10 := by
  apply (NativeLoop.run_aborts _ _ _ _ _).mpr
  exact .next ⟨(), 10, ⟨rfl, rfl⟩, rfl, rfl⟩ (.here rfl)

example : ¬(NativeLoop.run failLater true (fun _ _ => True)).aborts 7 7 := by
  intro failed
  have ran := (NativeLoop.run_aborts _ _ _ _ _).mp failed
  cases ran with
  | here failed => simp [failLater, Spec.bind, Spec.modify, Spec.pure] at failed
  | next step rest =>
    change ∃ value middle, (value = () ∧ middle = 7 + 3) ∧
      NativeLoop.Step.next _ = .next false ∧ _ = middle at step
    rcases step with ⟨_, _, ⟨rfl, rfl⟩, equal, rfl⟩
    cases equal
    cases rest with
    | here failed => cases failed
    | next impossible _ => cases impossible

def undefinedLater : Bool → Spec Nat Unit (NativeLoop.Step Bool Unit)
  | true => Spec.pure (.next false)
  | false => {
      ok := fun _ _ _ => False
      aborts := fun _ _ => False
      undefined := fun _ => True }

example : (NativeLoop.run undefinedLater true (fun _ _ => True)).undefined 0 := by
  apply (NativeLoop.run_undefined _ _ _ _).mpr
  exact .next ⟨rfl, rfl⟩ (.here trivial)

def diverge (_ : Unit) : Spec Unit Unit (NativeLoop.Step Unit Unit) :=
  Spec.pure (.next ())

example (locals initial result final : Unit) :
    ¬(NativeLoop.run diverge locals (fun _ _ => True)).ok initial result final := by
  intro ran
  have finite := (NativeLoop.run_ok _ _ _ _ _ _).mp ran
  clear ran
  induction finite with
  | done impossible => cases impossible.1
  | next _ _ ih => exact ih

example (locals initial error : Unit) :
    ¬(NativeLoop.run diverge locals (fun _ _ => True)).aborts initial error := by
  intro failed
  have finite := (NativeLoop.run_aborts _ _ _ _ _).mp failed
  clear failed
  induction finite with
  | here impossible => cases impossible
  | next _ _ ih => exact ih

example : ¬(NativeLoop.run diverge () (fun _ _ => True)).undefined () :=
  NativeLoop.defined _ _ _ (fun _ _ => False.elim) _

open Denotation ComputationAgreement BigStep SemanticOperations

def flagFrame (flag : Bool) : RuntimeFrame := { locals := #[some (.bool flag)] }

def toggleBody : ExprDenotation :=
  nativeBranch (localVar ⟨0⟩) (nativeAssignLocal ⟨0⟩ (value (.bool false)))
    (some (nativeBreak 0 none))

def toggle (flag : Bool) : Spec RuntimeState Failure (NativeLoop.Step Bool Bool) :=
  Spec.pure (if flag then .next false else .done false)

private theorem toggle_body (flag : Bool) (initial finalState : RuntimeState)
    (finalFrame : RuntimeFrame) (flow : Control) :
    toggleBody (flagFrame flag) initial finalFrame finalState flow ↔
      finalFrame = flagFrame false ∧ finalState = initial ∧
        flow = if flag then .value .unit else .break_ 0 none := by
  have read : Returns (localVar ⟨0⟩) (flagFrame flag) (flagFrame flag) (.bool flag) := by
    intro state frame final control
    change (∃ actual, some (.bool flag) = some actual ∧
      frame = flagFrame flag ∧ final = state ∧ control = .value actual) ↔ _
    simp
  have assign : Returns (nativeAssignLocal ⟨0⟩ (value (.bool false)))
      (flagFrame flag) (flagFrame false) .unit := by
    intro state frame final control
    constructor
    · rintro (⟨⟨_, _, rfl⟩, abrupt⟩ | ⟨_, _, actual, ⟨rfl, rfl, equal⟩, _, rfl, rfl, rfl⟩)
      · cases abrupt
      · cases equal; exact ⟨rfl, rfl, rfl⟩
    · rintro ⟨rfl, rfl, rfl⟩
      exact Or.inr ⟨flagFrame flag, _, .bool false, ⟨rfl, rfl, rfl⟩,
        Nat.zero_lt_succ 0, rfl, rfl, rfl⟩
  unfold Returns at read assign
  constructor
  · rintro (⟨tested, abrupt⟩ | ⟨_, _, tested, yes⟩ | ⟨_, _, tested, no⟩)
    · obtain ⟨_, _, rfl⟩ := (read _ _ _ _).mp tested
      cases abrupt
    · obtain ⟨rfl, rfl, equal⟩ := (read _ _ _ _).mp tested
      cases equal
      exact (assign _ _ _ _).mp yes
    · obtain ⟨rfl, rfl, equal⟩ := (read _ _ _ _).mp tested
      cases equal
      exact no
  · intro executed
    cases flag
    · exact Or.inr (Or.inr ⟨flagFrame false, initial,
        (read _ _ _ _).mpr ⟨rfl, rfl, rfl⟩, executed⟩)
    · exact Or.inr (Or.inl ⟨flagFrame true, initial,
        (read _ _ _ _).mpr ⟨rfl, rfl, rfl⟩, (assign _ _ _ _).mpr executed⟩)

theorem toggle_agreement : LoopIteration toggleBody flagFrame flagFrame toggle := by
  constructor
  · intro flag initial finalFrame finalState
    constructor
    · rintro ⟨flow, repeats, ran⟩
      obtain ⟨same, state, equal⟩ := (toggle_body _ _ _ _ _).mp ran
      cases flag
      · cases equal; cases repeats
      · exact ⟨false, ⟨rfl, state⟩, same⟩
    · rintro ⟨next, ⟨equal, state⟩, same⟩
      cases flag
      · cases equal
      · cases equal
        exact ⟨.value .unit, .value _, (toggle_body _ _ _ _ _).mpr ⟨same, state, rfl⟩⟩
  · intro flag initial finalFrame finalState
    cases flag <;> simp [toggle_body, toggle, Spec.pure, and_comm]
  · intro flag initial error
    cases flag <;> simp [toggle_body, toggle, Spec.pure]
  · intro flag initial finalFrame finalState flow executed
    obtain ⟨_, _, rfl⟩ := (toggle_body _ _ _ _ _).mp executed
    cases flag
    · exact Or.inr (Or.inl rfl)
    · exact Or.inl (.value _)
  · intro flag initial impossible; cases impossible

example (initial : RuntimeState) :
    nativeLoop ⟨0⟩ toggleBody (flagFrame true) initial (flagFrame false) initial (.value .unit) := by
  apply (loop_normal toggle_agreement _ _ (fun _ _ => True) _ _ _ _).mpr
  refine ⟨false, (NativeLoop.run_ok _ _ _ _ _ _).mpr ?_, rfl, rfl⟩
  exact .next ⟨rfl, rfl⟩ (.done ⟨rfl, rfl⟩)

example (initial : RuntimeState) (error : Failure) :
    ¬∃ finalFrame finalState, nativeLoop ⟨0⟩ toggleBody (flagFrame true) initial
      finalFrame finalState (.throw_ error.1 error.2) := by
  rw [loop_aborts toggle_agreement _ _ (fun _ _ => True)]
  intro failed
  have ran := (NativeLoop.run_aborts _ _ _ _ _).mp failed
  clear failed
  generalize h : true = flag at ran
  clear h
  induction ran with
  | here impossible => cases impossible
  | next _ _ ih => exact ih

-- Continue is a recursive iteration, not a loop exit.
def keepGoing (flag : Bool) : Spec RuntimeState Failure (NativeLoop.Step Bool Bool) :=
  Spec.pure (.next flag)

theorem continue_agreement : LoopIteration (nativeContinue 0) flagFrame flagFrame keepGoing := by
  constructor
  · intro flag initial finalFrame finalState
    constructor
    · rintro ⟨_, _, same, state, _⟩
      exact ⟨flag, ⟨rfl, state⟩, same⟩
    · rintro ⟨next, ⟨equal, state⟩, same⟩
      cases equal
      exact ⟨.continue_ 0, .continue_, same, state, rfl⟩
  · intro flag initial finalFrame finalState
    constructor
    · rintro ⟨_, _, impossible⟩; cases impossible
    · rintro ⟨_, ⟨impossible, _⟩, _⟩; cases impossible
  · intro flag initial error
    constructor
    · rintro ⟨_, _, _, _, impossible⟩; cases impossible
    · exact False.elim
  · rintro flag initial finalFrame finalState flow ⟨_, _, rfl⟩
    exact Or.inl .continue_
  · intro flag initial impossible; cases impossible

-- This agreement slice explicitly rejects control targeting an outer loop.
example (iteration : Bool → Spec RuntimeState Failure (NativeLoop.Step Bool Bool)) :
    ¬LoopIteration (nativeContinue 1) flagFrame flagFrame iteration := by
  intro agreement
  have control := agreement.control false {} (flagFrame false) {} (.continue_ 1)
    ⟨rfl, rfl, rfl⟩
  rcases control with repeats | equal | ⟨_, _, equal⟩
  · cases repeats
  · cases equal
  · cases equal

-- A law-level state-changing iteration. Allocation metadata must reach the
-- continuation even when the typed locals are unchanged. This is not a
-- source-level borrow fixture: it tests the generic composition boundary.
def advance (state : RuntimeState) : RuntimeState :=
  { state with nextLoan := state.nextLoan + 1 }

def effectBody : ExprDenotation := fun frame initial finalFrame final control =>
  finalFrame = frame ∧ final = advance initial ∧ control = .break_ 0 none

def effectIteration (_ : Unit) : Spec RuntimeState Failure (NativeLoop.Step Unit Unit) :=
  Spec.bind (Spec.modify advance) fun _ => Spec.pure (.done ())

theorem effect_agreement :
    LoopIteration effectBody (fun _ : Unit => {}) (fun _ : Unit => {}) effectIteration := by
  constructor
  · intro locals initial finalFrame final
    constructor
    · rintro ⟨_, repeats, _, _, rfl⟩
      cases repeats
    · rintro ⟨_, ⟨_, _, _, impossible, _⟩, _⟩
      cases impossible
  · intro locals initial finalFrame final
    simp [effectBody, effectIteration, Spec.bind, Spec.modify, Spec.pure,
      and_comm, and_left_comm]
  · intro locals initial error
    simp [effectBody, effectIteration, Spec.bind, Spec.modify, Spec.pure]
  · rintro locals initial finalFrame final control ⟨_, _, rfl⟩
    exact Or.inr (Or.inl rfl)
  · intro locals initial
    simp [effectIteration, Spec.bind, Spec.modify, Spec.pure]

example : ¬StatePreserving (effectIteration ()) := by
  intro preserves
  have same := preserves {} (.done ()) (advance {}) ⟨(), advance {}, ⟨rfl, rfl⟩, rfl, rfl⟩
  have counter := congrArg RuntimeState.nextLoan same
  contradiction

/-- Applying the source/typed loop sequencing law requires no state-preserving
certificate for the iteration. An effectful next step sees the changed store. -/
theorem effectful_sequence (unit : Validation.ExecutableUnit)
    (shape : SemanticOperations.FunctionShape) (site : ExprId)
    (tail : StatementsDenotation) (result : ExprDenotation)
    (next : Unit → Spec RuntimeState Failure Result) (codec : Codec Result (Array RuntimeValue))
    (continuation : ∀ value : Unit,
      Spec.Equiv (fromFrame unit shape (blockResult tail result) {}) (encodeSpec codec (next value))) :
    Spec.Equiv (fromFrame unit shape
      (blockResult (statementsCons (nativeLoop site effectBody) tail) result) {})
      (encodeSpec codec (Spec.bind
        (NativeLoop.run effectIteration () (fun _ _ => True)) next)) := by
  exact fromFrame_loop unit shape site effectBody result tail
    (fun _ : Unit => {}) (fun _ : Unit => {}) effectIteration () (fun _ _ => True)
    next codec effect_agreement continuation

example (initial : RuntimeState) :
    (Spec.bind (NativeLoop.run effectIteration () (fun _ _ => True))
      (fun _ => (Spec.get : Spec RuntimeState Failure RuntimeState))).ok
      initial (advance initial) (advance initial) := by
  refine ⟨(), advance initial, (NativeLoop.run_ok ..).mpr ?_, rfl, rfl⟩
  exact .done ⟨(), advance initial, ⟨rfl, rfl⟩, rfl, rfl⟩

open Lean Elab Command in
run_cmd do
  for name in [``NativeLoop.run_ok, ``NativeLoop.run_aborts, ``NativeLoop.run_undefined,
      ``NativeLoop.preserves, ``NativeLoop.defined, ``NativeLoop.wp_run,
      ``loop_normal, ``loop_aborts, ``loop_abrupt, ``fromFrame_loop,
      ``countdown_runs, ``toggle_agreement, ``continue_agreement] do
    let axioms ← Lean.collectAxioms name
    if axioms.contains ``sorryAx then throwError "native loop admission in {name}"
  for name in [``NativeLoop.Step, ``NativeLoop.body, ``NativeLoop.run] do
    let some info := (← getEnv).find? name | throwError "missing typed loop declaration {name}"
    let constants := info.type.getUsedConstants ++
      ((info.value? (allowOpaque := true)).map (·.getUsedConstants)).getD #[]
    for retired in [``RuntimeFrame, ``RuntimeValue] do
      if constants.contains retired then throwError "{name} contains retired representation {retired}"

open Lean Elab Command in
run_cmd do
  for name in [``effect_agreement, ``effectful_sequence] do
    if (← collectAxioms name).contains ``sorryAx then
      throwError "state-changing loop agreement contains an admission: {name}"

end LeanerIR.Tests.NativeLoop
