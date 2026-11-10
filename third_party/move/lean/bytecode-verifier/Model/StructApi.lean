-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Spec.Syntax
import Model.Result

/-!
# Struct-API wrappers

UNTRUSTED. Transcribes `move-bytecode-verifier/src/struct_api_checker.rs`,
which runs for modules of bytecode version 10 and later. A function whose
name has the form `pack$S`, `unpack$S`, `pack$S$V`, `unpack$S$V`,
`test_variant$S$V`, `borrow$S$k[$t]` or `borrow_mut$S$k[$t]`, for a struct
`S` defined in the module, must carry the matching struct-API attribute, and
a function with such an attribute must have such a name. The wrapper's
signature must match the struct's fields, and its code must be the
`MoveLoc(0) … MoveLoc(n-1); <op>; Ret` the compiler emits. Reference safety
trusts a `BorrowFieldMutable` attribute only because this pass has checked
it.

Every failure is `INVALID_STRUCT_API_CODE`. Rust's maps keyed by name keep
the last definition with a name; the model keeps every entry and looks up
the last one.
-/

namespace MoveBytecode.Model

namespace StructApi

def apiError {α : Type} : VResult α := reject "INVALID_STRUCT_API_CODE"

def isStructApi : FunctionAttribute → Bool
  | .pack | .packVariant _ | .unpack | .unpackVariant _ | .testVariant _
  | .borrowFieldImmutable _ | .borrowFieldMutable _ => true
  | .persistent | .moduleLock => false

/-- `try_get_struct_api_attr`: at most one struct-API attribute. -/
def structApiAttr (attrs : List FunctionAttribute) : VResult (Option FunctionAttribute) :=
  match attrs.filter isStructApi with
  | [] => pure none
  | [a] => pure (some a)
  | _ :: _ :: _ => apiError

/-- A field position and type in an enum. -/
abbrev Key := Nat × SignatureToken

/-- The maps `build_variant_type_order_and_indices_map` builds for an enum:
the order of each distinct (offset, type) pair by first occurrence, and the
variants that have each pair, in ascending order. -/
structure EnumMaps where
  typeOrder : List (Key × Nat) := []
  variantIndices : List (Key × List Nat) := []

/-- `build_variant_type_order_and_indices_map`. -/
def buildMaps (variants : List VariantDefinition) : VResult EnumMaps :=
  variants.zipIdx.foldlM (fun maps (v, vi) => do
    if vi ≥ 65536 then apiError
    v.fields.zipIdx.foldlM (fun (maps : EnumMaps) (f, fo) => do
      if fo ≥ 65536 then apiError
      let key : Key := (fo, f.signature)
      let typeOrder ←
        if maps.typeOrder.any (·.1 == key) then pure maps.typeOrder
        else
          let next := maps.typeOrder.length
          if next = 65535 then apiError
          pure (maps.typeOrder ++ [(key, next)])
      let variantIndices := match maps.variantIndices.lookup key with
        | some vs => maps.variantIndices.map fun e => if e.1 == key then (key, vs ++ [vi]) else e
        | none => maps.variantIndices ++ [(key, [vi])]
      pure { typeOrder, variantIndices }) maps) {}

/-- `StructApiContext`. -/
structure Context where
  /-- Each struct definition's name, handle and definition index. -/
  structs : List (String × Nat × Nat)
  /-- Each enum definition's name and maps. -/
  enums : List (String × EnumMaps)

def Context.struct? (ctx : Context) (name : String) : Option (Nat × Nat) :=
  ((ctx.structs.filter (·.1 == name)).getLast?).map (·.2)

def Context.enum? (ctx : Context) (name : String) : Option EnumMaps :=
  ((ctx.enums.filter (·.1 == name)).getLast?).map (·.2)

