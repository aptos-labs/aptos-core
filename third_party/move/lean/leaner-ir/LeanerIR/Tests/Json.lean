-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR

namespace LeanerIR.Tests.Json

open LeanerIR
open LeanerIR.Import

private def emptyUnit : RawUnit where
  tables := {}
  profiles := #[]
  namespaces := #[]

private def emptyUnitJson : String :=
  "{\"dependencies\":[],\"evidence\":[],\"namespaces\":[],\"profiles\":[],\"tables\":{\"alignments\":[],\"files\":[],\"lifetimes\":[],\"locations\":[],\"names\":[],\"namespaces\":[],\"origins\":[],\"types\":[]},\"version\":{\"major\":1,\"minor\":1}}"

#guard encodeJson emptyUnit == emptyUnitJson

#guard match decodeJson (encodeJson emptyUnit) with
  | .ok decoded => decoded == emptyUnit
  | .error _ => false

private def jsonWithUnknownTopLevelField : String :=
  "{\"unknownSemanticField\":true," ++ (encodeJson emptyUnit).drop 1

#guard match decodeJson jsonWithUnknownTopLevelField with
  | .error message => message == "unknown raw LIR JSON field `$.unknownSemanticField`"
  | .ok _ => false

private def jsonWithUnknownNestedField : String :=
  (encodeJson emptyUnit).replace
    "\"version\":{\"major\":1,\"minor\":1}"
    "\"version\":{\"major\":1,\"minor\":1,\"patch\":0}"

#guard match decodeJson jsonWithUnknownNestedField with
  | .error message => message == "unknown raw LIR JSON field `$.version.patch`"
  | .ok _ => false

#guard match decodeJson
    ((encodeJson emptyUnit).replace
      "\"version\":{\"major\":1,\"minor\":1}"
      "\"version\":{\"major\":1,\"major\":1,\"minor\":1}") with
  | .error message => message.contains "duplicate JSON object field `major`"
  | .ok _ => false

#guard match decodeJson
    ("{\"version\":{\"major\":1,\"minor\":1}," ++ (encodeJson emptyUnit).drop 1) with
  | .error message => message.contains "duplicate JSON object field `version`"
  | .ok _ => false

#guard match decodeJson
    "{\"namespaces\":[],\"profiles\":[],\"tables\":{},\"version\":{\"major\":2,\"minor\":0}}" with
  | .error message => message.contains "unsupported raw LIR JSON version 2.0"
  | .ok _ => false

#guard match decodeJson "{\"profiles\":[],\"namespaces\":[]}" with
  | .error _ => true
  | .ok _ => false

private def profiledUnit : RawUnit :=
  { emptyUnit with profiles := #[{ profile := .rust, name := "rust" }] }

/-- Representative malformed documents cover the structural families in the
RawUnit v1 codec: document/object/array syntax, required records and arrays,
schema-number fields, and closed enum constructors. Individual semantic ID and
arena errors remain the responsibility of shared validation. -/
private def malformedDocuments : Array String := #[
  "",
  "null",
  "[]",
  "{}",
  (encodeJson emptyUnit) ++ " trailing",
  ((encodeJson emptyUnit).dropEnd 1).toString ++ ",}",
  (encodeJson emptyUnit).replace "\"dependencies\":[]" "\"dependencies\":{}",
  (encodeJson emptyUnit).replace "\"evidence\":[]" "\"evidence\":null",
  (encodeJson emptyUnit).replace "\"namespaces\":[]" "\"namespaces\":false",
  (encodeJson emptyUnit).replace "\"profiles\":[]" "\"profiles\":{}",
  (encodeJson emptyUnit).replace "\"tables\":{" "\"tables\":[",
  (encodeJson emptyUnit).replace "\"major\":1" "\"major\":-1",
  (encodeJson emptyUnit).replace "\"major\":1" "\"major\":1.5",
  (encodeJson emptyUnit).replace "\"minor\":1" "\"minor\":\"0\"",
  (encodeJson emptyUnit).replace "\"minor\":1" "\"patch\":0",
  (encodeJson profiledUnit).replace "\"profile\":\"rust\"" "\"profile\":\"unknown\""
]

private def decodeFails (document : String) : Bool :=
  match decodeJson document with
  | .error _ => true
  | .ok _ => false

#guard malformedDocuments.all decodeFails

private def bytes : ConstValue := .bytes #[0, 255]

#guard match Lean.fromJson? (α := ConstValue) (Lean.toJson bytes) with
  | .ok decoded => decoded == bytes
  | .error _ => false

private def genericArguments : Array GenericArgument := #[
  .typeArg { typeId := ⟨3⟩, loc := ⟨4⟩ },
  .const (.integer 7),
  .lifetime ⟨2⟩,
  .evidence ⟨1⟩]

#guard match Lean.fromJson? (α := Array GenericArgument) (Lean.toJson genericArguments) with
  | .ok decoded => decoded == genericArguments
  | .error _ => false

private def mirCfg : RawCfg := {
  entry := ⟨0⟩
  blocks := #[
    {
      loc := ⟨0⟩
      statements := #[.storageLive ⟨0⟩, .deinit ⟨0⟩,
        .setDiscriminant ⟨0⟩ ⟨0⟩, .retag ⟨0⟩]
      terminator := .assert ⟨0⟩ true .boundsCheck ⟨1⟩ (.cleanup ⟨2⟩)
    },
    { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] },
    { loc := ⟨0⟩, terminator := .resume }] }

#guard match Lean.fromJson? (α := RawCfg) (Lean.toJson mirCfg) with
  | .ok decoded => decoded == mirCfg
  | .error _ => false

