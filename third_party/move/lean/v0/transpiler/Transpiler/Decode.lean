-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import Transpiler.Xast

/-!
# XAST JSON decoder

Reads the wire form of the XAST exchange format (the serde serialization of
`move-model-exchange::ast`, see `third_party/move/move-model/exchange/src/ast.rs`)
into the `Transpiler.Xast` structures.

Wire conventions: structs are objects with snake_case field names; enums are
externally tagged with snake_case variant names — a unit variant is a bare
string, a newtype variant a single-key object holding the payload, a struct
variant a single-key object holding the named fields.  `Exp` and `Pattern`
are the exception: their node enum is flattened into the node object with an
internal `"kind"` tag beside `"ty"` and `"loc"`.  Numbers travel as decimal
strings, addresses as `0x`-hex strings, `Option` as `null` or the value.

Every decoder lives in `Except String`; errors name the field or tag that
failed, prefixed by the decoders the value was nested in.
-/

namespace Transpiler.Decode

open Lean (Json)
open Transpiler.Xast

/-- The interned tables of an export unit: types, locations, module references,
and qualified names are referenced by index from every node (see the schema's
*Interned tables* convention); the decoder resolves the indices, so the
decoded structures hold the logical tree. -/
structure Tables where
  locs : Array Loc := #[]
  types : Array Ty := #[]
  modules : Array ModuleRef := #[]
  names : Array QualifiedName := #[]

/-- The decoding monad: the tables of the document, over `Except String`. -/
abbrev Dec := ReaderT Tables (Except String)

def fail {α : Type} (msg : String) : Dec α := throw msg

/-- Prefixes an error with the decoder it arose in. -/
def ctx {α : Type} (what : String) (d : Dec α) : Dec α := fun tables =>
  match d tables with
  | .ok a => pure a
  | .error e => .error s!"{what}: {e}"

-- -------------------------------------------------------------------------------------------------
-- Helpers

def field (j : Json) (name : String) : Dec Json :=
  match j.getObjVal? name with
  | .ok v => pure v
  | .error _ => fail s!"missing field `{name}`"

/-- A field that may be absent or `null`. -/
def optField (j : Json) (name : String) : Dec (Option Json) :=
  match j.getObjVal? name with
  | .ok .null => pure none
  | .ok v => pure (some v)
  | .error _ => pure none

def str (j : Json) : Dec String :=
  match j with
  | .str s => pure s
  | _ => fail "expected a string"

def nat (j : Json) : Dec Nat :=
  match j.getNat? with
  | .ok n => pure n
  | .error _ => fail "expected a natural number"

def bool (j : Json) : Dec Bool :=
  match j with
  | .bool b => pure b
  | _ => fail "expected a boolean"

def arr (j : Json) : Dec (Array Json) :=
  match j with
  | .arr a => pure a
  | _ => fail "expected an array"

def list {α : Type} (j : Json) (f : Json → Dec α) : Dec (List α) := do
  let a ← arr j
  a.toList.mapM f

/-- `null` or a value. -/
def opt {α : Type} (j : Json) (f : Json → Dec α) : Dec (Option α) :=
  match j with
  | .null => pure none
  | _ => some <$> f j

def strField (j : Json) (name : String) : Dec String := ctx name (field j name >>= str)
def natField (j : Json) (name : String) : Dec Nat := ctx name (field j name >>= nat)
def boolField (j : Json) (name : String) : Dec Bool := ctx name (field j name >>= bool)
def listField {α : Type} (j : Json) (name : String) (f : Json → Dec α) : Dec (List α) :=
  ctx name (field j name >>= fun v => list v f)
def optFieldWith {α : Type} (j : Json) (name : String) (f : Json → Dec α) : Dec (Option α) :=
  ctx name do
    match ← optField j name with
    | none => pure none
    | some v => some <$> f v
def fieldWith {α : Type} (j : Json) (name : String) (f : Json → Dec α) : Dec α :=
  ctx name (field j name >>= f)

