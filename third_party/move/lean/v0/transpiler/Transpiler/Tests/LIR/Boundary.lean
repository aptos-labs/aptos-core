-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Driver

/-! Pure end-to-end checks for the XAST → checked LIR → Leaner backend view. -/

namespace Transpiler.Tests.LIR.Boundary

open Transpiler Xast Effects

private def loc : Loc := { file := 0, start := 0, stop := 1 }

private def truth : Exp := .mk .bool loc (.value (.bool true) none)

private def requiresTruth : Spec :=
  .mk (some loc) [] [.mk .requires loc [] truth none [] none none none] none

private def sharedConditionKinds : Array ConditionKind := #[
  .letPost "post", .letPre "pre", .assert, .assume, .decreases,
  .abortsIf, .abortsWith, .succeedsIf, .emits, .ensures, .requires,
  .structInvariant, .functionInvariant, .loopInvariant,
  .globalInvariant ["T"], .globalInvariantUpdate ["T"], .schemaInvariant,
  .axiom ["T"], .update]

#guard sharedConditionKinds.all fun kind =>
  Transpiler.LIR.Codec.xastConditionKind
      (Transpiler.LIR.Codec.lirConditionKind kind) == kind

private def sharedSpecOperations : Array Operation := #[
  .behavior .requiresOf { pre := none, post := none },
  .behavior .abortsOf { pre := none, post := none },
  .behavior .ensuresOf { pre := some 1, post := some 2 },
  .behavior .resultOf { pre := some 1, post := some 2 },
  .behavior .unchangedOf { pre := some 1, post := none },
  .behavior .foldsOf { pre := some 1, post := none },
  .behavior (.writeOf 1) { pre := some 1, post := some 2 },
  .result 1, .typeValue, .typeDomain, .resourceDomain, .stateDomain,
  .global (some 2), .canModify, .old, .saveStateAnchor 3,
  .withStateAnchor 4, .foldsCaptureAnchor 5, .inlineCallSummary,
  .trace .subAuto, .specPublish { pre := some 1, post := some 2 },
  .specRemove { pre := none, post := some 2 },
  .specUpdate { pre := some 1, post := none }, .emptyVec, .singleVec,
  .updateVec, .concatVec, .indexOfVec, .containsVec, .len, .index, .slice, .inRangeRange,
  .inRangeVec, .rangeVec, .maxU8, .maxU16, .maxU32, .maxU64,
  .maxU128, .maxU256, .bv2Int, .int2Bv, .abortFlag, .abortCode,
  .wellFormed, .boxValue, .unboxValue, .emptyEventStore,
  .extendEventStore, .eventStoreIncludes, .eventStoreIncludedIn, .noOp]

#guard sharedSpecOperations.all fun operation =>
  match Transpiler.LIR.Codec.specOperation? operation with
  | some lirOperation =>
      match Transpiler.LIR.Codec.specXastOperation lirOperation with
      | .ok decoded => decoded == operation
      | .error _ => false
  | none => false

private def fixture : Package := { modules := [{
  address := "0x42"
  addressAlias := none
  name := "Fixture"
  doc := "fixture"
  loc
  namedAddresses := []
  friends := []
  pragmas := []
  constants := [{
    name := "answer"
    doc := ""
    loc
    ty := .u64
    value := .number 42 }]
  structs := [{
    name := "Box"
    doc := ""
    loc
    abilities := [.copy, .drop]
    typeParams := [{ name := "T", abilities := [.copy, .drop], isPhantom := false }]
    attributes := []
    isNative := false
    fields := [{ name := "value", doc := "", ty := .typeParam 0 }]
    variants := none
    spec := .empty
    intrinsic := none }]
  functions := [{
    name := "truth"
    doc := ""
    loc
    visibility := .public
    isEntry := false
    kind := .regular
    isReceiver := false
    attributes := [.apply "lint" [.assign "level" (.value (.number 2))]]
    typeParams := [{ name := "T", abilities := [.copy, .drop], isPhantom := false }]
    params := [
      { name := "input", ty := .reference false .bool },
      { name := "items", ty := .vector .u8 },
      { name := "callback", ty := .function (.tuple [.u8]) .bool [.copy] }]
    result := .bool
    pragmas := [{ name := "verify", value := .value (.bool true) }]
    spec := requiresTruth
    body := some truth }]
  specFuns := []
  specVars := []
  invariants := [{
    kind := .axiom
    loc
    typeParams := []
    properties := []
    exp := truth }]
  sources := #["fixture.move"].toList
  comments := [] }] }