/-! The raw-control constructor corpus below is deliberately exhaustive. The
case classifiers make an added constructor a compile error until this wire
contract test is extended. The corpus itself then checks canonical round-trip
and recursive closed-object rejection through the public RawUnit decoder. -/

private def classifyRawUnwindAction : RawUnwindAction → Nat
  | .continue_ => 0
  | .unreachable => 1
  | .terminate _ => 2
  | .cleanup _ => 3

private def classifyRawAssertKind : RawAssertKind → Nat
  | .boundsCheck => 0
  | .overflow => 1
  | .divisionByZero => 2
  | .remainderByZero => 3
  | .misalignedPointerDereference => 4
  | .profile _ => 5

private def classifyRawStatement : RawStatement → Nat
  | .execute _ => 0
  | .storageLive _ => 1
  | .storageDead _ => 2
  | .deinit _ => 3
  | .setDiscriminant _ _ => 4
  | .retag _ => 5
  | .placeMention _ => 6
  | .ascribeUserType _ _ => 7
  | .profile _ => 8

private def classifyRawTerminator : RawTerminator → Nat
  | .goto _ => 0
  | .branch _ _ _ => 1
  | .switch _ _ _ => 2
  | .call _ _ _ => 3
  | .drop _ _ _ => 4
  | .assert _ _ _ _ _ => 5
  | .return_ _ => 6
  | .throw_ _ _ => 7
  | .unreachable => 8
  | .resume => 9
  | .abort => 10

private def classifyRawBody : RawBody → Nat
  | .absent => 0
  | .structured _ => 1
  | .cfg _ => 2

private def profileValue : ProfileValue := {
  profile := .rust, tag := "wire-test", payload := "payload" }

private def allRawStatements : Array RawStatement := #[
  .execute ⟨0⟩,
  .storageLive ⟨0⟩,
  .storageDead ⟨0⟩,
  .deinit ⟨0⟩,
  .setDiscriminant ⟨0⟩ ⟨0⟩,
  .retag ⟨0⟩,
  .placeMention ⟨0⟩,
  .ascribeUserType ⟨0⟩ { typeId := ⟨0⟩, loc := ⟨0⟩ },
  .profile profileValue]

private def rawBlock (terminator : RawTerminator)
    (statements : Array RawStatement := #[]) : RawBasicBlock := {
  loc := ⟨0⟩, statements, terminator }

private def allRawTerminators : Array RawTerminator := #[
  .goto ⟨1⟩,
  .branch ⟨0⟩ ⟨1⟩ ⟨2⟩,
  .switch ⟨0⟩ #[(.integer 7, ⟨1⟩)] ⟨2⟩,
  .call ⟨0⟩ none .continue_,
  .call ⟨0⟩ (some { place := ⟨0⟩, target := ⟨1⟩ }) (.cleanup ⟨2⟩),
  .drop ⟨0⟩ ⟨1⟩ .unreachable,
  .assert ⟨0⟩ true .boundsCheck ⟨1⟩ (.terminate "bounds"),
  .assert ⟨0⟩ false .overflow ⟨1⟩ .continue_,
  .assert ⟨0⟩ true .divisionByZero ⟨1⟩ (.cleanup ⟨2⟩),
  .assert ⟨0⟩ false .remainderByZero ⟨1⟩ .unreachable,
  .assert ⟨0⟩ true .misalignedPointerDereference ⟨1⟩ .continue_,
  .assert ⟨0⟩ true (.profile profileValue) ⟨1⟩ .continue_,
  .return_ #[⟨0⟩],
  .throw_ (.profile profileValue) #[⟨0⟩],
  .unreachable,
  .resume,
  .abort]

private def rawFunction (name : Nat) (body : RawBody) : FunctionDecl RawBody := {
  loc := ⟨0⟩
  name := ⟨name⟩
  profile := .rust
  signature := {}
  body
  origin := ⟨0⟩
  alignment := ⟨0⟩ }