/-- `StructApiContext::new`, which also checks the attributes of every
function handle, imported ones included. -/
def context (m : CompiledModule) : VResult Context := do
  let (structs, enums) ← m.structDefs.toList.zipIdx.foldlM (fun (structs, enums) (sd, idx) => do
    let sh ← lookup m.structHandles sd.structHandle "struct_handle_at"
    let name ← lookup m.identifiers sh.name "identifier_at"
    let structs := structs ++ [(name, sd.structHandle, idx)]
    match sd.fieldInformation with
    | .declaredVariants variants => pure (structs, enums ++ [(name, ← buildMaps variants)])
    | .declared _ | .native => pure (structs, enums)) ([], [])
  m.functionHandles.toList.forM fun fh => do
    let _ ← structApiAttr fh.attributes
  pure ⟨structs, enums⟩

inductive NamePrefix where
  | pack
  | unpack
  | testVariant
  | borrow
  deriving DecidableEq, Repr

/-- `ParsedApiData`. -/
structure Parsed where
  prefix_ : NamePrefix
  isMutable : Bool
  structDef : Nat
  structHandle : Nat
  structName : String
  variantName : Option String
  offset : Option Nat
  typeOrder : Option Nat

/-- `str::parse::<u16>`: ASCII digits after an optional `+`. -/
def parseU16 (s : String) : Option Nat :=
  let cs := match s.toList with
    | '+' :: rest => rest
    | cs => cs
  if cs.isEmpty || !cs.all Char.isDigit then none
  else
    let n := cs.foldl (fun n c => n * 10 + (c.toNat - '0'.toNat)) 0
    if n < 65536 then some n else none

/-- `parse_struct_api_name`. The struct name is the shortest run of parts
naming a local struct when a variant name should follow, else the longest. -/
def parseName (name : String) (ctx : Context) (expectVariant : Bool) : Option Parsed :=
  match name.splitOn "$" with
  | [] | [_] => none
  | prefixStr :: rest =>
    let pfx? : Option NamePrefix := match prefixStr with
      | "pack" => some .pack
      | "unpack" => some .unpack
      | "test_variant" => some .testVariant
      | "borrow" | "borrow_mut" => some .borrow
      | _ => none
    match pfx? with
    | none => none
    | some pfx =>
      let found := (List.range rest.length).filterMap fun j =>
        let candidate := "$".intercalate (rest.take (j + 1))
        (ctx.struct? candidate).map fun (h, d) => (j + 1, candidate, h, d)
      let chosen := if expectVariant then found.head? else found.getLast?
      match chosen with
      | none => none
      | some (structEnd, structName, structHandle, structDef) =>
        let remaining := rest.drop structEnd
        let parts? : Option (Option String × Option Nat × Option Nat) :=
          match pfx with
          | .pack | .unpack =>
            if remaining.isEmpty then some (none, none, none)
            else some (some ("$".intercalate remaining), none, none)
          | .testVariant =>
            if remaining.isEmpty then none
            else some (some ("$".intercalate remaining), none, none)
          | .borrow =>
            match remaining with
            | [o] => (parseU16 o).map fun o => (none, some o, none)
            | [o, t] => do
              let o ← parseU16 o
              let t ← parseU16 t
              pure (none, some o, some t)
            | _ => none
        parts?.map fun (variantName, offset, typeOrder) =>
          { prefix_ := pfx, isMutable := prefixStr == "borrow_mut", structDef, structHandle,
            structName, variantName, offset, typeOrder }

/-- `ParsedApiData::matches_attr`. -/
def Parsed.matchesAttr (p : Parsed) : FunctionAttribute → Bool
  | .pack => p.prefix_ == .pack && p.variantName.isNone
  | .packVariant _ => p.prefix_ == .pack && p.variantName.isSome
  | .unpack => p.prefix_ == .unpack && p.variantName.isNone
  | .unpackVariant _ => p.prefix_ == .unpack && p.variantName.isSome
  | .testVariant _ => p.prefix_ == .testVariant
  | .borrowFieldImmutable _ => p.prefix_ == .borrow && !p.isMutable
  | .borrowFieldMutable _ => p.prefix_ == .borrow && p.isMutable
  | .persistent | .moduleLock => false