private def generatedChild (raw : LeanerIR.Import.RawUnit) (parent child : LeanerIR.LocId) : Bool :=
  match raw.tables.locations[child.index]? with
  | some location => location.primary.isNone &&
      location.generatedBy == some "Transpiler.LIR.Encode" && location.parent == some parent
  | none => false

#guard match Transpiler.LIR.Encode.package fixture with
  | .ok raw =>
      let ns := raw.namespaces[0]!
      let constant := ns.constants[0]!
      let constantValue := ns.expressions[constant.value.index]!
      let structDecl := ns.structs[0]!
      let function := ns.functions[0]!
      let bodyHasAuthoredLocation := match function.body with
        | .structured root =>
            let bodyLoc := ns.expressions[root.index]!.loc
            raw.tables.locations[bodyLoc.index]!.primary.isSome &&
              raw.tables.locations[bodyLoc.index]!.generatedBy.isNone
        | _ => false
      raw.tables.locations[function.loc.index]!.primary.isSome &&
        raw.tables.locations[function.loc.index]!.generatedBy.isNone &&
        bodyHasAuthoredLocation &&
        generatedChild raw constant.loc constant.type.loc &&
        generatedChild raw constant.loc constantValue.loc &&
        generatedChild raw structDecl.loc structDecl.generics[0]!.loc &&
        generatedChild raw structDecl.loc structDecl.fields[0]!.loc &&
        generatedChild raw function.loc function.signature.generics[0]!.loc &&
        (function.signature.parameters.all fun parameter =>
          generatedChild raw function.loc parameter.typeUse.loc) &&
        generatedChild raw function.loc function.signature.results[0]!.loc
  | .error _ => false

#guard match Transpiler.LIR.Backend.fromXast fixture >>=
    Transpiler.LIR.Backend.toPrinterPackage .move with
  | .ok package => package.modules.length == 1 &&
      package.modules.head!.functions.head!.name == "truth" &&
      package.modules.head!.functions.head!.params.head!.ty == .reference false .bool &&
      package.modules.head!.functions.head!.params[1]!.ty == .vector .u8 &&
      package.modules.head!.functions.head!.params[2]!.ty ==
        .function (.tuple [.u8]) .bool [.copy] &&
      package.modules.head!.functions.head!.typeParams.head!.abilities == [.copy, .drop] &&
      package.modules.head!.functions.head!.pragmas.head!.name == "verify" &&
      package.modules.head!.functions.head!.attributes ==
        [.apply "lint" [.assign "level" (.value (.number 2))]] &&
      package.modules.head!.functions.head!.body.any (fun body => body.ty == .bool) &&
      package.modules.head!.functions.head!.spec.conditions.length == 1 &&
      package.modules.head!.invariants.length == 1 &&
      package.modules.head!.invariants.head!.kind == .axiom
  | .error _ => false

#guard match Transpiler.LIR.Backend.fromXast fixture >>= fun checked =>
    Transpiler.Driver.transpileValidated checked with
  | .ok [(outcome, _)] => outcome.module.name == "Fixture"
  | .ok _ => false
  | .error _ => false

private def inlineFixture : Package :=
  let sourceModule := fixture.modules.head!
  let sourceFunction := sourceModule.functions.head!
  let inlineFunction := { sourceFunction with
    name := "expanded_inline"
    kind := .inlineRetained }
  let inlineSpecFunction : SpecFun := {
    name := "expanded_inline"
    doc := ""
    loc
    typeParams := sourceFunction.typeParams
    params := sourceFunction.params
    result := sourceFunction.result
    uninterpreted := false
    isNative := false
    isMoveFun := true
    usesOld := false
    body := some truth
    spec := .empty }
  { fixture with modules := [{ sourceModule with
      functions := sourceModule.functions ++ [inlineFunction]
      specFuns := [inlineSpecFunction]
      skipped := [{
        name := "expanded_inline"
        reason := "in function `Fixture::expanded_inline`: behavior predicates are not supported by XAST" }, {
        name := "expanded_only"
        reason := "in function `Fixture::expanded_only`: behavior predicates are not supported by XAST" }] }] }

#guard match Transpiler.LIR.Backend.fromXast inlineFixture with
  | .ok checked =>
      let ns := checked.namespaces[0]!
      ns.functions.size == 1 && ns.specFunctions.isEmpty &&
        (Transpiler.LIR.Report.initial ns).toOption.any (·.unsupported.isEmpty)
  | .error _ => false

