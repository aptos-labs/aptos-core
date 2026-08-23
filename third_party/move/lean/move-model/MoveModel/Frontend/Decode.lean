-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import MoveModel.Frontend.XIR

/-!
# JSON Decoder for the Exchange Format

This module decodes the JSON produced by `aptos move exchange` into an
`MProgram`.

The serde-annotated Rust types in `move-model-exchange` define version 7 of
the schema.  Struct fields use snake_case names.  Enums use external tags: a
unit variant is a string such as `"add"`, while variants with data are
single-key objects such as `{"move_from": 0}` or
`{"load": [1, {"u64": "5"}]}`.
-/

namespace MoveModel.Frontend

open MoveModel.IR
open MoveModel.Frontend.XIR
open Lean (Json)

abbrev Dec := Except String

private def decodeNat (j : Json) : Dec Nat := j.getNat?

private def decodeNatFromString (s : String) : Dec Nat :=
  match s.toNat? with
  | some n => pure n
  | none => throw s!"expected a decimal number, got `{s}`"

private def decodeIntFromString (s : String) : Dec Int := do
  if s.startsWith "-" then
    return -(← decodeNatFromString ((s.drop 1).toString) : Nat)
  return (← decodeNatFromString s : Nat)

/-- Parses `0x…` hex address literals. -/
private def decodeHex (s : String) : Dec Nat := do
  unless s.startsWith "0x" do
    throw s!"expected a 0x-prefixed address, got `{s}`"
  let some n := Lean.Syntax.decodeNatLitVal? s
    | throw s!"invalid address literal `{s}`"
  pure n

/-- Decomposes a serde externally-tagged enum value: a bare string (unit
variant) or a single-key object `{tag: payload}`. -/
private def decodeTagged (j : Json) : Dec (String × Option Json) :=
  match j with
  | .str s => pure (s, none)
  | .obj o =>
    match o.toList with
    | [(k, v)] => pure (k, some v)
    | _ => throw "expected a single-key object"
  | _ => throw "expected a string or a single-key object"

/-- The payload of a non-unit variant. -/
private def payload (tag : String) : Option Json → Dec Json
  | some j => pure j
  | none => throw s!"missing payload for `{tag}`"

/-- The payload of a tuple variant: an array of exactly `n` arguments. -/
private def payloadArr (tag : String) (n : Nat) (p : Option Json) :
    Dec (Array Json) := do
  let a ← (← payload tag p).getArr?
  if a.size ≠ n then
    throw s!"expected {n} arguments for `{tag}`, got {a.size}"
  pure a

mutual

