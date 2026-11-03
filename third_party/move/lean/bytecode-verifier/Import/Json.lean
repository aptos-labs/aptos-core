-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import Spec.Syntax
import Spec.Semantics.Program

/-!
# Decoding the exporter's JSON

Decodes the output of `exporter/` (format `move-bytecode-lean/1`) into
`Spec.Syntax`. Field names and enum tags follow
`move_binary_format::file_format`; values that may exceed 2^53 are decimal
strings. A malformed document is an error, never a default value.

This is part of deliverable A4 (decoding correspondence): the production
deserializer reads the bytes, and this decoder must reproduce its result
exactly. `Tests/Import.lean` checks every instruction tag against the
exporter's instruction catalog.
-/

namespace MoveBytecode.Import

open Lean (Json)

abbrev Dec := Except String

def formatTag : String := "move-bytecode-lean/1"

def context {α : Type} (what : String) (d : Dec α) : Dec α :=
  d.mapError (fun e => s!"{what}: {e}")

def get (j : Json) (key : String) : Dec Json := context key (j.getObjVal? key)

def nat (j : Json) : Dec Nat := j.getNat?

def bool (j : Json) : Dec Bool := j.getBool?

def str (j : Json) : Dec String := j.getStr?

def array {α : Type} (f : Json → Dec α) (j : Json) : Dec (Array α) := do
  (← j.getArr?).mapM f

def list {α : Type} (f : Json → Dec α) (j : Json) : Dec (List α) := do
  pure (← array f j).toList

def natField (j : Json) (key : String) : Dec Nat := do context key (nat (← get j key))

def boolField (j : Json) (key : String) : Dec Bool := do context key (bool (← get j key))

/-- A fixed-length array. -/
def tuple (j : Json) (n : Nat) : Dec (Array Json) := do
  let a ← j.getArr?
  if a.size = n then pure a else throw s!"expected {n} elements, got {a.size}"

/-- A decimal string. -/
def intOfString (s : String) : Dec Int :=
  match s.toInt? with
  | some i => pure i
  | none => throw s!"not a decimal integer: {s}"

/-- A decimal string denoting a literal of `n` bits, signed or unsigned. -/
def bitVec (n : Nat) (signed : Bool) (j : Json) : Dec (BitVec n) := do
  let s ← str j
  let v ← intOfString s
  let lo : Int := if signed then -(2 ^ (n - 1)) else 0
  let hi : Int := if signed then 2 ^ (n - 1) - 1 else 2 ^ n - 1
  if lo ≤ v ∧ v ≤ hi then pure (BitVec.ofInt n v)
  else throw s!"literal {s} out of range for {if signed then "i" else "u"}{n}"

/-- A decimal string denoting a natural number. -/
def natString (j : Json) : Dec Nat := do
  let s ← str j
  match s.toNat? with
  | some n => pure n
  | none => throw s!"not a natural number: {s}"

/-- An externally tagged enum value: a bare tag, or an object with one key. -/
def tagged (j : Json) : Dec (String × Option Json) :=
  match j with
  | .str s => pure (s, none)
  | .obj kvs =>
    match kvs.toList with
    | [(k, v)] => pure (k, some v)
    | _ => throw s!"expected an object with one key: {j.compress}"
  | _ => throw s!"expected an enum value: {j.compress}"

def abilitySet (j : Json) : Dec AbilitySet := do
  let b ← nat j
  if b ≥ 16 then throw s!"invalid ability set {b}"
  else pure ⟨b % 2 = 1, (b / 2) % 2 = 1, (b / 4) % 2 = 1, (b / 8) % 2 = 1⟩

/-- A 32-byte address written as `0x` followed by 64 hexadecimal digits. -/
def address (j : Json) : Dec Address := do
  let s ← str j
  let digits := s.toList.drop 2
  if !s.startsWith "0x" || digits.length != 64 then throw s!"invalid address {s}"
  else
    let hexDigit (c : Char) : Dec Nat :=
      if '0' ≤ c ∧ c ≤ '9' then pure (c.toNat - '0'.toNat)
      else if 'a' ≤ c ∧ c ≤ 'f' then pure (c.toNat - 'a'.toNat + 10)
      else throw s!"invalid address {s}"
    let n ← digits.foldlM (fun acc c => do pure (acc * 16 + (← hexDigit c))) 0
    pure (BitVec.ofNat 256 n)