/-- The variant tag and payload of an externally tagged enum value: a bare
string (unit variant, payload `null`) or a single-key object. -/
def tagged (j : Json) : Dec (String × Json) :=
  match j with
  | .str s => pure (s, .null)
  | .obj kvs =>
    match kvs.toList with
    | [(k, v)] => pure (k, v)
    | [] => fail "expected a tagged enum value, found an empty object"
    | _ => fail "expected a single-key tagged enum object"
  | _ => fail "expected a tagged enum value (string or single-key object)"

def intOfString (s : String) : Dec Int :=
  match s.toInt? with
  | some i => pure i
  | none => fail s!"expected a decimal integer string, found `{s}`"

-- -------------------------------------------------------------------------------------------------
-- Leaves

/-- A table entry of `locs`. -/
def decodeLocEntry (j : Json) : Dec Loc := ctx "loc" do
  return { file := ← natField j "file", start := ← natField j "start", stop := ← natField j "end" }

/-- A location: an index into the `locs` table. -/
def decodeLoc (j : Json) : Dec Loc := ctx "loc" do
  let i ← nat j
  match (← read).locs[i]? with
  | some l => pure l
  | none => fail s!"location index {i} out of range"

/-- A table entry of `modules`. -/
def decodeModuleRefEntry (j : Json) : Dec ModuleRef := ctx "module ref" do
  return {
    address := ← strField j "address"
    addressAlias := ← optFieldWith j "address_alias" str
    name := ← strField j "name"
  }

/-- A module reference: an index into the `modules` table. -/
def decodeModuleRef (j : Json) : Dec ModuleRef := ctx "module ref" do
  let i ← nat j
  match (← read).modules[i]? with
  | some m => pure m
  | none => fail s!"module index {i} out of range"

/-- A table entry of `names` (its module is an index into `modules`). -/
def decodeQualifiedNameEntry (j : Json) : Dec QualifiedName := ctx "qualified name" do
  return { module := ← fieldWith j "module" decodeModuleRef, name := ← strField j "name" }

/-- A qualified name: an index into the `names` table. -/
def decodeQualifiedName (j : Json) : Dec QualifiedName := ctx "qualified name" do
  let i ← nat j
  match (← read).names[i]? with
  | some n => pure n
  | none => fail s!"name index {i} out of range"

/-- A type: an index into the `types` table. -/
def decodeTy (j : Json) : Dec Ty := ctx "type" do
  let i ← nat j
  match (← read).types[i]? with
  | some t => pure t
  | none => fail s!"type index {i} out of range"

def decodeNamedAddress (j : Json) : Dec NamedAddress := ctx "named address" do
  return { name := ← strField j "name", address := ← strField j "address" }

def decodeComment (j : Json) : Dec Comment := ctx "comment" do
  return {
    loc := ← fieldWith j "loc" decodeLoc
    text := ← strField j "text"
    ownLine := ← boolField j "own_line"
  }

def decodeAbility (j : Json) : Dec Ability := ctx "ability" do
  match ← str j with
  | "copy" => pure .copy
  | "drop" => pure .drop
  | "store" => pure .store
  | "key" => pure .key
  | s => fail s!"unknown ability `{s}`"

def decodeTypeParam (j : Json) : Dec TypeParam := ctx "type param" do
  return {
    name := ← strField j "name"
    abilities := ← listField j "abilities" decodeAbility
    isPhantom := ← boolField j "is_phantom"
  }