private def inlineCommentFixture : Package :=
  let sourceModule := inlineFixture.modules.head!
  let sourceFunction := sourceModule.functions.head!
  let retainedAfter := { sourceFunction with
    name := "retained_after"
    loc := { file := 0, start := 60, stop := 80 } }
  let functions := sourceModule.functions.map fun function =>
    if function.kind == .inlineRetained then
      { function with loc := { file := 0, start := 30, stop := 50 } }
    else { function with loc := { file := 0, start := 0, stop := 10 } }
  { inlineFixture with modules := [{ sourceModule with
      functions := functions ++ [retainedAfter]
      comments := [
        { loc := { file := 0, start := 20, stop := 29 },
          text := "// belongs to expanded inline", ownLine := true },
        { loc := { file := 0, start := 40, stop := 49 },
          text := "// inside expanded inline", ownLine := true },
        { loc := { file := 0, start := 52, stop := 59 },
          text := "// retained", ownLine := true }] }] }

#guard match Transpiler.LIR.Encode.package inlineCommentFixture with
  | .ok raw => raw.namespaces[0]!.comments.map (·.text) == #["// retained"]
  | .error _ => false

private def behaviorFixture : Package :=
  let sourceModule := fixture.modules.head!
  let functionType := Ty.function (.tuple [.bool]) .bool [.copy]
  let target : Exp := .mk functionType loc (.«local» "f")
  let value : Exp := .mk .bool loc (.«local» "value")
  let body : Exp := .mk .bool loc (.call
    (.behavior .resultOf { pre := some 1, post := some 2 }) [] [target, value] none)
  let declaration : SpecFun := {
    name := "summarized"
    doc := ""
    loc
    typeParams := []
    params := [{ name := "f", ty := functionType }, { name := "value", ty := .bool }]
    result := .bool
    uninterpreted := false
    isNative := false
    isMoveFun := false
    usesOld := true
    body := some body
    spec := .empty }
  { fixture with modules := [{ sourceModule with specFuns := [declaration] }] }

#guard match Transpiler.LIR.Encode.package behaviorFixture with
  | .ok raw =>
      let ns := raw.namespaces[0]!
      match ns.specFunctions[0]?.bind (fun declaration => declaration.body) with
      | some body => match ns.expressions[body.index]? with
          | some expression => match expression.kind with
              | .operation (.specification (.behavior .resultOf range))
                  instantiations arguments surface =>
                  range.pre == some 1 && range.post == some 2 &&
                    instantiations.isEmpty && arguments.size == 2 && surface.isNone
              | _ => false
          | none => false
      | none => false
  | .error _ => false

private def sequenceFixture : Package :=
  let sourceModule := fixture.modules.head!
  let sourceFunction := sourceModule.functions.head!
  let sequence : Exp := .mk .bool loc (.sequence [truth, truth])
  { fixture with modules := [{ sourceModule with functions := [{ sourceFunction with
      name := "sequence_result"
      typeParams := []
      params := []
      body := some sequence }] }] }

#guard match Transpiler.LIR.Encode.package sequenceFixture with
  | .ok raw =>
      let ns := raw.namespaces[0]!
      match ns.functions[0]!.body with
      | .structured root => match ns.expressions[root.index]!.kind with
          | .block statements (some _) => statements.size == 1
          | _ => false
      | .absent | .cfg _ => false
  | .error _ => false

private def logicalSliceFixture : Package :=
  let sourceModule := fixture.modules.head!
  let sourceFunction := sourceModule.functions.head!
  let bound (value : Int) : Exp := .mk .num loc (.value (.number value) none)
  let values : Exp := .mk (.vector .u64) loc
    (.value (.vector [.number 1, .number 2]) none)
  let range : Exp := .mk .range loc (.call .range [] [bound 0, bound 1] none)
  let slice : Exp := .mk (.vector .u64) loc (.call .slice [] [values, range] none)
  { fixture with modules := [{ sourceModule with functions := [{ sourceFunction with
      name := "logical_slice"
      typeParams := []
      params := []
      result := .vector .u64
      body := some slice }] }] }

#guard match Transpiler.LIR.Encode.package logicalSliceFixture with
  | .ok raw => raw.namespaces[0]!.expressions.any fun expression =>
      match expression.kind with
      | .operation (.specification .sliceVector) #[.typeArg _] arguments _ =>
          arguments.size == 2
      | _ => false
  | .error _ => false

private def skippedFixture : Package :=
  { fixture with modules := fixture.modules.map fun module =>
      { module with skipped := [{
          name := "callback_value"
          reason := "function-value construction is not represented by XAST v3" }] } }

private def skippedReport : List String :=
  ["callback_value: function-value construction is not represented by XAST v3"]

