-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import LeanerIR
import Transpiler.Xast

/-!
# Move-profile payload codec

Profile payloads contain only the leaf data which is not represented by LIR
IDs or tree edges.  They use compact JSON arrays of strings, so arbitrary
source names round-trip without separator or Unicode ambiguities.
-/

namespace Transpiler.LIR.Codec

open Lean (Json)
open Transpiler.Xast

/-- Project a known shared pure operation out of the transitional Move XAST. -/
def primitiveOperation? : Xast.Operation → Option LeanerIR.PrimitiveOperation
  | .tuple => some .tuple
  | .vector => some .vector
  | .add => some (.checkedAdd .abort)
  | .sub => some (.checkedSubtract .abort)
  | .mul => some (.checkedMultiply .abort)
  | .mod => some (.checkedModulo .abort)
  | .div => some (.checkedDivide .abort)
  | .bitOr => some .bitwiseOr
  | .bitAnd => some .bitwiseAnd
  | .xor => some .bitwiseXor
  | .shl => some (.checkedShiftLeft .abort)
  | .shr => some (.checkedShiftRight .abort)
  | .and => some .logicalAnd
  | .or => some .logicalOr
  | .eq => some .equal
  | .neq => some .notEqual
  | .lt => some .less
  | .gt => some .greater
  | .le => some .lessEqual
  | .ge => some .greaterEqual
  | .not => some .logicalNot
  | .negate => some (.checkedNegate .abort)
  | .copy => some .copyValue
  | .move => some .moveValue
  | .cast => some (.checkedCast .abort)
  | .range => some .range
  | .implies => some .implies
  | .iff => some .equivalent
  | .identical => some .identical
  | _ => none

/-- Recover the transitional Move XAST spelling of a shared pure primitive. -/
def primitiveXastOperation : LeanerIR.PrimitiveOperation → Except String Xast.Operation
  | .tuple => pure .tuple
  | .vector => pure .vector
  | .length => pure .len
  | .index => pure .index
  | .slice => pure .slice
  | .checkedAdd .abort => pure .add
  | .checkedSubtract .abort => pure .sub
  | .checkedMultiply .abort => pure .mul
  | .checkedModulo .abort => pure .mod
  | .checkedDivide .abort => pure .div
  | .bitwiseOr => pure .bitOr
  | .bitwiseAnd => pure .bitAnd
  | .bitwiseXor => pure .xor
  | .checkedShiftLeft .abort => pure .shl
  | .checkedShiftRight .abort => pure .shr
  | .logicalAnd => pure .and
  | .logicalOr => pure .or
  | .equal => pure .eq
  | .notEqual => pure .neq
  | .less => pure .lt
  | .greater => pure .gt
  | .lessEqual => pure .le
  | .greaterEqual => pure .ge
  | .logicalNot => pure .not
  | .checkedNegate .abort => pure .negate
  | .copyValue => pure .copy
  | .moveValue => pure .move
  | .checkedCast .abort => pure .cast
  | .range => pure .range
  | .implies => pure .implies
  | .equivalent => pure .iff
  | .identical => pure .identical
  | operation => .error s!"primitive {repr operation} cannot be projected as Move XAST"

private def lirMemoryRange (range : Xast.MemoryRange) : LeanerIR.MemoryRange :=
  { pre := range.pre, post := range.post }

private def xastMemoryRange (range : LeanerIR.MemoryRange) : Xast.MemoryRange :=
  { pre := range.pre, post := range.post }

private def lirBehaviorKind : Xast.BehaviorKind → LeanerIR.BehaviorKind
  | .requiresOf => .requiresOf
  | .abortsOf => .abortsOf
  | .ensuresOf => .ensuresOf
  | .resultOf => .resultOf
  | .unchangedOf => .unchangedOf
  | .foldsOf => .foldsOf
  | .writeOf index => .writeOf index

private def xastBehaviorKind : LeanerIR.BehaviorKind → Xast.BehaviorKind
  | .requiresOf => .requiresOf
  | .abortsOf => .abortsOf
  | .ensuresOf => .ensuresOf
  | .resultOf => .resultOf
  | .unchangedOf => .unchangedOf
  | .foldsOf => .foldsOf
  | .writeOf index => .writeOf index