/-- A table entry of `types`; its components are indices into the prefix of
the table already resolved. -/
partial def decodeTyEntry (j : Json) : Dec Ty := ctx "type entry" do
  let (tag, payload) ← tagged j
  match tag with
  | "bool" => pure .bool
  | "u8" => pure .u8
  | "u16" => pure .u16
  | "u32" => pure .u32
  | "u64" => pure .u64
  | "u128" => pure .u128
  | "u256" => pure .u256
  | "i8" => pure .i8
  | "i16" => pure .i16
  | "i32" => pure .i32
  | "i64" => pure .i64
  | "i128" => pure .i128
  | "i256" => pure .i256
  | "address" => pure .address
  | "signer" => pure .signer
  | "num" => pure .num
  | "range" => pure .range
  | "event_store" => pure .eventStore
  | "tuple" => .tuple <$> list payload decodeTy
  | "vector" => .vector <$> decodeTy payload
  | "struct" =>
    return .struct (← fieldWith payload "name" decodeQualifiedName)
      (← listField payload "args" decodeTy)
  | "function" =>
    return .function (← fieldWith payload "args" decodeTy)
      (← fieldWith payload "result" decodeTy)
      (← listField payload "abilities" decodeAbility)
  | "type_param" => .typeParam <$> nat payload
  | "reference" =>
    return .reference (← boolField payload "mutable") (← fieldWith payload "ty" decodeTy)
  | "type_domain" => .typeDomain <$> decodeTy payload
  | "resource_domain" =>
    return .resourceDomain (← fieldWith payload "name" decodeQualifiedName)
      (← optFieldWith payload "args" fun v => list v decodeTy)
  | "state_domain" => pure .stateDomain
  | s => fail s!"unknown type tag `{s}`"

partial def decodeValue (j : Json) : Dec Value := ctx "value" do
  let (tag, payload) ← tagged j
  match tag with
  | "address" => .address <$> str payload
  | "number" => .number <$> (str payload >>= intOfString)
  | "bool" => .bool <$> bool payload
  | "vector" => .vector <$> list payload decodeValue
  | "tuple" => .tuple <$> list payload decodeValue
  | s => fail s!"unknown value tag `{s}`"

def decodePragmaValue (j : Json) : Dec PragmaValue := ctx "pragma value" do
  let (tag, payload) ← tagged j
  match tag with
  | "value" => .value <$> decodeValue payload
  | "name" => .name <$> str payload
  | "qualified_name" => .qualifiedName <$> str payload
  | s => fail s!"unknown pragma value tag `{s}`"

def decodePragma (j : Json) : Dec Pragma := ctx "pragma" do
  return { name := ← strField j "name", value := ← fieldWith j "value" decodePragmaValue }

def decodeAttributeValue (j : Json) : Dec AttributeValue := ctx "attribute value" do
  let (tag, payload) ← tagged j
  match tag with
  | "value" => .value <$> decodeValue payload
  | "name" =>
    return .name (← optFieldWith payload "module" decodeModuleRef) (← strField payload "name")
  | s => fail s!"unknown attribute value tag `{s}`"

partial def decodeAttribute (j : Json) : Dec Attribute := ctx "attribute" do
  let (tag, payload) ← tagged j
  match tag with
  | "apply" =>
    return .apply (← strField payload "name") (← listField payload "args" decodeAttribute)
  | "assign" =>
    return .assign (← strField payload "name") (← fieldWith payload "value" decodeAttributeValue)
  | s => fail s!"unknown attribute tag `{s}`"

def decodeField (j : Json) : Dec Field := ctx "field" do
  return { name := ← strField j "name", doc := ← strField j "doc", ty := ← fieldWith j "ty" decodeTy }

def decodeVariant (j : Json) : Dec Variant := ctx "variant" do
  return {
    name := ← strField j "name"
    loc := ← fieldWith j "loc" decodeLoc
    fields := ← listField j "fields" decodeField
  }

def decodeVisibility (j : Json) : Dec Visibility := ctx "visibility" do
  match ← str j with
  | "private" => pure .private
  | "public" => pure .public
  | "friend" => pure .friend
  | "package" => pure .package
  | s => fail s!"unknown visibility `{s}`"

def decodeFunctionKind (j : Json) : Dec FunctionKind := ctx "function kind" do
  match ← str j with
  | "regular" => pure .regular
  | "inline_retained" => pure .inlineRetained
  | "native" => pure .native
  | s => fail s!"unknown function kind `{s}`"

def decodeParam (j : Json) : Dec Param := ctx "param" do
  return { name := ← strField j "name", ty := ← fieldWith j "ty" decodeTy }

def decodeSurfaceSyntax (j : Json) : Dec SurfaceSyntax := ctx "surface syntax" do
  match ← str j with
  | "receiver_call" => pure .receiverCall
  | "index_notation" => pure .indexNotation
  | s => fail s!"unknown surface syntax `{s}`"