/-- `ensure_canonical_type_params`: exactly `[T0, …, Tn-1]`. -/
def ensureCanonical (args : List SignatureToken) (n : Nat) : VResult Unit :=
  if args.length == n && args.zipIdx.all (fun (t, i) => t == .typeParameter i) then pure ()
  else apiError

mutual
/-- `check_token_canonical_type_params`. -/
def checkTokenCanonical (sh n : Nat) : SignatureToken → VResult Unit
  | .structInstantiation idx args => do
    if idx == sh then ensureCanonical args n
    checkTokensCanonical sh n args
  | .reference t | .mutableReference t | .vector t => checkTokenCanonical sh n t
  | .function ps rs _ => do
    checkTokensCanonical sh n ps
    checkTokensCanonical sh n rs
  | .bool | .u8 | .u16 | .u32 | .u64 | .u128 | .u256 | .i8 | .i16 | .i32 | .i64 | .i128
  | .i256 | .address | .signer | .struct _ | .typeParameter _ => pure ()

def checkTokensCanonical (sh n : Nat) : List SignatureToken → VResult Unit
  | [] => pure ()
  | t :: ts => do
    checkTokenCanonical sh n t
    checkTokensCanonical sh n ts
end

/-- `check_struct_token`. -/
def checkStructToken (t : SignatureToken) (expected : Nat) : VResult Unit :=
  match t with
  | .struct idx | .structInstantiation idx _ => if idx == expected then pure () else apiError
  | _ => apiError

/-- `validate_signature_against_fields`. -/
def validateAgainstFields (sig : Signature) (fields : List FieldDefinition) : VResult Unit :=
  if sig.length != fields.length then apiError
  else if (sig.zip fields).all (fun (t, f) => t == f.signature) then pure ()
  else apiError

/-- `check_struct_api_bytecode_pattern`: `MoveLoc(0) … MoveLoc(n-1)`, the
operation, `Ret`. -/
def checkPattern (code : CodeUnit) (numMoveLocs : Nat) (isExpected : Bytecode → Bool) :
    VResult Unit := do
  let instrs := code.code.toList
  if instrs.length != numMoveLocs + 2 then apiError
  match instrs.getLast? with
  | some .ret => pure ()
  | _ => apiError
  match instrs[instrs.length - 2]? with
  | some op => if !isExpected op then apiError
  | none => apiError
  (instrs.take numMoveLocs).zipIdx.forM fun (bc, i) =>
    match bc with
    | .moveLoc j => if j == i then pure () else apiError
    | _ => apiError

/-- A variant instruction's handle, direct or through an instantiation. -/
inductive VariantRef where
  | direct (h : Nat)
  | generic (i : Nat)

/-- `StructApiWrapperValidator`. -/
structure Wrapper where
  m : CompiledModule
  ctx : Context
  handle : FunctionHandle
  code : CodeUnit
  structDef : Nat
  structHandle : Nat
  numTypeParams : Nat

namespace Wrapper

variable (w : Wrapper)

def signatureAt (i : Nat) : VResult Signature := lookup w.m.signatures i "signature_at"

def structDefinition : VResult StructDefinition :=
  match w.m.structDefs[w.structDef]? with
  | some sd => pure sd
  | none => panicAt "struct_defs()[idx]"

def identifierAt (i : Nat) : VResult String := lookup w.m.identifiers i "identifier_at"

/-- `check_signature_canonical_type_params`. -/
def checkSignatureCanonical : VResult Unit :=
  [w.handle.parameters, w.handle.returns].forM fun idx => do
    checkTokensCanonical w.structHandle w.numTypeParams (← w.signatureAt idx)

