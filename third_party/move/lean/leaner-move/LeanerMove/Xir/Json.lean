-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import LeanerMove.Xir.Syntax

/-!
# XIR JSON encoding

The layout of the Rust schema's serde derivation: enums are externally
tagged in snake case, a unit variant is its tag string, and an operation or
type with type arguments uses its `_inst` variant. Optional fields the schema
skips when empty are omitted.
-/

namespace LeanerIR.Move.Xir

open Lean (Json)

private def tagged (tag : String) (value : Json) : Json := Json.mkObj [(tag, value)]

private def nat (value : Nat) : Json := (value : Int)

private def nats (values : Array Nat) : Json := Json.arr (values.map nat)

def IntType.toJson : IntType → Json
  | .u8 => "u8" | .u16 => "u16" | .u32 => "u32" | .u64 => "u64" | .u128 => "u128"
  | .u256 => "u256" | .i8 => "i8" | .i16 => "i16" | .i32 => "i32" | .i64 => "i64"
  | .i128 => "i128" | .i256 => "i256"

partial def Ty.toJson : Ty → Json
  | .bool => "bool"
  | .int type => type.toJson
  | .address => "address"
  | .signer => "signer"
  | .typeParameter index => tagged "type_parameter" (nat index)
  | .struct id arguments =>
      if arguments.isEmpty then tagged "struct" (nat id)
      else tagged "struct_inst" (Json.arr #[nat id, Json.arr (arguments.map Ty.toJson)])
  | .enum id arguments =>
      if arguments.isEmpty then tagged "enum" (nat id)
      else tagged "enum_inst" (Json.arr #[nat id, Json.arr (arguments.map Ty.toJson)])
  | .vector element => tagged "vector" element.toJson
  | .ref referent => tagged "ref" referent.toJson
  | .mutRef referent => tagged "mut_ref" referent.toJson

private def types (arguments : Array Ty) : Json := Json.arr (arguments.map Ty.toJson)

def TypeParameter.toJson (parameter : TypeParameter) : Json :=
  Json.mkObj <| [("name", Json.str parameter.name)] ++
    (if parameter.abilities.isEmpty then []
      else [("abilities", Json.arr (parameter.abilities.map Json.str))]) ++
    (if parameter.phantom then [("phantom", Json.bool true)] else [])

def Field.toJson (field : Field) : Json :=
  Json.mkObj [("name", Json.str field.name), ("ty", field.ty.toJson)]

partial def AttributeArg.toJson : AttributeArg → Json
  | .name head args => Json.mkObj <| [("name", Json.str head)] ++
      (if args.isEmpty then [] else [("args", Json.arr (args.map AttributeArg.toJson))])
  | .num value => Json.mkObj [("num", Json.str (toString value))]
  | .bool value => Json.mkObj [("bool", Json.bool value)]

def Attribute.toJson (source : Attribute) : Json :=
  Json.mkObj <| [("name", Json.str source.name)] ++
    (if source.args.isEmpty then []
      else [("args", Json.arr (source.args.map AttributeArg.toJson))])

/-- The `attributes` field, which the schema omits when empty. -/
private def attributesField (attributes : Array Attribute) : List (String × Json) :=
  if attributes.isEmpty then [] else [("attributes", Json.arr (attributes.map Attribute.toJson))]

def Struct.toJson (declaration : Struct) : Json :=
  Json.mkObj <| [
    ("name", Json.str declaration.name),
    ("abilities", Json.arr (declaration.abilities.map Json.str)),
    ("type_parameters", Json.arr (declaration.typeParameters.map TypeParameter.toJson)),
    ("fields", Json.arr (declaration.fields.map Field.toJson)),
    ("variants", match declaration.variants with
      | none => Json.null
      | some variants => Json.arr <| variants.map fun variant =>
          Json.mkObj [("name", Json.str variant.name),
            ("fields", Json.arr (variant.fields.map Field.toJson))])] ++
    attributesField declaration.attributes

partial def Value.toJson : Value → Json
  | .bool value => tagged "bool" (Json.bool value)
  | .num value => tagged "num" (Json.str (toString value))
  | .address value => tagged "address" (Json.str value)
  | .vector elements => tagged "vector" (Json.arr (elements.map Value.toJson))

/-- An operation with an optional instantiation: the plain tag without type
arguments, the `_inst` tag with them. -/
private def instantiated (tag : String) (plain : Option Json) (arguments : Array Ty)
    (operands : Array Json := #[]) : Json :=
  if arguments.isEmpty then
    match plain with
    | some value => tagged tag value
    | none => Json.str tag
  else if operands.isEmpty then tagged s!"{tag}_inst" (types arguments)
  else tagged s!"{tag}_inst" (Json.arr (operands.push (types arguments)))

def Oper.toJson : Oper → Json
  | .add type => tagged "add" type.toJson
  | .sub type => tagged "sub" type.toJson
  | .mul type => tagged "mul" type.toJson
  | .div type => tagged "div" type.toJson
  | .mod type => tagged "mod" type.toJson
  | .bitAnd type => tagged "bit_and" type.toJson
  | .bitOr type => tagged "bit_or" type.toJson
  | .bitXor type => tagged "bit_xor" type.toJson
  | .shl type => tagged "shl" type.toJson
  | .shr type => tagged "shr" type.toJson
  | .cast type => tagged "cast" type.toJson
  | .lt => "lt" | .le => "le" | .eq => "eq" | .and => "and" | .or => "or" | .not => "not"
  | .pack arguments => instantiated "pack" none arguments
  | .unpack arguments => instantiated "unpack" none arguments
  | .packVariant variant arguments =>
      instantiated "pack_variant" (some (nat variant)) arguments #[nat variant]
  | .unpackVariant variant arguments =>
      instantiated "unpack_variant" (some (nat variant)) arguments #[nat variant]
  | .testVariant variant arguments =>
      instantiated "test_variant" (some (nat variant)) arguments #[nat variant]
  | .getField field arguments =>
      instantiated "get_field" (some (nat field)) arguments #[nat field]
  | .vecPack => "vec_pack" | .vecLen => "vec_len" | .vecGet => "vec_get"
  | .vecSet => "vec_set" | .vecPush => "vec_push" | .vecPop => "vec_pop"
  | .vecInsert => "vec_insert" | .vecRemove => "vec_remove" | .vecSwap => "vec_swap"
  | .moveTo struct arguments => instantiated "move_to" (some (nat struct)) arguments #[nat struct]
  | .moveFrom struct arguments =>
      instantiated "move_from" (some (nat struct)) arguments #[nat struct]
  | .exists struct arguments => instantiated "exists" (some (nat struct)) arguments #[nat struct]
  | .function id arguments => instantiated "function" (some (nat id)) arguments #[nat id]
  | .borrowLoc => "borrow_loc"
  | .borrowField field arguments =>
      instantiated "borrow_field" (some (nat field)) arguments #[nat field]
  | .borrowGlobal struct arguments =>
      instantiated "borrow_global" (some (nat struct)) arguments #[nat struct]
  | .borrowVecElem => "borrow_vec_elem"
  | .borrowVariantField variants field arguments =>
      instantiated "borrow_variant_field" (some (Json.arr #[nats variants, nat field])) arguments
        #[nats variants, nat field]
  | .testVariantRef variant arguments =>
      instantiated "test_variant_ref" (some (nat variant)) arguments #[nat variant]
  | .readRef => "read_ref" | .writeRef => "write_ref" | .freezeRef => "freeze_ref"

def Instr.toJson : Instr → Json
  | .load destination value => tagged "load" (Json.arr #[nat destination, value.toJson])
  | .assign destination source => tagged "assign" (Json.arr #[nat destination, nat source])
  | .call destinations operation sources =>
      tagged "call" (Json.arr #[nats destinations, operation.toJson, nats sources])

def Term.toJson : Term → Json
  | .jump block => tagged "jump" (nat block)
  | .branch condition thenBlock elseBlock =>
      tagged "branch" (Json.arr #[nat condition, nat thenBlock, nat elseBlock])
  | .ret sources => tagged "ret" (nats sources)
  | .abort code => tagged "abort" (nat code)

private def spanJson : Option Span → Json
  | none => Json.null
  | some span => Json.mkObj [("start", nat span.start), ("end", nat span.stop)]

def Visibility.toJson : Visibility → Json
  | .private_ => "private" | .public_ => "public" | .friend => "friend"

def Function.toJson (function : Function) : Json :=
  let emptyContract := Json.mkObj [("requires", Json.arr #[]), ("aborts_if", Json.arr #[]),
    ("ensures", Json.arr #[]), ("modifies", Json.arr #[])]
  let hasSpans := function.span.isSome ||
    function.blocks.any fun block => block.termSpan.isSome || block.instrSpans.any (·.isSome)
  Json.mkObj <| [
    ("name", Json.str function.name),
    ("type_parameters", Json.arr (function.typeParameters.map TypeParameter.toJson)),
    ("visibility", function.visibility.toJson),
    ("is_entry", Json.bool function.isEntry),
    ("is_native", Json.bool function.isNative),
    ("acquires", nats function.acquires),
    ("params", nat function.params),
    ("locals", types function.locals),
    ("returns", types function.returns),
    ("blocks", Json.arr <| function.blocks.map fun block =>
      Json.mkObj [("instrs", Json.arr (block.instrs.map Instr.toJson)),
        ("term", block.term.toJson)]),
    ("entry", nat 0),
    ("loops", Json.arr #[]),
    -- Contracts are not transported: verification runs over the source.
    ("spec", emptyContract)] ++
    attributesField function.attributes ++
    (if function.localNames.isEmpty then []
      else [("local_names", Json.arr <| function.localNames.map fun
        | some name => Json.str name
        | none => Json.null)]) ++
    (if hasSpans then [("source_map", Json.mkObj [
        ("span", spanJson function.span),
        ("blocks", Json.arr <| function.blocks.map fun block =>
          Json.mkObj [("instrs", Json.arr (block.instrSpans.map spanJson)),
            ("term", spanJson block.termSpan)])])]
      else [])

private def External.toJson (field : String) (reference : External) : Json :=
  Json.mkObj [("address", Json.str reference.address), ("module", Json.str reference.module),
    (field, Json.str reference.name)]

def Module.toJson (module : Module) : Json :=
  Json.mkObj [
    ("schema", "move-xir-module"),
    ("version", nat version),
    ("module", Json.mkObj [("address", Json.str module.address),
      ("name", Json.str module.name), ("dialect", "stackless")]),
    ("structs", Json.arr (module.structs.map Struct.toJson)),
    ("functions", Json.arr (module.functions.map Function.toJson)),
    ("external_functions", Json.arr (module.externalFunctions.map (External.toJson "function"))),
    ("external_structs", Json.arr (module.externalStructs.map (External.toJson "name")))]

end LeanerIR.Move.Xir