def decodeQuantKind (j : Json) : Dec QuantKind := ctx "quantifier kind" do
  match ← str j with
  | "forall" => pure .forall
  | "exists" => pure .exists
  | "choose" => pure .choose
  | "choose_min" => pure .chooseMin
  | s => fail s!"unknown quantifier kind `{s}`"

def decodeRefKind (j : Json) : Dec RefKind := ctx "reference kind" do
  match ← str j with
  | "immutable" => pure .immutable
  | "mutable" => pure .mutable
  | s => fail s!"unknown reference kind `{s}`"

def decodeAbortKind (j : Json) : Dec AbortKind := ctx "abort kind" do
  match ← str j with
  | "code" => pure .code
  | "message" => pure .message
  | s => fail s!"unknown abort kind `{s}`"

def decodeTraceKind (j : Json) : Dec TraceKind := ctx "trace kind" do
  match ← str j with
  | "user" => pure .user
  | "auto" => pure .auto
  | "sub_auto" => pure .subAuto
  | s => fail s!"unknown trace kind `{s}`"

def decodeMemoryRange (j : Json) : Dec MemoryRange := ctx "memory range" do
  return { pre := ← optFieldWith j "pre" nat, post := ← optFieldWith j "post" nat }

def decodeBehaviorKind (j : Json) : Dec BehaviorKind := ctx "behavior kind" do
  let (tag, payload) ← tagged j
  match tag with
  | "requires_of" => pure .requiresOf
  | "aborts_of" => pure .abortsOf
  | "ensures_of" => pure .ensuresOf
  | "result_of" => pure .resultOf
  | "unchanged_of" => pure .unchangedOf
  | "folds_of" => pure .foldsOf
  | "write_of" => .writeOf <$> nat payload
  | s => fail s!"unknown behavior kind `{s}`"

/-- A `null` or natural number payload (an optional memory label). -/
def optNat (j : Json) : Dec (Option Nat) := opt j nat