def specOperation? : Xast.Operation → Option LeanerIR.SpecOperation
  | .behavior kind range => some (.behavior (lirBehaviorKind kind) (lirMemoryRange range))
  | .result index => some (.result index)
  | .typeValue => some .typeValue
  | .typeDomain => some .typeDomain
  | .resourceDomain => some .resourceDomain
  | .stateDomain => some .stateDomain
  | .global label => some (.global label)
  | .canModify => some .canModify
  | .old => some .old
  | .saveStateAnchor label => some (.saveStateAnchor label)
  | .withStateAnchor label => some (.withStateAnchor label)
  | .foldsCaptureAnchor label => some (.foldsCaptureAnchor label)
  | .inlineCallSummary => some .inlineCallSummary
  | .trace .user => some (.trace .user)
  | .trace .auto => some (.trace .automatic)
  | .trace .subAuto => some (.trace .subAutomatic)
  | .specPublish range => some (.publish (lirMemoryRange range))
  | .specRemove range => some (.remove (lirMemoryRange range))
  | .specUpdate range => some (.update (lirMemoryRange range))
  | .emptyVec => some .emptyVector
  | .singleVec => some .singletonVector
  | .updateVec => some .updateVector
  | .concatVec => some .concatVector
  | .indexOfVec => some .indexOfVector
  | .containsVec => some .containsVector
  | .len => some .lengthVector
  | .index => some .indexVector
  | .slice => some .sliceVector
  | .inRangeRange => some .inRange
  | .inRangeVec => some .inVectorRange
  | .rangeVec => some .vectorRange
  | .maxU8 => some (.maxValue 8)
  | .maxU16 => some (.maxValue 16)
  | .maxU32 => some (.maxValue 32)
  | .maxU64 => some (.maxValue 64)
  | .maxU128 => some (.maxValue 128)
  | .maxU256 => some (.maxValue 256)
  | .bv2Int => some .bitVectorToInt
  | .int2Bv => some .intToBitVector
  | .abortFlag => some .abortFlag
  | .abortCode => some .abortCode
  | .wellFormed => some .wellFormed
  | .boxValue => some .boxValue
  | .unboxValue => some .unboxValue
  | .emptyEventStore => some .emptyEventStore
  | .extendEventStore => some .extendEventStore
  | .eventStoreIncludes => some .eventStoreIncludes
  | .eventStoreIncludedIn => some .eventStoreIncludedIn
  | .noOp => some .noOp
  | _ => none

def specXastOperation : LeanerIR.SpecOperation → Except String Xast.Operation
  | .functionCall .. => .error "specification function target requires namespace decoding"
  | .behavior kind range => pure (.behavior (xastBehaviorKind kind) (xastMemoryRange range))
  | .result index => pure (.result index)
  | .typeValue => pure .typeValue
  | .typeDomain => pure .typeDomain
  | .resourceDomain => pure .resourceDomain
  | .stateDomain => pure .stateDomain
  | .global label => pure (.global label)
  | .canModify => pure .canModify
  | .old => pure .old
  | .saveStateAnchor label => pure (.saveStateAnchor label)
  | .withStateAnchor label => pure (.withStateAnchor label)
  | .foldsCaptureAnchor label => pure (.foldsCaptureAnchor label)
  | .inlineCallSummary => pure .inlineCallSummary
  | .trace .user => pure (.trace .user)
  | .trace .automatic => pure (.trace .auto)
  | .trace .subAutomatic => pure (.trace .subAuto)
  | .publish range => pure (.specPublish (xastMemoryRange range))
  | .remove range => pure (.specRemove (xastMemoryRange range))
  | .update range => pure (.specUpdate (xastMemoryRange range))
  | .emptyVector => pure .emptyVec
  | .singletonVector => pure .singleVec
  | .updateVector => pure .updateVec
  | .concatVector => pure .concatVec
  | .indexOfVector => pure .indexOfVec
  | .containsVector => pure .containsVec
  | .lengthVector => pure .len
  | .indexVector => pure .index
  | .sliceVector => pure .slice
  | .inRange => pure .inRangeRange
  | .inVectorRange => pure .inRangeVec
  | .vectorRange => pure .rangeVec
  | .maxValue 8 => pure .maxU8
  | .maxValue 16 => pure .maxU16
  | .maxValue 32 => pure .maxU32
  | .maxValue 64 => pure .maxU64
  | .maxValue 128 => pure .maxU128
  | .maxValue 256 => pure .maxU256
  | .maxValue width => .error s!"unsupported maximum integer width {width}"
  | .bitVectorToInt => pure .bv2Int
  | .intToBitVector => pure .int2Bv
  | .abortFlag => pure .abortFlag
  | .abortCode => pure .abortCode
  | .wellFormed => pure .wellFormed
  | .boxValue => pure .boxValue
  | .unboxValue => pure .unboxValue
  | .emptyEventStore => pure .emptyEventStore
  | .extendEventStore => pure .extendEventStore
  | .eventStoreIncludes => pure .eventStoreIncludes
  | .eventStoreIncludedIn => pure .eventStoreIncludedIn
  | .noOp => pure .noOp

