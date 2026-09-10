-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR
import LeanerIR.Tests.NominalCycles

namespace LeanerIR.Tests.Validation

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

private def testProfile : Profile := .extension ⟨0⟩
private def profile : ProfileConfig := { profile := testProfile, name := "test" }

private def schema : ProfileSchema where
  profile := testProfile
  name := "test"
  checkType := fun value =>
    if value.tag == "word" then #[]
    else #[.error "TEST-TYPE" s!"unknown test type {value.tag}"]
  checkOperation := fun value =>
    if value.tag == "add" then #[]
    else #[.error "TEST-OP" s!"unknown test operation {value.tag}"]
  checkSurface := fun value =>
    if value.tag == "pretty" then #[]
    else #[.error "TEST-SURFACE" s!"unknown test surface {value.tag}"]

private def semantics : SemanticProfile where
  profile := testProfile
  name := "test"
  classify := fun site value => match site, value.tag with
    | .type, "word" => some (.unsupported .executable "test word semantics are unavailable")
    | .operation, "add" => some (.unsupported .executable "test add semantics are unavailable")
    | .call, "add" => some (.unsupported .executable "test add semantics are unavailable")
    | .surface, "pretty" => some .frontendOnly
    | _, _ => none

private def otherProfile : Profile := .extension ⟨1⟩
private def otherConfig : ProfileConfig := { profile := otherProfile, name := "other" }
private def otherSchema : ProfileSchema := { profile := otherProfile, name := "other" }

private def tables : Tables where
  files := #[{ name := "fixture.move" }]
  locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
  origins := #[{ kind := .moveSource, location := ⟨0⟩ }]
  alignments := #[{ source := ⟨0⟩, trust := .checked, description := "fixture" }]
  types := #[.bool]
  namespaces := #[{ segments := #["0x1", "Fixture"] }]
  names := #[{ namespaceId := ⟨0⟩, name := "answer" }]

private def validNamespace : RawNamespace where
  loc := ⟨0⟩
  identity := ⟨0⟩
  profile := some testProfile
  expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) }]
  functions := #[{
    loc := ⟨0⟩
    name := ⟨0⟩
    profile := testProfile
    signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
    body := .structured ⟨0⟩
    origin := ⟨0⟩
    alignment := ⟨0⟩ }]

private def validUnit : RawUnit where
  tables := tables
  profiles := #[profile]
  namespaces := #[validNamespace]

private def withUnitType (raw : RawUnit) : RawUnit :=
  if raw.tables.types.contains .unit then raw
  else { raw with tables := { raw.tables with types := raw.tables.types.push .unit } }

private def constBinderUnit (kind : BinderKind := .const)
    (type : Option TypeUse := some { typeId := ⟨1⟩, loc := ⟨0⟩ }) : RawUnit :=
  let ns := validUnit.namespaces[0]!
  { validUnit with
    tables := { validUnit.tables with
      types := #[.bool, .integer .pointer false] }
    namespaces := #[{ ns with functions := #[{ ns.functions[0]! with
      signature := {
        generics := #[{ name := "N", kind, type, loc := ⟨0⟩ }]
        results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] } }] }] }

#guard (validate #[schema] constBinderUnit).isOk

#guard match validate #[schema] (constBinderUnit (type := none)) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-GENERIC-CONST-TYPE")
  | .ok _ => false

#guard match validate #[schema] (constBinderUnit (kind := .typeArg)) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-GENERIC-CONST-TYPE")
  | .ok _ => false

#guard match validate #[schema]
    (constBinderUnit (type := some { typeId := ⟨2⟩, loc := ⟨0⟩ })) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ID-BOUNDS")
  | .ok _ => false

#guard testProfile != .move && testProfile != .rust && ((.move : Profile) != .rust)

#guard match validate #[schema] validUnit with
  | .ok unit => unit.indexes.namespaceCount == 1 && unit.indexes.functionCounts == #[1]
  | .error _ => false

private def invalidCharacterUnit (value : Nat) : RawUnit :=
  { validUnit with
    tables := { tables with types := #[.character] }
    namespaces := #[{ validNamespace with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.character value) }] }] }

#guard match validate #[schema] (invalidCharacterUnit 0xd800) with
  | .error diagnostics => diagnostics.any (fun diagnostic =>
      diagnostic.code == "LIR-CONSTANT-WELL-FORMED" &&
        diagnostic.message.contains "not a Unicode scalar value")
  | .ok _ => false

#guard match validate #[schema] (invalidCharacterUnit 0x110000) with
  | .error diagnostics => diagnostics.any (fun diagnostic =>
      diagnostic.code == "LIR-CONSTANT-WELL-FORMED")
  | .ok _ => false

private def evidenceUnit : RawUnit :=
  { validUnit with evidence := #[{
      producer := "fixture compiler"
      description := "type checked fixture source"
      trusted := true }] }

#guard match validate #[schema] evidenceUnit with
  | .ok unit => unit.evidence == #[{
      producer := "fixture compiler"
      description := "type checked fixture source"
      trusted := true }]
  | .error _ => false

#guard match validate #[schema] { evidenceUnit with evidence := #[{
    producer := "", description := "claim", trusted := false }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-EVIDENCE-PRODUCER")
  | .ok _ => false

#guard match validate #[schema] { evidenceUnit with evidence := #[{
    producer := "compiler", description := "", trusted := false }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-EVIDENCE-DESCRIPTION")
  | .ok _ => false

private def misplacedExtensionProfileUnit : RawUnit :=
  { validUnit with profiles := #[{ profile := .extension ⟨1⟩, name := "test" }] }

#guard match validate #[schema] misplacedExtensionProfileUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-PROFILE-IDENTITY")
  | .ok _ => false

private def duplicateProfileUnit : RawUnit :=
  { validUnit with profiles := #[profile, profile] }

#guard match validate #[schema] duplicateProfileUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-PROFILE-DUPLICATE")
  | .ok _ => false

private def executableUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with
        signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] } }] }] }

#guard match validate #[schema] executableUnit with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

#guard match validate #[schema] executableUnit with
  | .ok unit => (prepareVerification #[semantics] unit).isOk
  | .error _ => false

private def emptyInitializationCertificate : InitializationCertificate := {
  namespaceId := ⟨0⟩
  functionId := ⟨0⟩
  root := ⟨0⟩
  parameterLocals := #[]
  localCount := 0 }

#guard match validate #[schema] executableUnit with
  | .ok unit => match prepareExecution #[semantics] unit with
      | .ok executable =>
          executable.initializationCertificates == #[emptyInitializationCertificate]
      | .error _ => false
  | .error _ => false

#guard match validate #[schema] executableUnit with
  | .ok unit => match prepareVerification #[semantics] unit with
      | .ok verifiable =>
          verifiable.initializationCertificates == #[emptyInitializationCertificate]
      | .error _ => false
  | .error _ => false

private def nonBooleanConditionUnit (kind : ConditionKind := .requires) : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with
      types := executableUnit.tables.types.push (.integer (.bits 64) false) }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 1) }
      functions := #[{ ns.functions[0]! with contract := {
        loc := some ⟨0⟩
        conditions := #[{
          loc := ⟨0⟩, kind, expression := ⟨1⟩ }] } }] }] }

#guard match validate #[schema] nonBooleanConditionUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "predicate condition expression is not Bool" &&
        diagnostic.primary == some ⟨0⟩
  | .ok _ => false

#guard match validate #[schema] (nonBooleanConditionUnit (.letPre "value")) with
  | .ok unit => match prepareVerification #[semantics] unit with
      | .error diagnostics => !diagnostics.any fun diagnostic =>
          diagnostic.code == "LIR-SEMANTIC-TYPE" && diagnostic.primary == some ⟨0⟩
      | .ok _ => false
  | .error _ => false

private def badEmitsConditionUnit : RawUnit :=
  let ns := (nonBooleanConditionUnit .emits).namespaces[0]!
  { nonBooleanConditionUnit .emits with namespaces := #[{ ns with
      functions := #[{ ns.functions[0]! with contract := {
        loc := some ⟨0⟩
        conditions := #[{
          loc := ⟨0⟩
          kind := .emits
          expression := ⟨1⟩
          auxiliary := #[("emitsCondition", ⟨1⟩)] }] } }] }] }

#guard match validate #[schema] badEmitsConditionUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "emits condition auxiliary is not Bool"
  | .ok _ => false

private def conditionPayloadUnit (kind : ConditionKind) (expression : ExprId)
    (auxiliary : Array (String × ExprId)) : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with types := #[
      .bool, .integer (.bits 64) false, .integer .unbounded true] }
    namespaces := #[{ ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 1) },
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .value (.integer 1) }]
      functions := #[{ ns.functions[0]! with contract := {
        loc := some ⟨0⟩
        conditions := #[{ loc := ⟨0⟩, kind, expression, auxiliary }] } }] }] }

private def verificationHasTypeMessage (raw : RawUnit) (message : String) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" && diagnostic.message.endsWith message
  | .ok _ => false

private def verificationHasConditionMessage (raw : RawUnit) (message : String) : Bool :=
  match validate #[schema] raw with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-CONDITION" && diagnostic.message.endsWith message
  | .ok _ => false

