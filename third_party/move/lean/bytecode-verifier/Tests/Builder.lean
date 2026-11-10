-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec

/-!
# A small module builder for tests

Builds `CompiledModule`s from function descriptions. Handle `i` is the `i`-th
local function; imported functions follow, in order. Likewise struct handle
`i` and struct definition `i` are the `i`-th local struct or enum; imported
structs follow in the handle table. The builder does not try to produce
modules that pass verification (identifiers are not deduplicated); it exists
to exercise the semantics.

Signatures are laid out as: for each local function its parameters, returns
and locals (indices `3 * i`, `3 * i + 1`, `3 * i + 2`); for each imported
function its parameters and returns; then `extraSignatures` in order.
Functions, imports and structs may be generic (`typeParameters`); the
instantiation tables are given as they are.
-/

namespace MoveBytecode.Tests

structure Fun where
  name : String
  params : Signature := []
  returns : Signature := []
  locals : Signature := []
  code : Array Bytecode := #[]
  visibility : Visibility := .public_
  attributes : List FunctionAttribute := []
  native : Bool := false
  typeParameters : List AbilitySet := []
  isEntry : Bool := false

structure Import where
  address : Nat
  module : String
  name : String
  params : Signature := []
  returns : Signature := []
  typeParameters : List AbilitySet := []

/-- A struct or enum defined by the module under construction. -/
structure StructDecl where
  name : String
  abilities : AbilitySet := {}
  typeParameters : List StructTypeParameter := []
  /-- The fields of a struct; ignored for an enum. -/
  fields : List SignatureToken := []
  /-- The names of the fields of a struct, in order. A field without one is
  named by identifier 0, the module's name. -/
  fieldNames : List String := []
  /-- `some variants` declares an enum: each variant's name and field types. -/
  variants : Option (List (String × List SignatureToken)) := none
  /-- A native struct, with no fields. -/
  native : Bool := false

/-- A struct handle naming a type of another module. -/
structure StructImport where
  address : Nat
  module : String
  name : String
  abilities : AbilitySet := {}
  typeParameters : List StructTypeParameter := []