/-- `validate_signature_matches_fields`. -/
def validateSignatureMatchesFields (sigIdx : Nat) (variantName : Option String) :
    VResult Unit := do
  let sig ← w.signatureAt sigIdx
  match (← w.structDefinition).fieldInformation with
  | .native => apiError
  | .declared fields => if variantName.isSome then apiError else validateAgainstFields sig fields
  | .declaredVariants variants =>
    match variantName with
    | none => apiError
    | some vn =>
      match ← variants.findM? (fun v => do pure ((← w.identifierAt v.name) == vn)) with
      | some v => validateAgainstFields sig v.fields
      | none => apiError

/-- `check_pack_return_type`. -/
def checkPackReturnType : VResult Unit := do
  match ← w.signatureAt w.handle.returns with
  | [t] => checkStructToken t w.structHandle
  | _ => apiError

/-- `check_struct_param`. -/
def checkStructParam (asRef : Bool) : VResult Unit := do
  match ← w.signatureAt w.handle.parameters with
  | [t] =>
    if asRef then
      match t with
      | .reference inner => checkStructToken inner w.structHandle
      | _ => apiError
    else checkStructToken t w.structHandle
  | _ => apiError

/-- `check_borrow_param`. -/
def checkBorrowParam (isMutable : Bool) : VResult Unit := do
  match ← w.signatureAt w.handle.parameters with
  | [t] =>
    let (inner, paramMutable) ← match t with
      | .reference inner => pure (inner, false)
      | .mutableReference inner => pure (inner, true)
      | _ => apiError
    let idx ← match inner with
      | .struct idx | .structInstantiation idx _ => pure idx
      | _ => apiError
    if idx != w.structHandle then apiError
    if isMutable != paramMutable then apiError
  | _ => apiError

/-- `validate_variant_index`. -/
def validateVariantIndex (variantName : String) (attrIdx : Nat) : VResult Nat := do
  let variants ← match (← w.structDefinition).fieldInformation with
    | .declaredVariants variants => pure variants
    | .declared _ | .native => apiError
  let found ← variants.zipIdx.findM? fun (v, _) => do
    pure ((← w.identifierAt v.name) == variantName)
  match found with
  | some (_, idx) => if idx < 65536 && idx == attrIdx then pure idx else apiError
  | none => apiError

/-- `validate_borrow_return_type`: the field type the wrapper borrows. -/
def validateBorrowReturnType (structName : String) (offset : Nat) (typeOrder : Option Nat)
    (isMutable : Bool) : VResult SignatureToken := do
  let actualInner ← match ← w.signatureAt w.handle.returns with
    | [.reference inner] => if !isMutable then pure inner else apiError
    | [.mutableReference inner] => if isMutable then pure inner else apiError
    | [_] => apiError
    | _ => apiError
  let expected ← match (← w.structDefinition).fieldInformation with
    | .native => apiError
    | .declared fields =>
      if typeOrder.isSome then apiError
      else match fields[offset]? with
        | some f => pure f.signature
        | none => apiError
    | .declaredVariants _ =>
      match typeOrder with
      | none => apiError
      | some order =>
        match w.ctx.enum? structName with
        | none => apiError
        | some maps =>
          match maps.typeOrder.find? (fun ((fo, _), ord) => fo == offset && ord == order) with
          | some ((_, ty), _) => pure ty
          | none => apiError
  if actualInner != expected then apiError
  pure expected

/-- `check_struct_def_instantiation`. -/
def checkStructDefInstantiation (i : Nat) : VResult Unit := do
  let sdi ← lookup w.m.structDefInstantiations i "struct_instantiation_at"
  if sdi.def_ != w.structDef then apiError
  ensureCanonical (← w.signatureAt sdi.typeParameters) w.numTypeParams

/-- The second-to-last instruction, as `code.get(len.wrapping_sub(2))`. -/
def penultimate : Option Bytecode :=
  if w.code.code.size ≥ 2 then w.code.code[w.code.code.size - 2]? else none