#guard verificationHasTypeMessage
  (conditionPayloadUnit .abortsIf ⟨0⟩ #[("abortCode", ⟨1⟩)])
  "abort-code auxiliary is not logical num"

#guard verificationHasTypeMessage
  (conditionPayloadUnit .abortsWith ⟨1⟩ #[])
  "aborts-with code is not logical num"

#guard verificationHasTypeMessage
  (conditionPayloadUnit .update ⟨1⟩ #[("updateTarget", ⟨0⟩)])
  "update target and value have different types"

#guard match validate #[schema]
    (conditionPayloadUnit .abortsWith ⟨2⟩ #[("additionalCode", ⟨2⟩)]) with
  | .ok unit => match prepareVerification #[semantics] unit with
      | .error diagnostics => !diagnostics.any (·.code == "LIR-SEMANTIC-TYPE")
      | .ok _ => false
  | .error _ => false

#guard verificationHasConditionMessage
  (conditionPayloadUnit .emits ⟨1⟩ #[])
  "condition is missing required auxiliary role `emitsHandle`"

#guard verificationHasConditionMessage
  (conditionPayloadUnit .requires ⟨0⟩ #[("abortCode", ⟨2⟩)])
  "auxiliary role `abortCode` is not valid for LeanerIR.ConditionKind.requires"

#guard verificationHasConditionMessage
  (conditionPayloadUnit .update ⟨1⟩
    #[("updateTarget", ⟨1⟩), ("updateTarget", ⟨1⟩)])
  "condition has duplicate auxiliary role `updateTarget`"

#guard match validate #[schema]
    (conditionPayloadUnit .emits ⟨1⟩ #[("emitsHandle", ⟨1⟩)]) with
  | .ok unit => match prepareVerification #[semantics] unit with
      | .error diagnostics => !diagnostics.any (·.code == "LIR-SEMANTIC-CONDITION")
      | .ok _ => false
  | .error _ => false

private def logicalDeclarationUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with
      types := executableUnit.tables.types.push (.integer (.bits 64) false)
      names := executableUnit.tables.names.push {
        namespaceId := ⟨0⟩, name := "logical" } }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 1) } }] }

private def badQuantifierUnit (badCondition : Bool) : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  let condition := if badCondition then some ⟨1⟩ else none
  let body : ExprId := if badCondition then ⟨0⟩ else ⟨1⟩
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩, typeId := ⟨0⟩,
        kind := .quantifier .forall #[] #[] condition body }
      specFunctions := #[{
        loc := ⟨0⟩
        name := ⟨1⟩
        profile := testProfile
        signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
        body := some ⟨2⟩
        origin := ⟨0⟩ }] }] }

#guard match validate #[schema] (badQuantifierUnit true) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "quantifier condition is not Bool"
  | .ok _ => false

#guard match validate #[schema] (badQuantifierUnit false) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "logical quantifier body is not Bool"
  | .ok _ => false

private def badSpecFunctionResultUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with specFunctions := #[{
      loc := ⟨0⟩
      name := ⟨1⟩
      profile := testProfile
      signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
      body := some ⟨1⟩
      origin := ⟨0⟩ }] }] }

#guard match validate #[schema] badSpecFunctionResultUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith
          "specification function body type differs from its declared results"
  | .ok _ => false

private def mismatchedSpecFunctionProfileUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    profiles := #[profile, otherConfig]
    namespaces := #[{ ns with specFunctions := #[{
      loc := ⟨0⟩
      name := ⟨1⟩
      profile := otherProfile
      signature := {}
      origin := ⟨0⟩ }] }] }

#guard match validate #[schema, otherSchema] mismatchedSpecFunctionProfileUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-PROFILE-MISMATCH" &&
        diagnostic.message.endsWith
          "specification function profile differs from its namespace profile"
  | .ok _ => false

private def badSpecVarInitializerUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with specVars := #[{
      loc := ⟨0⟩
      name := ⟨1⟩
      type := { typeId := ⟨0⟩, loc := ⟨0⟩ }
      profile := testProfile
      init := some ⟨1⟩ }] }] }

#guard match validate #[schema] badSpecVarInitializerUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith
          "specification variable initializer type differs from its declaration"
  | .ok _ => false

private def mismatchedSpecVarProfileUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    profiles := #[profile, otherConfig]
    namespaces := #[{ ns with specVars := #[{
      loc := ⟨0⟩
      name := ⟨1⟩
      type := { typeId := ⟨0⟩, loc := ⟨0⟩ }
      profile := otherProfile }] }] }

#guard match validate #[schema, otherSchema] mismatchedSpecVarProfileUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-PROFILE-MISMATCH" &&
        diagnostic.message.endsWith
          "specification variable profile differs from its namespace profile"
  | .ok _ => false

private def badNamespaceInvariantUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with invariants := #[{
      loc := ⟨0⟩
      condition := { loc := ⟨0⟩, kind := .axiom_, expression := ⟨1⟩ } }] }] }

#guard match validate #[schema] badNamespaceInvariantUnit with
  | .ok unit => match prepareVerification #[semantics] unit with
      | .error diagnostics => diagnostics.any fun diagnostic =>
          diagnostic.code == "LIR-SEMANTIC-TYPE" &&
            diagnostic.message.endsWith "predicate condition expression is not Bool"
      | .ok _ => false
  | .error _ => false

private def badStructInvariantUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with structs := #[{
      loc := ⟨0⟩
      name := ⟨1⟩
      contract := { conditions := #[{
        loc := ⟨0⟩, kind := .structInvariant, expression := ⟨1⟩ }] } }] }] }

#guard match validate #[schema] badStructInvariantUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "predicate condition expression is not Bool"
  | .ok _ => false

private def badSpecFunctionCallUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  let boolUse : TypeUse := { typeId := ⟨0⟩, loc := ⟨0⟩ }
  { executableUnit with
    tables := { executableUnit.tables with
      types := executableUnit.tables.types.push (.integer (.bits 64) false)
      names := executableUnit.tables.names.push {
        namespaceId := ⟨0⟩, name := "predicate" } }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 1) },
        { loc := ⟨0⟩, typeId := ⟨0⟩,
          kind := .operation (.specification (.functionCall {
            namespaceId := ⟨0⟩, name := ⟨1⟩ } {})) #[] #[⟨1⟩] }]
      functions := #[{ ns.functions[0]! with body := .structured ⟨2⟩ }]
      specFunctions := #[{
        loc := ⟨0⟩
        name := ⟨1⟩
        profile := testProfile
        signature := {
          parameters := #[{ name := "value", typeUse := boolUse }]
          results := #[boolUse] }
        origin := ⟨0⟩
        locals := #[{
          id := ⟨0⟩, name := "value", type := boolUse, loc := ⟨0⟩ }] }] }] }

#guard match validate #[schema] badSpecFunctionCallUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.contains "specification function argument types"
  | .ok _ => false

private def behaviorSummaryUnit (resultType : TypeId := ⟨1⟩) : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  let boolUse : TypeUse := { typeId := ⟨0⟩, loc := ⟨0⟩ }
  let integerUse : TypeUse := { typeId := ⟨1⟩, loc := ⟨0⟩ }
  let functionUse : TypeUse := { typeId := ⟨2⟩, loc := ⟨0⟩ }
  { executableUnit with
    tables := { executableUnit.tables with
      types := #[.bool, .integer (.bits 64) false, .function #[⟨1⟩] ⟨1⟩ #[.copy]]
      names := executableUnit.tables.names.push {
        namespaceId := ⟨0⟩, name := "summarized" } }
    namespaces := #[{ ns with
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .localVar ⟨0⟩ },
        { loc := ⟨0⟩, typeId := ⟨1⟩, kind := .localVar ⟨1⟩ },
        { loc := ⟨0⟩, typeId := resultType,
          kind := .operation (.specification (.behavior .resultOf {})) #[] #[⟨0⟩, ⟨1⟩] }]
      functions := #[]
      specFunctions := #[{
        loc := ⟨0⟩
        name := ⟨1⟩
        profile := testProfile
        signature := {
          parameters := #[
            { name := "f", typeUse := functionUse },
            { name := "value", typeUse := integerUse }]
          results := #[if resultType == ⟨0⟩ then boolUse else integerUse] }
        body := some ⟨2⟩
        origin := ⟨0⟩
        locals := #[
          { id := ⟨0⟩, name := "f", type := functionUse, loc := ⟨0⟩ },
          { id := ⟨1⟩, name := "value", type := integerUse, loc := ⟨0⟩ }] }] }] }

#guard match validate #[schema] behaviorSummaryUnit with
  | .ok unit => match prepareVerification #[semantics] unit with
      | .error diagnostics =>
          diagnostics.any (·.code == "LIR-VERIFY-UNSUPPORTED") &&
            !diagnostics.any (·.code == "LIR-SEMANTIC-TYPE")
      | .ok _ => false
  | .error _ => false

#guard match validate #[schema] (behaviorSummaryUnit ⟨0⟩) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "result_of result differs from the callable result type"
  | .ok _ => false

