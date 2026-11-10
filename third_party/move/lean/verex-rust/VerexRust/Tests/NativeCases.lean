-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Elab.Command
import VerexRust.Profile
import VerexIR.Proofs.Denote.Compile

/-!
# Every primitive and type the Rust profile admits has a native case

Each primitive of the shared vocabulary, and each type the semantic inventory
classifies, is taken once: a function holding it is validated and typed, then
compiled to a term. `primitiveCase` matches every constructor, so a new one
does not build until it states a case; `checkInventory` fails if a case is
missing from the lists run here.
-/

namespace VerexIR.Rust.Tests.NativeCases

open Lean
open VerexIR.Import
open VerexIR.Validation
open VerexIR.Proofs.Denote

/-- The type table every case reads. -/
private def types : Array Ty := #[
  /- 0 -/ .unit, /- 1 -/ .bool, /- 2 -/ .integer (.bits 32) false,
  /- 3 -/ .integer (.bits 32) true, /- 4 -/ .integer .pointer false, /- 5 -/ .character,
  /- 6 -/ .tuple #[⟨2⟩, ⟨1⟩], /- 7 -/ .vector ⟨2⟩, /- 8 -/ .vector ⟨2⟩ (some (.integer 2)),
  /- 9 -/ .never, /- 10 -/ .string, /- 11 -/ .bytes, /- 12 -/ .address, /- 13 -/ .signer,
  /- 14 -/ .integer (.bits 8) true, /- 15 -/ .tuple #[⟨2⟩, ⟨7⟩], /- 16 -/ .tuple #[⟨1⟩, ⟨4⟩],
  /- 17 -/ .reference { profile := .rust, kind := .shared, referent := ⟨13⟩, lifetime := ⟨0⟩ }]

private abbrev unit' : Nat := 0
private abbrev bool' : Nat := 1
private abbrev u32' : Nat := 2
private abbrev i32' : Nat := 3
private abbrev usize' : Nat := 4
private abbrev char' : Nat := 5
private abbrev overflow' : Nat := 6
private abbrev vector' : Nat := 7
private abbrev array' : Nat := 8
private abbrev string' : Nat := 10
private abbrev address' : Nat := 12
private abbrev signerRef' : Nat := 17
private abbrev order' : Nat := 14
private abbrev removed' : Nat := 15
private abbrev found' : Nat := 16

/-- A primitive's native case: the operand and result types of a use of it, or
why a body of the Rust profile never reaches it. -/
private inductive PrimitiveCase where
  | carried (operands : List Nat) (result : Nat)
  | exempt (reason : String)

private def logical : String := "a specification's connective, classified logical only"

