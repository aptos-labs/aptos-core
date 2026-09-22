-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Proofs.Certify
import LeanerIR.Semantics.Focus
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Finalization

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def moveConfig : ProfileConfig := { profile := .move, name := "move-test" }
private def moveSchema : ProfileSchema := { profile := .move, name := "move-test" }
private def moveSemantics : SemanticProfile := {
  profile := .move
  name := "move-test"
  classify := fun _ _ => none
  rollbackThrow := fun kind => kind == .abort }

private def rustConfig : ProfileConfig := { profile := .rust, name := "rust-test" }
private def rustSchema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def rustSemantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def fixture : RawUnit where
  tables := {
    files := #[{ name := "finalization.move" }]
    locations := (Array.range 10).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "throw finalization" }]
    lifetimes := #[{ kind := .inference, loc := ⟨0⟩ }]
    types := #[
      .unit,
      .integer (.bits 64) false,
      .reference { profile := .move, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ },
      .never]
    namespaces := #[{ segments := #["test", "Finalization"] }]
    names := #[{ namespaceId := ⟨0⟩, name := "mutate_then_abort" }] }
  profiles := #[moveConfig]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .move
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 9) },
      { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .operation (.write ⟨1⟩) #[] #[⟨0⟩] },
      { loc := ⟨2⟩, typeId := ⟨3⟩, kind := .throw_ .abort #[] },
      { loc := ⟨3⟩, typeId := ⟨3⟩, kind := .block #[⟨1⟩] (some ⟨2⟩) }]
    places := #[.localVar ⟨0⟩, .deref ⟨0⟩]
    functions := #[{
      loc := ⟨4⟩
      name := ⟨0⟩
      profile := .move
      signature := {
        parameters := #[{
          name := "reference", typeUse := typeUse 2 4 }]
        results := #[typeUse 1 4] }
      body := .structured ⟨3⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩
      locals := #[{
        id := ⟨0⟩, name := "reference", type := typeUse 2 4,
        mutable := false, loc := ⟨4⟩ }]
    }] }]

private def rustFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  let referenceType : Ty := .reference {
    profile := .rust, kind := .mutable, referent := ⟨1⟩, lifetime := ⟨0⟩ }
  { fixture with
    profiles := #[rustConfig]
    tables := { fixture.tables with types := fixture.tables.types.set! 2 referenceType }
    namespaces := #[{
      ns with
      profile := some .rust
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with kind := .throw_ .panic #[] }
      functions := ns.functions.map fun declaration => { declaration with profile := .rust }
    }] }