/-- `check_pack_pattern`. -/
def checkPackPattern : VResult Unit := do
  let numFields ← match (← w.structDefinition).fieldInformation with
    | .declared fields => pure fields.length
    | .native | .declaredVariants _ => apiError
  let genericOk ← match w.penultimate with
    | some (.packGeneric i) => do w.checkStructDefInstantiation i; pure true
    | _ => pure false
  checkPattern w.code numFields fun bc => match bc with
    | .pack d => d == w.structDef
    | .packGeneric _ => genericOk
    | _ => false

/-- `check_unpack_pattern`. -/
def checkUnpackPattern : VResult Unit := do
  let genericOk ← match w.penultimate with
    | some (.unpackGeneric i) => do w.checkStructDefInstantiation i; pure true
    | _ => pure false
  checkPattern w.code 1 fun bc => match bc with
    | .unpack d => d == w.structDef
    | .unpackGeneric _ => genericOk
    | _ => false

/-- `check_variant_op_pattern`. -/
def checkVariantOpPattern (numMoveLocs expectedIdx : Nat) (toRef : Bytecode → Option VariantRef) :
    VResult Unit := do
  checkPattern w.code numMoveLocs fun bc => (toRef bc).isSome
  let found ← match w.code.code.toList.findSome? toRef with
    | none => pure none
    | some r => do
      let (variant, structIdx) ← match r with
        | .direct h => do
          let handle ← lookup w.m.structVariantHandles h "struct_variant_handle_at"
          pure (handle.variant, handle.structIndex)
        | .generic i => do
          let inst ← lookup w.m.structVariantInstantiations i "struct_variant_instantiation_at"
          let handle ← lookup w.m.structVariantHandles inst.handle "struct_variant_handle_at"
          ensureCanonical (← w.signatureAt inst.typeParameters) w.numTypeParams
          pure (handle.variant, handle.structIndex)
      if structIdx != w.structDef then apiError
      pure (some variant)
  match found with
  | some idx => if idx != expectedIdx then apiError
  | none => apiError

/-- `check_pack_variant_pattern`. -/
def checkPackVariantPattern (expectedIdx : Nat) : VResult Unit := do
  let numFields ← match (← w.structDefinition).fieldInformation with
    | .declaredVariants variants =>
      match variants[expectedIdx]? with
      | some v => pure v.fields.length
      | none => apiError
    | .declared _ | .native => apiError
  w.checkVariantOpPattern numFields expectedIdx fun bc => match bc with
    | .packVariant h => some (.direct h)
    | .packVariantGeneric i => some (.generic i)
    | _ => none

/-- `check_unpack_variant_pattern`. -/
def checkUnpackVariantPattern (expectedIdx : Nat) : VResult Unit :=
  w.checkVariantOpPattern 1 expectedIdx fun bc => match bc with
    | .unpackVariant h => some (.direct h)
    | .unpackVariantGeneric i => some (.generic i)
    | _ => none

/-- `check_test_variant_pattern`. -/
def checkTestVariantPattern (expectedIdx : Nat) : VResult Unit :=
  w.checkVariantOpPattern 1 expectedIdx fun bc => match bc with
    | .testVariant h => some (.direct h)
    | .testVariantGeneric i => some (.generic i)
    | _ => none

/-- `resolve_field` and `resolve_field_generic`. -/
def resolveField (h : Nat) (generic? : Option Nat) : VResult Nat := do
  let fh ← lookup w.m.fieldHandles h "field_handle_at"
  if fh.owner != w.structDef then apiError
  match generic? with
  | some sig => ensureCanonical (← w.signatureAt sig) w.numTypeParams
  | none => pure ()
  pure fh.field