private def rawConstructorUnit : RawUnit := {
  tables := {
    names := (Array.range 3).map fun index => {
      namespaceId := ⟨0⟩, name := s!"body{index}" }
    namespaces := #[{ segments := #["wire"] }] }
  profiles := #[{ profile := .rust, name := "rust" }]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    functions := #[
      rawFunction 0 .absent,
      rawFunction 1 (.structured ⟨0⟩),
      rawFunction 2 (.cfg {
        entry := ⟨0⟩
        blocks := allRawTerminators.mapIdx fun index terminator =>
          rawBlock terminator (if index == 0 then allRawStatements else #[]) })] }] }

#guard allRawStatements.map classifyRawStatement == Array.range 9
#guard allRawTerminators.map classifyRawTerminator ==
  #[0, 1, 2, 3, 3, 4, 5, 5, 5, 5, 5, 5, 6, 7, 8, 9, 10]
#guard #[RawUnwindAction.continue_, .unreachable, .terminate "reason", .cleanup ⟨0⟩].map
  classifyRawUnwindAction == Array.range 4
#guard #[RawAssertKind.boundsCheck, .overflow, .divisionByZero, .remainderByZero,
  .misalignedPointerDereference, .profile profileValue].map classifyRawAssertKind == Array.range 6
#guard #[RawBody.absent, .structured ⟨0⟩, .cfg mirCfg].map classifyRawBody == Array.range 3

#guard match decodeJson (encodeJson rawConstructorUnit) with
  | .ok decoded => decoded == rawConstructorUnit
  | .error _ => false

private partial def unknownFieldMutations : Lean.Json → Array Lean.Json
  | .obj fields =>
      fields.toList.foldl (init := #[.obj (fields.insert "unknownSemanticField" (.bool true))])
        fun mutations (name, value) =>
          (unknownFieldMutations value).foldl (init := mutations) fun mutations replacement =>
            mutations.push (.obj (fields.insert name replacement))
  | .arr values =>
      (Array.range values.size).foldl (init := #[]) fun mutations index =>
        (unknownFieldMutations values[index]!).foldl (init := mutations) fun mutations replacement =>
          mutations.push (.arr (values.set! index replacement))
  | _ => #[]

private def rejectsUnknownFieldMutation (expected received : Lean.Json) : Bool :=
  match ensureClosedJsonShape received expected with
  | .error _ => true
  | .ok _ => false

#guard !(unknownFieldMutations (Lean.toJson rawConstructorUnit)).isEmpty
#guard let json := Lean.toJson rawConstructorUnit
  (unknownFieldMutations json).all (rejectsUnknownFieldMutation json)

private def classifyOriginKind : OriginKind → Nat
  | .moveSource => 0
  | .leanerSource => 1
  | .rustMir => 2
  | .generated _ => 3

private def classifyTrust : Trust → Nat
  | .authored => 0
  | .checked => 1
  | .assumed => 2

private def classifyProfile : Profile → Nat
  | .move => 0
  | .rust => 1
  | .extension _ => 2

private def classifyIntWidth : IntWidth → Nat
  | .bits _ => 0
  | .pointer => 1
  | .unbounded => 2

private def classifyReferenceKind : ReferenceKind → Nat
  | .shared => 0
  | .mutable => 1

private def classifyLifetimeKind : LifetimeKind → Nat
  | .static => 0
  | .parameter _ => 1
  | .inference => 2
  | .local => 3

private def classifyConstValue : ConstValue → Nat
  | .unit => 0
  | .bool _ => 1
  | .character _ => 2
  | .integer _ => 3
  | .address _ => 4
  | .string _ => 5
  | .bytes _ => 6
  | .vector _ => 7
  | .tuple _ => 8
  | .profile _ => 9

private def classifyGenericArgument : GenericArgument → Nat
  | .typeArg _ => 0
  | .const _ => 1
  | .lifetime _ => 2
  | .evidence _ => 3

private def classifyAbility : Ability → Nat
  | .copy => 0
  | .drop => 1
  | .store => 2
  | .key => 3

private def classifyGenericPredicate : GenericPredicate → Nat
  | .ability _ _ => 0
  | .implements _ _ => 1
  | .associatedTypeEq _ _ _ => 2
  | .associatedConstEq _ _ _ => 3
  | .lifetimeOutlives _ _ => 4
  | .constEq _ _ => 5
  | .profile _ => 6

private def classifyTy : Ty → Nat
  | .unit => 0
  | .never => 1
  | .bool => 2
  | .character => 3
  | .string => 4
  | .bytes => 5
  | .address => 6
  | .signer => 7
  | .integer _ _ => 8
  | .tuple _ => 9
  | .vector _ _ => 10
  | .range => 11
  | .eventStore => 12
  | .typeDomain _ => 13
  | .resourceDomain _ _ => 14
  | .stateDomain => 15
  | .nominal _ _ => 16
  | .function _ _ _ => 17
  | .typeParameter _ => 18
  | .reference _ => 19
  | .profile _ => 20

private def classifyAttributeValue : AttributeValue → Nat
  | .constant _ => 0
  | .name _ _ => 1
  | .qualifiedName _ => 2

private def classifyAttribute : Attribute → Nat
  | .call _ _ _ => 0
  | .assign _ _ _ => 1

private def classifyPlace : Place → Nat
  | .localVar _ => 0
  | .deref _ => 1
  | .field _ _ _ => 2
  | .index _ _ => 3
  | .subslice _ _ _ _ => 4
  | .downcast _ _ => 5

private def classifyPatternKind : PatternKind → Nat
  | .wildcard => 0
  | .variable _ => 1
  | .tuple _ => 2
  | .constructor _ _ _ _ => 3
  | .literal _ => 4
  | .range _ _ _ => 5

private def classifyBinderKind : BinderKind → Nat
  | .typeArg => 0
  | .const => 1
  | .lifetime => 2
  | .evidence => 3

private def extensionValue : ProfileValue := {
  profile := .extension ⟨0⟩, tag := "extension", payload := "value" }

private def allConstValues : Array ConstValue := #[
  .unit,
  .bool true,
  .character 0x1f980,
  .integer (-7),
  .address "0x1",
  .string "text",
  .bytes #[0, 255],
  .vector #[.integer 1],
  .tuple #[.bool false],
  .profile extensionValue]

private def allGenericArguments : Array GenericArgument := #[
  .typeArg { typeId := ⟨0⟩, loc := ⟨0⟩ },
  .const (.integer 1),
  .lifetime ⟨0⟩,
  .evidence ⟨0⟩]

private def traitRef : TraitRef := {
  trait := { namespaceId := ⟨0⟩, name := ⟨0⟩ }
  arguments := allGenericArguments }

private def allGenericPredicates : Array GenericPredicate := #[
  .ability ⟨0⟩ .copy,
  .implements ⟨0⟩ traitRef,
  .associatedTypeEq traitRef ⟨0⟩ ⟨0⟩,
  .associatedConstEq traitRef ⟨0⟩ (.integer 1),
  .lifetimeOutlives ⟨0⟩ ⟨1⟩,
  .constEq (.integer 1) (.integer 1),
  .profile extensionValue]

private def allTypes : Array Ty := #[
  .unit,
  .never,
  .bool,
  .character,
  .string,
  .bytes,
  .address,
  .signer,
  .integer (.bits 32) false,
  .tuple #[⟨0⟩],
  .vector ⟨0⟩ (some (.integer 1)),
  .range,
  .eventStore,
  .typeDomain ⟨0⟩,
  .resourceDomain ⟨0⟩ (some #[⟨0⟩]),
  .stateDomain,
  .nominal ⟨0⟩ allGenericArguments,
  .function #[⟨0⟩] ⟨0⟩ #[.copy, .drop, .store, .key],
  .typeParameter 0,
  .reference { profile := .rust, kind := .shared, referent := ⟨0⟩, lifetime := ⟨0⟩ },
  .profile extensionValue]

private def allPlaces : Array Place := #[
  .localVar ⟨0⟩,
  .deref ⟨0⟩,
  .field ⟨0⟩ { namespaceId := ⟨0⟩, name := ⟨0⟩ } ⟨0⟩,
  .index ⟨0⟩ ⟨0⟩,
  .subslice ⟨0⟩ 1 2 false,
  .downcast ⟨0⟩ ⟨0⟩]

private def allPatterns : Array Pattern := #[
  { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .wildcard },
  { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .variable ⟨0⟩ },
  { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .tuple #[⟨0⟩] },
  { loc := ⟨0⟩, typeId := ⟨0⟩,
    kind := .constructor ⟨0⟩ allGenericArguments (some "variant") #[⟨0⟩] },
  { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .literal (.integer 1) },
  { loc := ⟨0⟩, typeId := ⟨0⟩,
    kind := .range (some (.integer 0)) (some (.integer 2)) true }]

private def allBinderKinds : Array GenericBinder := #[
  { name := "T", kind := .typeArg, loc := ⟨0⟩ },
  { name := "N", kind := .const,
    type := some { typeId := ⟨0⟩, loc := ⟨0⟩ }, loc := ⟨0⟩ },
  { name := "a", kind := .lifetime, loc := ⟨0⟩ },
  { name := "E", kind := .evidence, loc := ⟨0⟩ }]

private def allAttributes : Array Attribute := #[
  .call "outer" #[.assign "constant" (.constant (.integer 1)),
    .assign "name" (.name (some ⟨0⟩) "item"),
    .assign "qualified" (.qualifiedName "wire::item")],
  .assign "plain" (.constant .unit) (some ⟨0⟩)]