def pack (fields : Array String) : String :=
  (Json.arr (fields.map Json.str)).compress

def unpack (payload : String) : Except String (Array String) := do
  let json ← Json.parse payload
  match json with
  | .arr fields => fields.mapM fun
      | .str value => .ok value
      | _ => .error "profile payload field is not a string"
  | _ => .error "profile payload is not an array"

def encodeOption (encode : α → String) : Option α → String
  | none => pack #[]
  | some value => pack #[encode value]

def decodeOption (decode : String → Except String α) (payload : String) : Except String (Option α) := do
  match ← unpack payload with
  | #[] => .ok none
  | #[value] => some <$> decode value
  | _ => .error "optional payload has more than one element"

def decodeNat (value : String) : Except String Nat :=
  match value.toNat? with
  | some value => .ok value
  | none => .error s!"expected natural number, found `{value}`"

def decodeBool (value : String) : Except String Bool :=
  match value with
  | "true" => .ok true
  | "false" => .ok false
  | _ => .error s!"expected boolean, found `{value}`"

def encodeModuleRef (module : ModuleRef) : String :=
  pack #[module.address, encodeOption id module.addressAlias, module.name]

def decodeModuleRef (payload : String) : Except String ModuleRef := do
  match ← unpack payload with
  | #[address, alias, name] =>
      return { address, addressAlias := ← decodeOption pure alias, name }
  | _ => .error "module reference payload has the wrong arity"

def encodeNamedAddress (address : NamedAddress) : String :=
  pack #[address.name, address.address]

def decodeNamedAddress (payload : String) : Except String NamedAddress := do
  match ← unpack payload with
  | #[name, address] => return { name, address }
  | _ => .error "named-address payload has the wrong arity"

def encodeMemoryRange (range : MemoryRange) : String :=
  pack #[encodeOption toString range.pre, encodeOption toString range.post]

def decodeMemoryRange (payload : String) : Except String MemoryRange := do
  match ← unpack payload with
  | #[pre, post] =>
      return { pre := ← decodeOption decodeNat pre, post := ← decodeOption decodeNat post }
  | _ => .error "invalid memory range payload"

private def encodeBehaviorKind : BehaviorKind → String
  | .requiresOf => pack #["requiresOf"]
  | .abortsOf => pack #["abortsOf"]
  | .ensuresOf => pack #["ensuresOf"]
  | .resultOf => pack #["resultOf"]
  | .unchangedOf => pack #["unchangedOf"]
  | .foldsOf => pack #["foldsOf"]
  | .writeOf index => pack #["writeOf", toString index]

private def decodeBehaviorKind (payload : String) : Except String BehaviorKind := do
  match ← unpack payload with
  | #["requiresOf"] => pure .requiresOf
  | #["abortsOf"] => pure .abortsOf
  | #["ensuresOf"] => pure .ensuresOf
  | #["resultOf"] => pure .resultOf
  | #["unchangedOf"] => pure .unchangedOf
  | #["foldsOf"] => pure .foldsOf
  | #["writeOf", index] => pure (.writeOf (← decodeNat index))
  | _ => .error "invalid behavior-kind payload"

structure EncodedOperation where
  tag : String
  payload : String := pack #[]
  targets : Array QualifiedName := #[]