def bytes (j : Json) : Dec (List UInt8) :=
  list (fun b => do
    let n ← nat b
    if n < 256 then pure n.toUInt8 else throw s!"invalid byte {n}") j

partial def token (j : Json) : Dec SignatureToken := do
  match ← tagged j with
  | ("Bool", none) => pure .bool
  | ("U8", none) => pure .u8
  | ("U16", none) => pure .u16
  | ("U32", none) => pure .u32
  | ("U64", none) => pure .u64
  | ("U128", none) => pure .u128
  | ("U256", none) => pure .u256
  | ("I8", none) => pure .i8
  | ("I16", none) => pure .i16
  | ("I32", none) => pure .i32
  | ("I64", none) => pure .i64
  | ("I128", none) => pure .i128
  | ("I256", none) => pure .i256
  | ("Address", none) => pure .address
  | ("Signer", none) => pure .signer
  | ("Vector", some e) => pure (.vector (← token e))
  | ("Function", some a) => do
    let a ← tuple a 3
    pure (.function (← list token a[0]!) (← list token a[1]!) (← abilitySet a[2]!))
  | ("Struct", some h) => pure (.struct (← nat h))
  | ("StructInstantiation", some a) => do
    let a ← tuple a 2
    pure (.structInstantiation (← nat a[0]!) (← list token a[1]!))
  | ("Reference", some t) => pure (.reference (← token t))
  | ("MutableReference", some t) => pure (.mutableReference (← token t))
  | ("TypeParameter", some i) => pure (.typeParameter (← nat i))
  | (tag, _) => throw s!"unknown signature token {tag}"

def moduleHandle (j : Json) : Dec ModuleHandle := do
  pure ⟨← natField j "address", ← natField j "name"⟩

def structHandle (j : Json) : Dec StructHandle := do
  let params ← list (fun p => do
    pure ⟨← context "constraints" (abilitySet (← get p "constraints")),
          ← boolField p "is_phantom"⟩) (← get j "type_parameters")
  pure ⟨← natField j "module", ← natField j "name",
        ← context "abilities" (abilitySet (← get j "abilities")), params⟩

def functionAttribute (j : Json) : Dec FunctionAttribute := do
  match ← tagged j with
  | ("Persistent", none) => pure .persistent
  | ("ModuleLock", none) => pure .moduleLock
  | ("Pack", none) => pure .pack
  | ("PackVariant", some v) => pure (.packVariant (← nat v))
  | ("Unpack", none) => pure .unpack
  | ("UnpackVariant", some v) => pure (.unpackVariant (← nat v))
  | ("TestVariant", some v) => pure (.testVariant (← nat v))
  | ("BorrowFieldImmutable", some o) => pure (.borrowFieldImmutable (← nat o))
  | ("BorrowFieldMutable", some o) => pure (.borrowFieldMutable (← nat o))
  | (tag, _) => throw s!"unknown function attribute {tag}"

def accessSpecifier (j : Json) : Dec AccessSpecifier := do
  let kind ← match ← str (← get j "kind") with
    | "Reads" => pure AccessKind.reads
    | "Writes" => pure AccessKind.writes
    | k => throw s!"unknown access kind {k}"
  let resource ← match ← tagged (← get j "resource") with
    | ("Any", none) => pure ResourceSpecifier.any
    | ("DeclaredAtAddress", some a) => pure (.declaredAtAddress (← nat a))
    | ("DeclaredInModule", some m) => pure (.declaredInModule (← nat m))
    | ("Resource", some h) => pure (.resource (← nat h))
    | ("ResourceInstantiation", some a) => do
      let a ← tuple a 2
      pure (.resourceInstantiation (← nat a[0]!) (← nat a[1]!))
    | (tag, _) => throw s!"unknown resource specifier {tag}"
  let address ← match ← tagged (← get j "address") with
    | ("Any", none) => pure AddressSpecifier.any
    | ("Literal", some a) => pure (.literal (← nat a))
    | ("Parameter", some a) => do
      let a ← tuple a 2
      let inst ← match a[1]! with
        | .null => pure none
        | i => do pure (some (← nat i))
      pure (.parameter (← nat a[0]!) inst)
    | (tag, _) => throw s!"unknown address specifier {tag}"
  pure ⟨kind, ← boolField j "negated", resource, address⟩