private def badSpecResultUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification (.result 1)) #[] #[] }
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨1⟩ }] } }] }] }

#guard match validate #[schema] badSpecResultUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-RESULT" &&
        diagnostic.message.contains "function has 1 results"
  | .ok _ => false

private def badSpecIdentityTypeUnit (operation : SpecOperation) : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification operation) #[] #[⟨1⟩] }
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨2⟩ }] } }] }] }

#guard match validate #[schema] (badSpecIdentityTypeUnit .old) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "value operation changes its operand type"
  | .ok _ => false

#guard match validate #[schema] (badSpecIdentityTypeUnit .bitVectorToInt) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "bit-vector-to-int result is not logical num"
  | .ok _ => false

#guard match validate #[schema] (badSpecIdentityTypeUnit .noOp) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "value operation changes its operand type"
  | .ok _ => false

#guard match validate #[schema] (badSpecIdentityTypeUnit (.trace .user)) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "value operation changes its operand type"
  | .ok _ => false

#guard match validate #[schema] (badSpecIdentityTypeUnit (.withStateAnchor 0)) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "value operation changes its operand type"
  | .ok _ => false

private def badSpecBuiltinInstantiationUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification .old)
          #[.typeArg { typeId := ⟨1⟩, loc := ⟨0⟩ }] #[⟨0⟩] }
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨2⟩ }] } }] }] }

#guard match validate #[schema] badSpecBuiltinInstantiationUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith
          "specification builtin type argument differs from its value type"
  | .ok _ => false

private def badSpecAnchorMarkerTypeUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨1⟩
        kind := .operation (.specification (.saveStateAnchor 0)) #[] #[] }
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨2⟩] } }] }] }

#guard match validate #[schema] badSpecAnchorMarkerTypeUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "state-anchor marker result is not Bool"
  | .ok _ => false

private def badInlineCallSummaryUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification .inlineCallSummary) #[] #[⟨1⟩, ⟨1⟩] }
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨2⟩ }] } }] }] }

#guard match validate #[schema] badInlineCallSummaryUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "inline-call summary abort operand is not Bool"
  | .ok _ => false

private def badEventStoreInclusionArityUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push .eventStore }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .operation (.specification .emptyEventStore) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨0⟩,
          kind := .operation (.specification .eventStoreIncludes) #[] #[⟨2⟩, ⟨2⟩] }]
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨3⟩ }] } }] }] }

#guard match validate #[schema] badEventStoreInclusionArityUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-ARITY" &&
        diagnostic.message.contains "event-store inclusion expects 1"
  | .ok _ => false

private def badSpecTypeDomainUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push (.typeDomain ⟨1⟩) }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification .typeDomain)
          #[.typeArg { typeId := ⟨1⟩, loc := ⟨0⟩ }] #[] }
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨2⟩ }] } }] }] }

#guard match validate #[schema] badSpecTypeDomainUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "type-domain result is not a type domain"
  | .ok _ => false

private def badSpecResourceDomainUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification .resourceDomain)
          #[.typeArg { typeId := ⟨0⟩, loc := ⟨0⟩ }] #[] }
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨2⟩] } }] }] }

#guard match validate #[schema] badSpecResourceDomainUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "resource-domain type argument is not nominal"
  | .ok _ => false

private def badSpecVectorContainsUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push (.vector ⟨0⟩) }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .value (.vector #[.bool true]) },
        { loc := ⟨0⟩, typeId := ⟨1⟩,
          kind := .operation (.specification .containsVector) #[] #[⟨2⟩, ⟨0⟩] }]
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨3⟩] } }] }] }

#guard match validate #[schema] badSpecVectorContainsUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "vector containment result is not Bool"
  | .ok _ => false

private def badSpecVectorIndexOfUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push (.vector ⟨0⟩) }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .value (.vector #[.bool true]) },
        { loc := ⟨0⟩, typeId := ⟨0⟩,
          kind := .operation (.specification .indexOfVector) #[] #[⟨2⟩, ⟨0⟩] }]
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨3⟩] } }] }] }

#guard match validate #[schema] badSpecVectorIndexOfUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "vector index-of result is not an unbounded integer"
  | .ok _ => false

private def badSpecVectorRangeUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push (.vector ⟨0⟩) }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .value (.vector #[.bool true]) },
        { loc := ⟨0⟩, typeId := ⟨0⟩,
          kind := .operation (.specification .vectorRange) #[] #[⟨2⟩] }]
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨3⟩] } }] }] }

#guard match validate #[schema] badSpecVectorRangeUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "vector range result is not a range"
  | .ok _ => false

private def badSpecLogicalVectorIndexUnit (operation : SpecOperation) : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push (.vector ⟨0⟩) }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .value (.vector #[.bool true]) },
        { loc := ⟨0⟩, typeId := if operation == .updateVector then ⟨2⟩ else ⟨0⟩,
          kind := .operation (.specification operation) #[]
            (if operation == .updateVector then #[⟨2⟩, ⟨1⟩, ⟨0⟩]
             else #[⟨2⟩, ⟨1⟩]) }]
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨3⟩] } }] }] }

#guard match validate #[schema] (badSpecLogicalVectorIndexUnit .updateVector) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "specification vector update index is not logical num"
  | .ok _ => false

#guard match validate #[schema] (badSpecLogicalVectorIndexUnit .inVectorRange) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "vector in-range index is not logical num"
  | .ok _ => false

private def badAbortCodeTypeUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩, typeId := ⟨0⟩,
        kind := .operation (.specification .abortCode) #[] #[] }
      functions := #[{ ns.functions[0]! with contract := {
        conditions := #[{
          loc := ⟨0⟩, kind := .ensures, expression := ⟨2⟩ }] } }] }] }

#guard match validate #[schema] badAbortCodeTypeUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "abort code result is not logical num"
  | .ok _ => false

private def badExtendEventStoreConditionUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push .eventStore }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .operation (.specification .emptyEventStore) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨2⟩,
          kind := .operation (.specification .extendEventStore) #[]
            #[⟨2⟩, ⟨0⟩, ⟨0⟩, ⟨1⟩] }]
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨3⟩] } }] }] }

#guard match validate #[schema] badExtendEventStoreConditionUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "event-store extension condition is not Bool"
  | .ok _ => false

private def badSpecMaxValueUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push (.integer (.bits 8) true) }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨2⟩
        kind := .operation (.specification (.maxValue 8)) #[] #[] }
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨2⟩] } }] }] }

#guard match validate #[schema] badSpecMaxValueUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.contains "unsigned fixed-width integer u8"
  | .ok _ => false

private def badSpecCanModifyUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩, typeId := ⟨0⟩,
        kind := .operation (.specification .canModify)
          #[.typeArg { typeId := ⟨0⟩, loc := ⟨0⟩ }] #[⟨1⟩] }
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨2⟩] } }] }] }

#guard match validate #[schema] badSpecCanModifyUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "can-modify key is not an address"
  | .ok _ => false

private def badSpecResourceMutationUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with
    tables := { logicalDeclarationUnit.tables with
      types := logicalDeclarationUnit.tables.types.push .address }
    namespaces := #[{ ns with
      expressions := ns.expressions ++ #[
        { loc := ⟨0⟩, typeId := ⟨2⟩, kind := .value (.address "0x1") },
        { loc := ⟨0⟩, typeId := ⟨0⟩,
          kind := .operation (.specification (.publish {}))
            #[.typeArg { typeId := ⟨0⟩, loc := ⟨0⟩ }] #[⟨1⟩, ⟨0⟩] }]
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨3⟩] } }] }] }

#guard match validate #[schema] badSpecResourceMutationUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "specification resource address is not an address"
  | .ok _ => false

private def badSpecGlobalUnit : RawUnit :=
  let ns := logicalDeclarationUnit.namespaces[0]!
  { logicalDeclarationUnit with namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.specification (.global none))
          #[.typeArg { typeId := ⟨1⟩, loc := ⟨0⟩ }] #[⟨1⟩] }
      functions := #[{ ns.functions[0]! with contract := {
        modifies := #[⟨2⟩] } }] }] }

#guard match validate #[schema] badSpecGlobalUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.endsWith "specification global result differs from its resource type"
  | .ok _ => false

private def badLiteralTypeUnit : RawUnit :=
  { executableUnit with namespaces := #[{
      executableUnit.namespaces[0]! with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.integer 7) }] }] }

#guard match validate #[schema] badLiteralTypeUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" && diagnostic.primary == some ⟨0⟩
  | .ok _ => false

private def integerLiteralUnit (value : Int) (width : Nat) (signed : Bool) : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with types := #[.integer (.bits width) signed] }
    namespaces := #[{
      ns with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.integer value) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
        body := .structured ⟨0⟩ }] }] }

#guard match validate #[schema] (integerLiteralUnit 255 8 false) with
  | .ok _ => true
  | .error _ => false

#guard match validate #[schema] (integerLiteralUnit 256 8 false) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" && diagnostic.primary == some ⟨0⟩
  | .ok _ => false

#guard match validate #[schema] (integerLiteralUnit (-128) 8 true) with
  | .ok _ => true
  | .error _ => false