def decodeOperation (j : Json) : Dec Operation := ctx "operation" do
  let (tag, payload) ← tagged j
  match tag with
  | "move_function" => .moveFunction <$> decodeQualifiedName payload
  | "pack" =>
    return .pack (← fieldWith payload "name" decodeQualifiedName)
      (← optFieldWith payload "variant" str)
  | "tuple" => pure .tuple
  | "select" =>
    return .select (← fieldWith payload "name" decodeQualifiedName) (← strField payload "field")
  | "select_variants" =>
    return .selectVariants (← fieldWith payload "name" decodeQualifiedName)
      (← listField payload "fields" str)
  | "test_variants" =>
    return .testVariants (← fieldWith payload "name" decodeQualifiedName)
      (← listField payload "variants" str)
  | "spec_function" =>
    return .specFunction (← fieldWith payload "name" decodeQualifiedName)
      (← fieldWith payload "range" decodeMemoryRange)
  | "behavior" =>
    return .behavior (← fieldWith payload "kind" decodeBehaviorKind)
      (← fieldWith payload "range" decodeMemoryRange)
  | "update_field" =>
    return .updateField (← fieldWith payload "name" decodeQualifiedName)
      (← strField payload "field")
  | "result" => .result <$> nat payload
  | "index" => pure .index
  | "slice" => pure .slice
  | "range" => pure .range
  | "implies" => pure .implies
  | "iff" => pure .iff
  | "identical" => pure .identical
  | "add" => pure .add
  | "sub" => pure .sub
  | "mul" => pure .mul
  | "mod" => pure .mod
  | "div" => pure .div
  | "bit_or" => pure .bitOr
  | "bit_and" => pure .bitAnd
  | "xor" => pure .xor
  | "shl" => pure .shl
  | "shr" => pure .shr
  | "and" => pure .and
  | "or" => pure .or
  | "eq" => pure .eq
  | "neq" => pure .neq
  | "lt" => pure .lt
  | "gt" => pure .gt
  | "le" => pure .le
  | "ge" => pure .ge
  | "copy" => pure .copy
  | "move" => pure .move
  | "not" => pure .not
  | "cast" => pure .cast
  | "negate" => pure .negate
  | "exists" => .exists <$> optNat payload
  | "borrow_global" => .borrowGlobal <$> decodeRefKind payload
  | "borrow" => .borrow <$> decodeRefKind payload
  | "deref" => pure .deref
  | "move_to" => pure .moveTo
  | "move_from" => pure .moveFrom
  | "freeze" => .freeze <$> bool payload
  | "abort" => .abort <$> decodeAbortKind payload
  | "vector" => pure .vector
  | "len" => pure .len
  | "type_value" => pure .typeValue
  | "type_domain" => pure .typeDomain
  | "resource_domain" => pure .resourceDomain
  | "state_domain" => pure .stateDomain
  | "global" => .global <$> optNat payload
  | "can_modify" => pure .canModify
  | "old" => pure .old
  | "save_state_anchor" => .saveStateAnchor <$> nat payload
  | "with_state_anchor" => .withStateAnchor <$> nat payload
  | "folds_capture_anchor" => .foldsCaptureAnchor <$> nat payload
  | "inline_call_summary" => pure .inlineCallSummary
  | "trace" => .trace <$> decodeTraceKind payload
  | "spec_publish" => .specPublish <$> decodeMemoryRange payload
  | "spec_remove" => .specRemove <$> decodeMemoryRange payload
  | "spec_update" => .specUpdate <$> decodeMemoryRange payload
  | "empty_vec" => pure .emptyVec
  | "single_vec" => pure .singleVec
  | "update_vec" => pure .updateVec
  | "concat_vec" => pure .concatVec
  | "index_of_vec" => pure .indexOfVec
  | "contains_vec" => pure .containsVec
  | "in_range_range" => pure .inRangeRange
  | "in_range_vec" => pure .inRangeVec
  | "range_vec" => pure .rangeVec
  | "max_u8" => pure .maxU8
  | "max_u16" => pure .maxU16
  | "max_u32" => pure .maxU32
  | "max_u64" => pure .maxU64
  | "max_u128" => pure .maxU128
  | "max_u256" => pure .maxU256
  | "bv2_int" => pure .bv2Int
  | "int2_bv" => pure .int2Bv
  | "abort_flag" => pure .abortFlag
  | "abort_code" => pure .abortCode
  | "well_formed" => pure .wellFormed
  | "box_value" => pure .boxValue
  | "unbox_value" => pure .unboxValue
  | "empty_event_store" => pure .emptyEventStore
  | "extend_event_store" => pure .extendEventStore
  | "event_store_includes" => pure .eventStoreIncludes
  | "event_store_included_in" => pure .eventStoreIncludedIn
  | "no_op" => pure .noOp
  | s => fail s!"unknown operation tag `{s}`"

def decodeConditionKind (j : Json) : Dec ConditionKind := ctx "condition kind" do
  let (tag, payload) ← tagged j
  match tag with
  | "let_post" => .letPost <$> strField payload "name"
  | "let_pre" => .letPre <$> strField payload "name"
  | "assert" => pure .assert
  | "assume" => pure .assume
  | "decreases" => pure .decreases
  | "aborts_if" => pure .abortsIf
  | "aborts_with" => pure .abortsWith
  | "succeeds_if" => pure .succeedsIf
  | "emits" => pure .emits
  | "ensures" => pure .ensures
  | "requires" => pure .requires
  | "struct_invariant" => pure .structInvariant
  | "function_invariant" => pure .functionInvariant
  | "loop_invariant" => pure .loopInvariant
  | "global_invariant" => .globalInvariant <$> listField payload "type_params" str
  | "global_invariant_update" => .globalInvariantUpdate <$> listField payload "type_params" str
  | "schema_invariant" => pure .schemaInvariant
  | "axiom" => .axiom <$> listField payload "type_params" str
  | "update" => pure .update
  | s => fail s!"unknown condition kind tag `{s}`"

def decodeInvariantKind (j : Json) : Dec InvariantKind := ctx "invariant kind" do
  match ← str j with
  | "global" => pure .global
  | "global_update" => pure .globalUpdate
  | "axiom" => pure .axiom
  | s => fail s!"unknown invariant kind `{s}`"