#guard match Transpiler.LIR.Backend.fromXast skippedFixture >>= fun checked =>
    Transpiler.LIR.Report.initial checked.namespaces[0]! with
  | .ok report => report.unsupported == skippedReport
  | .error _ => false

#guard match Transpiler.LIR.Backend.fromXast skippedFixture >>= fun checked =>
    Transpiler.Driver.transpileValidated checked with
  | .ok [(outcome, _)] =>
      outcome.report.unsupported.filter (· == skippedReport.head!) == skippedReport
  | .ok _ => false
  | .error _ => false

private def number (value : Int) : Exp := .mk .u64 loc (.value (.number value) none)

private def vaultName : QualifiedName :=
  { module := (fixture.modules.head!).ref, name := "Vault" }

private def vaultStruct : Struct := {
  name := "Vault"
  doc := ""
  loc
  abilities := [.key]
  typeParams := []
  attributes := []
  isNative := false
  fields := []
  variants := none
  spec := .empty
  intrinsic := none }

private def publish : Exp := .mk (.tuple []) loc
  (.call .moveTo [.struct vaultName []]
    [.mk .address loc (.value (.address "0x42") none),
     .mk (.struct vaultName []) loc (.call (.pack vaultName none) [] [] none)] none)

private def globalFixture : Package :=
  let sourceModule := fixture.modules.head!
  let sourceFunction := sourceModule.functions.head!
  { fixture with modules := [{
      sourceModule with functions := [{
        sourceFunction with
        name := "publish"
        typeParams := []
        params := []
        result := .tuple []
        body := some publish }] }] }

private def effectFixture : Package :=
  let sourceModule := fixture.modules.head!
  let sourceModule := { sourceModule with structs := sourceModule.structs ++ [vaultStruct] }
  let sourceFunction := sourceModule.functions.head!
  let unit : Exp := .mk (.tuple []) loc (.value (.tuple []) none)
  let functionName (name : String) : QualifiedName := { module := sourceModule.ref, name }
  let writer := { sourceFunction with
    name := "writer"
    typeParams := []
    params := []
    result := .tuple []
    body := some publish }
  let caller := { writer with
    name := "caller"
    body := some (.mk (.tuple []) loc (.call (.moveFunction (functionName "writer")) [] [] none)) }
  let entry := { writer with
    name := "entry"
    isEntry := true
    body := some unit }
  let native := { writer with
    name := "native_mut"
    kind := .native
    params := [{ name := "value", ty := .reference true .u64 }]
    body := none }
  { fixture with modules := [{
      sourceModule with functions := [writer, caller, entry, native] }] }

#guard match Transpiler.LIR.Backend.fromXast effectFixture with
  | .error _ => false
  | .ok checked => match Transpiler.LIR.Effects.computePrinterTable checked with
      | .error _ => false
      | .ok table =>
          let module := effectFixture.modules.head!.ref
          let writer := table.get { module, name := "writer" }
          let caller := table.get { module, name := "caller" }
          let entry := table.get { module, name := "entry" }
          let native := table.get { module, name := "native_mut" }
          writer.isAction && writer.writesGlobal &&
            caller.isAction && caller.writesGlobal &&
            entry.isAction && !entry.writesGlobal &&
            native.isAction && !native.writesGlobal

#guard match Transpiler.LIR.Encode.package globalFixture with
  | .error _ => false
  | .ok raw => raw.namespaces.any fun ns => ns.expressions.any fun expression =>
      expression.kind matches .operation (.global .publish) _ _ _

private def multiModuleFixture : Package :=
  let first := fixture.modules.head!
  let second := { first with
    name := "Second"
    functions := []
    invariants := []
    sources := ["second.move"] }
  { fixture with modules := [first, second] }

#guard match Transpiler.LIR.Encode.package multiModuleFixture with
  | .error _ => false
  | .ok raw => match LeanerIR.Move.validate raw with
      | .error _ => false
      | .ok checked =>
          raw.namespaces.size == 2 && raw.tables.namespaces.size == 2 &&
            raw.tables.files.size == 2 &&
            raw.namespaces[0]!.identity == ⟨0⟩ && raw.namespaces[1]!.identity == ⟨1⟩ &&
            checked.namespaces.all (·.tables == checked.tables) &&
            match Transpiler.LIR.Backend.toPrinterPackage .move checked with
            | .ok package => package.modules.map (·.name) == ["Fixture", "Second"]
            | .error _ => false

#guard match Transpiler.LIR.Backend.fromXast fixture with
  | .ok checked => match Transpiler.LIR.Backend.toPrinterPackage .rust checked with
      | .error _ => true
      | .ok _ => false
  | .error _ => false

end Transpiler.Tests.LIR.Boundary