/-- `resolve_variant_field` and `resolve_variant_field_generic`. -/
def resolveVariantField (h : Nat) (generic? : Option Nat) : VResult Nat := do
  let vfh ← lookup w.m.variantFieldHandles h "variant_field_handle_at"
  if vfh.structIndex != w.structDef then apiError
  match generic? with
  | some sig => ensureCanonical (← w.signatureAt sig) w.numTypeParams
  | none => pure ()
  pure vfh.field

/-- `get_borrow_field_offset`. -/
def getBorrowFieldOffset (bc : Bytecode) (expectedMutable : Bool) : VResult Nat := do
  let fieldInst (i : Nat) := lookup w.m.fieldInstantiations i "field_instantiation_at"
  let variantInst (i : Nat) :=
    lookup w.m.variantFieldInstantiations i "variant_field_instantiation_at"
  let (isMutable, offset) ← match bc with
    | .immBorrowField h => do pure (false, ← w.resolveField h none)
    | .mutBorrowField h => do pure (true, ← w.resolveField h none)
    | .immBorrowFieldGeneric i => do
      let inst ← fieldInst i
      pure (false, ← w.resolveField inst.handle (some inst.typeParameters))
    | .mutBorrowFieldGeneric i => do
      let inst ← fieldInst i
      pure (true, ← w.resolveField inst.handle (some inst.typeParameters))
    | .immBorrowVariantField h => do pure (false, ← w.resolveVariantField h none)
    | .mutBorrowVariantField h => do pure (true, ← w.resolveVariantField h none)
    | .immBorrowVariantFieldGeneric i => do
      let inst ← variantInst i
      pure (false, ← w.resolveVariantField inst.handle (some inst.typeParameters))
    | .mutBorrowVariantFieldGeneric i => do
      let inst ← variantInst i
      pure (true, ← w.resolveVariantField inst.handle (some inst.typeParameters))
    | _ => apiError
  if isMutable != expectedMutable then apiError
  pure offset

/-- `check_borrow_variant_completeness`: a variant-field borrow names exactly
the variants with the borrowed field's type at its offset, in order. -/
def checkBorrowVariantCompleteness (bc : Bytecode) (structName : String)
    (expectedTy : SignatureToken) : VResult Unit := do
  let vfh? ← match bc with
    | .immBorrowVariantField h | .mutBorrowVariantField h => do
      pure (some (← lookup w.m.variantFieldHandles h "variant_field_handle_at"))
    | .immBorrowVariantFieldGeneric i | .mutBorrowVariantFieldGeneric i => do
      let inst ← lookup w.m.variantFieldInstantiations i "variant_field_instantiation_at"
      pure (some (← lookup w.m.variantFieldHandles inst.handle "variant_field_handle_at"))
    | _ => pure none
  match vfh? with
  | none => pure ()
  | some vfh =>
    let variants ← match (← w.structDefinition).fieldInformation with
      | .declaredVariants variants => pure variants
      | .declared _ | .native => apiError
    let maps ← match w.ctx.enum? structName with
      | some maps => pure maps
      | none => apiError
    let first ← match vfh.variants with
      | first :: _ => pure first
      | [] => apiError
    let firstFields ← match variants[first]? with
      | some v => pure v.fields
      | none => apiError
    let derived ← match firstFields[vfh.field]? with
      | some f => pure f.signature
      | none => apiError
    if derived != expectedTy then apiError
    match maps.variantIndices.lookup (vfh.field, expectedTy) with
    | some expected => if vfh.variants != expected then apiError
    | none => apiError

/-- `check_borrow_pattern`. -/
def checkBorrowPattern (isMutable : Bool) (structName : String) (expectedOffset : Nat)
    (expectedTy : SignatureToken) : VResult Unit := do
  checkPattern w.code 1 fun bc => match bc with
    | .immBorrowField _ | .immBorrowFieldGeneric _ | .immBorrowVariantField _
    | .immBorrowVariantFieldGeneric _ => !isMutable
    | .mutBorrowField _ | .mutBorrowFieldGeneric _ | .mutBorrowVariantField _
    | .mutBorrowVariantFieldGeneric _ => isMutable
    | _ => false
  let op ← lookup w.code.code 1 "code[1]"
  if (← w.getBorrowFieldOffset op isMutable) != expectedOffset then apiError
  w.checkBorrowVariantCompleteness op structName expectedTy