private def coreConstructorUnit : RawUnit := {
  tables := {
    files := #[{ name := "wire.lir", contentHash := "hash" }]
    locations := #[{
      primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 }
      related := #[{ file := ⟨0⟩, startByte := 1, endByte := 2 }]
      expansion := #[{ file := ⟨0⟩, startByte := 2, endByte := 3 }]
      generatedBy := some "wire-test"
      parent := some ⟨0⟩ }]
    origins := #[
      { kind := .moveSource, location := ⟨0⟩ },
      { kind := .leanerSource, location := ⟨0⟩ },
      { kind := .rustMir, location := ⟨0⟩ },
      { kind := .generated "wire-test", location := ⟨0⟩ }]
    alignments := #[
      { source := ⟨0⟩, trust := .authored, description := "authored" },
      { source := ⟨1⟩, trust := .checked, description := "checked" },
      { source := ⟨2⟩, trust := .assumed, description := "assumed" }]
    lifetimes := #[
      { kind := .static, loc := ⟨0⟩, name := some "'static" },
      { kind := .parameter 0, loc := ⟨0⟩, name := some "'a" },
      { kind := .inference, loc := ⟨0⟩ },
      { kind := .local, loc := ⟨0⟩ }]
    types := allTypes ++ #[
      .integer .pointer true,
      .integer .unbounded true,
      .reference { profile := .rust, kind := .mutable, referent := ⟨0⟩, lifetime := ⟨0⟩ }]
    namespaces := #[{ segments := #["wire"] }]
    names := #[{ namespaceId := ⟨0⟩, name := "item" }] }
  profiles := #[
    { profile := .move, name := "move" },
    { profile := .rust, name := "rust" },
    { profile := .extension ⟨2⟩, name := "extension" }]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := allConstValues.map fun value => {
      loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value value }
    patterns := allPatterns
    places := allPlaces
    attributes := allAttributes
    functions := #[{
      loc := ⟨0⟩
      name := ⟨0⟩
      profile := .rust
      signature := { generics := allBinderKinds, predicates := allGenericPredicates }
      body := .absent
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }] }

#guard #[OriginKind.moveSource, .leanerSource, .rustMir, .generated "test"].map
  classifyOriginKind == Array.range 4
#guard #[Trust.authored, .checked, .assumed].map classifyTrust == Array.range 3
#guard #[Profile.move, .rust, .extension ⟨0⟩].map classifyProfile == Array.range 3
#guard #[IntWidth.bits 8, .pointer, .unbounded].map classifyIntWidth == Array.range 3
#guard #[ReferenceKind.shared, .mutable].map classifyReferenceKind == Array.range 2
#guard #[LifetimeKind.static, .parameter 0, .inference, .local].map
  classifyLifetimeKind == Array.range 4
#guard allConstValues.map classifyConstValue == Array.range 10
#guard allGenericArguments.map classifyGenericArgument == Array.range 4
#guard #[Ability.copy, .drop, .store, .key].map classifyAbility == Array.range 4
#guard allGenericPredicates.map classifyGenericPredicate == Array.range 7
#guard allTypes.map classifyTy == Array.range 21
#guard #[AttributeValue.constant .unit, .name none "item", .qualifiedName "wire::item"].map
  classifyAttributeValue == Array.range 3