-- -------------------------------------------------------------------------------------------------
-- Expressions, patterns, specifications (mutually recursive)

mutual
  partial def decodeExp (j : Json) : Dec Exp := ctx "exp" do
    let ty ← fieldWith j "ty" decodeTy
    let loc ← fieldWith j "loc" decodeLoc
    let kind ← strField j "kind"
    let node ← ctx s!"`{kind}` node" do
      match kind with
      | "value" => return .value (← fieldWith j "value" decodeValue) (← optFieldWith j "constant" str)
      | "local" => .local <$> strField j "name"
      | "param" => .param <$> natField j "index"
      | "call" =>
        return .call (← fieldWith j "op" decodeOperation) (← listField j "inst" decodeTy)
          (← listField j "args" decodeExp) (← optFieldWith j "surface" decodeSurfaceSyntax)
      | "invoke" =>
        return .invoke (← fieldWith j "function" decodeExp) (← listField j "args" decodeExp)
      | "block" =>
        return .block (← fieldWith j "pattern" decodePattern)
          (← optFieldWith j "binding" decodeExp) (← fieldWith j "body" decodeExp)
      | "if" =>
        return .ite (← fieldWith j "cond" decodeExp) (← fieldWith j "then_branch" decodeExp)
          (← fieldWith j "else_branch" decodeExp)
      | "match" =>
        return .match (← fieldWith j "scrutinee" decodeExp) (← listField j "arms" decodeMatchArm)
      | "sequence" => .sequence <$> listField j "exps" decodeExp
      | "loop" => .loop <$> fieldWith j "body" decodeExp
      | "loop_cont" =>
        return .loopCont (← natField j "nest") (← boolField j "is_continue")
      | "return" => .return <$> fieldWith j "value" decodeExp
      | "assign" =>
        return .assign (← fieldWith j "pattern" decodePattern) (← fieldWith j "value" decodeExp)
      | "mutate" =>
        return .mutate (← fieldWith j "target" decodeExp) (← fieldWith j "value" decodeExp)
      | "spec_block" => .specBlock <$> fieldWith j "spec" decodeSpec
      | "quant" =>
        return .quant (← fieldWith j "quant" decodeQuantKind)
          (← listField j "ranges" decodeQuantRange)
          (← listField j "triggers" fun v => list v decodeExp)
          (← optFieldWith j "condition" decodeExp) (← fieldWith j "body" decodeExp)
      | s => fail s!"unknown expression kind `{s}`"
    return .mk ty loc node

  partial def decodeMatchArm (j : Json) : Dec MatchArm := ctx "match arm" do
    return .mk (← fieldWith j "loc" decodeLoc) (← fieldWith j "pattern" decodePattern)
      (← optFieldWith j "guard" decodeExp) (← fieldWith j "body" decodeExp)

  partial def decodeQuantRange (j : Json) : Dec QuantRange := ctx "quantifier range" do
    return .mk (← fieldWith j "pattern" decodePattern) (← fieldWith j "domain" decodeExp)

  partial def decodePattern (j : Json) : Dec Pattern := ctx "pattern" do
    let ty ← fieldWith j "ty" decodeTy
    let loc ← fieldWith j "loc" decodeLoc
    let kind ← strField j "kind"
    let node ← ctx s!"`{kind}` node" do
      match kind with
      | "var" => .var <$> strField j "name"
      | "wildcard" => pure .wildcard
      | "tuple" => .tuple <$> listField j "elements" decodePattern
      | "struct" =>
        return .struct (← fieldWith j "name" decodeQualifiedName) (← listField j "inst" decodeTy)
          (← optFieldWith j "variant" str) (← listField j "fields" decodePattern)
      | "literal" => .literal <$> fieldWith j "value" decodeValue
      | "range" =>
        return .range (← optFieldWith j "lower" decodeValue) (← optFieldWith j "upper" decodeValue)
          (← boolField j "inclusive")
      | s => fail s!"unknown pattern kind `{s}`"
    return .mk ty loc node

  partial def decodeSpec (j : Json) : Dec Spec := ctx "spec" do
    return .mk (← optFieldWith j "loc" decodeLoc) (← listField j "pragmas" decodePragma)
      (← listField j "conditions" decodeCondition) (← optFieldWith j "frame" decodeFrame)

  partial def decodeCondition (j : Json) : Dec Condition := ctx "condition" do
    return .mk (← fieldWith j "kind" decodeConditionKind) (← fieldWith j "loc" decodeLoc)
      (← listField j "properties" decodePragma) (← fieldWith j "exp" decodeExp)
      (← optFieldWith j "abort_code" decodeExp) (← listField j "additional_codes" decodeExp)
      (← optFieldWith j "emits_handle" decodeExp) (← optFieldWith j "emits_condition" decodeExp)
      (← optFieldWith j "update_target" decodeExp)

  partial def decodeFrame (j : Json) : Dec Frame := ctx "frame" do
    return .mk (← listField j "modifies" decodeExp) (← listField j "reads" decodeTy)
      (← boolField j "modifies_all") (← boolField j "reads_all")
