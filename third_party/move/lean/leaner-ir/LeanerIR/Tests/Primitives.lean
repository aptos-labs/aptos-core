-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Interpreter
import LeanerIR.Validation.Check
import LeanerIR.Semantics.Typing

namespace LeanerIR.Tests.Primitives

open LeanerIR
open LeanerIR.Import
open LeanerIR.SemanticOperations
open LeanerIR.Validation

private def testNamespace : ValidatedNamespace := {
  loc := ⟨0⟩
  identity := ⟨0⟩
  tables := { types := #[.integer (.bits 8) false] } }

private def signedTestNamespace : ValidatedNamespace := {
  loc := ⟨0⟩
  identity := ⟨0⟩
  tables := { types := #[.integer (.bits 8) true] } }

private def pointerTestNamespace : ValidatedNamespace := {
  loc := ⟨0⟩
  identity := ⟨0⟩
  tables := { types := #[.integer .pointer false, .integer .pointer true, .bool,
    .tuple #[⟨0⟩, ⟨2⟩]] } }

-- Insertion permits the end index; removal returns the removed value and
-- rejects the end index. Both reject negative indexes and preserve siblings.
example : insertVector? #[.vector #[], .integer 0, .integer 7] =
  some (.ok (.vector #[.integer 7])) := by simp [insertVector?, Array.insertIdxIfInBounds]
example : insertVector? #[.vector #[.integer 10, .integer 30], .integer 1, .integer 20] =
  some (.ok (.vector #[.integer 10, .integer 20, .integer 30])) := by
    simp [insertVector?, Array.insertIdxIfInBounds]
example : insertVector? #[.vector #[.integer 10], .integer 1, .integer 20] =
  some (.ok (.vector #[.integer 10, .integer 20])) := by
    simp [insertVector?, Array.insertIdxIfInBounds]
example : insertVector? #[.vector #[.integer 10], .integer 2, .integer 20] =
  some (.error (.abort, #[.integer 2])) := by rfl
example : insertVector? #[.vector #[.integer 10], .integer (-1), .integer 20] =
  some (.error (.abort, #[.integer (-1)])) := by rfl
example : removeVector? #[.vector #[.integer 10, .integer 20, .integer 30], .integer 1] =
  some (.ok (.tuple #[.integer 20, .vector #[.integer 10, .integer 30]])) := by
    simp [removeVector?, Array.eraseIdxIfInBounds]
example : removeVector? #[.vector #[.integer 7], .integer 0] =
  some (.ok (.tuple #[.integer 7, .vector #[]])) := by
    simp [removeVector?, Array.eraseIdxIfInBounds]
example : removeVector? #[.vector #[], .integer 0] = some (.error (.abort, #[.integer 0])) := by rfl
example : removeVector? #[.vector #[.integer 7], .integer 1] =
  some (.error (.abort, #[.integer 1])) := by rfl
example : removeVector? #[.vector #[.integer 7], .integer (-1)] =
  some (.error (.abort, #[.integer (-1)])) := by rfl
#guard (insertVector? #[.bool true, .integer 0, .integer 7]).isNone
#guard (removeVector? #[.vector #[], .bool true]).isNone

#guard (evaluatePrimitiveOperation? pointerTestNamespace ⟨0⟩ .add
  #[.integer 65535, .integer 1]).isNone

#guard (evaluatePrimitiveOperation? pointerTestNamespace ⟨0⟩ .add
  #[.integer 255, .integer 1] (some 8)).isNone

#guard match evaluatePrimitiveOperation? pointerTestNamespace ⟨0⟩ .add
    #[.integer 65535, .integer 1] (some 16) with
  | some (.ok (.integer 0)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? pointerTestNamespace ⟨1⟩ (.checkedAdd .panic)
    #[.integer 32767, .integer 1] (some 16) with
  | some (.error (.panic, #[.integer 32768])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? pointerTestNamespace ⟨3⟩ .overflowingAdd
    #[.integer 65535, .integer 1] (some 16) with
  | some (.ok (.tuple #[.integer 0, .bool true])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? pointerTestNamespace ⟨0⟩ .cast
    #[.integer (-1)] (some 16) with
  | some (.ok (.integer 65535)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? pointerTestNamespace ⟨1⟩ .shiftRight
    #[.integer (-2), .integer 1] (some 16) with
  | some (.ok (.integer (-1))) => true
  | _ => false

#guard match evaluatePrimitiveOperation? pointerTestNamespace ⟨0⟩ .length
    #[.vector #[.integer 10, .integer 20, .integer 30]] (some 16) with
  | some (.ok (.integer 3)) => true
  | _ => false

#guard constValue? (.address "0x1") == some (.address "0x1")
#guard constValue? (.character 0x1f980) == some (.character 0x1f980)
#guard (constValue? (.character 0xd800)).isNone
#guard (constValue? (.character 0x110000)).isNone

private def textTypingTables : Tables := {
  types := #[.string, .bytes] }

example : ValueHasType default textTypingTables (.string "Leaner") ⟨0⟩ := by
  exact .string ⟨0⟩ "Leaner" rfl

example : ValueHasType default textTypingTables (.bytes #[0, 127, 255]) ⟨1⟩ := by
  exact .bytes ⟨1⟩ #[0, 127, 255] rfl

private def textTestNamespace : ValidatedNamespace := {
  loc := ⟨0⟩
  identity := ⟨0⟩
  tables := { types := #[.string, .bytes, .integer (.bits 64) false] } }

#guard match evaluatePrimitiveOperation? textTestNamespace ⟨2⟩ .length
    #[.string "Lean🦀"] with
  | some (.ok (.integer 8)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? textTestNamespace ⟨2⟩ .length
    #[.bytes #[0, 127, 255]] with
  | some (.ok (.integer 3)) => true
  | _ => false

private def characterTestNamespace : ValidatedNamespace := {
  loc := ⟨0⟩
  identity := ⟨0⟩
  tables := { types := #[.character, .bool,
    .integer (.bits 8) false, .integer (.bits 32) false] } }

#guard match evaluatePrimitiveOperation? characterTestNamespace ⟨1⟩ .less
    #[.character 0x61, .character 0x1f980] with
  | some (.ok (.bool true)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? characterTestNamespace ⟨1⟩ .greaterEqual
    #[.character 0x1f980, .character 0x1f980] with
  | some (.ok (.bool true)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? characterTestNamespace ⟨3⟩ .cast
    #[.character 0x1f980] with
  | some (.ok (.integer 0x1f980)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? characterTestNamespace ⟨0⟩ .cast
    #[.integer 0x61] with
  | some (.ok (.character 0x61)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .add
    #[.integer 255, .integer 1] with
  | some (.ok (.integer 0)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedAdd .abort)
    #[.integer 255, .integer 1] with
  | some (.error (.abort, #[.integer 256])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedSubtract .panic)
    #[.integer 0, .integer 1] with
  | some (.error (.panic, #[.integer (-1)])) => true
  | _ => false

#guard (evaluatePrimitiveOperation? testNamespace ⟨0⟩ .divide
  #[.integer 1, .integer 0]).isNone

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedDivide .panic)
    #[.integer 1, .integer 0] with
  | some (.error (.panic, #[])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .divide
    #[.integer (-7), .integer 3] with
  | some (.ok (.integer (-2))) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .divide
    #[.integer 7, .integer (-3)] with
  | some (.ok (.integer (-2))) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .modulo
    #[.integer (-7), .integer 3] with
  | some (.ok (.integer (-1))) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .modulo
    #[.integer 7, .integer (-3)] with
  | some (.ok (.integer 1)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ (.checkedDivide .panic)
    #[.integer (-128), .integer (-1)] with
  | some (.error (.panic, #[.integer 128])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ (.checkedModulo .abort)
    #[.integer (-128), .integer (-1)] with
  | some (.error (.abort, #[.integer 128])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .bitwiseOr
    #[.integer 240, .integer 15] with
  | some (.ok (.integer 255)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .bitwiseAnd
    #[.integer 240, .integer 15] with
  | some (.ok (.integer 0)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .bitwiseXor
    #[.integer 170, .integer 255] with
  | some (.ok (.integer 85)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .bitwiseAnd
    #[.integer (-1), .integer 15] with
  | some (.ok (.integer 15)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .bitwiseXor
    #[.integer (-1), .integer 0] with
  | some (.ok (.integer (-1))) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .bitwiseNot #[.integer 15] with
  | some (.ok (.integer 240)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .bitwiseNot #[.integer 0] with
  | some (.ok (.integer (-1))) => true
  | _ => false

private def overflowingTestNamespace : ValidatedNamespace := {
  testNamespace with
  tables := { testNamespace.tables with
    types := #[.integer (.bits 8) false, .bool, .tuple #[⟨0⟩, ⟨1⟩]] }
}

#guard match evaluatePrimitiveOperation? overflowingTestNamespace ⟨2⟩ .overflowingAdd
    #[.integer 250, .integer 10] with
  | some (.ok (.tuple #[.integer 4, .bool true])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? overflowingTestNamespace ⟨2⟩ .overflowingMultiply
    #[.integer 10, .integer 12] with
  | some (.ok (.tuple #[.integer 120, .bool false])) => true
  | _ => false

private def signedOverflowingTestNamespace : ValidatedNamespace := {
  signedTestNamespace with
  tables := { signedTestNamespace.tables with
    types := #[.integer (.bits 8) true, .bool, .tuple #[⟨0⟩, ⟨1⟩]] }
}

#guard match evaluatePrimitiveOperation? signedOverflowingTestNamespace ⟨2⟩
    .overflowingSubtract #[.integer (-120), .integer 20] with
  | some (.ok (.tuple #[.integer 116, .bool true])) => true
  | _ => false

private def boolTestNamespace : ValidatedNamespace := {
  loc := ⟨0⟩
  identity := ⟨0⟩
  tables := { types := #[.bool] } }

#guard match evaluatePrimitiveOperation? boolTestNamespace ⟨0⟩ .bitwiseAnd
    #[.bool true, .bool false] with
  | some (.ok (.bool false)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? boolTestNamespace ⟨0⟩ .bitwiseOr
    #[.bool true, .bool false] with
  | some (.ok (.bool true)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? boolTestNamespace ⟨0⟩ .bitwiseXor
    #[.bool true, .bool true] with
  | some (.ok (.bool false)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? boolTestNamespace ⟨0⟩ .less
    #[.bool false, .bool true] with
  | some (.ok (.bool true)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? boolTestNamespace ⟨0⟩ .greaterEqual
    #[.bool false, .bool true] with
  | some (.ok (.bool false)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedShiftLeft .abort)
    #[.integer 128, .integer 1] with
  | some (.ok (.integer 0)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .shiftLeft
    #[.integer 1, .integer 7] with
  | some (.ok (.integer 128)) => true
  | _ => false

#guard (evaluatePrimitiveOperation? testNamespace ⟨0⟩ .shiftLeft
  #[.integer 1, .integer 8]).isNone

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedShiftRight .panic)
    #[.integer 128, .integer 7] with
  | some (.ok (.integer 1)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ (.checkedShiftRight .panic)
    #[.integer (-2), .integer 1] with
  | some (.ok (.integer (-1))) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedShiftLeft .abort)
    #[.integer 1, .integer 8] with
  | some (.error (.abort, #[.integer 8])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedCast .abort)
    #[.integer 255] with
  | some (.ok (.integer 255)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ (.checkedCast .abort)
    #[.integer 256] with
  | some (.error (.abort, #[.integer 256])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ (.checkedCast .panic)
    #[.integer 128] with
  | some (.error (.panic, #[.integer 128])) => true
  | _ => false

#guard match evaluatePrimitiveOperation? testNamespace ⟨0⟩ .cast #[.integer (-1)] with
  | some (.ok (.integer 255)) => true
  | _ => false

#guard match evaluatePrimitiveOperation? signedTestNamespace ⟨0⟩ .cast #[.integer 255] with
  | some (.ok (.integer (-1))) => true
  | _ => false

private def config : ProfileConfig := { profile := .rust, name := "rust-test" }
private def schema : ProfileSchema := { profile := .rust, name := "rust-test" }
private def semantics : SemanticProfile := {
  profile := .rust, name := "rust-test", classify := fun _ _ => none }

private def typeUse (typeId loc : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨loc⟩ }

/-- A single expression tree exercises all three executable bitwise forms:
`((170 xor 255) or 240) and 15 = 5`. -/
private def fixture : RawUnit where
  tables := {
    files := #[{ name := "bitwise.rs" }]
    locations := (Array.range 8).map fun index => {
      primary := some { file := ⟨0⟩, startByte := index, endByte := index + 1 } }
    origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
    alignments := #[{
      source := ⟨0⟩, trust := .checked, description := "bitwise fixture" }]
    types := #[.integer (.bits 8) false]
    namespaces := #[{ segments := #["test", "Primitives"] }]
    names := #[{ namespaceId := ⟨0⟩, name := "main" }] }
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[
      { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.integer 170) },
      { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .value (.integer 255) },
      { loc := ⟨2⟩, typeId := ⟨0⟩,
        kind := .operation (.primitive .bitwiseXor) #[] #[⟨0⟩, ⟨1⟩] },
      { loc := ⟨3⟩, typeId := ⟨0⟩, kind := .value (.integer 240) },
      { loc := ⟨4⟩, typeId := ⟨0⟩,
        kind := .operation (.primitive .bitwiseOr) #[] #[⟨2⟩, ⟨3⟩] },
      { loc := ⟨5⟩, typeId := ⟨0⟩, kind := .value (.integer 15) },
      { loc := ⟨6⟩, typeId := ⟨0⟩,
        kind := .operation (.primitive .bitwiseAnd) #[] #[⟨4⟩, ⟨5⟩] }]
    functions := #[{
      loc := ⟨7⟩
      name := ⟨0⟩
      profile := .rust
      signature := { results := #[typeUse 0 7] }
      body := .structured ⟨6⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }]

private def executable? : Option ExecutableUnit := do
  let checked ← (validate #[schema] fixture).toOption
  (prepareExecution #[semantics] checked).toOption

private def preparationHasDiagnosticAt (raw : RawUnit) (code : String) (loc : LocId) : Bool :=
  match validate #[schema] raw with
  | .error _ => false
  | .ok checked => match prepareExecution #[semantics] checked with
    | .error diagnostics => diagnostics.any fun diagnostic =>
        diagnostic.code == code && diagnostic.primary == some loc
    | .ok _ => false

private def validationHasDiagnosticAt (raw : RawUnit) (code : String) (loc : LocId) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == code && diagnostic.primary == some loc
  | .ok _ => false

private def vectorEditFixture (operation : PrimitiveOperation)
    (resultType : Nat) (badIndex := false) (badElement := false) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := #[.integer (.bits 8) false, .bool,
      .vector ⟨0⟩ none, .tuple #[⟨0⟩, ⟨2⟩], .tuple #[⟨1⟩, ⟨2⟩]] }
    namespaces := #[{ ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .operation (.primitive .vector) #[] #[] },
        { loc := ⟨1⟩, typeId := ⟨if badIndex then 1 else 0⟩,
          kind := .value (if badIndex then .bool true else .integer 0) },
        { loc := ⟨2⟩, typeId := ⟨if badElement then 1 else 0⟩,
          kind := .value (if badElement then .bool true else .integer 7) },
        { loc := ⟨3⟩, typeId := ⟨resultType⟩,
          kind := .operation (.primitive operation) #[]
            (if operation == .insertVector then #[⟨0⟩, ⟨1⟩, ⟨2⟩] else #[⟨0⟩, ⟨1⟩]) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse resultType 7] }
        body := .structured ⟨3⟩ }] }] }

#guard (validate #[schema] (vectorEditFixture .insertVector 2)).isOk
#guard (validate #[schema] (vectorEditFixture .removeVector 3)).isOk
#guard validationHasDiagnosticAt (vectorEditFixture .insertVector 2 true)
  "LIR-SEMANTIC-TYPE" ⟨3⟩
#guard validationHasDiagnosticAt (vectorEditFixture .insertVector 2 false true)
  "LIR-SEMANTIC-TYPE" ⟨3⟩
#guard validationHasDiagnosticAt (vectorEditFixture .removeVector 3 true)
  "LIR-SEMANTIC-TYPE" ⟨3⟩
#guard validationHasDiagnosticAt (vectorEditFixture .removeVector 2)
  "LIR-SEMANTIC-TYPE" ⟨3⟩
#guard validationHasDiagnosticAt (vectorEditFixture .removeVector 4)
  "LIR-SEMANTIC-TYPE" ⟨3⟩

private def pointerWidthFixture (width : Option String) : RawUnit :=
  let options := width.map (fun value => #[
    ("target_pointer_width", value)]) |>.getD #[]
  { fixture with
    profiles := #[{ config with options }]
    tables := { fixture.tables with types := #[.integer .pointer false] } }

#guard preparationHasDiagnosticAt (pointerWidthFixture none) "LIR-EXEC-UNSUPPORTED" ⟨7⟩

#guard match validate #[schema] (pointerWidthFixture (some "16")) with
  | .ok checked => match prepareExecution #[semantics] checked with
      | .ok executable => executable.targetPointerWidth == some 16 &&
          (match Interpreter.run executable 16
              { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
            | .ok (_, { value := .returned #[.integer 5], .. }) => true
            | _ => false)
      | .error _ => false
  | .error _ => false

#guard match validate #[schema] (pointerWidthFixture (some "16")) with
  | .ok checked => match prepareVerification #[semantics] checked with
      | .ok verifiable => verifiable.targetPointerWidth == some 16
      | .error _ => false
  | .error _ => false

private def characterRangeFixture (lower upper : Nat) (inclusive : Bool := true) : RawUnit :=
  let ns := fixture.namespaces[0]!
  let rangePattern : Pattern :=
    { loc := ⟨3⟩, typeId := ⟨0⟩,
      kind := .range (some (.character lower)) (some (.character upper)) inclusive }
  { fixture with
    tables := { fixture.tables with types := #[.character] }
    namespaces := #[{ ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.character 0x6d) },
        { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .value (.character 0x1f980) },
        { loc := ⟨2⟩, typeId := ⟨0⟩,
          kind := .match_ ⟨0⟩ #[{ pattern := ⟨0⟩, body := ⟨1⟩ }] }]
      patterns := #[rangePattern]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[typeUse 0 7] }
        body := .structured ⟨2⟩ }] }] }

#guard match validate #[schema] (characterRangeFixture 0x61 0x7a) with
  | .ok checked => (prepareExecution #[semantics] checked).isOk
  | .error _ => false

#guard validationHasDiagnosticAt (characterRangeFixture 0x7a 0x61)
  "LIR-SEMANTIC-TYPE" ⟨3⟩

#guard validationHasDiagnosticAt (characterRangeFixture 0x61 0x61 false)
  "LIR-SEMANTIC-TYPE" ⟨3⟩

private def characterRangeNamespace : ValidatedNamespace :=
  let rangePattern : Pattern :=
    { loc := ⟨0⟩, typeId := ⟨0⟩,
      kind := .range (some (.character 0x61)) (some (.character 0x7a)) true }
  { loc := ⟨0⟩
    identity := ⟨0⟩
    tables := { types := #[.character] }
    patterns := #[rangePattern] }

#guard (bindPattern default characterRangeNamespace {} ⟨0⟩ (.character 0x6d)).isSome
#guard (bindPattern default characterRangeNamespace {} ⟨0⟩ (.character 0x1f980)).isNone

private def badBitwiseOperandFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push .bool }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 1 {
        ns.expressions[1]! with typeId := ⟨1⟩, kind := .value (.bool true) } }] }

#guard validationHasDiagnosticAt badBitwiseOperandFixture "LIR-SEMANTIC-TYPE" ⟨2⟩

private def tupleArityFixture : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with types := fixture.tables.types.push (.tuple #[⟨0⟩, ⟨0⟩]) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with
        typeId := ⟨1⟩
        kind := .operation (.primitive .tuple) #[] #[⟨0⟩] }
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 1 7] }
        body := .structured ⟨2⟩ }] }] }

#guard validationHasDiagnosticAt tupleArityFixture "LIR-SEMANTIC-ARITY" ⟨2⟩

private def fixedVectorFixture (length : Int) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.vector ⟨0⟩ (some (.integer length))) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with
        typeId := ⟨1⟩
        kind := .operation (.primitive .vector) #[] #[⟨0⟩, ⟨1⟩] }
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 1 7] }
        body := .structured ⟨2⟩ }] }] }

#guard match validate #[schema] (fixedVectorFixture 2) with
  | .ok checked => (prepareExecution #[semantics] checked).isOk
  | .error _ => false

#guard validationHasDiagnosticAt (fixedVectorFixture 3) "LIR-SEMANTIC-TYPE" ⟨2⟩

private def repeatedVectorFixture (length : Option ConstValue) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.vector ⟨0⟩ length) }
    namespaces := #[{
      ns with
      expressions := ns.expressions.set! 2 {
        ns.expressions[2]! with
        typeId := ⟨1⟩
        kind := .operation (.primitive .repeatVector) #[] #[⟨0⟩] }
      functions := #[{
        ns.functions[0]! with
        signature := { results := #[typeUse 1 7] }
        body := .structured ⟨2⟩ }] }] }

#guard match validate #[schema] (repeatedVectorFixture (some (.integer 4))) with
  | .ok checked => match prepareExecution #[semantics] checked with
      | .ok executable => match Interpreter.run executable 16
          { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[] with
          | .ok (_, { value := .returned #[.vector values], .. }) =>
              values == #[.integer 170, .integer 170, .integer 170, .integer 170]
          | _ => false
      | .error _ => false
  | .error _ => false

#guard validationHasDiagnosticAt (repeatedVectorFixture none)
  "LIR-SEMANTIC-TYPE" ⟨2⟩

private def nonCopyRepeatedVectorFixture : RawUnit :=
  let raw := repeatedVectorFixture (some (.integer 2))
  let ns := raw.namespaces[0]!
  let mutableReference : Ty := .reference {
    profile := .rust
    kind := .mutable
    referent := ⟨0⟩
    lifetime := ⟨0⟩ }
  { raw with
    tables := {
      raw.tables with
      lifetimes := #[{ kind := .local, loc := ⟨0⟩ }]
      types := (raw.tables.types.push mutableReference).push
        (.vector ⟨2⟩ (some (.integer 2))) }
    namespaces := #[{
      ns with
      expressions := ns.expressions
        |>.set! 0 { ns.expressions[0]! with
          typeId := ⟨2⟩, kind := .localVar ⟨0⟩ }
        |>.set! 2 { ns.expressions[2]! with typeId := ⟨3⟩ }
      functions := #[{
        ns.functions[0]! with
        signature := {
          parameters := #[{ name := "value", typeUse := typeUse 2 7 }]
          results := #[typeUse 3 7] }
        locals := #[{
          id := ⟨0⟩, name := "value", type := typeUse 2 7,
          mutable := false, loc := ⟨7⟩ }] }] }] }

#guard preparationHasDiagnosticAt nonCopyRepeatedVectorFixture
  "LIR-SEMANTIC-ABILITY" ⟨2⟩

private def handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }

private def binaryPrimitiveFixture (operation : PrimitiveOperation)
    (left right : Int) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.integer left) },
        { loc := ⟨1⟩, typeId := ⟨0⟩, kind := .value (.integer right) },
        { loc := ⟨2⟩, typeId := ⟨0⟩,
          kind := .operation (.primitive operation) #[] #[⟨0⟩, ⟨1⟩] }]
      functions := #[{ ns.functions[0]! with body := .structured ⟨2⟩ }] }] }

private def preparedBinary? (operation : PrimitiveOperation)
    (left right : Int) : Option ExecutableUnit := do
  let checked ← (validate #[schema] (binaryPrimitiveFixture operation left right)).toOption
  (prepareExecution #[semantics] checked).toOption

private def shiftExecutable? : Option ExecutableUnit :=
  preparedBinary? (.checkedShiftLeft .panic) 1 1

private def castFixture (operation : PrimitiveOperation) (value : Int) : RawUnit :=
  let ns := fixture.namespaces[0]!
  { fixture with
    tables := { fixture.tables with
      types := fixture.tables.types.push (.integer (.bits 16) false) }
    namespaces := #[{
      ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer value) },
        { loc := ⟨1⟩, typeId := ⟨0⟩,
          kind := .operation (.primitive operation) #[] #[⟨0⟩] }]
      functions := #[{ ns.functions[0]! with body := .structured ⟨1⟩ }] }] }

private def preparedCast? (operation : PrimitiveOperation) (value : Int) :
    Option ExecutableUnit := do
  let checked ← (validate #[schema] (castFixture operation value)).toOption
  (prepareExecution #[semantics] checked).toOption

private def castExecutable? : Option ExecutableUnit :=
  preparedCast? (.checkedCast .abort) 255

#guard match shiftExecutable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.integer 2], .. }) => true
      | _ => false
  | none => false

#guard match preparedBinary? (.checkedShiftLeft .panic) 1 8 with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .threw .panic #[.integer 8], .. }) => true
      | _ => false
  | none => false

#guard match preparedBinary? .shiftLeft 1 1 with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.integer 2], .. }) => true
      | _ => false
  | none => false

#guard match castExecutable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.integer 255], .. }) => true
      | _ => false
  | none => false

#guard match preparedCast? (.checkedCast .abort) 256 with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .threw .abort #[.integer 256], .. }) => true
      | _ => false
  | none => false

#guard match preparedCast? .cast 256 with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.integer 0], .. }) => true
      | _ => false
  | none => false

#guard match executable? with
  | some executable => match Interpreter.run executable 16 handle #[] with
      | .ok (_, { value := .returned #[.integer 5], .. }) => true
      | _ => false
  | none => false

private def prepared : ExecutableUnit := executable?.get (by native_decide)

private def shiftPrepared : ExecutableUnit := shiftExecutable?.get (by native_decide)

private def castPrepared : ExecutableUnit := castExecutable?.get (by native_decide)

private def integerTypingTables : Tables where
  types := #[.integer (.bits 8) false]

example : ValueHasType default integerTypingTables (.integer 255) ⟨0⟩ := by
  apply ValueHasType.integer (width := .bits 8) (signed := false)
  · rfl
  · rfl

example : ¬ ValueHasType default integerTypingTables (.integer 256) ⟨0⟩ := by
  intro typed
  cases typed with
  | integer _ _ _ _ type_eq value_fits =>
      simp [integerTypingTables] at type_eq
      rcases type_eq with ⟨rfl, rfl⟩
      simp [IntegerValueFits, Ty.integerValueFits?, Ty.integerBounds?] at value_fits
  | sharedReference _ _ _ type_eq _ _ =>
      simp [integerTypingTables] at type_eq

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction castPrepared handle #[] {} #[] finalState outcome.value := by
  have success : (Interpreter.run castPrepared 16 handle #[]).isOk := by native_decide
  generalize result_eq : Interpreter.run castPrepared 16 handle #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound castPrepared 16 handle #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction shiftPrepared handle #[] {} #[] finalState outcome.value := by
  have success : (Interpreter.run shiftPrepared 16 handle #[]).isOk := by native_decide
  generalize result_eq : Interpreter.run shiftPrepared 16 handle #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound shiftPrepared 16 handle #[] {}
          result.1 result.2 result_eq⟩

example : ∃ (finalState : RuntimeState) (outcome : LocatedOutcome),
    BigStep.EvalFunction prepared handle #[] {} #[] finalState outcome.value := by
  have success : (Interpreter.run prepared 16 handle #[]).isOk := by native_decide
  generalize result_eq : Interpreter.run prepared 16 handle #[] = result
  cases result with
  | error error => simp [result_eq, Except.isOk, Except.toBool] at success
  | ok result =>
      exact ⟨result.1, result.2,
        LeanerIR.Proofs.Interpreter.run_sound prepared 16 handle #[] {}
          result.1 result.2 result_eq⟩

end LeanerIR.Tests.Primitives