private def prepare? (schema : ProfileSchema) (semantics : SemanticProfile)
    (raw : RawUnit) : Option ExecutableUnit := do
  let checked ← (validate #[schema] raw).toOption
  (prepareExecution #[semantics] checked).toOption

private def moveExecutable? := prepare? moveSchema moveSemantics fixture
private def rustExecutable? := prepare? rustSchema rustSemantics rustFixture

/-- The caller minted loan 0 and keeps its hole; the argument borrow owns
the current value. -/
private def initialState : RuntimeState := { nextLoan := 1 }

open LeanerIR.SemanticOperations in
set_option maxHeartbeats 1000 in
example (state : RuntimeState) (key : GlobalKey) (loan : Nat)
    (fresh : FreshGlobalLoanIds state) (bound : state.nextLoan + 2 ≤ loan) :
    globalLoanKeyIn? ((state.nextLoan, key) :: state.globalLoans) loan = none := by
  leaner_fresh_loan

open LeanerIR.SemanticOperations in
set_option maxHeartbeats 1000 in
example (state : RuntimeState) (key : GlobalKey) (fresh : FreshGlobalLoanIds state) :
    ∃ carried : Nat, carried = state.nextLoan + 1 ∧
      FreshGlobalLoanIds { state with
        globalLoans := (state.nextLoan, key) :: state.globalLoans
        nextLoan := state.nextLoan + 2 } := by
  leaner_certified_close!

open LeanerIR.SemanticOperations in
set_option maxHeartbeats 1000 in
example (value : RuntimeValue) (plain : Plain value) (loans : Array Nat) :
    maskReturnedBorrows loans value = value :=
  plain.maskReturnedBorrows_eq_self loans

open LeanerIR.SemanticOperations in
set_option maxHeartbeats 1000 in
example (value : RuntimeValue) (plain : Plain value)
    (frame : RuntimeFrame) (state : RuntimeState) :
    exportReturnedFrameLoans #[value] frame state = exportFrameLoans frame state :=
  exportReturnedFrameLoans_singlePlain plain frame state

open LeanerIR.SemanticOperations in
set_option maxHeartbeats 1000 in
example (loan next : Nat) (prior : loan < next) (value : RuntimeValue) (initial : RuntimeState) :
    exportReturnedFrameLoans #[.borrow next value]
      { locals := #[some (.borrow loan (.loanHole next))] } initial =
    exportFrameLoans { locals := #[some (.borrow loan (.loanHole next))] } initial := by
  leaner_finalize

open LeanerIR.SemanticOperations in
-- A returned identity stays live even when copies remain in several local
-- aggregate slots. Unrelated references still export their final contents.
#guard exportReturnedFrameLoans #[.borrow 0 (.integer 7)]
    { locals := #[some (.borrow 0 (.integer 7)),
        some (.tuple #[.borrow 0 (.integer 7), .borrow 1 (.integer 9)]),
        some (.vector #[.borrow 0 (.integer 7)])] }
    { nextLoan := 2 } == { nextLoan := 2, pending := #[(1, .integer 9)] }

open LeanerIR.SemanticOperations in
-- Both handles can escape through one aggregate result without generating
-- an early write-back into the caller's suspended owners.
#guard exportReturnedFrameLoans #[.tuple #[.borrow 0 (.integer 7), .borrow 1 (.integer 9)]]
    { locals := #[some (.borrow 0 (.integer 7)), some (.borrow 1 (.integer 9))] }
    { nextLoan := 2 } == { nextLoan := 2 }

open LeanerIR.SemanticOperations in
-- Pruning stops at an outer borrow's ownership boundary.
#guard maskReturnedBorrows #[1] (.borrow 0 (.tuple #[.borrow 1 (.integer 7)])) ==
    .borrow 0 (.tuple #[.borrow 1 (.integer 7)])

open LeanerIR.SemanticOperations in
-- Plain results retain the old retirement behavior.
#guard exportReturnedFrameLoans #[.integer 11]
    { locals := #[some (.borrow 0 (.integer 7))] } initialState ==
    { nextLoan := 1, pending := #[(0, .integer 7)] }

private def reference (_profile : Profile) : RuntimeValue :=
  .borrow 0 (.integer 7)

private def handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }

-- A Move abort rolls the invocation back: the loan's write-back never
-- becomes visible.
#guard match moveExecutable? with
  | none => false
  | some executable => match LeanerIR.Interpreter.run executable 16 handle
      #[reference .move] initialState with
    | .ok (state, outcome) =>
        outcome.value == .threw .abort #[] && state.pending == #[]
    | .error _ => false

-- A Rust panic exposes the evaluated state: the dying frame exports the
-- borrowed loan's final value for the lender's hole.
#guard match rustExecutable? with
  | none => false
  | some executable => match LeanerIR.Interpreter.run executable 16 handle
      #[reference .rust] initialState with
    | .ok (state, outcome) =>
        outcome.value == .threw .panic #[] &&
          state.pending == #[(0, .integer 9)]
    | .error _ => false

private theorem successfulRunHasDerivation (executable : ExecutableUnit)
    (arguments : Array RuntimeValue) (state : RuntimeState)
    (success : (LeanerIR.Interpreter.run executable 16 handle arguments state).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction executable handle #[] state arguments finalState outcome.value := by
  generalize result_eq : LeanerIR.Interpreter.run executable 16 handle arguments state = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound executable 16 handle arguments state
          result.1 result.2 result_eq⟩

private def preparedMove : ExecutableUnit := moveExecutable?.get (by native_decide)
private def preparedRust : ExecutableUnit := rustExecutable?.get (by native_decide)

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedMove handle #[] initialState #[reference .move]
      finalState outcome.value := by
  apply successfulRunHasDerivation preparedMove #[reference .move] initialState
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedRust handle #[] initialState #[reference .rust]
      finalState outcome.value := by
  apply successfulRunHasDerivation preparedRust #[reference .rust] initialState
  native_decide

-- An unknown boundary must stay folded: proving a callee's continuation
-- must not demand that an arbitrary final frame has no returned aliases.
set_option maxHeartbeats 1000 in
example (results : Array RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    SemanticOperations.exportReturnedFrameLoans results frame state =
      SemanticOperations.exportReturnedFrameLoans results frame state := by
  leaner_finalize

-- These lookups can be exposed only when closing the final contract, after
-- continuation normalization has finished. Keep registry tails opaque.
set_option maxHeartbeats 1000 in
example (initial final : RuntimeState) (retired loan : Nat)
    (discipline : SemanticOperations.LoanDiscipline initial final)
    (minted : initial.nextLoan ≤ retired) (older : loan < initial.nextLoan) :
    SemanticOperations.globalLoanKeyIn?
      (SemanticOperations.removeGlobalLoan final.globalLoans retired) loan =
      SemanticOperations.globalLoanKeyIn? initial.globalLoans loan := by
  leaner_certified_close!

set_option maxHeartbeats 1000 in
example (final : RuntimeState) (retired loan : Nat)
    (fresh : SemanticOperations.FreshGlobalLoanIds final)
    (live : retired < final.nextLoan) (unminted : final.nextLoan ≤ loan) :
    SemanticOperations.globalLoanKeyIn?
      (SemanticOperations.removeGlobalLoan final.globalLoans retired) loan = none := by
  leaner_certified_close!

end LeanerIR.Tests.Finalization