end

-- -------------------------------------------------------------------------------------------------
-- Declarations

def decodeConstant (j : Json) : Dec Constant := ctx "constant" do
  return {
    name := ← strField j "name"
    doc := ← strField j "doc"
    loc := ← fieldWith j "loc" decodeLoc
    ty := ← fieldWith j "ty" decodeTy
    value := ← fieldWith j "value" decodeValue
  }

def decodeIntrinsicBinding (j : Json) : Dec IntrinsicBinding := ctx "intrinsic binding" do
  return {
    role := ← strField j "role"
    target := ← fieldWith j "target" decodeQualifiedName
  }

def decodeIntrinsic (j : Json) : Dec Intrinsic := ctx "intrinsic" do
  return {
    name := ← strField j "name"
    moveFunctions := ← listField j "move_functions" decodeIntrinsicBinding
    specFunctions := ← listField j "spec_functions" decodeIntrinsicBinding
  }

def decodeStruct (j : Json) : Dec Struct := ctx "struct" do
  return {
    name := ← strField j "name"
    doc := ← strField j "doc"
    loc := ← fieldWith j "loc" decodeLoc
    abilities := ← listField j "abilities" decodeAbility
    typeParams := ← listField j "type_params" decodeTypeParam
    attributes := ← listField j "attributes" decodeAttribute
    isNative := ← boolField j "is_native"
    fields := ← listField j "fields" decodeField
    variants := ← optFieldWith j "variants" fun v => list v decodeVariant
    spec := ← fieldWith j "spec" decodeSpec
    intrinsic := ← optFieldWith j "intrinsic" decodeIntrinsic
  }

def decodeFunction (j : Json) : Dec Function := ctx "function" do
  return {
    name := ← strField j "name"
    doc := ← strField j "doc"
    loc := ← fieldWith j "loc" decodeLoc
    visibility := ← fieldWith j "visibility" decodeVisibility
    isEntry := ← boolField j "is_entry"
    kind := ← fieldWith j "kind" decodeFunctionKind
    isReceiver := ← boolField j "is_receiver"
    attributes := ← listField j "attributes" decodeAttribute
    typeParams := ← listField j "type_params" decodeTypeParam
    params := ← listField j "params" decodeParam
    result := ← fieldWith j "result" decodeTy
    pragmas := ← listField j "pragmas" decodePragma
    spec := ← fieldWith j "spec" decodeSpec
    body := ← optFieldWith j "body" decodeExp
  }

def decodeSpecFun (j : Json) : Dec SpecFun := ctx "spec fun" do
  return {
    name := ← strField j "name"
    doc := ← strField j "doc"
    loc := ← fieldWith j "loc" decodeLoc
    typeParams := ← listField j "type_params" decodeTypeParam
    params := ← listField j "params" decodeParam
    result := ← fieldWith j "result" decodeTy
    uninterpreted := ← boolField j "uninterpreted"
    isNative := ← boolField j "is_native"
    isMoveFun := ← boolField j "is_move_fun"
    usesOld := ← boolField j "uses_old"
    body := ← optFieldWith j "body" decodeExp
    spec := ← fieldWith j "spec" decodeSpec
  }