#guard match validate #[schema] (integerLiteralUnit 128 8 true) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-SEMANTIC-TYPE")
  | .ok _ => false

#guard match validate #[schema] (integerLiteralUnit (-129) 8 true) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-SEMANTIC-TYPE")
  | .ok _ => false

private def vectorExecutionUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with
      locations := executableUnit.tables.locations.push {
        primary := some { file := ⟨0⟩, startByte := 2, endByte := 3 } }
      types := #[.bool, .vector ⟨0⟩ none] }
    namespaces := #[{
      ns with
      expressions := #[{ loc := ⟨1⟩, typeId := ⟨1⟩, kind := .value (.vector #[]) }]
      functions := #[{ ns.functions[0]! with
        signature := { results := #[{ typeId := ⟨1⟩, loc := ⟨1⟩ }] }
        body := .structured ⟨0⟩ }] }] }

#guard match validate #[schema] vectorExecutionUnit with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

private def withExtraType (ty : Ty) : RawUnit :=
  { validUnit with tables := { validUnit.tables with types := #[.bool, ty] } }

#guard match validate #[schema] (withExtraType (.integer (.bits 0) false)) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-TYPE-WELL-FORMED" &&
        diagnostic.message.contains "zero-width fixed integer"
  | .ok _ => false

#guard match validate #[schema] (withExtraType (.vector ⟨0⟩ (some (.integer (-1))))) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-TYPE-WELL-FORMED" &&
        diagnostic.message.contains "negative fixed-vector length"
  | .ok _ => false

#guard match validate #[schema] (withExtraType (.vector ⟨0⟩ (some (.integer 0)))) with
  | .ok _ => true
  | .error _ => false

#guard match validate #[schema] (withExtraType (.vector ⟨0⟩ (some (.bool true)))) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-TYPE-WELL-FORMED" &&
        diagnostic.message.contains "non-integer fixed-vector length"
  | .ok _ => false

#guard match validate #[schema]
    (withExtraType (.function #[⟨0⟩] ⟨0⟩ #[.copy, .copy])) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-ABILITY-DUPLICATE" &&
        diagnostic.message.contains "function type table entry 1"
  | .ok _ => false

private def nonTypeBinderAbilityUnit : RawUnit :=
  let function := validNamespace.functions[0]!
  { validUnit with namespaces := #[{ validNamespace with functions := #[{
      function with signature := { function.signature with generics := #[{
        name := "a", kind := .lifetime, abilities := #[.copy], loc := ⟨0⟩ }] } }] }] }

#guard match validate #[schema] nonTypeBinderAbilityUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-GENERIC-ABILITY-KIND" && diagnostic.primary == some ⟨0⟩
  | .ok _ => false

private def namedGenericBindersUnit (first second : String) : RawUnit :=
  let function := validNamespace.functions[0]!
  { validUnit with namespaces := #[{ validNamespace with functions := #[{
      function with signature := { function.signature with generics := #[
        { name := first, kind := .lifetime, loc := ⟨0⟩ },
        { name := second, kind := .lifetime, loc := ⟨0⟩ }] } }] }] }

#guard match validate #[schema] (namedGenericBindersUnit "a" "a") with
  | .error diagnostics => diagnostics.any (·.code == "LIR-GENERIC-NAME-DUPLICATE")
  | .ok _ => false

#guard match validate #[schema] (namedGenericBindersUnit "" "a") with
  | .error diagnostics => diagnostics.any (·.code == "LIR-GENERIC-NAME")
  | .ok _ => false

#guard match validate #[schema] (namedGenericBindersUnit "a" "b") with
  | .ok _ => true
  | .error _ => false

#guard match validate #[schema] (withExtraType (.typeDomain ⟨1⟩)) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-ARENA-CYCLE" && diagnostic.message.contains "type arena"
  | .ok _ => false

#guard match validate #[schema]
    (withExtraType (.resourceDomain ⟨0⟩ (some #[⟨1⟩]))) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-ARENA-CYCLE" && diagnostic.message.contains "type arena"
  | .ok _ => false

private def unresolvedResourceDomainUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with
      types := executableUnit.tables.types.push (.resourceDomain ⟨0⟩) }
    namespaces := #[{ ns with functions := #[{ ns.functions[0]! with
      signature := { results := #[{ typeId := ⟨1⟩, loc := ⟨0⟩ }] } }] }] }

#guard match validate #[schema] unresolvedResourceDomainUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TARGET" &&
        diagnostic.message.contains "resource domain"
  | .ok _ => false

private def badResourceDomainArityUnit : RawUnit :=
  let ns := unresolvedResourceDomainUnit.namespaces[0]!
  { unresolvedResourceDomainUnit with namespaces := #[{ ns with structs := #[{
      loc := ⟨0⟩
      name := ⟨0⟩
      generics := #[{ name := "T", kind := .typeArg, loc := ⟨0⟩ }] }] }] }

#guard match validate #[schema] badResourceDomainArityUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-ARITY" &&
        diagnostic.message.contains "resource domain expects 1"
  | .ok _ => false

private def callable : QualifiedRef := { namespaceId := ⟨0⟩, name := ⟨0⟩ }

private def callKindsUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      structs := #[{ loc := ⟨0⟩, name := ⟨0⟩ }]
      expressions := #[
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.call (.function callable)) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.call (.constructor callable)) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.call (.destructor callable)) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.call (.closure callable)) #[] #[] },
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation (.call .invoke) #[] #[⟨0⟩] },
        { loc := ⟨0⟩, typeId := ⟨0⟩, kind := .operation
            (.call (.extension { profile := testProfile, tag := "add" })) #[] #[] }] }] }

#guard match validate #[schema] callKindsUnit with
  | .ok unit => unit.namespaces[0]!.expressions.size == 6
  | .error _ => false

private def extensionSurface (tag : String) : SurfaceSyntax :=
  .extension { profile := testProfile, tag := tag }

private def surfaceUnit (tag : String) : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := (.operation
        (.call (.function callable)) #[] #[]
        (some (extensionSurface tag))) }] }] }

#guard match validate #[schema] (surfaceUnit "pretty") with
  | .ok unit => unit.namespaces[0]!.expressions.size == 1
  | .error _ => false

#guard match validate #[schema] (surfaceUnit "unknown") with
  | .error diagnostics => diagnostics.any (·.code == "TEST-SURFACE")
  | .ok _ => false

private def missingSemantics : SemanticProfile where
  profile := testProfile
  name := "test"
  classify := fun _ _ => none

#guard match validate #[schema] (surfaceUnit "pretty") with
  | .ok unit => match prepareExecution #[missingSemantics] unit with
      | .error diagnostics => diagnostics.any fun diagnostic =>
          diagnostic.code == "LIR-SEMANTICS-UNCLASSIFIED" &&
            diagnostic.primary == some ⟨0⟩
      | .ok _ => false
  | .error _ => false

private def coreUnionUnit : RawUnit :=
  { validUnit with
    tables := { tables with types := #[
      .bool,
      .function #[⟨0⟩] ⟨0⟩ #[.copy, .drop]] }
    namespaces := #[{
      validNamespace with
      attributes := #[.call "cfg" #[.assign "feature" (.constant (.string "borrow-check"))]]
      traits := #[{ id := ⟨0⟩, loc := ⟨0⟩, name := ⟨0⟩ }]
      structs := #[{
        loc := ⟨0⟩
        name := ⟨0⟩
        generics := #[{
          name := "T"
          kind := .typeArg
          abilities := #[.copy]
          predicates := #[.implements ⟨0⟩ { trait := callable }]
          loc := ⟨0⟩ }]
        abilities := #[.drop, .store, .key] }] }] }

#guard match validate #[schema] coreUnionUnit with
  | .ok unit =>
      unit.namespaces[0]!.attributes.size == 1 &&
        unit.namespaces[0]!.structs[0]!.abilities.size == 3
  | .error _ => false

private def traitTables : Tables :=
  { tables with
    types := #[.bool, .typeParameter 0]
    names := #[
      { namespaceId := ⟨0⟩, name := "Step" },
      { namespaceId := ⟨0⟩, name := "step" },
      { namespaceId := ⟨0⟩, name := "bool_step" }
    ] }

