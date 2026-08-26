-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import MoveModel.Frontend.Decode
import MoveModel.Frontend.XIR

/-!
# Versioned JSON for deployable XIR modules

The module schema is distinct from the legacy compiler exchange schema.  The
instruction and specification encodings deliberately reuse its externally
tagged representation.
-/

namespace MoveModel.Frontend.XIR

open Lean
open MoveModel.IR

abbrev JsonResult := Except String

private def arr (values : List Json) : Json := .arr values.toArray
private def nat (n : Nat) : Json := .num n
private def tag (name : String) (value : Json) : Json := Json.mkObj [(name, value)]
private def tupleTag (name : String) (values : List Json) : Json := tag name (arr values)

private def encodeAddress (address : Address) : String :=
  "0x" ++ String.ofList (Nat.toDigits 16 address)

private def widthName : IntWidth → String
  | .w8 => "u8"
  | .w16 => "u16"
  | .w32 => "u32"
  | .w64 => "u64"
  | .w128 => "u128"
  | .w256 => "u256"

/-- Signed integer width names.  Signedness is carried by the `i`/`u` prefix of
the width string (mirroring the exchange format's `IntType`), so signed
operations reuse the same operation tags as their unsigned counterparts. -/
private def signedWidthName : IntWidth → String
  | .w8 => "i8"
  | .w16 => "i16"
  | .w32 => "i32"
  | .w64 => "i64"
  | .w128 => "i128"
  | .w256 => "i256"

/-- The width string of a numeric type: `uN` unsigned, `iN` signed. -/
private def numName (nt : NumType) : String :=
  if nt.signed then signedWidthName nt.width else widthName nt.width

private partial def encodeTy : Ty → Json
  | .bool => .str "bool"
  | .int nt => .str (numName nt)
  | .address => .str "address"
  | .signer => .str "signer"
  | .typeParam i => tag "type_parameter" (nat i)
  | .struct r => tag "struct" (nat r)
  | .structInst r args => tupleTag "struct_inst" [nat r, arr (args.map encodeTy)]
  | .enum r => tag "enum" (nat r)
  | .enumInst r args => tupleTag "enum_inst" [nat r, arr (args.map encodeTy)]
  | .vector elem => tag "vector" (encodeTy elem)
  | .ref elem => tag "ref" (encodeTy elem)
  | .mutRef elem => tag "mut_ref" (encodeTy elem)

private def encodeValue : Value → JsonResult Json
  | .int i => pure (tag "num" (.str (toString i)))
  | .bool b => pure (tag "bool" (.bool b))
  | .address a => pure (tag "address" (.str (encodeAddress a)))
  | .struct _ => throw "struct values are not valid XIR load constants"
  | .variant _ _ => throw "enum values are not valid XIR load constants"
  | .vector _ => throw "vector values are not valid XIR load constants"
  | .ref _ | .mut _ _ => throw "references are not valid XIR load constants"

private def encodeOper : Oper → JsonResult Json
  | .add nt => pure (tag "add" (.str (numName nt)))
  | .sub nt => pure (tag "sub" (.str (numName nt)))
  | .mul nt => pure (tag "mul" (.str (numName nt)))
  | .div nt => pure (tag "div" (.str (numName nt)))
  | .mod nt => pure (tag "mod" (.str (numName nt)))
  | .bitAnd nt => pure (tag "bit_and" (.str (numName nt)))
  | .bitOr nt => pure (tag "bit_or" (.str (numName nt)))
  | .bitXor nt => pure (tag "bit_xor" (.str (numName nt)))
  | .shl nt => pure (tag "shl" (.str (numName nt)))
  | .shr nt => pure (tag "shr" (.str (numName nt)))
  | .cast target => pure (tag "cast" (.str (numName target)))
  | .lt => pure (.str "lt")
  | .le => pure (.str "le") | .eq => pure (.str "eq")
  | .and => pure (.str "and") | .or => pure (.str "or")
  | .not => pure (.str "not") | .pack => pure (.str "pack")
  | .packInst args => pure (tag "pack_inst" (arr (args.map encodeTy)))
  | .unpack => pure (.str "unpack")
  | .unpackInst args => pure (tag "unpack_inst" (arr (args.map encodeTy)))
  | .packVariant i => pure (tag "pack_variant" (nat i))
  | .packVariantInst i args =>
      pure (tupleTag "pack_variant_inst" [nat i, arr (args.map encodeTy)])
  | .unpackVariant i => pure (tag "unpack_variant" (nat i))
  | .unpackVariantInst i args =>
      pure (tupleTag "unpack_variant_inst" [nat i, arr (args.map encodeTy)])
  | .testVariant i => pure (tag "test_variant" (nat i))
  | .testVariantInst i args =>
      pure (tupleTag "test_variant_inst" [nat i, arr (args.map encodeTy)])
  | .getField i => pure (tag "get_field" (nat i))
  | .getFieldInst i args =>
      pure (tupleTag "get_field_inst" [nat i, arr (args.map encodeTy)])
  | .updateField i => pure (tag "update_field" (nat i))
  | .vecPack => pure (.str "vec_pack") | .vecLen => pure (.str "vec_len")
  | .vecGet => pure (.str "vec_get") | .vecSet => pure (.str "vec_set")
  | .vecPush => pure (.str "vec_push") | .vecPop => pure (.str "vec_pop")
  | .vecInsert => pure (.str "vec_insert") | .vecRemove => pure (.str "vec_remove")
  | .vecSwap => pure (.str "vec_swap")
  | .vecSwapRemove => pure (.str "vec_swap_remove")
  | .vecAppend => pure (.str "vec_append")
  | .vecReverse => pure (.str "vec_reverse")
  | .vecReverseSlice => pure (.str "vec_reverse_slice")
  | .vecContains => pure (.str "vec_contains")
  | .vecIndexOf => pure (.str "vec_index_of")
  | .vecTrim => pure (.str "vec_trim")
  | .vecTrimReverse => pure (.str "vec_trim_reverse")
  | .vecRotate => pure (.str "vec_rotate")
  | .vecRotateSlice => pure (.str "vec_rotate_slice")
  | .vecDestroyEmpty => pure (.str "vec_destroy_empty")
  | .getGlobal r => pure (tag "get_global" (nat r))
  | .getGlobalInst r args =>
      pure (tupleTag "get_global_inst" [nat r, arr (args.map encodeTy)])
  | .writeGlobal r => pure (tag "write_global" (nat r))
  | .moveTo r => pure (tag "move_to" (nat r))
  | .moveToInst r args =>
      pure (tupleTag "move_to_inst" [nat r, arr (args.map encodeTy)])
  | .moveFrom r => pure (tag "move_from" (nat r))
  | .moveFromInst r args =>
      pure (tupleTag "move_from_inst" [nat r, arr (args.map encodeTy)])
  | .exists_ r => pure (tag "exists" (nat r))
  | .existsInst r args =>
      pure (tupleTag "exists_inst" [nat r, arr (args.map encodeTy)])
  | .function f => pure (tag "function" (nat f))
  | .functionInst f args =>
      pure (tupleTag "function_inst" [nat f, arr (args.map encodeTy)])
  | .borrowLoc => pure (.str "borrow_loc")
  | .borrowField i => pure (tag "borrow_field" (nat i))
  | .borrowFieldInst i args =>
      pure (tupleTag "borrow_field_inst" [nat i, arr (args.map encodeTy)])
  | .borrowGlobal r => pure (tag "borrow_global" (nat r))
  | .borrowGlobalInst r args =>
      pure (tupleTag "borrow_global_inst" [nat r, arr (args.map encodeTy)])
  | .borrowVecElem => pure (.str "borrow_vec_elem")
  | .readRef => pure (.str "read_ref")
  | .writeRef => pure (.str "write_ref")
  | .freezeRef => pure (.str "freeze_ref")
  | .mkMutLoc _ | .mkMutGlobal _ | .childMutField _ | .childMutIndex
  | .getMut | .setMut | .isParent _ | .mutPathIndex _ | .isMutLoc _
  | .isMutGlobal _ | .mutAddr =>
      throw "reference-elimination mutation operations are not deployable XIR"

private def encodeNats (values : List Nat) : Json := arr (values.map nat)

private def encodeInstr : Instr → JsonResult Json
  | .load dst value => return tupleTag "load" [nat dst, ← encodeValue value]
  | .assign dst src => pure (tupleTag "assign" [nat dst, nat src])
  | .call dsts op srcs =>
      return tupleTag "call" [encodeNats dsts, ← encodeOper op, encodeNats srcs]
  | .nop => pure (.str "nop")

private def encodeTerm : MoveModel.IR.Term → Json
  | .jump target => tag "jump" (nat target)
  | .branch cond thenB elseB => tupleTag "branch" [nat cond, nat thenB, nat elseB]
  | .ret srcs => tag "ret" (encodeNats srcs)
  | .abort code => tag "abort" (nat code)

private def encodeBlock (block : Block) : JsonResult Json := do
  return Json.mkObj [
    ("instrs", arr (← block.instrs.mapM encodeInstr)),
    ("term", encodeTerm block.term)
  ]

private def encodeBinop : SpecBinop → String
  | .add => "add" | .sub => "sub" | .mul => "mul" | .div => "div"
  | .mod => "mod" | .lt => "lt" | .le => "le" | .eq => "eq"
  | .index => "index" | .and => "and" | .or => "or"
  | .implies => "implies" | .iff => "iff"

private def encodeLabel : Option MemLabel → Json
  | none => .null
  | some label => nat label

private partial def encodeSpecExp : SpecExp → JsonResult Json
  | .value value => return tag "value" (← encodeValue value)
  | .loc index => pure (tag "local" (nat index))
  | .bvar index => pure (tag "bvar" (nat index))
  | .result index => pure (tag "result" (nat index))
  | .binop op lhs rhs =>
      return tupleTag "binop" [.str (encodeBinop op), ← encodeSpecExp lhs,
        ← encodeSpecExp rhs]
  | .not exp => return tag "not" (← encodeSpecExp exp)
  | .select field exp => return tupleTag "select" [nat field, ← encodeSpecExp exp]
  | .len exp => return tag "len" (← encodeSpecExp exp)
  | .mutVal _ => throw "mutation values are not deployable XIR specifications"
  | .global resource label address =>
      return tupleTag "global" [nat resource, encodeLabel label, ← encodeSpecExp address]
  | .exists_ resource label address =>
      return tupleTag "exists" [nat resource, encodeLabel label, ← encodeSpecExp address]
  | .ite cond thenExp elseExp =>
      return tupleTag "ite" [← encodeSpecExp cond, ← encodeSpecExp thenExp,
        ← encodeSpecExp elseExp]
  | .quant kind domain body =>
      let kind := match kind with | .all => "all" | .ex => "ex"
      return tupleTag "quant" [.str kind, encodeTy domain, ← encodeSpecExp body]

private def encodeSpecExps (expressions : List SpecExp) : JsonResult Json :=
  return arr (← expressions.mapM encodeSpecExp)

private def encodeLoop (loop : MLoop) : JsonResult Json := do
  return Json.mkObj [
    ("header", nat loop.header),
    ("members", encodeNats loop.members),
    ("val_targets", encodeNats loop.valTargets),
    ("mem_targets", encodeNats loop.memTargets),
    ("invariants", ← encodeSpecExps loop.invariants)
  ]

private def encodeContract (contract : MContract) : JsonResult Json := do
  let modifies ← contract.modifies.mapM fun (resource, address) => do
    return Json.mkObj [("resource", nat resource), ("addr", ← encodeSpecExp address)]
  return Json.mkObj [
    ("requires", ← encodeSpecExps contract.requires),
    ("aborts_if", ← encodeSpecExps contract.abortsIf),
    ("ensures", ← encodeSpecExps contract.ensures),
    ("modifies", arr modifies)
  ]

private def encodeAbilities (abilities : AbilitySet) : Json :=
  let values := []
  let values := if abilities.copy then values ++ [.str "copy"] else values
  let values := if abilities.drop then values ++ [.str "drop"] else values
  let values := if abilities.store then values ++ [.str "store"] else values
  let values := if abilities.key then values ++ [.str "key"] else values
  arr values

private def encodeTypeParam (param : TypeParamDecl) : Json :=
  Json.mkObj [
    ("name", .str param.name),
    ("abilities", encodeAbilities param.abilities),
    ("phantom", .bool param.phantom)
  ]

private def encodeTypeParams (params : List TypeParamDecl) : Json :=
  arr (params.map encodeTypeParam)

private partial def encodeAttributeArg : AttributeArg → Json
  | .name path args =>
      if args.isEmpty then Json.mkObj [("name", .str path)] else
        Json.mkObj [("name", .str path), ("args", arr (args.map encodeAttributeArg))]
  | .num value => Json.mkObj [("num", .str (toString value))]
  | .bool value => Json.mkObj [("bool", .bool value)]

private def encodeAttribute (decl : Attribute) : Json :=
  if decl.args.isEmpty then Json.mkObj [("name", .str decl.name)] else
    Json.mkObj [
      ("name", .str decl.name),
      ("args", arr (decl.args.map encodeAttributeArg))
    ]

/-- Attributes are an optional additive field: absent when empty, so modules
without attributes keep their established encoding. -/
private def attributeFields (attributes : List Attribute) :
    List (String × Json) :=
  if attributes.isEmpty then [] else
    [("attributes", arr (attributes.map encodeAttribute))]

private def encodeStruct (decl : MStruct) (info : StructMeta) : JsonResult Json := do
  unless decl.name = info.name do
    throw s!"struct body `{decl.name}` does not match metadata `{info.name}`"
  unless decl.fields.map (·.1) = info.fieldNames do
    throw s!"field metadata for struct `{decl.name}` does not match its body"
  let fields := decl.fields.map fun (name, ty) =>
    Json.mkObj [("name", .str name), ("ty", encodeTy ty)]
  unless decl.variants.map (·.map fun variant => (variant.1, variant.2.map (·.1))) =
      info.variantNames do
    throw s!"variant metadata for enum `{decl.name}` does not match its body"
  let variants := decl.variants.map fun variants => arr <| variants.map fun variant =>
    Json.mkObj [
      ("name", .str variant.1),
      ("fields", arr <| variant.2.map fun field =>
        Json.mkObj [("name", .str field.1), ("ty", encodeTy field.2)])
    ]
  let fields := [
    ("name", .str decl.name),
    ("type_parameters", encodeTypeParams decl.typeParams),
    ("abilities", encodeAbilities info.abilities),
    ("fields", arr fields)
  ] ++ attributeFields info.attributes
  return Json.mkObj <| match variants with
    | none => fields
    | some variants => fields ++ [("variants", variants)]

private def encodeVisibility : Visibility → String
  | .private_ => "private"
  | .public_ => "public"
  | .friend => "friend"

private def encodeSourceSpan (span : SourceSpan) : Json :=
  Json.mkObj [("start", nat span.start), ("end", nat span.end)]

private def encodeBlockSourceMap (sourceMap : BlockSourceMap) : Json :=
  Json.mkObj [
    ("instrs", arr <| sourceMap.instrs.map fun
      | some span => encodeSourceSpan span
      | none => .null),
    ("term", sourceMap.term.map encodeSourceSpan |>.getD .null)
  ]

private def sourceMapFields : Option FunSourceMap → List (String × Json)
  | none => []
  | some sourceMap => [("source_map", Json.mkObj [
      ("span", sourceMap.span.map encodeSourceSpan |>.getD .null),
      ("blocks", arr (sourceMap.blocks.map encodeBlockSourceMap))
    ])]

private def localNameFields (localNames : List (Option String)) : List (String × Json) :=
  if localNames.isEmpty then []
  else [("local_names", arr <| localNames.map fun
    | some name => .str name
    | none => .null)]

private def encodeFun (decl : MFun) (info : FunMeta) : JsonResult Json := do
  unless decl.name = info.name do
    throw s!"function body `{decl.name}` does not match metadata `{info.name}`"
  unless info.localNames.isEmpty || info.localNames.length = decl.locals.length do
    throw s!"function `{decl.name}` has {info.localNames.length} local names, but {decl.locals.length} locals"
  return Json.mkObj <| [
    ("name", .str decl.name),
    ("type_parameters", encodeTypeParams decl.typeParams),
    ("visibility", .str (encodeVisibility info.visibility)),
    ("is_entry", .bool info.isEntry),
    ("is_native", .bool decl.native),
    ("acquires", encodeNats info.acquires),
    ("params", nat decl.params),
    ("locals", arr (decl.locals.map encodeTy)),
    ("returns", arr (decl.returns.map encodeTy)),
    ("blocks", arr (← decl.blocks.mapM encodeBlock)),
    ("entry", nat decl.entry),
    ("loops", arr (← decl.loops.mapM encodeLoop)),
    ("spec", ← encodeContract decl.spec)
  ] ++ attributeFields info.attributes ++ localNameFields info.localNames ++
    sourceMapFields info.sourceMap

private def encodeDialect : Dialect → String
  | .stackless => "stackless"
  | .referenceEliminated => "reference_eliminated"

private def encodeExternalFun (reference : ExternalFunRef) : Json :=
  Json.mkObj [
    ("address", .str (encodeAddress reference.address)),
    ("module", .str reference.moduleName),
    ("function", .str reference.functionName)
  ]

private def encodeFriend (reference : ExternalModuleRef) : Json :=
  Json.mkObj [
    ("address", .str (encodeAddress reference.address)),
    ("module", .str reference.moduleName)
  ]

/-- Encode a deployable XIR module as schema-versioned JSON. -/
def MModule.toJson (module : MModule) : JsonResult Json := do
  unless module.structs.length = module.structMeta.length do
    throw "struct declarations and metadata have different lengths"
  unless module.funs.length = module.funMeta.length do
    throw "function declarations and metadata have different lengths"
  if module.dialect != .stackless then
    throw "only stackless XIR modules can be encoded for deployment"
  let structs ← (module.structs.zip module.structMeta).mapM fun (decl, info) =>
    encodeStruct decl info
  let functions ← (module.funs.zip module.funMeta).mapM fun (decl, info) =>
    encodeFun decl info
  let fields := [
    ("schema", .str "move-xir-module"),
    ("version", nat 5),
    ("module", Json.mkObj [
      ("address", .str (encodeAddress module.address)),
      ("name", .str module.name),
      ("dialect", .str (encodeDialect module.dialect))
    ]),
    ("structs", arr structs),
    ("functions", arr functions)
  ]
  let fields := if module.externalFuns.isEmpty then fields else
    fields ++ [("external_functions", arr (module.externalFuns.map encodeExternalFun))]
  return Json.mkObj <| if module.friends.isEmpty then fields else
    fields ++ [("friends", arr (module.friends.map encodeFriend))]

/-- Pretty, deterministic JSON text for a deployable XIR module. -/
def MModule.encodeJson (module : MModule) : JsonResult String :=
  return (← module.toJson).pretty

private def decodeAddress (text : String) : JsonResult Address := do
  unless text.startsWith "0x" do throw s!"expected a 0x-prefixed address, got `{text}`"
  let some address := Lean.Syntax.decodeNatLitVal? text
    | throw s!"invalid module address `{text}`"
  if address < 2 ^ 256 then return address
  throw s!"module address `{text}` does not fit in 256 bits"

private def decodeAbilities (json : Json) : JsonResult AbilitySet := do
  let names ← (← json.getArr?).toList.mapM Json.getStr?
  let known := ["copy", "drop", "store", "key"]
  for name in names do
    unless name ∈ known do throw s!"unknown Move ability `{name}`"
  unless names.eraseDups.length = names.length do throw "duplicate Move ability"
  return {
    copy := "copy" ∈ names
    drop := "drop" ∈ names
    store := "store" ∈ names
    key := "key" ∈ names
  }

private def decodeVisibility (text : String) : JsonResult Visibility :=
  match text with
  | "private" => pure .private_
  | "public" => pure .public_
  | "friend" => pure .friend
  | other => throw s!"unknown function visibility `{other}`"

private def decodeDialect (text : String) : JsonResult Dialect :=
  match text with
  | "stackless" => pure .stackless
  | "reference_eliminated" => pure .referenceEliminated
  | other => throw s!"unknown XIR dialect `{other}`"

private def decodeNatArray (json : Json) : JsonResult (List Nat) := do
  (← json.getArr?).toList.mapM Json.getNat?

private def decodeAttributeArgs (json : Json)
    (decodeArg : Json → JsonResult AttributeArg) :
    JsonResult (List AttributeArg) := do
  match json.getObjVal? "args" with
  | .ok argsJson => (← argsJson.getArr?).toList.mapM decodeArg
  | .error _ => return []

private partial def decodeAttributeArg (json : Json) :
    JsonResult AttributeArg := do
  match json.getObjVal? "name" with
  | .ok nameJson =>
      return .name (← nameJson.getStr?)
        (← decodeAttributeArgs json decodeAttributeArg)
  | .error _ =>
  match json.getObjVal? "num" with
  | .ok value =>
      let text ← value.getStr?
      let some number := text.toNat?
        | throw s!"invalid numeric attribute argument `{text}`"
      unless number < 2 ^ 64 do
        throw s!"attribute argument `{text}` does not fit in 64 bits"
      return .num number
  | .error _ =>
  match json.getObjVal? "bool" with
  | .ok value => return .bool (← value.getBool?)
  | .error _ => throw "unknown attribute argument"

private def decodeAttribute (json : Json) : JsonResult Attribute := do
  return {
    name := ← (← json.getObjVal? "name").getStr?
    args := ← decodeAttributeArgs json decodeAttributeArg
  }

private def decodeAttributes (json : Json) : JsonResult (List Attribute) := do
  match json.getObjVal? "attributes" with
  | .ok attributesJson => (← attributesJson.getArr?).toList.mapM decodeAttribute
  | .error _ => return []

private def decodeSourceSpan (json : Json) : JsonResult SourceSpan := do
  let start ← (← json.getObjVal? "start").getNat?
  let stop ← (← json.getObjVal? "end").getNat?
  unless start ≤ stop do throw "source span has a reversed range"
  return { start, «end» := stop }

private def decodeOptionalSourceSpan (json : Json) : JsonResult (Option SourceSpan) :=
  match json with
  | .null => pure none
  | _ => return some (← decodeSourceSpan json)

private def decodeSourceMap (json : Json) : JsonResult (Option FunSourceMap) := do
  let sourceMap ← match json.getObjVal? "source_map" with
    | .ok value => pure value
    | .error _ => return none
  let blocksJson ← (← sourceMap.getObjVal? "blocks").getArr?
  let blocks ← blocksJson.toList.mapM fun blockJson => do
    let instrsJson ← (← blockJson.getObjVal? "instrs").getArr?
    let instrs ← instrsJson.toList.mapM decodeOptionalSourceSpan
    let term ← decodeOptionalSourceSpan (← blockJson.getObjVal? "term")
    return ({ instrs, term } : BlockSourceMap)
  return some {
    span := ← decodeOptionalSourceSpan (← sourceMap.getObjVal? "span")
    blocks
  }

private def decodeLocalNames (json : Json) : JsonResult (List (Option String)) := do
  let namesJson ← match json.getObjVal? "local_names" with
    | .ok value => value.getArr?
    | .error _ => return []
  namesJson.toList.mapM fun
    | .null => pure none
    | value => return some (← value.getStr?)

/-- Decode schema-versioned deployable XIR JSON.  The body decoder is shared
with the established exchange-v5 format, while module metadata is checked
separately. -/
def decodeMModule (text : String) : JsonResult MModule := do
  let json ← Json.parse text
  let schema ← (← json.getObjVal? "schema").getStr?
  unless schema = "move-xir-module" do throw s!"unsupported XIR schema `{schema}`"
  let version ← (← json.getObjVal? "version").getNat?
  unless version = 3 || version = 4 || version = 5 do
    throw s!"unsupported XIR schema version {version}"
  let moduleJson ← json.getObjVal? "module"
  let address ← decodeAddress (← (← moduleJson.getObjVal? "address").getStr?)
  let name ← (← moduleJson.getObjVal? "name").getStr?
  let dialect ← decodeDialect (← (← moduleJson.getObjVal? "dialect").getStr?)
  let structsJson ← (← json.getObjVal? "structs").getArr?
  let functionsJson ← (← json.getObjVal? "functions").getArr?
  let externalFunsJson ← match json.getObjVal? "external_functions" with
    | .ok value => value.getArr?
    | .error _ => pure #[]
  let friendsJson ← match json.getObjVal? "friends" with
    | .ok value => value.getArr?
    | .error _ => pure #[]
  let legacy := Json.mkObj [
    ("version", nat 10),
    ("structs", .arr structsJson),
    ("funs", .arr functionsJson)
  ]
  let body ← MoveModel.Frontend.decodeMProgram legacy.compress
  let structMeta ← structsJson.toList.mapM fun structJson => do
    let structName ← (← structJson.getObjVal? "name").getStr?
    let fieldNames ← (← (← structJson.getObjVal? "fields").getArr?).toList.mapM fun field => do
      (← field.getObjVal? "name").getStr?
    let variantNames ← match structJson.getObjVal? "variants" with
      | .ok variantsJson => do
          let variants ← (← variantsJson.getArr?).toList.mapM fun variant => do
            let variantName ← (← variant.getObjVal? "name").getStr?
            let fields ← (← (← variant.getObjVal? "fields").getArr?).toList.mapM fun field => do
              (← field.getObjVal? "name").getStr?
            pure (variantName, fields)
          pure (some variants)
      | .error _ => pure none
    return ({
      name := structName
      fieldNames := fieldNames
      variantNames := variantNames
      abilities := ← decodeAbilities (← structJson.getObjVal? "abilities")
      attributes := ← decodeAttributes structJson
    } : StructMeta)
  let funMeta ← functionsJson.toList.mapM fun functionJson => do
    return ({
      name := ← (← functionJson.getObjVal? "name").getStr?
      visibility := ← decodeVisibility (← (← functionJson.getObjVal? "visibility").getStr?)
      isEntry := ← (← functionJson.getObjVal? "is_entry").getBool?
      acquires := ← decodeNatArray (← functionJson.getObjVal? "acquires")
      attributes := ← decodeAttributes functionJson
      localNames := ← decodeLocalNames functionJson
      sourceMap := ← decodeSourceMap functionJson
    } : FunMeta)
  let externalFuns ← externalFunsJson.toList.mapM fun functionJson => do
    return ({
      address := ← decodeAddress (← (← functionJson.getObjVal? "address").getStr?)
      moduleName := ← (← functionJson.getObjVal? "module").getStr?
      functionName := ← (← functionJson.getObjVal? "function").getStr?
    } : ExternalFunRef)
  let friends ← friendsJson.toList.mapM fun friendJson => do
    return ({
      address := ← decodeAddress (← (← friendJson.getObjVal? "address").getStr?)
      moduleName := ← (← friendJson.getObjVal? "module").getStr?
    } : ExternalModuleRef)
  let entries ← functionsJson.toList.mapM fun functionJson => do
    (← functionJson.getObjVal? "entry").getNat?
  unless body.funs.length = entries.length do throw "function entry metadata length mismatch"
  let natives ← functionsJson.toList.mapM fun functionJson =>
    match functionJson.getObjVal? "is_native" with
    | .ok value => value.getBool?
    | .error _ => pure false
  let funs := ((body.funs.zip entries).zip natives).map fun ((decl, entry), native) =>
    { decl with entry := entry, native := native }
  return {
    structs := body.structs
    funs := funs
    address := address
    name := name
    dialect := dialect
    structMeta := structMeta
    funMeta := funMeta
    externalFuns := externalFuns
    friends := friends
  }

end MoveModel.Frontend.XIR