private def noPayload (tag : String) : EncodedOperation := { tag }
private def oneTarget (tag : String) (target : QualifiedName) (fields : Array String := #[]) : EncodedOperation :=
  { tag, payload := pack fields, targets := #[target] }

def encodeOperation : Operation → EncodedOperation
  | .moveFunction name => oneTarget "moveFunction" name
  | .pack name variant => oneTarget "pack" name #[encodeOption id variant]
  | .tuple => noPayload "tuple"
  | .select name field => oneTarget "select" name #[field]
  | .selectVariants name fields => oneTarget "selectVariants" name #[pack fields.toArray]
  | .testVariants name variants => oneTarget "testVariants" name #[pack variants.toArray]
  | .specFunction name range => oneTarget "specFunction" name #[encodeMemoryRange range]
  | .behavior kind range => {
      tag := "behavior", payload := pack #[encodeBehaviorKind kind, encodeMemoryRange range] }
  | .updateField name field => oneTarget "updateField" name #[field]
  | .result index => { tag := "result", payload := pack #[toString index] }
  | .index => noPayload "index" | .slice => noPayload "slice" | .range => noPayload "range"
  | .implies => noPayload "implies" | .iff => noPayload "iff" | .identical => noPayload "identical"
  | .add => noPayload "add" | .sub => noPayload "sub" | .mul => noPayload "mul"
  | .mod => noPayload "mod" | .div => noPayload "div" | .bitOr => noPayload "bitOr"
  | .bitAnd => noPayload "bitAnd" | .xor => noPayload "xor" | .shl => noPayload "shl"
  | .shr => noPayload "shr" | .and => noPayload "and" | .or => noPayload "or"
  | .eq => noPayload "eq" | .neq => noPayload "neq" | .lt => noPayload "lt"
  | .gt => noPayload "gt" | .le => noPayload "le" | .ge => noPayload "ge"
  | .copy => noPayload "copy" | .move => noPayload "move" | .not => noPayload "not"
  | .cast => noPayload "cast" | .negate => noPayload "negate"
  | .exists label => { tag := "exists", payload := encodeOption toString label }
  | .borrowGlobal kind => { tag := "borrowGlobal", payload := pack #[encodeRefKind kind] }
  | .borrow kind => { tag := "borrow", payload := pack #[encodeRefKind kind] }
  | .deref => noPayload "deref" | .moveTo => noPayload "moveTo" | .moveFrom => noPayload "moveFrom"
  | .freeze explicit => { tag := "freeze", payload := pack #[toString explicit] }
  | .abort kind => { tag := "abort", payload := pack #[encodeAbortKind kind] }
  | .vector => noPayload "vector" | .len => noPayload "len" | .typeValue => noPayload "typeValue"
  | .typeDomain => noPayload "typeDomain" | .resourceDomain => noPayload "resourceDomain"
  | .stateDomain => noPayload "stateDomain"
  | .global label => { tag := "global", payload := encodeOption toString label }
  | .canModify => noPayload "canModify" | .old => noPayload "old"
  | .saveStateAnchor label => { tag := "saveStateAnchor", payload := pack #[toString label] }
  | .withStateAnchor label => { tag := "withStateAnchor", payload := pack #[toString label] }
  | .foldsCaptureAnchor label => { tag := "foldsCaptureAnchor", payload := pack #[toString label] }
  | .inlineCallSummary => noPayload "inlineCallSummary"
  | .trace kind => { tag := "trace", payload := pack #[encodeTraceKind kind] }
  | .specPublish range => { tag := "specPublish", payload := pack #[encodeMemoryRange range] }
  | .specRemove range => { tag := "specRemove", payload := pack #[encodeMemoryRange range] }
  | .specUpdate range => { tag := "specUpdate", payload := pack #[encodeMemoryRange range] }
  | .emptyVec => noPayload "emptyVec" | .singleVec => noPayload "singleVec"
  | .updateVec => noPayload "updateVec" | .concatVec => noPayload "concatVec"
  | .indexOfVec => noPayload "indexOfVec" | .containsVec => noPayload "containsVec"
  | .inRangeRange => noPayload "inRangeRange" | .inRangeVec => noPayload "inRangeVec"
  | .rangeVec => noPayload "rangeVec" | .maxU8 => noPayload "maxU8"
  | .maxU16 => noPayload "maxU16" | .maxU32 => noPayload "maxU32"
  | .maxU64 => noPayload "maxU64" | .maxU128 => noPayload "maxU128"
  | .maxU256 => noPayload "maxU256" | .bv2Int => noPayload "bv2Int"
  | .int2Bv => noPayload "int2Bv" | .abortFlag => noPayload "abortFlag"
  | .abortCode => noPayload "abortCode" | .wellFormed => noPayload "wellFormed"
  | .boxValue => noPayload "boxValue" | .unboxValue => noPayload "unboxValue"
  | .emptyEventStore => noPayload "emptyEventStore"
  | .extendEventStore => noPayload "extendEventStore"
  | .eventStoreIncludes => noPayload "eventStoreIncludes"
  | .eventStoreIncludedIn => noPayload "eventStoreIncludedIn"
  | .noOp => noPayload "noOp"
where
  encodeRefKind : RefKind → String | .immutable => "immutable" | .mutable => "mutable"
  encodeAbortKind : AbortKind → String | .code => "code" | .message => "message"
  encodeTraceKind : TraceKind → String | .user => "user" | .auto => "auto" | .subAuto => "subAuto"

private def expectNoTargets (targets : Array QualifiedName) : Except String Unit :=
  if targets.isEmpty then .ok () else .error "operation unexpectedly has qualified targets"

private def expectOneTarget (targets : Array QualifiedName) : Except String QualifiedName :=
  match targets with | #[target] => .ok target | _ => .error "operation requires one qualified target"

def expectFields (payload : String) (arity : Nat) : Except String (Array String) := do
  let fields ← unpack payload
  if fields.size == arity then .ok fields
  else .error s!"operation payload has {fields.size} fields; expected {arity}"

def decodeOperation (tag payload : String) (targets : Array QualifiedName) : Except String Operation := do
  let none (operation : Operation) := expectNoTargets targets *> pure operation
  match tag with
  | "moveFunction" => return .moveFunction (← expectOneTarget targets)
  | "pack" =>
      let fields ← expectFields payload 1
      return .pack (← expectOneTarget targets) (← decodeOption pure fields[0]!)
  | "tuple" => none .tuple
  | "select" => return .select (← expectOneTarget targets) (← expectFields payload 1)[0]!
  | "selectVariants" =>
      return .selectVariants (← expectOneTarget targets) (← unpack (← expectFields payload 1)[0]!).toList
  | "testVariants" =>
      return .testVariants (← expectOneTarget targets) (← unpack (← expectFields payload 1)[0]!).toList
  | "specFunction" =>
      return .specFunction (← expectOneTarget targets) (← decodeMemoryRange (← expectFields payload 1)[0]!)
  | "behavior" =>
      let fields ← expectFields payload 2
      return .behavior (← decodeBehaviorKind fields[0]!) (← decodeMemoryRange fields[1]!)
  | "updateField" => return .updateField (← expectOneTarget targets) (← expectFields payload 1)[0]!
  | "result" => return .result (← decodeNat (← expectFields payload 1)[0]!)
  | "index" => none .index | "slice" => none .slice | "range" => none .range
  | "implies" => none .implies | "iff" => none .iff | "identical" => none .identical
  | "add" => none .add | "sub" => none .sub | "mul" => none .mul | "mod" => none .mod
  | "div" => none .div | "bitOr" => none .bitOr | "bitAnd" => none .bitAnd | "xor" => none .xor
  | "shl" => none .shl | "shr" => none .shr | "and" => none .and | "or" => none .or
  | "eq" => none .eq | "neq" => none .neq | "lt" => none .lt | "gt" => none .gt
  | "le" => none .le | "ge" => none .ge | "copy" => none .copy | "move" => none .move
  | "not" => none .not | "cast" => none .cast | "negate" => none .negate
  | "exists" => return .exists (← decodeOption decodeNat payload)
  | "borrowGlobal" => return .borrowGlobal (← decodeRefKind (← expectFields payload 1)[0]!)
  | "borrow" => return .borrow (← decodeRefKind (← expectFields payload 1)[0]!)
  | "deref" => none .deref | "moveTo" => none .moveTo | "moveFrom" => none .moveFrom
  | "freeze" => return .freeze (← decodeBool (← expectFields payload 1)[0]!)
  | "abort" => return .abort (← decodeAbortKind (← expectFields payload 1)[0]!)
  | "vector" => none .vector | "len" => none .len | "typeValue" => none .typeValue
  | "typeDomain" => none .typeDomain | "resourceDomain" => none .resourceDomain
  | "stateDomain" => none .stateDomain
  | "global" => return .global (← decodeOption decodeNat payload)
  | "canModify" => none .canModify | "old" => none .old
  | "saveStateAnchor" => return .saveStateAnchor (← decodeNat (← expectFields payload 1)[0]!)
  | "withStateAnchor" => return .withStateAnchor (← decodeNat (← expectFields payload 1)[0]!)
  | "foldsCaptureAnchor" => return .foldsCaptureAnchor (← decodeNat (← expectFields payload 1)[0]!)
  | "inlineCallSummary" => none .inlineCallSummary
  | "trace" => return .trace (← decodeTraceKind (← expectFields payload 1)[0]!)
  | "specPublish" => return .specPublish (← decodeMemoryRange (← expectFields payload 1)[0]!)
  | "specRemove" => return .specRemove (← decodeMemoryRange (← expectFields payload 1)[0]!)
  | "specUpdate" => return .specUpdate (← decodeMemoryRange (← expectFields payload 1)[0]!)
  | "emptyVec" => none .emptyVec | "singleVec" => none .singleVec | "updateVec" => none .updateVec
  | "concatVec" => none .concatVec | "indexOfVec" => none .indexOfVec
  | "containsVec" => none .containsVec | "inRangeRange" => none .inRangeRange
  | "inRangeVec" => none .inRangeVec | "rangeVec" => none .rangeVec | "maxU8" => none .maxU8
  | "maxU16" => none .maxU16 | "maxU32" => none .maxU32 | "maxU64" => none .maxU64
  | "maxU128" => none .maxU128 | "maxU256" => none .maxU256 | "bv2Int" => none .bv2Int
  | "int2Bv" => none .int2Bv | "abortFlag" => none .abortFlag | "abortCode" => none .abortCode
  | "wellFormed" => none .wellFormed | "boxValue" => none .boxValue | "unboxValue" => none .unboxValue
  | "emptyEventStore" => none .emptyEventStore | "extendEventStore" => none .extendEventStore
  | "eventStoreIncludes" => none .eventStoreIncludes
  | "eventStoreIncludedIn" => none .eventStoreIncludedIn | "noOp" => none .noOp
  | _ => .error s!"unknown Move operation tag `{tag}`"
where
  decodeRefKind : String → Except String RefKind
    | "immutable" => .ok .immutable | "mutable" => .ok .mutable
    | value => .error s!"unknown reference kind `{value}`"
  decodeAbortKind : String → Except String AbortKind
    | "code" => .ok .code | "message" => .ok .message
    | value => .error s!"unknown abort kind `{value}`"
  decodeTraceKind : String → Except String TraceKind
    | "user" => .ok .user | "auto" => .ok .auto | "subAuto" => .ok .subAuto
    | value => .error s!"unknown trace kind `{value}`"

def lirConditionKind : Xast.ConditionKind → LeanerIR.ConditionKind
  | .letPost name => .letPost name
  | .letPre name => .letPre name
  | .assert => .assertion
  | .assume => .assumption
  | .decreases => .decreases
  | .abortsIf => .abortsIf
  | .abortsWith => .abortsWith
  | .succeedsIf => .succeedsIf
  | .emits => .emits
  | .ensures => .ensures
  | .requires => .requires
  | .structInvariant => .structInvariant
  | .functionInvariant => .functionInvariant
  | .loopInvariant => .loopInvariant
  | .globalInvariant parameters => .globalInvariant parameters.toArray
  | .globalInvariantUpdate parameters => .globalInvariantUpdate parameters.toArray
  | .schemaInvariant => .schemaInvariant
  | .axiom parameters => .axiom_ parameters.toArray
  | .update => .update

def xastConditionKind : LeanerIR.ConditionKind → Xast.ConditionKind
  | .letPost name => .letPost name
  | .letPre name => .letPre name
  | .assertion => .assert
  | .assumption => .assume
  | .decreases => .decreases
  | .abortsIf => .abortsIf
  | .abortsWith => .abortsWith
  | .succeedsIf => .succeedsIf
  | .emits => .emits
  | .ensures => .ensures
  | .requires => .requires
  | .structInvariant => .structInvariant
  | .functionInvariant => .functionInvariant
  | .loopInvariant => .loopInvariant
  | .globalInvariant parameters => .globalInvariant parameters.toList
  | .globalInvariantUpdate parameters => .globalInvariantUpdate parameters.toList
  | .schemaInvariant => .schemaInvariant
  | .axiom_ parameters => .axiom parameters.toList
  | .update => .update

end Transpiler.LIR.Codec