private def stepTraitRef (typeId : TypeId) : TraitRef :=
  { trait := { namespaceId := ⟨0⟩, name := ⟨0⟩ }
    arguments := #[.typeArg { typeId, loc := ⟨0⟩ }] }

private def traitUnit : RawUnit :=
  { validUnit with
    tables := traitTables
    namespaces := #[{
      validNamespace with
      functions := #[{
        loc := ⟨0⟩
        name := ⟨2⟩
        profile := testProfile
        signature := {
          generics := #[{
            name := "T"
            kind := .typeArg
            predicates := #[.implements ⟨1⟩ (stepTraitRef ⟨1⟩)]
            loc := ⟨0⟩ }]
          parameters := #[{
            name := "value"
            typeUse := { typeId := ⟨1⟩, loc := ⟨0⟩ } }] }
        body := .absent
        origin := ⟨0⟩
        alignment := ⟨0⟩
        locals := #[{
          id := ⟨0⟩
          name := "value"
          type := { typeId := ⟨1⟩, loc := ⟨0⟩ }
          loc := ⟨0⟩ }] }]
      associatedItems := #[{
        id := ⟨0⟩
        loc := ⟨0⟩
        owner := ⟨0⟩
        name := ⟨1⟩
        kind := .method {
          parameters := #[{
            name := "self"
            typeUse := { typeId := ⟨1⟩, loc := ⟨0⟩ } }] } }]
      traits := #[{
        id := ⟨0⟩
        loc := ⟨0⟩
        name := ⟨0⟩
        generics := #[{ name := "T", kind := .typeArg, loc := ⟨0⟩ }]
        associatedItems := #[⟨0⟩] }]
      implementations := #[{
        id := ⟨0⟩
        loc := ⟨0⟩
        trait := stepTraitRef ⟨0⟩
        target := { typeId := ⟨0⟩, loc := ⟨0⟩ }
        bindings := #[{
          loc := ⟨0⟩
          item := ⟨0⟩
          value := .method { namespaceId := ⟨0⟩, name := ⟨2⟩ } }] }] }] }

#guard match validate #[schema] traitUnit with
  | .ok unit =>
      unit.namespaces[0]!.traits.size == 1 &&
        unit.namespaces[0]!.associatedItems.size == 1 &&
        unit.namespaces[0]!.implementations.size == 1
  | .error _ => false

private def cyclicSuperTraitUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  let trait := ns.traits[0]!
  { traitUnit with namespaces := #[{ ns with traits := #[{
      trait with superTraits := #[stepTraitRef ⟨1⟩] }] }] }

#guard match validate #[schema] cyclicSuperTraitUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-TRAIT-CYCLE" && diagnostic.primary == some ⟨0⟩
  | .ok _ => false

private def orphanAssociatedItemUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  { traitUnit with namespaces := #[{ ns with
      traits := #[{ ns.traits[0]! with associatedItems := #[] }] }] }

#guard match validate #[schema] orphanAssociatedItemUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ASSOCIATED-ITEM-MEMBERSHIP")
  | .ok _ => false

private def duplicateAssociatedItemUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  { traitUnit with namespaces := #[{ ns with
      traits := #[{ ns.traits[0]! with associatedItems := #[⟨0⟩, ⟨0⟩] }] }] }

#guard match validate #[schema] duplicateAssociatedItemUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ASSOCIATED-ITEM-DUPLICATE")
  | .ok _ => false

private def duplicateAssociatedItemNameUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  let duplicate : AssociatedItemDecl := { ns.associatedItems[0]! with id := ⟨1⟩ }
  { traitUnit with namespaces := #[{ ns with
      associatedItems := ns.associatedItems.push duplicate
      traits := #[{ ns.traits[0]! with associatedItems := #[⟨0⟩, ⟨1⟩] }] }] }

#guard match validate #[schema] duplicateAssociatedItemNameUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ASSOCIATED-ITEM-NAME-DUPLICATE")
  | .ok _ => false

private def sameAssociatedTypeAndMethodNameUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  let typeItem := { ns.associatedItems[0]! with id := ⟨1⟩ }
  let typeItem : AssociatedItemDecl := { typeItem with
    kind := .type #[] (some { typeId := ⟨0⟩, loc := ⟨0⟩ }) }
  { traitUnit with namespaces := #[{ ns with
      associatedItems := ns.associatedItems.push typeItem
      traits := #[{ ns.traits[0]! with associatedItems := #[⟨0⟩, ⟨1⟩] }] }] }

#guard match validate #[schema] sameAssociatedTypeAndMethodNameUnit with
  | .ok _ => true
  | .error _ => false

private def missingAssociatedBindingUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  { traitUnit with namespaces := #[{ ns with implementations := #[{
      ns.implementations[0]! with bindings := #[] }] }] }

#guard match validate #[schema] missingAssociatedBindingUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ASSOCIATED-BINDING-MISSING")
  | .ok _ => false

private def defaultedAssociatedBindingUnit : RawUnit :=
  let ns := missingAssociatedBindingUnit.namespaces[0]!
  let item := ns.associatedItems[0]!
  match item.kind with
  | .method signature _ =>
      { missingAssociatedBindingUnit with namespaces := #[{ ns with
          associatedItems := #[{ item with kind := .method signature (some {
            namespaceId := ⟨0⟩, name := ⟨2⟩ }) }] }] }
  | _ => traitUnit

#guard (validate #[schema] defaultedAssociatedBindingUnit).isOk

private def foreignOwnerAssociatedBindingUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  let secondTrait : TraitDecl := { ns.traits[0]! with id := ⟨1⟩, associatedItems := #[⟨0⟩] }
  { traitUnit with namespaces := #[{ ns with
      associatedItems := #[{ ns.associatedItems[0]! with owner := ⟨1⟩ }]
      traits := #[{ ns.traits[0]! with associatedItems := #[] }, secondTrait] }] }

#guard match validate #[schema] foreignOwnerAssociatedBindingUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ASSOCIATED-BINDING-OWNER")
  | .ok _ => false

/- Trait method parameters are signature declarations, not body locals. The
owner trait's binder remains in scope without a synthetic `LocalId`. -/
#guard match validate #[schema] traitUnit with
  | .ok unit => match unit.namespaces[0]!.associatedItems[0]!.kind with
      | .method signature _ => signature.parameters.size == 1
      | _ => false
  | .error _ => false

private def outOfScopeGenericUnit : RawUnit :=
  let ns := traitUnit.namespaces[0]!
  let function := ns.functions[0]!
  { traitUnit with namespaces := #[{ ns with functions := #[{
      function with signature := { function.signature with generics := #[] } }] }] }

#guard match validate #[schema] outOfScopeGenericUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-GENERIC-SCOPE")
  | .ok _ => false

/- Expression annotations are scoped by their owning body rather than by the
global type arena, so they receive a second semantic scope check after raw
declaration validation. -/
private def outOfScopeBodyTypeUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with
      types := executableUnit.tables.types.push (.typeParameter 0) }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨1⟩
        kind := .operation (.primitive .logicalNot) #[] #[⟨0⟩] }
      functions := #[{ ns.functions[0]! with body := .structured ⟨1⟩ }] }] }

#guard match validate #[schema] outOfScopeBodyTypeUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-SEMANTIC-GENERIC-SCOPE")
  | .ok _ => false

private def outOfScopeBodyLifetimeUnit : RawUnit :=
  let ns := executableUnit.namespaces[0]!
  { executableUnit with
    tables := { executableUnit.tables with
      lifetimes := #[{ kind := .parameter 0, loc := ⟨0⟩, name := some "'a" }]
      types := executableUnit.tables.types.push (.reference {
        profile := testProfile
        kind := .shared
        referent := ⟨0⟩
        lifetime := ⟨0⟩ }) }
    namespaces := #[{ ns with
      expressions := ns.expressions.push {
        loc := ⟨0⟩
        typeId := ⟨1⟩
        kind := .operation (.primitive .copyValue) #[] #[⟨0⟩] }
      functions := #[{ ns.functions[0]! with body := .structured ⟨1⟩ }] }] }

#guard match validate #[schema] outOfScopeBodyLifetimeUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-SEMANTIC-GENERIC-SCOPE")
  | .ok _ => false

private def compilationUnitTablesUnit : RawUnit :=
  let second : RawNamespace := {
    loc := ⟨0⟩
    identity := ⟨1⟩
    profile := some testProfile
    imports := #[⟨2⟩] }
  { validUnit with
    tables := { tables with
      namespaces := #[
        { segments := #["0x1", "Fixture"] },
        { segments := #["0x1", "Second"] },
        { segments := #["0x2", "Dependency"] }]
      names := tables.names.push { namespaceId := ⟨2⟩, name := "external" } }
    namespaces := #[validNamespace, second]
    dependencies := #[{
      namespaceId := ⟨2⟩
      profile := some testProfile
      exportedNames := #[⟨1⟩] }] }

#guard match validate #[schema] compilationUnitTablesUnit with
  | .ok unit =>
      unit.tables.namespaces.size == 3 && unit.namespaces.size == 2 &&
        unit.namespaces.all (·.tables == unit.tables)
  | .error _ => false

private def hasDiagnostic (code : String) (unit : RawUnit) : Bool :=
  match validate #[schema] unit with
  | .error diagnostics => diagnostics.any (·.code == code)
  | .ok _ => false

private def selfImportUnit : RawUnit :=
  let ns := compilationUnitTablesUnit.namespaces[0]!
  { compilationUnitTablesUnit with
    namespaces := compilationUnitTablesUnit.namespaces.set! 0
      { ns with imports := #[⟨0⟩] } }

#guard hasDiagnostic "LIR-IMPORT-SELF" selfImportUnit

private def duplicateImportUnit : RawUnit :=
  let ns := compilationUnitTablesUnit.namespaces[1]!
  { compilationUnitTablesUnit with
    namespaces := compilationUnitTablesUnit.namespaces.set! 1
      { ns with imports := #[⟨2⟩, ⟨2⟩] } }

#guard hasDiagnostic "LIR-IMPORT-DUPLICATE" duplicateImportUnit

private def undeclaredExternalImportUnit : RawUnit :=
  { compilationUnitTablesUnit with dependencies := #[] }

#guard hasDiagnostic "LIR-IMPORT-INTERFACE" undeclaredExternalImportUnit

private def ownedDependencyUnit : RawUnit :=
  { compilationUnitTablesUnit with dependencies := #[{
      namespaceId := ⟨0⟩
      profile := some testProfile
      exportedNames := #[⟨0⟩] }] }

#guard hasDiagnostic "LIR-DEPENDENCY-OWNED" ownedDependencyUnit

private def duplicateDependencyUnit : RawUnit :=
  { compilationUnitTablesUnit with
    dependencies := compilationUnitTablesUnit.dependencies ++
      compilationUnitTablesUnit.dependencies }

#guard hasDiagnostic "LIR-DEPENDENCY-DUPLICATE" duplicateDependencyUnit

private def duplicateDependencyNameUnit : RawUnit :=
  let dependency := compilationUnitTablesUnit.dependencies[0]!
  { compilationUnitTablesUnit with
    dependencies := #[{ dependency with exportedNames := #[⟨1⟩, ⟨1⟩] }] }

#guard hasDiagnostic "LIR-DEPENDENCY-NAME-DUPLICATE" duplicateDependencyNameUnit

private def unconfiguredDependencyProfileUnit : RawUnit :=
  let dependency := compilationUnitTablesUnit.dependencies[0]!
  { compilationUnitTablesUnit with
    dependencies := #[{ dependency with profile := some .rust }] }

#guard hasDiagnostic "LIR-PROFILE-CONFIG" unconfiguredDependencyProfileUnit

#guard match decodeJson (encodeJson traitUnit) with
  | .ok decoded => match validate #[schema] decoded with
      | .ok unit => unit.namespaces[0]!.functions[0]!.signature.generics.size == 1
      | .error _ => false
  | .error _ => false

private def wrongAssociatedBindingKindUnit : RawUnit :=
  { traitUnit with namespaces := #[{
      traitUnit.namespaces[0]! with
      implementations := #[{ traitUnit.namespaces[0]!.implementations[0]! with
        bindings := #[{ loc := ⟨0⟩, item := ⟨0⟩, value := .type {
          typeId := ⟨0⟩, loc := ⟨0⟩ } }] }] }] }

#guard match validate #[schema] wrongAssociatedBindingKindUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ASSOCIATED-BINDING-KIND")
  | .ok _ => false

private def unsupportedVersionUnit : RawUnit :=
  { validUnit with version := { major := 2, minor := 0 } }

#guard match validate #[schema] unsupportedVersionUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-SCHEMA-VERSION")
  | .ok _ => false

private def cyclicUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .block #[⟨0⟩] none }] }] }

#guard match validate #[schema] cyclicUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ARENA-CYCLE")
  | .ok _ => false

private def cyclicTypeUnit : RawUnit :=
  { validUnit with tables := { tables with types := #[.tuple #[⟨0⟩]] } }

#guard match validate #[schema] cyclicTypeUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ARENA-CYCLE")
  | .ok _ => false

private def vectorUnit : RawUnit :=
  { validUnit with tables := { tables with types := #[.bool, .vector ⟨0⟩ none] } }

#guard match validate #[schema] vectorUnit with
  | .ok unit => unit.namespaces[0]!.tables.types[1]? == some (.vector ⟨0⟩ none)
  | .error _ => false

private def fixedVectorUnit : RawUnit :=
  { validUnit with
    tables := { tables with types := #[.bool, .vector ⟨0⟩ (some (.integer 4))] } }

#guard match validate #[schema] fixedVectorUnit with
  | .ok unit =>
      unit.namespaces[0]!.tables.types[1]? == some (.vector ⟨0⟩ (some (.integer 4)))
  | .error _ => false

private def cyclicVectorUnit : RawUnit :=
  { validUnit with tables := { tables with types := #[.vector ⟨0⟩ none] } }

#guard match validate #[schema] cyclicVectorUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ARENA-CYCLE")
  | .ok _ => false

private def referenceUnit : RawUnit :=
  { validUnit with tables := { tables with
      lifetimes := #[{ kind := .inference, loc := ⟨0⟩ }]
      types := #[.bool, .reference {
        profile := testProfile
        kind := .shared
        referent := ⟨0⟩
        lifetime := ⟨0⟩ }] } }

#guard match validate #[schema] referenceUnit with
  | .ok unit => match unit.namespaces[0]!.tables.types[1]? with
      | some (Ty.reference reference) =>
          reference.referent == ⟨0⟩ && reference.lifetime == ⟨0⟩
      | _ => false
  | .error _ => false

private def badReferenceLifetimeUnit : RawUnit :=
  { referenceUnit with tables := { referenceUnit.tables with
      types := #[.bool, .reference {
        profile := testProfile
        kind := .mutable
        referent := ⟨0⟩
        lifetime := ⟨7⟩ }] } }

#guard match validate #[schema] badReferenceLifetimeUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-ID-BOUNDS" && diagnostic.message.contains "lifetime"
  | .ok _ => false

private def cyclicReferenceUnit : RawUnit :=
  { referenceUnit with tables := { referenceUnit.tables with
      types := #[.reference {
        profile := testProfile
        kind := .shared
        referent := ⟨0⟩
        lifetime := ⟨0⟩ }] } }

#guard match validate #[schema] cyclicReferenceUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ARENA-CYCLE")
  | .ok _ => false

private def badIdUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨3⟩, kind := .value (.bool true) }] }] }

#guard match validate #[schema] badIdUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-ID-BOUNDS")
  | .ok _ => false

private def cfgUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[{ loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] cfgUnit with
  | .ok unit => match unit.namespaces[0]!.functions[0]!.body with
      | .structured root => root.index >= validNamespace.expressions.size
      | .absent => false
  | .error _ => false

private def cfgWithDeadBlockUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] },
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ }] } }] }] }

