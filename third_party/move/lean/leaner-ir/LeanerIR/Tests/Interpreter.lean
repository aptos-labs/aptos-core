-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check

namespace LeanerIR.Tests.Interpreter

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def testProfile : Profile := .extension ⟨0⟩
private def profile : ProfileConfig := { profile := testProfile, name := "test" }
private def schema : ProfileSchema := { profile := testProfile, name := "test" }
private def semantics : SemanticProfile := {
  profile := testProfile, name := "test", classify := fun _ _ => none }

private def typeUse (typeId : Nat) (loc : Nat) : TypeUse :=
  { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

private def localDecl (id : Nat) (name : String) (typeId : Nat) (loc : Nat) : LocalDecl :=
  { id := ⟨id⟩, name, type := typeUse typeId loc, mutable := true, loc := ⟨loc⟩ }

private def parameter (_id : Nat) (name : String) (typeId : Nat) (loc : Nat) : Parameter :=
  { name, typeUse := typeUse typeId loc, mutable := true }

private def function (name loc result body : Nat) (locals : Array LocalDecl := #[])
    (parameters : Array Parameter := #[]) : FunctionDecl RawBody := {
  loc := ⟨loc⟩
  name := ⟨name⟩
  profile := testProfile
  signature := { parameters, results := #[typeUse result loc] }
  body := .structured ⟨body⟩
  origin := ⟨0⟩
  alignment := ⟨0⟩
  locals }

private def ownRef (name : Nat) : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨name⟩ }

private def locations : Array Location := (Array.range 48).map fun index => {
  primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }

private def names : Array QualifiedName :=
  #["main", "loopOnce", "callee", "caller", "boom", "callBoom",
    "answer", "useConstant", "matchRange", "stuck", "identity"].map fun name =>
      { namespaceId := ⟨0⟩, name }

private def expressions : Array Expr := #[
  { loc := ⟨0⟩, typeId := ⟨3⟩, kind := .value (.tuple #[.bool true, .integer 7]) },
  { loc := ⟨1⟩, typeId := ⟨2⟩, kind := .localVar ⟨1⟩ },
  { loc := ⟨2⟩, typeId := ⟨4⟩, kind := .return_ #[⟨1⟩] },
  { loc := ⟨3⟩, typeId := ⟨4⟩, kind := .letDecl ⟨0⟩ (some ⟨0⟩) ⟨2⟩ },

  { loc := ⟨4⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
  { loc := ⟨5⟩, typeId := ⟨1⟩, kind := .value (.bool false) },
  { loc := ⟨6⟩, typeId := ⟨0⟩, kind := .assign ⟨0⟩ ⟨5⟩ },
  { loc := ⟨7⟩, typeId := ⟨4⟩, kind := .continue_ 0 },
  { loc := ⟨8⟩, typeId := ⟨4⟩, kind := .block #[⟨6⟩] (some ⟨7⟩) },
  { loc := ⟨9⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ },
  { loc := ⟨10⟩, typeId := ⟨2⟩, kind := .value (.integer 9) },
  { loc := ⟨11⟩, typeId := ⟨4⟩, kind := .break_ 0 (some ⟨10⟩) },
  { loc := ⟨12⟩, typeId := ⟨4⟩, kind := .ifElse ⟨9⟩ ⟨8⟩ (some ⟨11⟩) },
  { loc := ⟨13⟩, typeId := ⟨2⟩, kind := .loop none ⟨12⟩ },
  { loc := ⟨14⟩, typeId := ⟨2⟩, kind := .letDecl ⟨1⟩ (some ⟨4⟩) ⟨13⟩ },

  { loc := ⟨15⟩, typeId := ⟨2⟩, kind := .value (.integer 24) },
  { loc := ⟨16⟩, typeId := ⟨2⟩,
    kind := .operation (.call (.function (ownRef 2))) #[] #[] },
  { loc := ⟨17⟩, typeId := ⟨2⟩, kind := .value (.integer 24) },
  { loc := ⟨18⟩, typeId := ⟨4⟩, kind := .throw_ .abort #[⟨17⟩, ⟨5⟩] },
  { loc := ⟨19⟩, typeId := ⟨2⟩,
    kind := .operation (.call (.function (ownRef 4))) #[] #[] },

  { loc := ⟨20⟩, typeId := ⟨2⟩, kind := .value (.integer 42) },
  { loc := ⟨21⟩, typeId := ⟨2⟩, kind := .constant (ownRef 6) },
  { loc := ⟨22⟩, typeId := ⟨2⟩, kind := .value (.integer 5) },
  { loc := ⟨23⟩, typeId := ⟨2⟩, kind := .value (.integer 7) },
  { loc := ⟨24⟩, typeId := ⟨2⟩,
    kind := .match_ ⟨22⟩ #[{ pattern := ⟨3⟩, body := ⟨23⟩ }] },
  { loc := ⟨25⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ },
  { loc := ⟨26⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ }
]

private def fixture : RawUnit := {
  tables := {
    files := #[{ name := "interpreter.lir" }]
    locations
    origins := #[{ kind := .leanerSource, location := ⟨30⟩ }]
    alignments := #[{ source := ⟨0⟩, trust := .checked, description := "M1 fixture" }]
    types := #[.unit, .bool, .integer (.bits 64) false, .tuple #[⟨1⟩, ⟨2⟩], .never]
    namespaces := #[{ segments := #["test", "Interpreter"] }]
    names }
  profiles := #[profile]
  namespaces := #[{
    loc := ⟨30⟩
    identity := ⟨0⟩
    profile := some testProfile
    expressions
    patterns := #[
      { loc := ⟨27⟩, typeId := ⟨3⟩, kind := .tuple #[⟨1⟩, ⟨2⟩] },
      { loc := ⟨28⟩, typeId := ⟨1⟩, kind := .variable ⟨0⟩ },
      { loc := ⟨29⟩, typeId := ⟨2⟩, kind := .variable ⟨1⟩ },
      { loc := ⟨30⟩, typeId := ⟨2⟩,
        kind := .range (some (.integer 0)) (some (.integer 10)) false }]
    places := #[.localVar ⟨0⟩]
    constants := #[{
      loc := ⟨31⟩, name := ⟨6⟩, type := typeUse 2 31, value := ⟨20⟩ }]
    functions := #[
      function 0 32 2 3 #[localDecl 0 "x" 1 32, localDecl 1 "y" 2 32],
      function 1 33 2 14 #[localDecl 0 "flag" 1 33],
      function 2 34 2 15,
      function 3 35 2 16,
      function 4 36 2 18,
      function 5 37 2 19,
      function 7 38 2 21,
      function 8 39 2 24,
      function 9 40 1 4 #[localDecl 0 "unused" 1 40],
      function 10 41 1 26 #[localDecl 0 "argument" 1 41] #[parameter 0 "argument" 1 41]
    ] }] }

private def executable? : Option ExecutableUnit := do
  let unit ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] unit).toOption

private def handle (functionId : Nat) : FunctionHandle :=
  { namespaceId := ⟨0⟩, functionId := ⟨functionId⟩ }

#guard executable?.isSome

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 0) #[] with
      | .ok (_, { value := .returned #[.integer 7], primary := { loc := ⟨2⟩, .. }, .. }) => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 1) #[] with
      | .ok (_, { value := .returned #[.integer 9], primary := { loc := ⟨14⟩, .. }, .. }) => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 3) #[] with
      | .ok (_, { value := .returned #[.integer 24], primary := { loc := ⟨16⟩, .. }, .. }) => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 5) #[] with
      | .ok (_, outcome) =>
          outcome.value == .threw .abort #[.integer 24, .bool false] &&
            outcome.primary.loc == ⟨18⟩ &&
            outcome.callers == #[{ namespaceId := ⟨0⟩, loc := ⟨19⟩ }]
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 6) #[] with
      | .ok (_, { value := .returned #[.integer 42], .. }) => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 7) #[] with
      | .ok (_, { value := .returned #[.integer 7], .. }) => true
      | _ => false
  | none => false

private def uninitializedFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{ ns with
      functions := ns.functions.set! 8 (function 9 40 1 25 #[localDecl 0 "unset" 1 40]) }] }

#guard match validate #[schema] uninitializedFixture with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-INITIALIZATION" &&
        diagnostic.primary == some ⟨25⟩
  | .ok _ => false

private def ownershipExpressions : Array Expr := #[
  { loc := ⟨27⟩, typeId := ⟨1⟩, kind := .value (.bool true) },
  { loc := ⟨28⟩, typeId := ⟨0⟩, kind := .assign ⟨0⟩ ⟨27⟩ },
  { loc := ⟨29⟩, typeId := ⟨1⟩, kind := .operation (.move ⟨0⟩) #[] #[] },
  { loc := ⟨30⟩, typeId := ⟨1⟩, kind := .localVar ⟨0⟩ },
  { loc := ⟨31⟩, typeId := ⟨1⟩, kind := .block #[⟨28⟩, ⟨29⟩] (some ⟨30⟩) },
  { loc := ⟨32⟩, typeId := ⟨1⟩,
    kind := .ifElse ⟨27⟩ ⟨29⟩ (some ⟨30⟩) },
  { loc := ⟨33⟩, typeId := ⟨1⟩, kind := .block #[⟨28⟩, ⟨32⟩] (some ⟨30⟩) },
  { loc := ⟨34⟩, typeId := ⟨4⟩, kind := .continue_ 0 },
  { loc := ⟨35⟩, typeId := ⟨4⟩, kind := .block #[⟨29⟩] (some ⟨34⟩) },
  { loc := ⟨36⟩, typeId := ⟨4⟩, kind := .loop none ⟨35⟩ },
  { loc := ⟨37⟩, typeId := ⟨4⟩, kind := .block #[⟨28⟩] (some ⟨36⟩) },
  { loc := ⟨38⟩, typeId := ⟨1⟩,
    kind := .block #[⟨28⟩, ⟨29⟩, ⟨28⟩] (some ⟨30⟩) }
]

private def ownershipFixture (root : Nat) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{ ns with
      expressions := ns.expressions ++ ownershipExpressions
      functions := ns.functions.set! 8 (function 9 40 1 root #[localDecl 0 "owned" 1 40]) }] }

private def hasInitializationError (root : Nat) (loc : Nat) : Bool :=
  match validate #[schema] (ownershipFixture root) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-INITIALIZATION" &&
        diagnostic.primary == some ⟨loc⟩
  | .ok _ => false

#guard hasInitializationError 31 30
#guard hasInitializationError 33 30
#guard hasInitializationError 37 29

#guard match validate #[schema] (ownershipFixture 38) with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

#guard match validate #[schema] (ownershipFixture 38) with
  | .ok unit => match prepareExecution #[semantics] unit with
      | .ok executable => match Interpreter.run executable 32 (handle 8) #[] with
          | .ok (_, { value := .returned #[.bool true], .. }) => true
          | _ => false
      | .error _ => false
  | .error _ => false

#guard match executable? with
  | some executable => match Interpreter.run executable 32 (handle 9) #[] with
      | .error { value := .argumentArity 1 0, primary := { loc := ⟨41⟩, .. }, .. } => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 1 (handle 1) #[] with
      | .error error => error.value.code == "LIR-EXEC-FUEL" &&
          (error.primary.primaryRange? executable.unit).isSome
      | _ => false
  | none => false

private theorem successfulFixtureRunHasDerivation (executable : ExecutableUnit)
    (fuel : Nat) (function : FunctionHandle) (arguments : Array RuntimeValue)
    (success : (Interpreter.run executable fuel function arguments).isOk) :
    ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
      BigStep.EvalFunction executable function {} arguments
      finalState outcome.value := by
  generalize result_eq : Interpreter.run executable fuel function arguments = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound executable fuel function arguments {}
          result.1 result.2 result_eq⟩

private def preparedExecutable : ExecutableUnit := executable?.get (by native_decide)

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedExecutable (handle 0) {} #[]
      finalState outcome.value := by
  apply successfulFixtureRunHasDerivation preparedExecutable 32 (handle 0) #[]
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedExecutable (handle 1) {} #[]
      finalState outcome.value := by
  apply successfulFixtureRunHasDerivation preparedExecutable 32 (handle 1) #[]
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedExecutable (handle 3) {} #[]
      finalState outcome.value := by
  apply successfulFixtureRunHasDerivation preparedExecutable 32 (handle 3) #[]
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedExecutable (handle 5) {} #[]
      finalState outcome.value := by
  apply successfulFixtureRunHasDerivation preparedExecutable 32 (handle 5) #[]
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedExecutable (handle 6) {} #[]
      finalState outcome.value := by
  apply successfulFixtureRunHasDerivation preparedExecutable 32 (handle 6) #[]
  native_decide

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction preparedExecutable (handle 7) {} #[]
      finalState outcome.value := by
  apply successfulFixtureRunHasDerivation preparedExecutable 32 (handle 7) #[]
  native_decide

end LeanerIR.Tests.Interpreter