/-- `validate`, by the attribute. -/
def validate (data : Parsed) : FunctionAttribute → VResult Unit
  | .pack => do
    w.validateSignatureMatchesFields w.handle.parameters none
    w.checkPackReturnType
    w.checkPackPattern
  | .packVariant attrIdx => do
    let vn ← match data.variantName with | some vn => pure vn | none => apiError
    w.validateSignatureMatchesFields w.handle.parameters (some vn)
    w.checkPackReturnType
    w.checkPackVariantPattern (← w.validateVariantIndex vn attrIdx)
  | .unpack => do
    w.checkStructParam false
    w.validateSignatureMatchesFields w.handle.returns none
    w.checkUnpackPattern
  | .unpackVariant attrIdx => do
    let vn ← match data.variantName with | some vn => pure vn | none => apiError
    w.checkStructParam false
    w.validateSignatureMatchesFields w.handle.returns (some vn)
    w.checkUnpackVariantPattern (← w.validateVariantIndex vn attrIdx)
  | .testVariant attrIdx => do
    let vn ← match data.variantName with | some vn => pure vn | none => apiError
    w.checkStructParam true
    w.checkTestVariantPattern (← w.validateVariantIndex vn attrIdx)
  | .borrowFieldImmutable attrOffset | .borrowFieldMutable attrOffset => do
    let nameOffset ← match data.offset with | some o => pure o | none => apiError
    if attrOffset != nameOffset then apiError
    w.checkBorrowParam data.isMutable
    let ty ← w.validateBorrowReturnType data.structName attrOffset data.typeOrder data.isMutable
    w.checkBorrowPattern data.isMutable data.structName attrOffset ty
  | .persistent | .moduleLock => apiError

end Wrapper

/-- `check_function`. -/
def checkFunction (m : CompiledModule) (ctx : Context) (fdef : FunctionDefinition) :
    VResult Unit := do
  let handle ← lookup m.functionHandles fdef.function "function_handle_at"
  let name ← lookup m.identifiers handle.name "identifier_at"
  let attr? ← structApiAttr handle.attributes
  let expectVariant := match attr? with
    | some (.packVariant _) | some (.unpackVariant _) | some (.testVariant _) => true
    | some .pack | some .unpack | some (.borrowFieldImmutable _)
    | some (.borrowFieldMutable _) | some .persistent | some .moduleLock | none => false
  match parseName name ctx expectVariant, attr? with
  | none, none => pure ()
  | some _, none | none, some _ => apiError
  | some data, some attr =>
    if !data.matchesAttr attr then apiError
    let sh ← lookup m.structHandles data.structHandle "struct_handle_at"
    if sh.abilities.key then apiError
    let numTypeParams := sh.typeParameters.length
    if handle.typeParameters.length != numTypeParams then apiError
    let code ← match fdef.code with
      | some code => pure code
      | none => apiError
    let w : Wrapper := { m, ctx, handle, code, structDef := data.structDef,
                         structHandle := data.structHandle, numTypeParams }
    w.checkSignatureCanonical
    w.validate data attr

/-- The context `verify_module_impl` builds, from version 10. -/
def contextFor (m : CompiledModule) : VResult (Option Context) :=
  if m.version ≥ 10 then some <$> context m else pure none

/-- The check for one function in `verify_module_impl`: none before
version 10. -/
def check (m : CompiledModule) (ctx? : Option Context) (fdef : FunctionDefinition) :
    VResult Unit :=
  match ctx? with
  | some ctx => checkFunction m ctx fdef
  | none => pure ()

end StructApi

end MoveBytecode.Model