#guard match validate #[schema] cfgWithDeadBlockUnit with
  | .ok unit => (unit.namespaces[0]!.functions[0]!.body matches .structured _)
  | .error _ => false

private def loopCfgUnit : RawUnit :=
  withUnitType { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ },
          { loc := ⟨0⟩, terminator := .branch ⟨0⟩ ⟨2⟩ ⟨3⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ },
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] loopCfgUnit with
  | .ok unit =>
      let ns := unit.namespaces[0]!
      ns.expressions.any (·.kind matches .loop ..)
  | .error _ => false

#guard match validate #[schema] { loopCfgUnit with
    tables := { loopCfgUnit.tables with types := #[.bool] } } with
  | .error diagnostics => diagnostics.any fun x => x.code == "LIR-CFG-UNIT-TYPE"
  | .ok _ => false

private def executableLoopCfgUnit : RawUnit :=
  let ns := loopCfgUnit.namespaces[0]!
  { loopCfgUnit with namespaces := #[{ ns with functions := #[{
      ns.functions[0]! with
      signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] } }] }] }

#guard match validate #[schema] executableLoopCfgUnit with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

private def exitingLoopCfgUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  withUnitType { validUnit with namespaces := #[{ ns with functions := #[{
      ns.functions[0]! with
      signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
      body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ },
          { loc := ⟨0⟩, terminator := .branch ⟨0⟩ ⟨2⟩ ⟨3⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨4⟩ },
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] exitingLoopCfgUnit with
  | .ok unit =>
      let ns := unit.namespaces[0]!
      ns.expressions.any (fun expression =>
        (expression.kind matches .loop ..) && expression.typeId == ⟨1⟩) &&
        (prepareExecution #[semantics] unit).isOk
  | .error _ => false

private def loopWithReturnCfgUnit : RawUnit :=
  withUnitType { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ },
          { loc := ⟨0⟩, terminator := .branch ⟨0⟩ ⟨3⟩ ⟨2⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨6⟩ },
          { loc := ⟨0⟩, terminator := .branch ⟨0⟩ ⟨4⟩ ⟨5⟩ },
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] },
          { loc := ⟨0⟩, terminator := .goto ⟨1⟩ },
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] loopWithReturnCfgUnit with
  | .ok unit => unit.namespaces[0]!.expressions.any fun expression =>
      match expression.kind with | .loop .. => true | _ => false
  | .error _ => false