def accessSpecifiers (j : Json) : Dec (Option (List AccessSpecifier)) :=
  match j with
  | .null => pure none
  | _ => do pure (some (← list accessSpecifier j))

def functionHandle (j : Json) : Dec FunctionHandle := do
  pure {
    module := ← natField j "module"
    name := ← natField j "name"
    parameters := ← natField j "parameters"
    returns := ← natField j "return_"
    typeParameters := ← context "type_parameters" (list abilitySet (← get j "type_parameters"))
    accessSpecifiers := ← context "access_specifiers" (accessSpecifiers (← get j "access_specifiers"))
    attributes := ← context "attributes" (list functionAttribute (← get j "attributes")) }

/-- Instructions without operands. -/
def nullaryInstructions : List (String × Bytecode) := [
  ("Pop", .pop), ("Ret", .ret), ("CastU8", .castU8), ("CastU64", .castU64),
  ("CastU128", .castU128), ("LdTrue", .ldTrue), ("LdFalse", .ldFalse), ("ReadRef", .readRef),
  ("WriteRef", .writeRef), ("FreezeRef", .freezeRef), ("Add", .add), ("Sub", .sub),
  ("Mul", .mul), ("Mod", .mod), ("Div", .div), ("BitOr", .bitOr), ("BitAnd", .bitAnd),
  ("Xor", .xor), ("Or", .or), ("And", .and), ("Not", .not), ("Eq", .eq), ("Neq", .neq),
  ("Lt", .lt), ("Gt", .gt), ("Le", .le), ("Ge", .ge), ("Abort", .abort), ("Nop", .nop),
  ("Shl", .shl), ("Shr", .shr), ("CastU16", .castU16), ("CastU32", .castU32),
  ("CastU256", .castU256), ("CastI8", .castI8), ("CastI16", .castI16), ("CastI32", .castI32),
  ("CastI64", .castI64), ("CastI128", .castI128), ("CastI256", .castI256),
  ("Negate", .negate), ("AbortMsg", .abortMsg)]

/-- Instructions with one index operand. -/
def indexInstructions : List (String × (Nat → Bytecode)) := [
  ("BrTrue", .brTrue), ("BrFalse", .brFalse), ("Branch", .branch), ("LdConst", .ldConst),
  ("CopyLoc", .copyLoc), ("MoveLoc", .moveLoc), ("StLoc", .stLoc), ("Call", .call),
  ("CallGeneric", .callGeneric), ("Pack", .pack), ("PackGeneric", .packGeneric),
  ("PackVariant", .packVariant), ("PackVariantGeneric", .packVariantGeneric),
  ("Unpack", .unpack), ("UnpackGeneric", .unpackGeneric), ("UnpackVariant", .unpackVariant),
  ("UnpackVariantGeneric", .unpackVariantGeneric), ("TestVariant", .testVariant),
  ("TestVariantGeneric", .testVariantGeneric), ("MutBorrowLoc", .mutBorrowLoc),
  ("ImmBorrowLoc", .immBorrowLoc), ("MutBorrowField", .mutBorrowField),
  ("MutBorrowVariantField", .mutBorrowVariantField),
  ("MutBorrowFieldGeneric", .mutBorrowFieldGeneric),
  ("MutBorrowVariantFieldGeneric", .mutBorrowVariantFieldGeneric),
  ("ImmBorrowField", .immBorrowField), ("ImmBorrowVariantField", .immBorrowVariantField),
  ("ImmBorrowFieldGeneric", .immBorrowFieldGeneric),
  ("ImmBorrowVariantFieldGeneric", .immBorrowVariantFieldGeneric),
  ("MutBorrowGlobal", .mutBorrowGlobal), ("MutBorrowGlobalGeneric", .mutBorrowGlobalGeneric),
  ("ImmBorrowGlobal", .immBorrowGlobal), ("ImmBorrowGlobalGeneric", .immBorrowGlobalGeneric),
  ("Exists", .exists_), ("ExistsGeneric", .existsGeneric), ("MoveFrom", .moveFrom),
  ("MoveFromGeneric", .moveFromGeneric), ("MoveTo", .moveTo),
  ("MoveToGeneric", .moveToGeneric), ("VecLen", .vecLen), ("VecImmBorrow", .vecImmBorrow),
  ("VecMutBorrow", .vecMutBorrow), ("VecPushBack", .vecPushBack),
  ("VecPopBack", .vecPopBack), ("VecSwap", .vecSwap), ("CallClosure", .callClosure)]