def mkModule (address : Nat) (name : String) (funs : List Fun) (imports : List Import := [])
    (constants : List Constant := []) (structs : List StructDecl := [])
    (structImports : List StructImport := []) (variantHandles : List StructVariantHandle := [])
    (extraSignatures : List Signature := []) (fieldHandles : List FieldHandle := [])
    (variantFieldHandles : List VariantFieldHandle := [])
    (functionInstantiations : List FunctionInstantiation := [])
    (structDefInstantiations : List StructDefInstantiation := [])
    (fieldInstantiations : List FieldInstantiation := [])
    (structVariantInstantiations : List StructVariantInstantiation := [])
    (variantFieldInstantiations : List VariantFieldInstantiation := []) : CompiledModule := Id.run do
  let mut identifiers : Array String := #[name]
  let mut addresses : Array Address := #[BitVec.ofNat 256 address]
  let mut moduleHandles : Array ModuleHandle := #[⟨0, 0⟩]
  let mut signatures : Array Signature := #[]
  let mut handles : Array FunctionHandle := #[]
  let mut defs : Array FunctionDefinition := #[]
  for f in funs do
    identifiers := identifiers.push f.name
    let nameIdx := identifiers.size - 1
    signatures := signatures.push f.params
    let paramsIdx := signatures.size - 1
    signatures := signatures.push f.returns
    let returnsIdx := signatures.size - 1
    signatures := signatures.push f.locals
    let localsIdx := signatures.size - 1
    handles := handles.push
      { module := 0, name := nameIdx, parameters := paramsIdx, returns := returnsIdx,
        typeParameters := f.typeParameters, accessSpecifiers := none, attributes := f.attributes }
    defs := defs.push
      { function := handles.size - 1, visibility := f.visibility, isEntry := f.isEntry,
        acquiresGlobalResources := [],
        code := if f.native then none else some { locals := localsIdx, code := f.code } }
  for i in imports do
    addresses := addresses.push (BitVec.ofNat 256 i.address)
    identifiers := identifiers.push i.module
    moduleHandles := moduleHandles.push ⟨addresses.size - 1, identifiers.size - 1⟩
    identifiers := identifiers.push i.name
    signatures := signatures.push i.params
    let paramsIdx := signatures.size - 1
    signatures := signatures.push i.returns
    handles := handles.push
      { module := moduleHandles.size - 1, name := identifiers.size - 1, parameters := paramsIdx,
        returns := signatures.size - 1, typeParameters := i.typeParameters, accessSpecifiers := none,
        attributes := [] }
  let mut structHandles : Array StructHandle := #[]
  let mut structDefs : Array StructDefinition := #[]
  for sd in structs do
    identifiers := identifiers.push sd.name
    structHandles := structHandles.push
      { module := 0, name := identifiers.size - 1, abilities := sd.abilities,
        typeParameters := sd.typeParameters }
    let mut fieldDefs : Array FieldDefinition := #[]
    for (t, k) in sd.fields.zipIdx do
      match sd.fieldNames[k]? with
      | some fieldName =>
        identifiers := identifiers.push fieldName
        fieldDefs := fieldDefs.push { name := identifiers.size - 1, signature := t }
      | none => fieldDefs := fieldDefs.push { name := 0, signature := t }
    let mut info : StructFieldInformation := .declared fieldDefs.toList
    if sd.native then info := .native
    match sd.variants with
    | some variants =>
      let mut vdefs : Array VariantDefinition := #[]
      for (vname, fields) in variants do
        identifiers := identifiers.push vname
        vdefs := vdefs.push
          { name := identifiers.size - 1, fields := fields.map fun t => { name := 0, signature := t } }
      info := .declaredVariants vdefs.toList
    | none => pure ()
    structDefs := structDefs.push { structHandle := structHandles.size - 1, fieldInformation := info }
  for si in structImports do
    addresses := addresses.push (BitVec.ofNat 256 si.address)
    identifiers := identifiers.push si.module
    moduleHandles := moduleHandles.push ⟨addresses.size - 1, identifiers.size - 1⟩
    identifiers := identifiers.push si.name
    structHandles := structHandles.push
      { module := moduleHandles.size - 1, name := identifiers.size - 1, abilities := si.abilities,
        typeParameters := si.typeParameters }
  for sig in extraSignatures do
    signatures := signatures.push sig
  return {
    version := 10, selfModuleHandleIdx := 0, moduleHandles, structHandles,
    functionHandles := handles, fieldHandles := fieldHandles.toArray, friendDecls := #[],
    structDefInstantiations := structDefInstantiations.toArray,
    functionInstantiations := functionInstantiations.toArray,
    fieldInstantiations := fieldInstantiations.toArray,
    signatures, identifiers, addressIdentifiers := addresses, constantPool := constants.toArray,
    metadata := #[], structDefs, functionDefs := defs,
    structVariantHandles := variantHandles.toArray,
    structVariantInstantiations := structVariantInstantiations.toArray,
    variantFieldHandles := variantFieldHandles.toArray,
    variantFieldInstantiations := variantFieldInstantiations.toArray }

/-- Function `index` of module `module`, at type arguments `typeArgs`. -/
def fn (module index : Nat) (typeArgs : List RuntimeType := []) : FunctionRef :=
  { module, index, typeArgs }

/-- Run function `index` of module `module` at type arguments `typeArgs` with
`args`, over global storage `storage`, with the transaction holding `held`,
with ample fuel. -/
def runFun (P : Program) (module index : Nat) (args : List Value) (fuel : Nat := 10000)
    (storage : GlobalStorage := []) (held : List Value := []) (typeArgs : List RuntimeType := []) :
    RunResult :=
  match initialState P (fn module index typeArgs) args storage held with
  | some s => run P noNatives fuel s
  | none => .wrong (.unsupported "no initial state")

def u64 (n : Nat) : Value := .int .u64 n

end MoveBytecode.Tests