private def integerSwitchCfgUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  withUnitType { validUnit with
    tables := { validUnit.tables with types := #[.bool, .integer (.bits 32) false] }
    namespaces := #[{ ns with
      expressions := #[{ loc := ⟨0⟩, typeId := ⟨1⟩, kind := .value (.integer 1) }]
      functions := #[{ ns.functions[0]! with signature := {}, body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩, terminator := .switch ⟨0⟩
              #[(.integer 0, ⟨1⟩), (.integer 1, ⟨2⟩)] ⟨3⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨4⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨4⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨4⟩ },
          { loc := ⟨0⟩, terminator := .return_ #[] }] } }] }] }

#guard match validate #[schema] integerSwitchCfgUnit with
  | .ok unit =>
      let ns := unit.namespaces[0]!
      ns.patterns.size == 3 && ns.expressions.any fun expression =>
        match expression.kind with
        | .match_ ⟨0⟩ arms =>
            arms.size == 3 && ns.tables.types[expression.typeId.index]? == some .unit
        | _ => false
  | .error _ => false

#guard match validate #[schema] integerSwitchCfgUnit with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

private def irreducibleCfgUnit : RawUnit :=
  withUnitType { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩, terminator := .branch ⟨0⟩ ⟨1⟩ ⟨2⟩ },
          { loc := ⟨0⟩, terminator := .goto ⟨2⟩ },
          { loc := ⟨0⟩, terminator := .branch ⟨0⟩ ⟨1⟩ ⟨3⟩ },
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] irreducibleCfgUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-CFG-NOT-STRUCTURABLE")
  | .ok _ => false

private def rawMirStatementUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  { validUnit with namespaces := #[{ ns with
      places := #[.localVar ⟨0⟩]
      functions := #[{ ns.functions[0]! with
        locals := #[{
          id := ⟨0⟩, name := "temporary", type := { typeId := ⟨0⟩, loc := ⟨0⟩ },
          loc := ⟨0⟩ }]
        body := .cfg {
          entry := ⟨0⟩
          blocks := #[{
            loc := ⟨0⟩
            statements := #[
              .storageLive ⟨0⟩,
              .placeMention ⟨0⟩,
              .ascribeUserType ⟨0⟩ { typeId := ⟨0⟩, loc := ⟨0⟩ },
              .storageDead ⟨0⟩]
            terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] rawMirStatementUnit with
  | .ok unit => match unit.namespaces[0]!.functions[0]!.body with
      | .structured _ => true
      | .absent => false
  | .error _ => false

private def destructiveRawMirStatementUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  { validUnit with namespaces := #[{ ns with
      places := #[.localVar ⟨0⟩]
      functions := #[{ ns.functions[0]! with
        locals := #[{
          id := ⟨0⟩, name := "temporary", type := { typeId := ⟨0⟩, loc := ⟨0⟩ },
          loc := ⟨0⟩ }]
        body := .cfg {
          entry := ⟨0⟩
          blocks := #[{
            loc := ⟨0⟩
            statements := #[.deinit ⟨0⟩]
            terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] destructiveRawMirStatementUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-CFG-MIR-STATEMENT")
  | .ok _ => false

private def rawMirAssertionUnit (expected : Bool)
    (unwind : RawUnwindAction := .unreachable) : RawUnit :=
  let ns := validUnit.namespaces[0]!
  withUnitType { validUnit with namespaces := #[{ ns with
      functions := #[{ ns.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[
          { loc := ⟨0⟩
            terminator := .assert ⟨0⟩ expected .boundsCheck ⟨1⟩ unwind },
          { loc := ⟨0⟩, terminator := .return_ #[⟨0⟩] }] } }] }] }

#guard match validate #[schema] (rawMirAssertionUnit true) with
  | .ok unit => unit.namespaces[0]!.expressions.any fun expression =>
      expression.kind == .throw_ .panic #[]
  | .error _ => false

#guard match validate #[schema] (rawMirAssertionUnit false) with
  | .ok unit => unit.namespaces[0]!.expressions.any fun expression =>
      match expression.kind with
      | .ifElse ⟨0⟩ thenBranch (some _) =>
          unit.namespaces[0]!.expressions[thenBranch.index]?.any
            (·.kind == .throw_ .panic #[])
      | _ => false
  | .error _ => false

private def executableAssertionUnit : RawUnit :=
  let raw := rawMirAssertionUnit true
  let ns := raw.namespaces[0]!
  { raw with namespaces := #[{ ns with functions := #[{
      ns.functions[0]! with
      signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] } }] }] }

#guard match validate #[schema] executableAssertionUnit with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

#guard match validate #[schema] (rawMirAssertionUnit true .continue_) with
  | .error diagnostics => diagnostics.any (·.code == "LIR-CFG-MIR-TERMINATOR")
  | .ok _ => false

private def rawMirAbortUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  { validUnit with namespaces := #[{ ns with functions := #[{
      ns.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[{ loc := ⟨0⟩, terminator := .abort }] } }] }] }

#guard match validate #[schema] rawMirAbortUnit with
  | .ok unit =>
      let ns := unit.namespaces[0]!
      match ns.functions[0]!.body with
      | .structured root =>
          ns.expressions[root.index]?.any (·.kind == .throw_ .panic #[])
      | .absent => false
  | .error _ => false

#guard match validate #[schema] rawMirAbortUnit with
  | .ok unit => (prepareExecution #[semantics] unit).isOk
  | .error _ => false

private def pointerAssertionUnit : RawUnit :=
  let base := rawMirAssertionUnit true
  let ns := base.namespaces[0]!
  let function := ns.functions[0]!
  let graph := match function.body with
    | .cfg graph => graph
    | _ => { entry := ⟨0⟩, blocks := #[] }
  let first := graph.blocks[0]!
  { base with namespaces := #[{ ns with functions := #[{
      function with body := .cfg { graph with blocks := graph.blocks.set! 0 {
        first with terminator :=
          (.assert ⟨0⟩ true .misalignedPointerDereference ⟨1⟩ .unreachable : RawTerminator) } }
    }] }] }

#guard match validate #[schema] pointerAssertionUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-CFG-MIR-TERMINATOR")
  | .ok _ => false

private def rawMirTerminatorUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  { validUnit with namespaces := #[{ ns with
      functions := #[{ ns.functions[0]! with body := .cfg {
        entry := ⟨0⟩
        blocks := #[{
          loc := ⟨0⟩
          terminator := .call ⟨0⟩ none (.terminate "panic") }] } }] }] }

#guard match validate #[schema] rawMirTerminatorUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-CFG-MIR-TERMINATOR")
  | .ok _ => false

private def rawCallContinuationUnit : RawUnit :=
  let ns := validUnit.namespaces[0]!
  { validUnit with
    tables := { validUnit.tables with types := #[.bool, .unit] }
    namespaces := #[{ ns with
      places := #[.localVar ⟨0⟩]
      functions := #[{ ns.functions[0]! with
        signature := {}
        locals := #[{
          id := ⟨0⟩, name := "result", type := { typeId := ⟨0⟩, loc := ⟨0⟩ },
          mutable := true, loc := ⟨0⟩ }]
        body := .cfg {
          entry := ⟨0⟩
          blocks := #[
            { loc := ⟨0⟩
              terminator := .call ⟨0⟩ (some { place := ⟨0⟩, target := ⟨1⟩ }) .unreachable },
            { loc := ⟨0⟩, terminator := .return_ #[] }] } }] }] }

#guard match validate #[schema] rawCallContinuationUnit with
  | .ok unit => unit.namespaces[0]!.expressions.any fun expression =>
      expression.typeId == ⟨1⟩ && (expression.kind matches .assign ⟨0⟩ ⟨0⟩)
  | .error _ => false

#guard match validate #[schema] loopCfgUnit, validate #[schema] integerSwitchCfgUnit,
    validate #[schema] rawCallContinuationUnit with
  | .ok loopUnit, .ok switchUnit, .ok callUnit =>
      loopUnit.structurizationWitnesses.size == 1 &&
      loopUnit.structurizationWitnesses[0]!.regions.any (·.kind == .branch) &&
      loopUnit.structurizationWitnesses[0]!.regions.any (·.kind == .loop) &&
      switchUnit.structurizationWitnesses[0]!.regions.any (·.kind == .switch) &&
      callUnit.structurizationWitnesses[0]!.regions.any (·.kind == .callContinuation)
  | _, _, _ => false

private def rejectsWitnessMutation (cfg : RawCfg) (expressions : Array Expr)
    (witness : StructurizationWitness) : Bool :=
  match Structurize.checkStructurizationWitness cfg expressions witness with
  | .error diagnostic => diagnostic.code == "LIR-CFG-WITNESS"
  | .ok () => false

#guard match validate #[schema] loopCfgUnit with
  | .ok unit =>
      let ns := unit.namespaces[0]!
      let witness := unit.structurizationWitnesses[0]!
      let cfg := match loopCfgUnit.namespaces[0]!.functions[0]!.body with
        | .cfg cfg => cfg
        | _ => { entry := ⟨0⟩, blocks := #[] }
      rejectsWitnessMutation cfg ns.expressions
          { witness with entry := ⟨1⟩ } &&
        rejectsWitnessMutation cfg ns.expressions
          { witness with blocks := witness.blocks.drop 1 } &&
        rejectsWitnessMutation cfg ns.expressions
          { witness with edges := witness.edges.drop 1 } &&
        rejectsWitnessMutation cfg ns.expressions
          { witness with regions := witness.regions.drop 1 }
  | .error _ => false

private def badProfileValueUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      expressions := #[{
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .operation (.profile { profile := testProfile, tag := "subtract" }) #[] #[] }] }] }

#guard match validate #[schema] badProfileValueUnit with
  | .error diagnostics => diagnostics.any (·.code == "TEST-OP")
  | .ok _ => false

private def badLocalIdentityUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with
      functions := #[{ validNamespace.functions[0]! with locals := #[{
        id := ⟨1⟩
        name := "x"
        type := { typeId := ⟨0⟩, loc := ⟨0⟩ }
        loc := ⟨0⟩ }] }] }] }