def instruction (j : Json) : Dec Bytecode := do
  let (tag, operand) ← tagged j
  match operand with
  | none =>
    match nullaryInstructions.lookup tag with
    | some i => pure i
    | none => throw s!"unknown instruction {tag}"
  | some a =>
    match indexInstructions.lookup tag with
    | some f => pure (f (← nat a))
    | none =>
      match tag with
      | "LdU8" => pure (.ldU8 (← bitVec 8 false a))
      | "LdU16" => pure (.ldU16 (← bitVec 16 false a))
      | "LdU32" => pure (.ldU32 (← bitVec 32 false a))
      | "LdU64" => pure (.ldU64 (← bitVec 64 false a))
      | "LdU128" => pure (.ldU128 (← bitVec 128 false a))
      | "LdU256" => pure (.ldU256 (← bitVec 256 false a))
      | "LdI8" => pure (.ldI8 (← bitVec 8 true a))
      | "LdI16" => pure (.ldI16 (← bitVec 16 true a))
      | "LdI32" => pure (.ldI32 (← bitVec 32 true a))
      | "LdI64" => pure (.ldI64 (← bitVec 64 true a))
      | "LdI128" => pure (.ldI128 (← bitVec 128 true a))
      | "LdI256" => pure (.ldI256 (← bitVec 256 true a))
      | "VecPack" => do
        let a ← tuple a 2
        pure (.vecPack (← nat a[0]!) (← natString a[1]!))
      | "VecUnpack" => do
        let a ← tuple a 2
        pure (.vecUnpack (← nat a[0]!) (← natString a[1]!))
      | "PackClosure" => do
        let a ← tuple a 2
        pure (.packClosure (← nat a[0]!) (← bitVec 64 false a[1]!))
      | "PackClosureGeneric" => do
        let a ← tuple a 2
        pure (.packClosureGeneric (← nat a[0]!) (← bitVec 64 false a[1]!))
      | _ => throw s!"unknown instruction {tag}"

def codeUnit (j : Json) : Dec CodeUnit := do
  pure ⟨← natField j "locals", ← context "code" (array instruction (← get j "code"))⟩

def fieldDefinition (j : Json) : Dec FieldDefinition := do
  pure ⟨← natField j "name", ← context "signature" (token (← get j "signature"))⟩

def structDefinition (j : Json) : Dec StructDefinition := do
  let info ← match ← tagged (← get j "field_information") with
    | ("Native", none) => pure StructFieldInformation.native
    | ("Declared", some fs) => pure (.declared (← list fieldDefinition fs))
    | ("DeclaredVariants", some vs) =>
      pure (.declaredVariants (← list (fun v => do
        pure ⟨← natField v "name", ← context "fields" (list fieldDefinition (← get v "fields"))⟩) vs))
    | (tag, _) => throw s!"unknown field information {tag}"
  pure ⟨← natField j "struct_handle", info⟩

def visibility (j : Json) : Dec Visibility := do
  match ← str j with
  | "Private" => pure .private_
  | "Public" => pure .public_
  | "Friend" => pure .friend
  | v => throw s!"unknown visibility {v}"

def functionDefinition (j : Json) : Dec FunctionDefinition := do
  let code ← match ← get j "code" with
    | .null => pure none
    | c => do pure (some (← context "code" (codeUnit c)))
  pure {
    function := ← natField j "function"
    visibility := ← context "visibility" (visibility (← get j "visibility"))
    isEntry := ← boolField j "is_entry"
    acquiresGlobalResources := ← context "acquires_global_resources"
      (list nat (← get j "acquires_global_resources"))
    code }

def constant (j : Json) : Dec Constant := do
  pure ⟨← context "type_" (token (← get j "type_")), ← context "data" (bytes (← get j "data"))⟩

def metadata (j : Json) : Dec Metadata := do
  pure ⟨← context "key" (bytes (← get j "key")), ← context "value" (bytes (← get j "value"))⟩

/-- An instantiation entry: a table index and a type-argument signature. -/
def instantiation (key : String) (j : Json) : Dec (Nat × Nat) := do
  pure (← natField j key, ← natField j "type_parameters")