#guard allAttributes.map classifyAttribute == Array.range 2
#guard allPlaces.map classifyPlace == Array.range 6
#guard allPatterns.map (classifyPatternKind ·.kind) == Array.range 6
#guard allBinderKinds.map (classifyBinderKind ·.kind) == Array.range 4

#guard match decodeJson (encodeJson coreConstructorUnit) with
  | .ok decoded => decoded == coreConstructorUnit
  | .error _ => false

#guard let json := Lean.toJson coreConstructorUnit
  (unknownFieldMutations json).all (rejectsUnknownFieldMutation json)

private def classifyBorrowKind : BorrowKind → Nat
  | .immutable => 0
  | .mutable => 1
  | .profile _ => 2

private def classifyThrowKind : ThrowKind → Nat
  | .abort => 0
  | .panic => 1
  | .profile _ => 2

private def classifyCallKind : CallKind → Nat
  | .function _ => 0
  | .constructor _ _ => 1
  | .destructor _ _ => 2
  | .closure _ => 3
  | .invoke => 4
  | .extension _ _ => 5

private def classifySurfaceSyntax : SurfaceSyntax → Nat
  | .receiverCall => 0
  | .indexNotation => 1
  | .extension _ => 2

private def classifyGlobalKind : GlobalKind → Nat
  | .contains => 0
  | .borrow _ => 1
  | .take => 2
  | .publish => 3

private def classifyPrimitiveOperation : PrimitiveOperation → Nat
  | .tuple => 0
  | .vector => 1
  | .repeatVector => 2
  | .pushVector => 60
  | .swapVector => 61
  | .insertVector => 62
  | .removeVector => 63
  | .concatVector => 64
  | .reverseSliceVector => 65
  | .destroyEmptyVector => 66
  | .containsVector => 67
  | .indexOfVector => 68
  | .checkVectorIndex _ => 69
  | .length => 3
  | .index => 4
  | .slice => 5
  | .add => 6
  | .checkedAdd _ => 7
  | .overflowingAdd => 8
  | .subtract => 9
  | .checkedSubtract _ => 10
  | .overflowingSubtract => 11
  | .multiply => 12
  | .checkedMultiply _ => 13
  | .overflowingMultiply => 14
  | .modulo => 15
  | .checkedModulo _ => 16
  | .divide => 17
  | .checkedDivide _ => 18
  | .bitwiseOr => 19
  | .bitwiseAnd => 20
  | .bitwiseXor => 21
  | .bitwiseNot => 22
  | .shiftLeft => 23
  | .checkedShiftLeft _ => 24
  | .shiftRight => 25
  | .checkedShiftRight _ => 26
  | .logicalAnd => 27
  | .logicalOr => 28
  | .equal => 29
  | .notEqual => 30
  | .less => 31
  | .greater => 32
  | .lessEqual => 33
  | .greaterEqual => 34
  | .logicalNot => 35
  | .negate => 36
  | .checkedNegate _ => 37
  | .copyValue => 38
  | .moveValue => 39
  | .cast => 40
  | .checkedCast _ => 41
  | .range => 42
  | .implies => 43
  | .equivalent => 44
  | .identical => 45

private def classifyReferenceOperation : ReferenceOperation → Nat
  | .borrow _ => 0
  | .dereference => 1
  | .freeze _ => 2
  | .mutate => 3
  | .endLoan _ => 4

private def classifyDataOperation : DataOperation → Nat
  | .select _ _ => 0
  | .selectVariants _ _ => 1
  | .testVariants _ _ => 2
  | .discriminant _ => 3
  | .updateField _ _ => 4

private def classifyTraceKind : TraceKind → Nat
  | .user => 0
  | .automatic => 1
  | .subAutomatic => 2

private def classifyBehaviorKind : BehaviorKind → Nat
  | .requiresOf => 0
  | .abortsOf => 1
  | .ensuresOf => 2
  | .resultOf => 3
  | .unchangedOf => 4
  | .foldsOf => 5
  | .writeOf _ => 6

private def classifySpecOperation : SpecOperation → Nat
  | .functionCall _ _ => 0
  | .behavior _ _ => 1
  | .result _ => 2
  | .typeValue => 3
  | .typeDomain => 4
  | .resourceDomain => 5
  | .stateDomain => 6
  | .global _ => 7
  | .canModify => 8
  | .old => 9
  | .saveStateAnchor _ => 10
  | .withStateAnchor _ => 11
  | .foldsCaptureAnchor _ => 12
  | .inlineCallSummary => 13
  | .trace _ => 14
  | .publish _ => 15
  | .remove _ => 16
  | .update _ => 17
  | .emptyVector => 18
  | .singletonVector => 19
  | .updateVector => 20
  | .concatVector => 21
  | .indexOfVector => 22
  | .containsVector => 23
  | .lengthVector => 24
  | .indexVector => 25
  | .sliceVector => 26
  | .inRange => 27
  | .inVectorRange => 28
  | .vectorRange => 29
  | .maxValue _ => 30
  | .bitVectorToInt => 31
  | .intToBitVector => 32
  | .abortFlag => 33
  | .abortCode => 34
  | .wellFormed => 35
  | .boxValue => 36
  | .unboxValue => 37
  | .emptyEventStore => 38
  | .extendEventStore => 39
  | .eventStoreIncludes => 40
  | .eventStoreIncludedIn => 41
  | .noOp => 42