private def primitiveCase : PrimitiveOperation → PrimitiveCase
  | .tuple => .carried [u32', bool'] overflow'
  | .vector => .carried [u32', u32'] array'
  | .repeatVector => .carried [u32'] array'
  | .pushVector => .carried [vector', u32'] vector'
  | .concatVector => .carried [vector', vector'] vector'
  | .insertVector => .carried [vector', usize', u32'] vector'
  | .removeVector => .carried [vector', usize'] removed'
  | .swapVector => .carried [vector', usize', usize'] vector'
  | .reverseSliceVector => .carried [vector', usize', usize'] vector'
  | .destroyEmptyVector => .carried [vector'] unit'
  | .containsVector => .carried [vector', u32'] bool'
  | .indexOfVector => .carried [vector', u32'] found'
  | .checkVectorIndex _ => .carried [vector', usize'] unit'
  | .length => .carried [vector'] usize'
  | .index => .carried [vector', usize'] u32'
  | .slice => .carried [vector', usize', usize'] vector'
  | .add | .subtract | .multiply | .modulo | .divide => .carried [u32', u32'] u32'
  | .checkedAdd _ | .checkedSubtract _ | .checkedMultiply _ | .checkedModulo _ | .checkedDivide _ =>
      .carried [u32', u32'] u32'
  | .overflowingAdd | .overflowingSubtract | .overflowingMultiply =>
      .carried [u32', u32'] overflow'
  | .bitwiseOr | .bitwiseAnd | .bitwiseXor => .carried [i32', i32'] i32'
  | .bitwiseNot => .carried [i32'] i32'
  | .shiftLeft | .shiftRight => .carried [i32', u32'] i32'
  | .checkedShiftLeft _ | .checkedShiftRight _ => .carried [i32', i32'] i32'
  | .logicalAnd | .logicalOr => .carried [bool', bool'] bool'
  | .equal | .notEqual => .carried [char', char'] bool'
  | .less | .greater | .lessEqual | .greaterEqual => .carried [char', char'] bool'
  | .compare => .carried [u32', u32'] order'
  | .signerAddress => .carried [signerRef'] address'
  | .logicalNot => .carried [bool'] bool'
  | .negate => .carried [i32'] i32'
  | .checkedNegate _ => .carried [i32'] i32'
  | .copyValue | .moveValue => .carried [u32'] u32'
  | .cast => .carried [char'] i32'
  | .checkedCast _ => .carried [i32'] u32'
  | .range => .exempt "no executable meaning: construction is classified unsupported"
  | .implies | .equivalent | .identical => .exempt logical

/-- Each primitive once, a failure-carrying one at the panic the Rust profile
folds its checks into. -/
private def primitives : List PrimitiveOperation := [
  .tuple, .vector, .repeatVector, .pushVector, .concatVector, .insertVector, .removeVector,
  .swapVector, .reverseSliceVector, .destroyEmptyVector, .containsVector, .indexOfVector,
  .checkVectorIndex .panic, .length, .index, .slice, .add, .checkedAdd .panic, .overflowingAdd,
  .subtract, .checkedSubtract .panic, .overflowingSubtract, .multiply, .checkedMultiply .panic,
  .overflowingMultiply, .modulo, .checkedModulo .panic, .divide, .checkedDivide .panic, .bitwiseOr,
  .bitwiseAnd, .bitwiseXor, .bitwiseNot, .shiftLeft, .checkedShiftLeft .panic, .shiftRight,
  .checkedShiftRight .panic, .logicalAnd, .logicalOr, .equal, .notEqual, .less, .greater,
  .lessEqual, .greaterEqual, .compare, .signerAddress, .logicalNot, .negate, .checkedNegate .panic,
  .copyValue, .moveValue, .cast, .checkedCast .panic, .range, .implies, .equivalent, .identical]

/-- Each type the inventory tells apart, and what its table needs. -/
private def typeCases : List Ty := [
  .unit, .never, .bool, .character, .string, .bytes, .address, .signer,
  .integer (.bits 32) false, .integer .pointer false, .integer .unbounded true,
  .tuple #[⟨0⟩], .vector ⟨0⟩, .vector ⟨0⟩ (some (.integer 2)), .slice ⟨0⟩, .range, .eventStore,
  .typeDomain ⟨0⟩, .resourceDomain ⟨0⟩, .stateDomain, .nominal ⟨1⟩ #[],
  .function #[⟨0⟩] ⟨0⟩, .typeParameter 0,
  .reference { profile := .rust, kind := .shared, referent := ⟨0⟩, lifetime := ⟨0⟩ },
  .reference { profile := .rust, kind := .mutable, referent := ⟨0⟩, lifetime := ⟨0⟩ },
  .profile { profile := .rust, tag := "fixture" }]

private def typeUse (typeId : Nat) : TypeUse := { typeId := ⟨typeId⟩, loc := ⟨0⟩ }

/-- The tables of a unit naming one function and one struct. -/
private def tables (types : Array Ty) : Tables where
  files := #[{ name := "fixture.rs", contentHash := "native-cases" }]
  locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
  origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
  alignments := #[{ source := ⟨0⟩, trust := .checked, description := "native cases" }]
  types
  namespaces := #[{ segments := #["fixture"] }]
  names := #[{ namespaceId := ⟨0⟩, name := "f" }, { namespaceId := ⟨0⟩, name := "S" },
    { namespaceId := ⟨0⟩, name := "x" }]
  lifetimes := #[{ kind := .static, loc := ⟨0⟩ }]

/-- A unit of one function over parameters of the given types. -/
private def unitOf (types : Array Ty) (generics : Array GenericBinder) (operands : List Nat)
    (result : Nat) (expressions : Array Expr) (root : Nat) : RawUnit where
  tables := tables types
  profiles := #[VerexIR.Rust.config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions
    structs := #[{ loc := ⟨0⟩, name := ⟨1⟩, fields := #[{ loc := ⟨0⟩, name := ⟨2⟩, type := typeUse 0 }] }]
    functions := #[{
      loc := ⟨0⟩
      name := ⟨0⟩
      profile := .rust
      signature := {
        generics
        parameters := operands.toArray.map fun typeId =>
          { name := "x", typeUse := typeUse typeId }
        results := #[typeUse result] }
      locals := (operands.toArray.mapIdx fun index typeId =>
        { id := ⟨index⟩, name := "x", type := typeUse typeId, loc := ⟨0⟩ })
      body := .structured ⟨root⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }]

private def diagnostics (found : Array Diagnostic) : String :=
  ", ".intercalate (found.toList.map (·.message))

private def compiled (raw : RawUnit) : Except String Unit := do
  let unit ← (VerexIR.Rust.validate raw).mapError diagnostics
  let _ ← (prepareExecution #[VerexIR.Rust.semantics] unit).mapError fun found =>
    diagnostics found
  let _ ← compileFunction unit ⟨⟨0⟩, ⟨0⟩⟩
  return ()

/-- The function applying a primitive to its parameters. -/
private def primitiveUnit (primitive : PrimitiveOperation) (operands : List Nat) (result : Nat) :
    RawUnit :=
  let reads : Array Expr := (operands.toArray.mapIdx fun index typeId =>
    { loc := ⟨0⟩, typeId := ⟨typeId⟩, kind := .localVar ⟨index⟩ })
  let arguments : Array ExprId := (Array.range operands.length).map (⟨·⟩)
  let node : Expr :=
    { loc := ⟨0⟩, typeId := ⟨result⟩, kind := .operation (.primitive primitive) #[] arguments none }
  unitOf types #[] operands result (reads.push node) operands.length

/-- The function taking a value of a type and returning the unit. -/
private def typeUnit (type : Ty) : RawUnit :=
  let generics : Array GenericBinder := match type with
    | .typeParameter _ => #[{ name := "T", kind := .typeArg, loc := ⟨0⟩ }]
    | _ => #[]
  let node : Expr := { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value .unit }
  match type with
  | .slice _ =>
      unitOf #[.unit, type,
        .reference { profile := .rust, kind := .shared, referent := ⟨1⟩, lifetime := ⟨0⟩ }]
        generics [2] 0 #[node] 0
  | _ => unitOf #[.unit, type] generics [1] 0 #[node] 0

/-- Whether the inventory classifies a type as one a Rust body may hold. -/
private def admitted (feature : SemanticFeature) : Bool :=
  feature.classification == .executable

private def checkPrimitives : IO Unit := do
  for primitive in primitives do
    let feature := corePrimitiveFeature primitive
    match primitiveCase primitive with
    | .exempt reason =>
        if admitted feature then
          throw <| IO.userError s!"{feature.name} is admitted yet exempt: {reason}"
    | .carried operands result =>
        match compiled (primitiveUnit primitive operands result) with
        | .ok () => pure ()
        | .error message =>
            throw <| IO.userError s!"{feature.name} has no native case: {message}"

private def checkTypes : IO Unit := do
  for type in typeCases do
    let feature := coreTypeFeature type
    if admitted feature then
      match compiled (typeUnit type) with
      | .ok () => pure ()
      | .error message => throw <| IO.userError s!"{feature.name} has no native case: {message}"

/-- The lists hold one entry per constructor; `integer` and `vector` count once
per form the inventory tells apart, a reference once per kind. -/
private def checkInventory : MetaM Unit := do
  let constructors (name : Name) : MetaM Nat := do
    let .inductInfo info ← getConstInfo name | throwError "{name} is not inductive"
    return info.ctors.length
  let primitiveNames := (primitives.map (corePrimitiveFeature · |>.name)).eraseDups
  unless primitiveNames.length == primitives.length do
    throwError "a primitive is listed twice"
  unless primitives.length == (← constructors ``PrimitiveOperation) do
    throwError "the primitives run here are not every constructor"
  unless typeCases.length == (← constructors ``Ty) + 2 + 1 + 1 do
    throwError "the types run here are not every constructor"

#guard_msgs in
run_meta checkInventory

#guard_msgs in
#eval checkPrimitives

#guard_msgs in
#eval checkTypes

end VerexIR.Rust.Tests.NativeCases