private partial def decodeTy (j : Json) : Dec Ty := do
  let (tag, p) ← decodeTagged j
  match tag with
  | "bool" => pure .bool
  | "u8" => pure (.uint .w8)
  | "u16" => pure (.uint .w16)
  | "u32" => pure (.uint .w32)
  | "u64" => pure (.uint .w64)
  | "u128" => pure (.uint .w128)
  | "u256" => pure (.uint .w256)
  | "i8" => pure (.sint .w8)
  | "i16" => pure (.sint .w16)
  | "i32" => pure (.sint .w32)
  | "i64" => pure (.sint .w64)
  | "i128" => pure (.sint .w128)
  | "i256" => pure (.sint .w256)
  | "address" => pure .address
  | "signer" => pure .signer
  | "type_parameter" => pure (.typeParam (← decodeNat (← payload tag p)))
  | "struct" => pure (.struct (← decodeNat (← payload tag p)))
  | "struct_inst" =>
    let a ← payloadArr tag 2 p
    pure (.structInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "enum" => pure (.enum (← decodeNat (← payload tag p)))
  | "enum_inst" =>
    let a ← payloadArr tag 2 p
    pure (.enumInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "vector" => pure (.vector (← decodeTy (← payload tag p)))
  | "ref" => pure (.ref (← decodeTy (← payload tag p)))
  | "mut_ref" => pure (.mutRef (← decodeTy (← payload tag p)))
  | k => throw s!"unknown type `{k}`"

private partial def decodeTys (j : Json) : Dec (List Ty) := do
  (← j.getArr?).toList.mapM decodeTy

end

private partial def decodeValue (j : Json) : Dec Value := do
  let (tag, p) ← decodeTagged j
  match tag with
  | "num" => pure (.int (← decodeIntFromString (← (← payload tag p).getStr?)))
  | "bool" => pure (.bool (← (← payload tag p).getBool?))
  | "address" => pure (.address (← decodeHex (← (← payload tag p).getStr?)))
  | "vector" =>
    pure (.vector (← (← (← payload tag p).getArr?).toList.mapM decodeValue))
  | k => throw s!"unknown value kind `{k}`"

private def decodeOper (j : Json) : Dec Oper := do
  let (tag, p) ← decodeTagged j
  let arg : Dec Nat := do decodeNat (← payload tag p)
  -- Every integer operation carries its `NumType`, spelled as a width string
  -- whose `u`/`i` prefix is the signedness.
  let numType : Dec NumType := do
    match (← (← payload tag p).getStr?) with
    | "u8" => pure ⟨.w8, false⟩
    | "u16" => pure ⟨.w16, false⟩
    | "u32" => pure ⟨.w32, false⟩
    | "u64" => pure ⟨.w64, false⟩
    | "u128" => pure ⟨.w128, false⟩
    | "u256" => pure ⟨.w256, false⟩
    | "i8" => pure ⟨.w8, true⟩
    | "i16" => pure ⟨.w16, true⟩
    | "i32" => pure ⟨.w32, true⟩
    | "i64" => pure ⟨.w64, true⟩
    | "i128" => pure ⟨.w128, true⟩
    | "i256" => pure ⟨.w256, true⟩
    | other => throw s!"unknown integer width `{other}`"
  match tag with
  | "add" => pure (.add (← numType))
  | "sub" => pure (.sub (← numType))
  | "mul" => pure (.mul (← numType))
  | "div" => pure (.div (← numType))
  | "mod" => pure (.mod (← numType))
  | "bit_and" => pure (.bitAnd (← numType))
  | "bit_or" => pure (.bitOr (← numType))
  | "bit_xor" => pure (.bitXor (← numType))
  | "shl" => pure (.shl (← numType))
  | "shr" => pure (.shr (← numType))
  | "cast" => pure (.cast (← numType))
  | "lt" => pure .lt
  | "le" => pure .le
  | "eq" => pure .eq
  | "and" => pure .and
  | "or" => pure .or
  | "not" => pure .not
  | "pack" => pure .pack
  | "pack_inst" => pure (.packInst (← decodeTys (← payload tag p)))
  | "unpack" => pure .unpack
  | "unpack_inst" => pure (.unpackInst (← decodeTys (← payload tag p)))
  | "pack_variant" => pure (.packVariant (← arg))
  | "pack_variant_inst" =>
    let a ← payloadArr tag 2 p
    pure (.packVariantInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "unpack_variant" => pure (.unpackVariant (← arg))
  | "unpack_variant_inst" =>
    let a ← payloadArr tag 2 p
    pure (.unpackVariantInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "test_variant" => pure (.testVariant (← arg))
  | "test_variant_inst" =>
    let a ← payloadArr tag 2 p
    pure (.testVariantInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "get_field" => pure (.getField (← arg))
  | "get_field_inst" =>
    let a ← payloadArr tag 2 p
    pure (.getFieldInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "update_field" => pure (.updateField (← arg))
  | "vec_pack" => pure .vecPack
  | "vec_len" => pure .vecLen
  | "vec_get" => pure .vecGet
  | "vec_set" => pure .vecSet
  | "vec_push" => pure .vecPush
  | "vec_pop" => pure .vecPop
  | "vec_insert" => pure .vecInsert
  | "vec_remove" => pure .vecRemove
  | "get_global" => pure (.getGlobal (← arg))
  | "get_global_inst" =>
    let a ← payloadArr tag 2 p
    pure (.getGlobalInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "write_global" => pure (.writeGlobal (← arg))
  | "move_to" => pure (.moveTo (← arg))
  | "move_to_inst" =>
    let a ← payloadArr tag 2 p
    pure (.moveToInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "move_from" => pure (.moveFrom (← arg))
  | "move_from_inst" =>
    let a ← payloadArr tag 2 p
    pure (.moveFromInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "exists" => pure (.exists_ (← arg))
  | "exists_inst" =>
    let a ← payloadArr tag 2 p
    pure (.existsInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "function" => pure (.function (← arg))
  | "function_inst" =>
    let a ← payloadArr tag 2 p
    pure (.functionInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "borrow_loc" => pure .borrowLoc
  | "borrow_field" => pure (.borrowField (← arg))
  | "borrow_field_inst" =>
    let a ← payloadArr tag 2 p
    pure (.borrowFieldInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "borrow_global" => pure (.borrowGlobal (← arg))
  | "borrow_global_inst" =>
    let a ← payloadArr tag 2 p
    pure (.borrowGlobalInst (← decodeNat a[0]!) (← decodeTys a[1]!))
  | "borrow_vec_elem" => pure .borrowVecElem
  | "read_ref" => pure .readRef
  | "write_ref" => pure .writeRef
  | "freeze_ref" => pure .freezeRef
  | k => throw s!"unknown operation `{k}`"

private def decodeNats (j : Json) : Dec (List Nat) := do
  (← j.getArr?).toList.mapM decodeNat

private def decodeInstr (j : Json) : Dec Instr := do
  let (tag, p) ← decodeTagged j
  match tag with
  | "load" =>
    let a ← payloadArr tag 2 p
    pure (.load (← decodeNat a[0]!) (← decodeValue a[1]!))
  | "assign" =>
    let a ← payloadArr tag 2 p
    pure (.assign (← decodeNat a[0]!) (← decodeNat a[1]!))
  | "call" =>
    let a ← payloadArr tag 3 p
    pure (.call (← decodeNats a[0]!) (← decodeOper a[1]!) (← decodeNats a[2]!))
  | "nop" => pure .nop
  | k => throw s!"unknown instruction `{k}`"

private def decodeTerm (j : Json) : Dec Term := do
  let (tag, p) ← decodeTagged j
  match tag with
  | "jump" => pure (.jump (← decodeNat (← payload tag p)))
  | "branch" =>
    let a ← payloadArr tag 3 p
    pure (.branch (← decodeNat a[0]!) (← decodeNat a[1]!) (← decodeNat a[2]!))
  | "ret" => pure (.ret (← decodeNats (← payload tag p)))
  | "abort" => pure (.abort (← decodeNat (← payload tag p)))
  | k => throw s!"unknown terminator `{k}`"

private def decodeBlock (j : Json) : Dec Block := do
  let instrs ← (← (← j.getObjVal? "instrs").getArr?).toList.mapM decodeInstr
  let term ← decodeTerm (← j.getObjVal? "term")
  pure ⟨instrs, term⟩

private def decodeBinop (s : String) : Dec SpecBinop :=
  match s with
  | "add" => pure .add
  | "sub" => pure .sub
  | "mul" => pure .mul
  | "div" => pure .div
  | "mod" => pure .mod
  | "lt" => pure .lt
  | "le" => pure .le
  | "eq" => pure .eq
  | "index" => pure .index
  | "and" => pure .and
  | "or" => pure .or
  | "implies" => pure .implies
  | "iff" => pure .iff
  | k => throw s!"unknown spec operator `{k}`"

private def decodeLabel (j : Json) : Dec (Option MemLabel) :=
  if j.isNull then pure none else do pure (some (← decodeNat j))

private partial def decodeSpecExp (j : Json) : Dec SpecExp := do
  let (tag, p) ← decodeTagged j
  match tag with
  | "value" => pure (.value (← decodeValue (← payload tag p)))
  | "local" => pure (.loc (← decodeNat (← payload tag p)))
  | "bvar" => pure (.bvar (← decodeNat (← payload tag p)))
  | "result" => pure (.result (← decodeNat (← payload tag p)))
  | "binop" =>
    let a ← payloadArr tag 3 p
    pure (.binop (← decodeBinop (← a[0]!.getStr?)) (← decodeSpecExp a[1]!)
      (← decodeSpecExp a[2]!))
  | "not" => pure (.not (← decodeSpecExp (← payload tag p)))
  | "select" =>
    let a ← payloadArr tag 2 p
    pure (.select (← decodeNat a[0]!) (← decodeSpecExp a[1]!))
  | "len" => pure (.len (← decodeSpecExp (← payload tag p)))
  | "global" =>
    let a ← payloadArr tag 3 p
    pure (.global (← decodeNat a[0]!) (← decodeLabel a[1]!)
      (← decodeSpecExp a[2]!))
  | "exists" =>
    let a ← payloadArr tag 3 p
    pure (.exists_ (← decodeNat a[0]!) (← decodeLabel a[1]!)
      (← decodeSpecExp a[2]!))
  | "ite" =>
    let a ← payloadArr tag 3 p
    pure (.ite (← decodeSpecExp a[0]!) (← decodeSpecExp a[1]!)
      (← decodeSpecExp a[2]!))
  | "quant" =>
    let a ← payloadArr tag 3 p
    let kind ← match (← a[0]!.getStr?) with
      | "all" => pure QuantKind.all
      | "ex" => pure QuantKind.ex
      | k => throw s!"unknown quantifier kind `{k}`"
    pure (.quant kind (← decodeTy a[1]!) (← decodeSpecExp a[2]!))
  | k => throw s!"unknown spec expression `{k}`"

private def decodeSpecExps (j : Json) : Dec (List SpecExp) := do
  (← j.getArr?).toList.mapM decodeSpecExp

private def decodeLoop (j : Json) : Dec MLoop := do
  pure {
    header := ← decodeNat (← j.getObjVal? "header")
    members := ← decodeNats (← j.getObjVal? "members")
    valTargets := ← decodeNats (← j.getObjVal? "val_targets")
    memTargets := ← decodeNats (← j.getObjVal? "mem_targets")
    invariants := ← decodeSpecExps (← j.getObjVal? "invariants")
  }

private def decodeModifies (j : Json) : Dec (ResourceId × SpecExp) := do
  pure (← decodeNat (← j.getObjVal? "resource"),
    ← decodeSpecExp (← j.getObjVal? "addr"))

private def decodeContract (j : Json) : Dec MContract := do
  pure {
    requires := ← decodeSpecExps (← j.getObjVal? "requires")
    abortsIf := ← decodeSpecExps (← j.getObjVal? "aborts_if")
    ensures := ← decodeSpecExps (← j.getObjVal? "ensures")
    modifies :=
      ← (← (← j.getObjVal? "modifies").getArr?).toList.mapM decodeModifies
  }

private def decodeAbilitySet (names : List String) : Dec AbilitySet := do
  let known := ["copy", "drop", "store", "key"]
  for name in names do
    unless name ∈ known do throw s!"unknown Move ability `{name}`"
  unless names.eraseDups.length = names.length do throw "duplicate Move ability"
  pure {
    copy := "copy" ∈ names
    drop := "drop" ∈ names
    store := "store" ∈ names
    key := "key" ∈ names
  }

private def decodeTypeParam (j : Json) : Dec TypeParamDecl := do
  let abilities ← match j.getObjVal? "abilities" with
    | .ok value => (← value.getArr?).toList.mapM Json.getStr?
    | .error _ => pure []
  let phantom ← match j.getObjVal? "phantom" with
    | .ok value => value.getBool?
    | .error _ => pure false
  pure {
    name := ← (← j.getObjVal? "name").getStr?
    abilities := ← decodeAbilitySet abilities
    phantom := phantom
  }

private def decodeTypeParams (j : Json) : Dec (List TypeParamDecl) :=
  match j.getObjVal? "type_parameters" with
  | .error _ => pure []
  | .ok value => do (← value.getArr?).toList.mapM decodeTypeParam

private def decodeFun (j : Json) : Dec MFun := do
  pure {
    name := ← (← j.getObjVal? "name").getStr?
    typeParams := ← decodeTypeParams j
    params := ← decodeNat (← j.getObjVal? "params")
    locals := ← decodeTys (← j.getObjVal? "locals")
    returns := ← decodeTys (← j.getObjVal? "returns")
    blocks := ← (← (← j.getObjVal? "blocks").getArr?).toList.mapM decodeBlock
    loops := ← (← (← j.getObjVal? "loops").getArr?).toList.mapM decodeLoop
    spec := ← decodeContract (← j.getObjVal? "spec")
  }

private def decodeField (j : Json) : Dec (String × Ty) := do
  pure (← (← j.getObjVal? "name").getStr?, ← decodeTy (← j.getObjVal? "ty"))

private def decodeStruct (j : Json) : Dec MStruct := do
  let variants ← match j.getObjVal? "variants" with
    | .error _ => pure none
    | .ok value => do
        let variants ← (← value.getArr?).toList.mapM fun variant => do
          let name ← (← variant.getObjVal? "name").getStr?
          let fields ← (← (← variant.getObjVal? "fields").getArr?).toList.mapM decodeField
          return (name, fields)
        pure (some variants)
  pure {
    name := ← (← j.getObjVal? "name").getStr?
    typeParams := ← decodeTypeParams j
    fields := ← (← (← j.getObjVal? "fields").getArr?).toList.mapM decodeField
    variants := variants
  }

/-- Decodes the frontend's JSON dump into the first-order program. -/
def decodeMProgram (s : String) : Dec MProgram := do
  let j ← Json.parse s
  let version ← decodeNat (← j.getObjVal? "version")
  if version ≠ 10 then
    throw s!"unsupported exchange schema version {version}"
  pure {
    structs := ← (← (← j.getObjVal? "structs").getArr?).toList.mapM decodeStruct
    funs := ← (← (← j.getObjVal? "funs").getArr?).toList.mapM decodeFun
  }

end MoveModel.Frontend