#guard match validate #[schema] badLocalIdentityUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-LOCAL-IDENTITY")
  | .ok _ => false

private def inconsistentQualifiedRefUnit : RawUnit :=
  { validUnit with
    tables := { tables with namespaces := #[
      { segments := #["0x1", "Fixture"] }, { segments := #["0x1", "Other"] }] }
    namespaces := #[{
      validNamespace with
      expressions := #[{
        loc := ⟨0⟩
        typeId := ⟨0⟩
        kind := .constant { namespaceId := ⟨1⟩, name := ⟨0⟩ } }] }] }

#guard match validate #[schema] inconsistentQualifiedRefUnit with
  | .error diagnostics => diagnostics.any (·.code == "LIR-QUALIFIED-REF")
  | .ok _ => false

private def duplicateFunctionNameUnit : RawUnit :=
  { validUnit with namespaces := #[{
      validNamespace with functions := #[
        validNamespace.functions[0]!, validNamespace.functions[0]!] }] }

#guard match validate #[schema] duplicateFunctionNameUnit with
  | .error diagnostics =>
      diagnostics.any (·.code == "LIR-DECLARATION-NAME-DUPLICATE")
  | .ok _ => false

private def nominalNameTables : Tables :=
  { tables with names := tables.names ++ #[
      { namespaceId := ⟨0⟩, name := "Container" },
      { namespaceId := ⟨0⟩, name := "member" },
      { namespaceId := ⟨0⟩, name := "Variant" }] }

private def nominalField : FieldDecl :=
  { loc := ⟨0⟩, name := ⟨2⟩, type := { typeId := ⟨0⟩, loc := ⟨0⟩ } }

private def nominalUnit (declaration : StructDecl) : RawUnit :=
  { validUnit with
    tables := nominalNameTables
    namespaces := #[{ validNamespace with structs := #[declaration] }] }

private def duplicateStructFieldUnit : RawUnit :=
  nominalUnit { loc := ⟨0⟩, name := ⟨1⟩, fields := #[nominalField, nominalField] }

#guard hasDiagnostic "LIR-DECLARATION-NAME-DUPLICATE" duplicateStructFieldUnit

private def duplicateVariantUnit : RawUnit :=
  let variant : VariantDecl := { loc := ⟨0⟩, name := ⟨3⟩ }
  nominalUnit { loc := ⟨0⟩, name := ⟨1⟩, variants := #[variant, variant] }

#guard hasDiagnostic "LIR-DECLARATION-NAME-DUPLICATE" duplicateVariantUnit

private def duplicateVariantFieldUnit : RawUnit :=
  let variant : VariantDecl := {
    loc := ⟨0⟩, name := ⟨3⟩, fields := #[nominalField, nominalField] }
  nominalUnit { loc := ⟨0⟩, name := ⟨1⟩, variants := #[variant] }

#guard hasDiagnostic "LIR-DECLARATION-NAME-DUPLICATE" duplicateVariantFieldUnit

private def mixedNominalShapeUnit : RawUnit :=
  let variant : VariantDecl := { loc := ⟨0⟩, name := ⟨3⟩ }
  nominalUnit {
    loc := ⟨0⟩, name := ⟨1⟩, fields := #[nominalField], variants := #[variant] }

#guard hasDiagnostic "LIR-NOMINAL-SHAPE" mixedNominalShapeUnit

private def foreignDeclarationNameUnit : RawUnit :=
  { validUnit with
    tables := {
      tables with
      namespaces := tables.namespaces.push { segments := #["0x1", "Other"] }
      names := #[{ namespaceId := ⟨1⟩, name := "answer" }] } }

#guard match validate #[schema] foreignDeclarationNameUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-DECLARATION-NAME-OWNER" &&
        diagnostic.primary == some ⟨0⟩
  | .ok _ => false

private def duplicateNamespacePathUnit : RawUnit :=
  { validUnit with
    tables := {
      tables with namespaces := tables.namespaces.push tables.namespaces[0]! }
    namespaces := #[validNamespace, {
      loc := ⟨0⟩, identity := ⟨1⟩, profile := some testProfile }] }

#guard match validate #[schema] duplicateNamespacePathUnit with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-NAMESPACE-DUPLICATE" &&
        diagnostic.primary == some ⟨0⟩
  | .ok _ => false

/-! Elided generic instantiations at call sites are solved by unification
against the call's argument and result types, and the solved instantiation
discharges the target's binder abilities. -/

private def elidedCallUnit (targetGenerics : Array GenericBinder)
    (targetParameters : Array Parameter) (targetResults : Array TypeUse)
    (callerValueType : TypeId) (callArguments : Array ExprId)
    (callResultType : TypeId) : RawUnit :=
  { validUnit with
    tables := { tables with
      types := #[.bool, .typeParameter 0, .signer]
      names := #[
        { namespaceId := ⟨0⟩, name := "target" },
        { namespaceId := ⟨0⟩, name := "caller" }] }
    namespaces := #[{ validNamespace with
      expressions := #[
        { loc := ⟨0⟩, typeId := callerValueType, kind := .localVar ⟨0⟩ },
        { loc := ⟨0⟩, typeId := callResultType, kind := .operation
            (.call (.function { namespaceId := ⟨0⟩, name := ⟨0⟩ })) #[] callArguments }]
      functions := #[
        { loc := ⟨0⟩
          name := ⟨0⟩
          profile := testProfile
          signature := {
            generics := targetGenerics
            parameters := targetParameters
            results := targetResults }
          locals := targetParameters.zipIdx.map fun (parameter, index) =>
            { id := ⟨index⟩, name := parameter.name, type := parameter.typeUse, loc := ⟨0⟩ }
          body := .absent
          origin := ⟨0⟩
          alignment := ⟨0⟩ },
        { loc := ⟨0⟩
          name := ⟨1⟩
          profile := testProfile
          signature := {
            parameters := #[{ name := "value", typeUse := { typeId := callerValueType, loc := ⟨0⟩ } }]
            results := #[{ typeId := callResultType, loc := ⟨0⟩ }] }
          locals := #[{
            id := ⟨0⟩
            name := "value"
            type := { typeId := callerValueType, loc := ⟨0⟩ }
            loc := ⟨0⟩ }]
          body := .structured ⟨1⟩
          origin := ⟨0⟩
          alignment := ⟨0⟩ }] }] }

private def genericBinderT (abilities : Array Ability := #[]) : GenericBinder :=
  { name := "T", kind := .typeArg, abilities, loc := ⟨0⟩ }

private def parameterT : Parameter := { name := "x", typeUse := { typeId := ⟨1⟩, loc := ⟨0⟩ } }

-- The call `target(value)` with an elided `<Bool>` instantiation solves
-- `T := Bool` from the argument and result occurrences.
#guard (validate #[schema] (elidedCallUnit #[genericBinderT] #[parameterT]
  #[{ typeId := ⟨1⟩, loc := ⟨0⟩ }] ⟨0⟩ #[⟨0⟩] ⟨0⟩)).isOk

-- A caller result that contradicts the argument-solved slot is a type
-- mismatch: `T` binds `Bool` from the argument, but the call claims `signer`.
#guard match validate #[schema] (elidedCallUnit #[genericBinderT] #[parameterT]
    #[{ typeId := ⟨1⟩, loc := ⟨0⟩ }] ⟨0⟩ #[⟨0⟩] ⟨2⟩) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-TYPE" &&
        diagnostic.message.contains "no function instantiation matches"
  | .ok _ => false

-- A binder that occurs in no parameter or result stays unsolved.
#guard match validate #[schema] (elidedCallUnit #[genericBinderT]
    #[{ name := "x", typeUse := { typeId := ⟨0⟩, loc := ⟨0⟩ } }]
    #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] ⟨0⟩ #[⟨0⟩] ⟨0⟩) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-TYPE-UNDETERMINED" &&
        diagnostic.message.contains "does not determine every generic parameter"
  | .ok _ => false

-- The solved instantiation discharges binder abilities: `T := signer` does
-- not satisfy the target's `Copy` bound in the test extension profile.
#guard match validate #[schema] (elidedCallUnit #[genericBinderT #[.copy]] #[parameterT]
    #[{ typeId := ⟨1⟩, loc := ⟨0⟩ }] ⟨2⟩ #[⟨0⟩] ⟨2⟩) with
  | .error diagnostics => diagnostics.any fun diagnostic =>
      diagnostic.code == "LIR-SEMANTIC-ABILITY" &&
        diagnostic.message.contains "function instantiation type argument 0"
  | .ok _ => false

-- The same solved instantiation with a `Copy`-satisfying argument passes.
#guard (validate #[schema] (elidedCallUnit #[genericBinderT #[.copy]] #[parameterT]
  #[{ typeId := ⟨1⟩, loc := ⟨0⟩ }] ⟨0⟩ #[⟨0⟩] ⟨0⟩)).isOk

end LeanerIR.Tests.Validation