def tableField {α : Type} (j : Json) (key : String) (f : Json → Dec α) : Dec (Array α) := do
  context key (array f (← get j key))

def compiledModule (j : Json) : Dec CompiledModule := do
  pure {
    version := ← natField j "version"
    selfModuleHandleIdx := ← natField j "self_module_handle_idx"
    moduleHandles := ← tableField j "module_handles" moduleHandle
    structHandles := ← tableField j "struct_handles" structHandle
    functionHandles := ← tableField j "function_handles" functionHandle
    fieldHandles := ← tableField j "field_handles" (fun h => do
      pure ⟨← natField h "owner", ← natField h "field"⟩)
    friendDecls := ← tableField j "friend_decls" moduleHandle
    structDefInstantiations := ← tableField j "struct_def_instantiations" (fun i => do
      let (d, t) ← instantiation "def" i; pure ⟨d, t⟩)
    functionInstantiations := ← tableField j "function_instantiations" (fun i => do
      let (h, t) ← instantiation "handle" i; pure ⟨h, t⟩)
    fieldInstantiations := ← tableField j "field_instantiations" (fun i => do
      let (h, t) ← instantiation "handle" i; pure ⟨h, t⟩)
    signatures := ← tableField j "signatures" (list token)
    identifiers := ← tableField j "identifiers" str
    addressIdentifiers := ← tableField j "address_identifiers" address
    constantPool := ← tableField j "constant_pool" constant
    metadata := ← tableField j "metadata" metadata
    structDefs := ← tableField j "struct_defs" structDefinition
    functionDefs := ← tableField j "function_defs" functionDefinition
    structVariantHandles := ← tableField j "struct_variant_handles" (fun h => do
      pure ⟨← natField h "struct_index", ← natField h "variant"⟩)
    structVariantInstantiations := ← tableField j "struct_variant_instantiations" (fun i => do
      let (h, t) ← instantiation "handle" i; pure ⟨h, t⟩)
    variantFieldHandles := ← tableField j "variant_field_handles" (fun h => do
      pure ⟨← natField h "struct_index", ← context "variants" (list nat (← get h "variants")),
            ← natField h "field"⟩)
    variantFieldInstantiations := ← tableField j "variant_field_instantiations" (fun i => do
      let (h, t) ← instantiation "handle" i; pure ⟨h, t⟩) }

def compiledScript (j : Json) : Dec CompiledScript := do
  pure {
    version := ← natField j "version"
    moduleHandles := ← tableField j "module_handles" moduleHandle
    structHandles := ← tableField j "struct_handles" structHandle
    functionHandles := ← tableField j "function_handles" functionHandle
    functionInstantiations := ← tableField j "function_instantiations" (fun i => do
      let (h, t) ← instantiation "handle" i; pure ⟨h, t⟩)
    signatures := ← tableField j "signatures" (list token)
    identifiers := ← tableField j "identifiers" str
    addressIdentifiers := ← tableField j "address_identifiers" address
    constantPool := ← tableField j "constant_pool" constant
    metadata := ← tableField j "metadata" metadata
    code := ← context "code" (codeUnit (← get j "code"))
    typeParameters := ← context "type_parameters" (list abilitySet (← get j "type_parameters"))
    parameters := ← natField j "parameters"
    accessSpecifiers := ← context "access_specifiers"
      (accessSpecifiers (← get j "access_specifiers")) }

/-- A decoded exporter document. -/
structure Document where
  modules : Array CompiledModule
  scripts : Array CompiledScript

def checkFormat (j : Json) : Dec Unit := do
  let f ← context "format" (str (← get j "format"))
  if f != formatTag then throw s!"unsupported format {f}, expected {formatTag}"

def document (text : String) : Dec Document := do
  let j ← Json.parse text
  checkFormat j
  pure ⟨← tableField j "modules" compiledModule, ← tableField j "scripts" compiledScript⟩

/-- The program made of a document's modules, in document order. -/
def Document.program (d : Document) : Program := ⟨d.modules⟩

/-- Decode an instruction catalog (`--instruction-catalog`). -/
def instructionCatalog (text : String) : Dec (Array Bytecode) := do
  let j ← Json.parse text
  checkFormat j
  tableField j "instructions" instruction

end MoveBytecode.Import