def decodeSpecVar (j : Json) : Dec SpecVar := ctx "spec var" do
  return {
    name := ← strField j "name"
    loc := ← fieldWith j "loc" decodeLoc
    typeParams := ← listField j "type_params" decodeTypeParam
    ty := ← fieldWith j "ty" decodeTy
    init := ← optFieldWith j "init" decodeExp
  }

def decodeInvariant (j : Json) : Dec Invariant := ctx "invariant" do
  return {
    kind := ← fieldWith j "kind" decodeInvariantKind
    loc := ← fieldWith j "loc" decodeLoc
    typeParams := ← listField j "type_params" str
    properties := ← listField j "properties" decodePragma
    exp := ← fieldWith j "exp" decodeExp
  }

/-- Decodes the interned tables of a document, in dependency order: locations,
module references, qualified names (over modules), types (over names and
earlier types). -/
def decodeTables (j : Json) : Except String Tables := do
  let run {α : Type} (t : Tables) (d : Dec α) : Except String α := d t
  let empty : Tables := {}
  let locs ← run empty (listField j "locs" decodeLocEntry)
  let modules ← run empty (listField j "modules" decodeModuleRefEntry)
  let t1 : Tables := { locs := locs.toArray, modules := modules.toArray }
  let names ← run t1 (listField j "names" decodeQualifiedNameEntry)
  let t2 : Tables := { t1 with names := names.toArray }
  let typeEntries ← run empty (listField j "types" pure)
  let types ← typeEntries.foldlM (init := #[]) fun (acc : Array Ty) entry => do
    let ty ← run { t2 with types := acc } (decodeTyEntry entry)
    pure (acc.push ty)
  pure { t2 with types }

/-- The module over its tables. -/
def decodeSkipped (j : Json) : Dec Skipped := ctx "skipped declaration" do
  return { name := ← strField j "name", reason := ← strField j "reason" }

def decodeModuleBody (j : Json) : Dec Module := ctx "module" do
  return {
    address := ← strField j "address"
    addressAlias := ← optFieldWith j "address_alias" str
    name := ← strField j "name"
    doc := ← strField j "doc"
    loc := ← fieldWith j "loc" decodeLoc
    namedAddresses := ← listField j "named_addresses" decodeNamedAddress
    friends := ← listField j "friends" decodeModuleRef
    pragmas := ← listField j "pragmas" decodePragma
    constants := ← listField j "constants" decodeConstant
    structs := ← listField j "structs" decodeStruct
    functions := ← listField j "functions" decodeFunction
    specFuns := ← listField j "spec_funs" decodeSpecFun
    specVars := ← listField j "spec_vars" decodeSpecVar
    invariants := ← listField j "invariants" decodeInvariant
    skipped := ← (if (j.getObjVal? "skipped").toOption.isSome then listField j "skipped" decodeSkipped
      else pure [])
    sources := ← listField j "sources" str
    comments := ← listField j "comments" decodeComment
  }

/-- Decodes a module document (after the schema and version have been
checked, see `parseModule`): the tables first, then the module over them. -/
def decodeModule (j : Json) : Except String Module := do
  let tables ← decodeTables j
  (decodeModuleBody j) tables


-- -------------------------------------------------------------------------------------------------
-- Entry points

/-- Parses an XAST document, checking its schema identifier and version. -/
def parseModule (text : String) : Except String Module := do
  let j ← match Json.parse text with
    | .ok j => pure j
    | .error e => throw s!"invalid JSON: {e}"
  let schema ← (strField j "schema") {}
  unless schema == Xast.schema do
    throw s!"unexpected XAST schema `{schema}` (expected `{Xast.schema}`)"
  let version ← (natField j "version") {}
  unless version == Xast.version do
    throw s!"unsupported XAST version {version} (this transpiler reads version {Xast.version})"
  decodeModule j

/-- Reads and parses an XAST document from a file. -/
def readModule (path : System.FilePath) : IO Module := do
  let text ← IO.FS.readFile path
  match parseModule text with
  | .ok m => pure m
  | .error e => throw <| IO.userError s!"{path}: {e}"

end Transpiler.Decode