private def classifyOperation : Operation → Nat
  | .move _ => 0
  | .copy _ => 1
  | .borrow _ _ => 2
  | .read _ => 3
  | .write _ => 4
  | .call _ => 5
  | .global _ => 6
  | .primitive _ => 7
  | .reference _ => 8
  | .data _ => 9
  | .specification _ => 10
  | .assert => 11
  | .drop _ => 12
  | .profile _ _ => 13

private def classifyQuantifierKind : QuantifierKind → Nat
  | .forall => 0
  | .exists => 1
  | .choose => 2
  | .chooseMin => 3
  | .profile _ => 4

private def classifyConditionKind : ConditionKind → Nat
  | .letPost _ => 0
  | .letPre _ => 1
  | .assertion => 2
  | .assumption => 3
  | .decreases => 4
  | .abortsIf => 5
  | .abortsWith => 6
  | .succeedsIf => 7
  | .emits => 8
  | .ensures => 9
  | .requires => 10
  | .structInvariant => 11
  | .functionInvariant => 12
  | .loopInvariant => 13
  | .globalInvariant _ => 14
  | .globalInvariantUpdate _ => 15
  | .schemaInvariant => 16
  | .axiom_ _ => 17
  | .update => 18

private def classifyExprKind : ExprKind → Nat
  | .value _ _ => 0
  | .constant _ => 1
  | .localVar _ => 2
  | .operation _ _ _ _ => 3
  | .block _ _ => 4
  | .letDecl _ _ _ => 5
  | .ifElse _ _ _ => 6
  | .match_ _ _ => 7
  | .loop _ _ => 8
  | .break_ _ _ => 9
  | .continue_ _ => 10
  | .return_ _ => 11
  | .throw_ _ _ => 12
  | .assign _ _ => 13
  | .assignPattern _ _ => 14
  | .quantifier _ _ _ _ _ => 15
  | .spec _ => 16

private def qref : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨0⟩ }

private def allBorrowKinds : Array BorrowKind := #[
  .immutable, .mutable, .profile extensionValue]

private def allThrowKinds : Array ThrowKind := #[
  .abort, .panic, .profile extensionValue]

private def allCallKinds : Array CallKind := #[
  .function qref,
  .constructor qref (some "Variant"),
  .destructor qref (some "Variant"),
  .closure qref,
  .invoke,
  .extension extensionValue #[qref]]

private def allPrimitiveOperations : Array PrimitiveOperation := #[
  .tuple, .vector, .repeatVector, .length, .index, .slice,
  .add, .checkedAdd .panic, .overflowingAdd,
  .subtract, .checkedSubtract .panic, .overflowingSubtract,
  .multiply, .checkedMultiply .panic, .overflowingMultiply,
  .modulo, .checkedModulo .panic,
  .divide, .checkedDivide .panic,
  .bitwiseOr, .bitwiseAnd, .bitwiseXor, .bitwiseNot,
  .shiftLeft, .checkedShiftLeft .panic, .shiftRight, .checkedShiftRight .panic,
  .logicalAnd, .logicalOr, .equal, .notEqual, .less, .greater, .lessEqual,
  .greaterEqual, .logicalNot, .negate, .checkedNegate .panic,
  .copyValue, .moveValue, .cast, .checkedCast .panic, .range, .implies,
  .equivalent, .identical, .insertVector, .removeVector, .concatVector, .reverseSliceVector,
  .destroyEmptyVector, .containsVector, .indexOfVector, .checkVectorIndex .abort]

private def allReferenceOperations : Array ReferenceOperation := #[
  .borrow (.profile extensionValue), .dereference, .freeze true, .mutate]

private def allDataOperations : Array DataOperation := #[
  .select qref "field",
  .selectVariants qref #["left", "right"],
  .testVariants qref #["Variant"],
  .discriminant qref,
  .updateField qref "field"]

private def allSpecOperations : Array SpecOperation := #[
  .functionCall qref { pre := some 0, post := some 1 },
  .behavior (.writeOf 1) { pre := some 0, post := some 1 },
  .result 0, .typeValue, .typeDomain, .resourceDomain, .stateDomain,
  .global (some 0), .canModify, .old, .saveStateAnchor 0,
  .withStateAnchor 0, .foldsCaptureAnchor 0, .inlineCallSummary,
  .trace .user, .publish { pre := some 0 }, .remove { post := some 1 },
  .update { pre := some 0, post := some 1 }, .emptyVector, .singletonVector,
  .updateVector, .concatVector, .indexOfVector, .containsVector,
  .lengthVector, .indexVector, .sliceVector, .inRange,
  .inVectorRange, .vectorRange, .maxValue 128, .bitVectorToInt,
  .intToBitVector, .abortFlag, .abortCode, .wellFormed, .boxValue,
  .unboxValue, .emptyEventStore, .extendEventStore, .eventStoreIncludes,
  .eventStoreIncludedIn, .noOp]

private def operationRepresentatives : Array Operation := #[
  .move ⟨0⟩,
  .copy ⟨0⟩,
  .borrow .immutable ⟨0⟩,
  .read ⟨0⟩,
  .write ⟨0⟩,
  .call (.function qref),
  .global .contains,
  .primitive .tuple,
  .reference .dereference,
  .data (.discriminant qref),
  .specification .noOp,
  .assert,
  .drop ⟨0⟩,
  .profile extensionValue #[qref]]

private def allOperations : Array Operation :=
  operationRepresentatives ++
  allBorrowKinds.map (Operation.borrow · ⟨0⟩) ++
  allCallKinds.map Operation.call ++
  #[GlobalKind.contains, .borrow (.profile extensionValue), .take, .publish].map
    Operation.global ++
  allPrimitiveOperations.map Operation.primitive ++
  allReferenceOperations.map Operation.reference ++
  allDataOperations.map Operation.data ++
  allSpecOperations.map Operation.specification

private def allConditionKinds : Array ConditionKind := #[
  .letPost "x", .letPre "x", .assertion, .assumption, .decreases, .abortsIf,
  .abortsWith, .succeedsIf, .emits, .ensures, .requires, .structInvariant,
  .functionInvariant, .loopInvariant, .globalInvariant #["T"],
  .globalInvariantUpdate #["T"], .schemaInvariant, .axiom_ #["T"], .update]

private def allConditions : Array Condition :=
  allConditionKinds.map fun kind => {
    loc := ⟨0⟩, kind, properties := allAttributes, expression := ⟨0⟩,
    auxiliary := #[("aux", ⟨0⟩)] }

private def allQuantifierKinds : Array QuantifierKind := #[
  .forall, .exists, .choose, .chooseMin, .profile extensionValue]

private def exprKindRepresentatives : Array ExprKind := #[
  .value (.integer 1) (some "1"),
  .constant qref,
  .localVar ⟨0⟩,
  .operation .assert allGenericArguments #[⟨0⟩] (some .receiverCall),
  .block #[⟨0⟩] (some ⟨0⟩),
  .letDecl ⟨0⟩ (some ⟨0⟩) ⟨0⟩,
  .ifElse ⟨0⟩ ⟨0⟩ (some ⟨0⟩),
  .match_ ⟨0⟩ #[{ pattern := ⟨0⟩, guard := some ⟨0⟩, body := ⟨0⟩ }],
  .loop (some "label") ⟨0⟩,
  .break_ 0 (some ⟨0⟩),
  .continue_ 0,
  .return_ #[⟨0⟩],
  .throw_ .panic #[⟨0⟩],
  .assign ⟨0⟩ ⟨0⟩,
  .assignPattern ⟨0⟩ ⟨0⟩,
  .quantifier .forall #[{ pattern := ⟨0⟩, domain := ⟨0⟩ }]
    #[#[⟨0⟩]] (some ⟨0⟩) ⟨0⟩,
  .spec {
    loc := ⟨0⟩, sourceLoc := some ⟨0⟩, pragmas := allAttributes,
    conditions := allConditions,
    frame := some {
      modifies := #[⟨0⟩], reads := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }],
      modifiesAll := true, readsAll := true } }]

private def allOperationExpressions : Array Expr :=
  (exprKindRepresentatives ++
    allOperations.map (ExprKind.operation · allGenericArguments #[⟨0⟩]) ++
    #[SurfaceSyntax.receiverCall, .indexNotation, .extension extensionValue].map
      (fun surface => ExprKind.operation .assert #[] #[] (some surface)) ++
    allThrowKinds.map (ExprKind.throw_ · #[⟨0⟩]) ++
    allQuantifierKinds.map fun kind =>
      ExprKind.quantifier kind #[] #[] none ⟨0⟩).map fun kind => {
        loc := ⟨0⟩, typeId := ⟨0⟩, kind }

private def operationConstructorUnit : RawUnit :=
  let ns := coreConstructorUnit.namespaces[0]!
  { coreConstructorUnit with namespaces := #[{
      ns with expressions := allOperationExpressions }] }

#guard allBorrowKinds.map classifyBorrowKind == Array.range 3
#guard allThrowKinds.map classifyThrowKind == Array.range 3
#guard allCallKinds.map classifyCallKind == Array.range 6
#guard #[SurfaceSyntax.receiverCall, .indexNotation, .extension extensionValue].map
  classifySurfaceSyntax == Array.range 3
#guard #[GlobalKind.contains, .borrow .immutable, .take, .publish].map
  classifyGlobalKind == Array.range 4
#guard allPrimitiveOperations.map classifyPrimitiveOperation == Array.range 46 ++ #[62, 63, 64, 65, 66, 67, 68, 69]
#guard allReferenceOperations.map classifyReferenceOperation == Array.range 4
#guard allDataOperations.map classifyDataOperation == Array.range 5
#guard #[TraceKind.user, .automatic, .subAutomatic].map classifyTraceKind == Array.range 3
#guard #[BehaviorKind.requiresOf, .abortsOf, .ensuresOf, .resultOf,
    .unchangedOf, .foldsOf, .writeOf 1].map classifyBehaviorKind == Array.range 7
#guard allSpecOperations.map classifySpecOperation == Array.range 43
#guard operationRepresentatives.map classifyOperation == Array.range 14
#guard allQuantifierKinds.map classifyQuantifierKind == Array.range 5
#guard allConditionKinds.map classifyConditionKind == Array.range 19
#guard exprKindRepresentatives.map classifyExprKind == Array.range 17

#guard match decodeJson (encodeJson operationConstructorUnit) with
  | .ok decoded => decoded == operationConstructorUnit
  | .error _ => false

#guard let json := Lean.toJson operationConstructorUnit
  (unknownFieldMutations json).all (rejectsUnknownFieldMutation json)

private def classifyAssociatedItemKind : AssociatedItemKind → Nat
  | .type _ _ => 0
  | .constant _ _ => 1
  | .method _ _ => 2

private def classifyAssociatedItemValue : AssociatedItemValue → Nat
  | .type _ => 0
  | .constant _ => 1
  | .method _ => 2

private def typeUse : TypeUse := { typeId := ⟨0⟩, loc := ⟨0⟩ }

private def populatedContract : FunctionContract := {
  loc := some ⟨0⟩
  conditions := allConditions
  modifies := #[⟨0⟩]
  reads := #[typeUse]
  hasFrame := true
  modifiesAll := true
  readsAll := true
  pragmas := allAttributes }

private def associatedItemKinds : Array AssociatedItemKind := #[
  .type allGenericPredicates (some typeUse),
  .constant typeUse (some ⟨0⟩),
  .method {
    generics := allBinderKinds,
    parameters := #[{ name := "value", typeUse, mutable := true }],
    results := #[typeUse],
    predicates := allGenericPredicates } (some qref)]

private def associatedItemValues : Array AssociatedItemValue := #[
  .type typeUse,
  .constant ⟨0⟩,
  .method qref]

private def fieldDecl : FieldDecl := {
  loc := ⟨0⟩, name := ⟨0⟩, type := typeUse, doc := "field" }

private def localDecl : LocalDecl := {
  id := ⟨0⟩, name := "local", type := typeUse, mutable := true, loc := ⟨0⟩ }

private def declarationConstructorUnit : RawUnit :=
  let ns := coreConstructorUnit.namespaces[0]!
  { coreConstructorUnit with
    profiles := #[{
      profile := .rust, name := "rust", version := 1,
      options := #[("panic", "abort")] }]
    dependencies := #[{
      namespaceId := ⟨0⟩, profile := some .rust, exportedNames := #[⟨0⟩] }]
    evidence := #[{
      producer := "wire-test", description := "declaration corpus", trusted := true }]
    namespaces := #[{
      ns with
      imports := #[⟨0⟩]
      profileMetadata := #[extensionValue]
      pragmas := allAttributes
      constants := #[{
        loc := ⟨0⟩, name := ⟨0⟩, type := typeUse, value := ⟨0⟩,
        doc := "constant", profileData := #[extensionValue],
        attributes := allAttributes }]
      structs := #[{
        loc := ⟨0⟩, name := ⟨0⟩, doc := "struct",
        generics := allBinderKinds, fields := #[fieldDecl],
        variants := #[{
          loc := ⟨0⟩, name := ⟨0⟩, fields := #[fieldDecl],
          discriminant := some (-1) }],
        abilities := #[.copy, .drop, .store, .key],
        properties := #[extensionValue], locals := #[localDecl],
        contract := populatedContract, attributes := allAttributes }]
      associatedItems := associatedItemKinds.mapIdx fun index kind => {
        id := ⟨index⟩, loc := ⟨0⟩, owner := ⟨0⟩, name := ⟨0⟩, kind,
        doc := s!"item{index}", attributes := allAttributes }
      traits := #[{
        id := ⟨0⟩, loc := ⟨0⟩, name := ⟨0⟩, doc := "trait",
        generics := allBinderKinds, superTraits := #[traitRef],
        predicates := allGenericPredicates, associatedItems := #[⟨0⟩, ⟨1⟩, ⟨2⟩],
        attributes := allAttributes }]
      implementations := #[{
        id := ⟨0⟩, loc := ⟨0⟩, doc := "impl", generics := allBinderKinds,
        trait := traitRef, target := typeUse, predicates := allGenericPredicates,
        bindings := associatedItemValues.mapIdx fun index value => {
          loc := ⟨0⟩, item := ⟨index⟩, value },
        attributes := allAttributes }]
      functions := #[{
        loc := ⟨0⟩, name := ⟨0⟩, doc := "function", profile := .rust,
        signature := {
          generics := allBinderKinds,
          parameters := #[{ name := "value", typeUse, mutable := true }],
          results := #[typeUse], predicates := allGenericPredicates },
        body := .structured ⟨0⟩, origin := ⟨0⟩, alignment := ⟨0⟩,
        locals := #[localDecl], contract := populatedContract,
        pragmas := allAttributes, profileData := #[extensionValue],
        attributes := allAttributes }]
      specFunctions := #[{
        loc := ⟨0⟩, name := ⟨0⟩, doc := "spec function", profile := .rust,
        signature := {
          generics := allBinderKinds, results := #[typeUse],
          predicates := allGenericPredicates },
        body := some ⟨0⟩, origin := ⟨0⟩, locals := #[localDecl],
        contract := populatedContract, profileData := #[extensionValue] }]
      specVars := #[{
        loc := ⟨0⟩, name := ⟨0⟩, generics := allBinderKinds,
        type := typeUse, profile := .rust, init := some ⟨0⟩,
        locals := #[localDecl], profileData := #[extensionValue] }]
      invariants := #[{
        loc := ⟨0⟩, condition := allConditions[0]!, locals := #[localDecl] }]
      intrinsics := #[{
        loc := ⟨0⟩, model := "model", owner := ⟨0⟩, profile := .rust,
        executableBindings := #[{
          loc := ⟨0⟩, role := "execute", target := qref }],
        specBindings := #[{
          loc := ⟨0⟩, role := "spec", target := qref }] }]
      comments := #[{
        loc := ⟨0⟩, text := "comment", isDoc := true, ownLine := true }] }] }

#guard associatedItemKinds.map classifyAssociatedItemKind == Array.range 3
#guard associatedItemValues.map classifyAssociatedItemValue == Array.range 3

#guard match decodeJson (encodeJson declarationConstructorUnit) with
  | .ok decoded => decoded == declarationConstructorUnit
  | .error _ => false

#guard let json := Lean.toJson declarationConstructorUnit
  (unknownFieldMutations json).all (rejectsUnknownFieldMutation json)

end LeanerIR.Tests.Json
