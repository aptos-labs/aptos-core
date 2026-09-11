-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Driver
import LeanerRust.Benchmark
import LeanerRust.Equivalence
import LeanerRust.Source

namespace LeanerIR.Rust.Tests.Source

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation
open LeanerIR.Rust.Driver

private structure RoundTripCase where
  baseline : String
  functionId : Nat := 0
  arguments : ValidatedUnit → Except String (Array RuntimeValue) := fun _ => pure #[]
  initialState : RuntimeState := {}

private def captureFailure (label : String) (action : IO Unit) :
    IO (Option (String × String)) := do
  try
    action
    pure none
  catch error =>
    pure <| some (label, toString error)

private def firstLine (message : String) : String :=
  (message.splitOn "\n").headD message

private structure EnumVariantShape where
  name : String
  discriminant : Option Int
  fields : Array String
  deriving BEq

private structure EnumShape where
  name : String
  variants : Array EnumVariantShape
  deriving BEq

private structure FunctionShape where
  name : String
  parameters : Array (Bool × String)
  results : Array String
  deriving BEq

private structure NominalVariantShape where
  name : String
  discriminant : Option Int
  fields : Array (String × String)
  deriving BEq

private structure NominalShape where
  name : String
  binders : Array (String × BinderKind × Option String)
  fields : Array (String × String)
  variants : Array NominalVariantShape
  deriving BEq

private def sourceName (unit : ValidatedUnit) (id : NameId) : Except String String :=
  match unit.tables.names[id.index]? with
  | some name => pure name.name
  | none => throw s!"validated source name {id.index} is missing"

private def enumShapes (unit : ValidatedUnit) : Except String (Array EnumShape) := do
  let some ns := unit.namespaces[0]? | throw "round-trip unit has no namespace"
  ns.structs.filterMapM fun declaration => do
    if declaration.variants.isEmpty then return none
    let variants ← declaration.variants.mapM fun variant => do
      let fields ← variant.fields.mapM fun field => sourceName unit field.name
      let name ← sourceName unit variant.name
      pure { name, discriminant := variant.discriminant, fields }
    let name ← sourceName unit declaration.name
    pure <| some { name, variants }

mutual
  private partial def typeShape (unit : ValidatedUnit) (id : TypeId) (fuel : Nat) :
      Except String String := do
    if fuel == 0 then throw "cyclic type in source shape comparison"
    let some ty := unit.tables.types[id.index]?
      | throw s!"source shape type {id.index} is missing"
    match ty with
    | .unit => pure "unit"
    | .never => pure "never"
    | .bool => pure "bool"
    | .character => pure "char"
    | .integer width signed => pure s!"integer:{repr width}:{signed}"
    | .tuple elements => do
        let elements ← elements.mapM (typeShape unit · (fuel - 1))
        pure s!"tuple:{repr elements}"
    | .vector element length =>
        pure s!"vector:{← typeShape unit element (fuel - 1)}:{repr length}"
    | .nominal name arguments => do
        let arguments ← arguments.mapM (genericShape unit · (fuel - 1))
        pure s!"nominal:{← sourceName unit name}:{repr arguments}"
    | .function arguments result abilities => do
        let arguments ← arguments.mapM (typeShape unit · (fuel - 1))
        pure s!"function:{repr arguments}:{← typeShape unit result (fuel - 1)}:{repr abilities}"
    | .typeParameter index => pure s!"parameter:{index}"
    | .reference reference =>
        pure s!"reference:{repr reference.kind}:{← typeShape unit reference.referent (fuel - 1)}"
    | ty => pure s!"other:{repr ty}"

  private partial def genericShape (unit : ValidatedUnit) (argument : GenericArgument)
      (fuel : Nat) : Except String String :=
    match argument with
    | .typeArg value => typeShape unit value.typeId fuel
    | .const value => pure s!"const:{repr value}"
    | .lifetime _ => pure "lifetime"
    | .evidence evidence => pure s!"evidence:{evidence.index}"
end

private def functionShapes (unit : ValidatedUnit) : Except String (Array FunctionShape) := do
  let some ns := unit.namespaces[0]? | throw "round-trip unit has no namespace"
  ns.functions.mapM fun declaration => do
    let parameters ← declaration.signature.parameters.mapM fun parameter => do
      pure (parameter.mutable, ← typeShape unit parameter.typeUse.typeId
        (unit.tables.types.size + 1))
    let results ← declaration.signature.results.mapM fun result =>
      typeShape unit result.typeId (unit.tables.types.size + 1)
    pure { name := ← sourceName unit declaration.name, parameters, results }

private def nominalShapes (unit : ValidatedUnit) : Except String (Array NominalShape) := do
  let some ns := unit.namespaces[0]? | throw "round-trip unit has no namespace"
  ns.structs.mapM fun declaration => do
    let fieldShapes (fields : Array FieldDecl) := fields.mapM fun field => do
      pure (← sourceName unit field.name,
        ← typeShape unit field.type.typeId (unit.tables.types.size + 1))
    let fields ← fieldShapes declaration.fields
    let variants ← declaration.variants.mapM fun variant => do
      pure {
        name := ← sourceName unit variant.name
        discriminant := variant.discriminant
        fields := ← fieldShapes variant.fields }
    let binders ← declaration.generics.mapM fun binder => do
      let type ← match binder.type with
        | some type => some <$> typeShape unit type.typeId (unit.tables.types.size + 1)
        | none => pure none
      pure (binder.name, binder.kind, type)
    pure {
      name := ← sourceName unit declaration.name
      binders
      fields
      variants }

private partial def runtimeEquivalent : RuntimeValue → RuntimeValue → Bool
  | .unit, .unit => true
  | .bool left, .bool right => left == right
  | .character left, .character right => left == right
  | .integer left, .integer right => left == right
  | .address left, .address right | .signer left, .signer right |
      .string left, .string right => left == right
  | .bytes left, .bytes right => left == right
  | .vector left, .vector right | .tuple left, .tuple right =>
      left.size == right.size && (left.zip right).all fun (left, right) =>
        runtimeEquivalent left right
  | .nominal leftSource leftVariant leftFields,
      .nominal rightSource rightVariant rightFields =>
      leftSource == rightSource && leftVariant == rightVariant &&
        leftFields.size == rightFields.size &&
        (leftFields.zip rightFields).all fun (left, right) => runtimeEquivalent left right
  | .borrow _ leftCurrent, .borrow _ rightCurrent =>
      -- Dynamic loan instances are run-local bookkeeping; the observable is
      -- the owned current value.
      runtimeEquivalent leftCurrent rightCurrent
  | .loanHole _, .loanHole _ => true
  | _, _ => false

/-- Observable state equivalence: `nextLoan` is run-local bookkeeping that
legitimately differs when a round-tripped body creates a different number of
intermediate borrows. -/
private def statesEquivalent (left right : RuntimeState) : Bool :=
  left.globals == right.globals && left.pending == right.pending

private def outcomesEquivalent : Outcome → Outcome → Bool
  | .returned left, .returned right =>
      left.size == right.size && (left.zip right).all fun (left, right) =>
        runtimeEquivalent left right
  | .threw leftKind left, .threw rightKind right =>
      leftKind == rightKind && left.size == right.size &&
        (left.zip right).all fun (left, right) => runtimeEquivalent left right
  | _, _ => false

private def executableOutcome (label : String) (unit : ValidatedUnit) (functionId : Nat)
    (arguments : Array RuntimeValue) (initialState : RuntimeState) :
    IO (RuntimeState × Outcome) := do
  let prepared ← Benchmark.measureExcept "lir.prepare" label fun _ =>
    prepareExecution #[semantics] unit
  let executable ← match prepared with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust source round-trip unit {label} does not prepare: \
          {repr diagnostics}"
  let interpreted ← Benchmark.measureExcept "lir.interpret" label fun _ =>
    Interpreter.run executable 128 { namespaceId := ⟨0⟩, functionId := ⟨functionId⟩ }
      arguments initialState
  match interpreted with
    | .ok (state, outcome) => pure (state, outcome.value)
    | .error error => throw <| IO.userError s!"Rust source round-trip unit {label} does not execute: \
        {repr error}"

private structure SourceReachability where
  expressions : Array ExprId := #[]
  patterns : Array PatternId := #[]
  places : Array PlaceId := #[]

mutual
  private partial def sourceExpressionReachability (ns : ValidatedNamespace) (id : ExprId)
      (state : SourceReachability) (fuel : Nat) : SourceReachability :=
    if fuel == 0 || state.expressions.contains id then state else
    let state := { state with expressions := state.expressions.push id }
    match ns.expressions[id.index]? with
    | none => state
    | some expression =>
        let visitExpression state child :=
          sourceExpressionReachability ns child state (fuel - 1)
        let visitOptionalExpression state child := match child with
          | some child => visitExpression state child
          | none => state
        match expression.kind with
        | .value .. | .constant _ | .localVar _ | .continue_ _ => state
        | .operation operation _ arguments _ =>
            let state := arguments.foldl visitExpression state
            match operation with
            | .move place | .copy place | .borrow _ place | .read place | .write place |
                .drop place => sourcePlaceReachability ns place state (fuel - 1)
            | _ => state
        | .block statements result =>
            visitOptionalExpression (statements.foldl visitExpression state) result
        | .letDecl pattern value body =>
            let state := match value with
              | some _ => sourcePatternReachability ns pattern state (fuel - 1)
              | none => state
            visitExpression (visitOptionalExpression state value) body
        | .ifElse condition thenBranch elseBranch =>
            let state := visitExpression state condition
            let state := visitExpression state thenBranch
            visitOptionalExpression state elseBranch
        | .match_ scrutinee arms =>
            arms.foldl (fun state arm =>
              let state := sourcePatternReachability ns arm.pattern state (fuel - 1)
              let state := visitOptionalExpression state arm.guard
              visitExpression state arm.body) (visitExpression state scrutinee)
        | .loop _ body => visitExpression state body
        | .break_ _ value => visitOptionalExpression state value
        | .return_ values | .throw_ _ values => values.foldl visitExpression state
        | .assign place value =>
            visitExpression (sourcePlaceReachability ns place state (fuel - 1)) value
        | .assignPattern pattern value =>
            visitExpression (sourcePatternReachability ns pattern state (fuel - 1)) value
        | .quantifier _ binders triggers condition body =>
            let state := binders.foldl (fun state binder =>
              let state := sourcePatternReachability ns binder.pattern state (fuel - 1)
              visitExpression state binder.domain) state
            let state := triggers.foldl (fun state trigger =>
              trigger.foldl visitExpression state) state
            visitExpression (visitOptionalExpression state condition) body
        | .spec _ => state

  private partial def sourcePlaceReachability (ns : ValidatedNamespace) (id : PlaceId)
      (state : SourceReachability) (fuel : Nat) : SourceReachability :=
    if fuel == 0 || state.places.contains id then state else
    let state := { state with places := state.places.push id }
    match ns.places[id.index]? with
    | some (.deref base) | some (.field base ..) | some (.subslice base ..) |
        some (.downcast base _) => sourcePlaceReachability ns base state (fuel - 1)
    | some (.index base index) =>
        sourceExpressionReachability ns index
          (sourcePlaceReachability ns base state (fuel - 1)) (fuel - 1)
    | some (.localVar _) | none => state

  private partial def sourcePatternReachability (ns : ValidatedNamespace) (id : PatternId)
      (state : SourceReachability) (fuel : Nat) : SourceReachability :=
    if fuel == 0 || state.patterns.contains id then state else
    let state := { state with patterns := state.patterns.push id }
    match ns.patterns[id.index]? with
    | some { kind := .tuple elements, .. } => elements.foldl (fun state child =>
        sourcePatternReachability ns child state (fuel - 1)) state
    | some { kind := .constructor _ _ _ fields, .. } => fields.foldl (fun state child =>
        sourcePatternReachability ns child state (fuel - 1)) state
    | _ => state
end

private def sourceReachability (ns : ValidatedNamespace) : SourceReachability :=
  let fuel := ns.expressions.size + ns.patterns.size + ns.places.size + 1
  ns.functions.foldl (fun state declaration => match declaration.body with
    | .structured root => sourceExpressionReachability ns root state fuel
    | .absent => state) {}

private def checkSourceMap (unit : ValidatedUnit) (rendered : Rust.Source.RenderedSource) :
    IO Unit := do
  let bytes := rendered.text.toUTF8
  unless !rendered.sourceMap.entries.isEmpty do
    throw <| IO.userError "canonical Rust source has an empty generated source map"
  unless !(rendered.text.toList.contains (Char.ofNat 31)) &&
      !(rendered.text.toList.contains (Char.ofNat 30)) do
    throw <| IO.userError "canonical Rust source retained an internal source-map marker"
  for entry in rendered.sourceMap.entries do
    unless entry.range.startByte < entry.range.stopByte && entry.range.stopByte ≤ bytes.size do
      throw <| IO.userError s!"invalid generated Rust byte range {repr entry.range}"
    unless (String.fromUTF8? <| bytes.extract entry.range.startByte entry.range.stopByte).isSome do
      throw <| IO.userError s!"generated Rust range splits a UTF-8 scalar: {repr entry.range}"
    unless unit.tables.locations[entry.originalLoc.index]?.isSome do
      throw <| IO.userError "generated source map references a missing original location"
    unless entry.origin.isSome == entry.alignment.isSome do
      throw <| IO.userError "generated source map has partial import provenance"
    match entry.origin with
    | some origin => unless unit.tables.origins[origin.index]?.isSome do
        throw <| IO.userError "generated source map references a missing original origin"
    | none => pure ()
    match entry.alignment with
    | some alignment => unless unit.tables.alignments[alignment.index]?.isSome do
        throw <| IO.userError "generated source map references a missing import alignment"
    | none => pure ()
    match entry.node with
    | .function namespaceId functionId =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.functions[functionId.index]?
          | throw <| IO.userError "generated source map references a missing function"
        unless declaration.loc == entry.originalLoc do
          throw <| IO.userError "generated function range has the wrong original location"
        unless some declaration.origin == entry.origin &&
            some declaration.alignment == entry.alignment do
          throw <| IO.userError "generated function range has the wrong provenance"
    | .functionBinder namespaceId functionId index =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.functions[functionId.index]?
          | throw <| IO.userError "generated source map references a missing binder owner"
        let some binder := declaration.signature.generics[index]?
          | throw <| IO.userError "generated source map references a missing function binder"
        unless binder.loc == entry.originalLoc && some declaration.origin == entry.origin &&
            some declaration.alignment == entry.alignment do
          throw <| IO.userError "generated function-binder range has the wrong provenance"
    | .nominal namespaceId nameId =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.structs.find? (·.name == nameId)
          | throw <| IO.userError "generated source map references a missing nominal declaration"
        unless declaration.loc == entry.originalLoc && entry.origin.isNone &&
            entry.alignment.isNone do
          throw <| IO.userError "generated nominal range has invented import provenance"
    | .nominalBinder namespaceId owner index =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.structs.find? (·.name == owner)
          | throw <| IO.userError "generated source map references a missing binder owner"
        let some binder := declaration.generics[index]?
          | throw <| IO.userError "generated source map references a missing nominal binder"
        unless binder.loc == entry.originalLoc && entry.origin.isNone &&
            entry.alignment.isNone do
          throw <| IO.userError "generated binder range has invented import provenance"
    | .variant namespaceId owner name =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.structs.find? (·.name == owner)
          | throw <| IO.userError "generated source map references a missing variant owner"
        let some variant := declaration.variants.find? (·.name == name)
          | throw <| IO.userError "generated source map references a missing variant"
        unless variant.loc == entry.originalLoc && entry.origin.isNone &&
            entry.alignment.isNone do
          throw <| IO.userError "generated variant range has invented import provenance"
    | .field namespaceId owner variantName name =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.structs.find? (·.name == owner)
          | throw <| IO.userError "generated source map references a missing field owner"
        let fields ← match variantName with
          | none => pure declaration.fields
          | some variantName =>
              let some variant := declaration.variants.find? (·.name == variantName)
                | throw <| IO.userError "generated source map references a missing field variant"
              pure variant.fields
        let some field := fields.find? (·.name == name)
          | throw <| IO.userError "generated source map references a missing field"
        unless field.loc == entry.originalLoc && entry.origin.isNone &&
            entry.alignment.isNone do
          throw <| IO.userError "generated field range has invented import provenance"
    | .typeUse namespaceId typeId loc =>
        unless unit.namespaces[namespaceId.index]?.isSome do
          throw <| IO.userError "generated source map references a missing namespace"
        unless unit.tables.types[typeId.index]?.isSome do
          throw <| IO.userError "generated source map references a missing type"
        unless loc == entry.originalLoc do
          throw <| IO.userError "generated type-use range has the wrong original location"
    | .expression namespaceId expressionId =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some expression := ns.expressions[expressionId.index]?
          | throw <| IO.userError "generated source map references a missing expression"
        unless expression.loc == entry.originalLoc do
          throw <| IO.userError "generated expression range has the wrong original location"
        match expression.kind with
        | .operation (.primitive .cast) _ _ _ =>
            unless rendered.sourceMap.entries.any fun candidate =>
                candidate.node == .typeUse namespaceId expression.typeId expression.loc &&
                  candidate.origin == entry.origin && candidate.alignment == entry.alignment &&
                  entry.range.startByte ≤ candidate.range.startByte &&
                  candidate.range.stopByte ≤ entry.range.stopByte do
              throw <| IO.userError
                "generated source map omits a cast's nested result-type use"
        | _ => pure ()
    | .pattern namespaceId patternId =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some pattern := ns.patterns[patternId.index]?
          | throw <| IO.userError "generated source map references a missing pattern"
        unless pattern.loc == entry.originalLoc do
          throw <| IO.userError "generated pattern range has the wrong original location"
    | .place namespaceId placeId =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        unless ns.places[placeId.index]?.isSome do
          throw <| IO.userError "generated source map references a missing place"
    | .local namespaceId functionId localId =>
        let some ns := unit.namespaces[namespaceId.index]?
          | throw <| IO.userError "generated source map references a missing namespace"
        let some declaration := ns.functions[functionId.index]?
          | throw <| IO.userError "generated source map references a missing local owner"
        let some localDecl := declaration.locals[localId.index]?
          | throw <| IO.userError "generated source map references a missing local"
        unless localDecl.loc == entry.originalLoc && some declaration.origin == entry.origin &&
            some declaration.alignment == entry.alignment do
          throw <| IO.userError "generated local range has the wrong provenance"
  for functionId in [:unit.namespaces[0]!.functions.size] do
    unless rendered.sourceMap.entries.any (·.node == .function ⟨0⟩ ⟨functionId⟩) do
      throw <| IO.userError s!"generated source map omits function {functionId}"
    let declaration := unit.namespaces[0]!.functions[functionId]!
    for index in [:declaration.signature.generics.size] do
      unless rendered.sourceMap.entries.any
          (·.node == .functionBinder ⟨0⟩ ⟨functionId⟩ index) do
        throw <| IO.userError s!"generated source map omits function {functionId}'s generic binder"
    for parameter in declaration.signature.parameters do
      unless rendered.sourceMap.entries.any
          (·.node == .typeUse ⟨0⟩ parameter.typeUse.typeId parameter.typeUse.loc) do
        throw <| IO.userError "generated source map omits a parameter type use"
    for result in declaration.signature.results do
      unless rendered.sourceMap.entries.any
          (·.node == .typeUse ⟨0⟩ result.typeId result.loc) do
        throw <| IO.userError "generated source map omits a result type use"
    for localId in [:declaration.locals.size] do
      let localDecl := declaration.locals[localId]!
      unless unit.tables.types[localDecl.type.typeId.index]? == some (Ty.never) do
        unless rendered.sourceMap.entries.any
            (·.node == .typeUse ⟨0⟩ localDecl.type.typeId localDecl.type.loc) do
          throw <| IO.userError s!"generated source map omits function {functionId}'s local type"
        unless rendered.sourceMap.entries.any
            (·.node == .local ⟨0⟩ ⟨functionId⟩ ⟨localId⟩) do
          throw <| IO.userError s!"generated source map omits function {functionId}'s local {localId}"
    match declaration.body with
    | .structured root =>
        unless rendered.sourceMap.entries.any (·.node == .expression ⟨0⟩ root) do
          throw <| IO.userError s!"generated source map omits function {functionId}'s root expression"
    | .absent => pure ()
  for declaration in unit.namespaces[0]!.structs do
    unless rendered.sourceMap.entries.any (·.node == .nominal ⟨0⟩ declaration.name) do
      throw <| IO.userError "generated source map omits a nominal declaration"
    for index in [:declaration.generics.size] do
      unless rendered.sourceMap.entries.any
          (·.node == .nominalBinder ⟨0⟩ declaration.name index) do
        throw <| IO.userError "generated source map omits a nominal generic binder"
    for field in declaration.fields do
      unless rendered.sourceMap.entries.any
          (·.node == .typeUse ⟨0⟩ field.type.typeId field.type.loc) do
        throw <| IO.userError "generated source map omits a struct-field type use"
      unless rendered.sourceMap.entries.any
          (·.node == .field ⟨0⟩ declaration.name none field.name) do
        throw <| IO.userError "generated source map omits a struct field"
    for variant in declaration.variants do
      unless rendered.sourceMap.entries.any
          (·.node == .variant ⟨0⟩ declaration.name variant.name) do
        throw <| IO.userError "generated source map omits an enum variant"
      for field in variant.fields do
        unless rendered.sourceMap.entries.any
            (·.node == .typeUse ⟨0⟩ field.type.typeId field.type.loc) do
          throw <| IO.userError "generated source map omits an enum-field type use"
        unless rendered.sourceMap.entries.any
            (·.node == .field ⟨0⟩ declaration.name (some variant.name) field.name) do
          throw <| IO.userError "generated source map omits an enum field"
  let ns := unit.namespaces[0]!
  let reachable := sourceReachability ns
  for expressionId in reachable.expressions do
    unless rendered.sourceMap.entries.any
        (·.node == .expression ns.identity expressionId) do
      throw <| IO.userError s!"generated source map omits expression {expressionId.index}"
  for placeId in reachable.places do
    match ns.places[placeId.index]! with
    | .downcast _ _ => pure ()
    | _ => do
      unless rendered.sourceMap.entries.any
          (·.node == .place ns.identity placeId) do
        throw <| IO.userError s!"generated source map omits place {placeId.index}"
  for patternId in reachable.patterns do
    unless rendered.sourceMap.entries.any (·.node == .pattern ns.identity patternId) do
      throw <| IO.userError s!"generated source map omits pattern {patternId.index}"

private def checkSemanticRoundTrip (label source : String) (original imported : ValidatedUnit) :
    IO Unit := do
  let originalProjectionResult ← Benchmark.measureExcept "semantic.project_original" label fun _ =>
    Rust.Equivalence.semanticProjection original
  let originalProjection ← match originalProjectionResult with
    | .ok projection => pure projection
    | .error message => throw <| IO.userError s!"source-backend original semantic \
        projection failed for {label}: {message}"
  let importedProjectionResult ← Benchmark.measureExcept "semantic.project_imported" label fun _ =>
    Rust.Equivalence.semanticProjection imported
  let importedProjection ← match importedProjectionResult with
    | .ok projection => pure projection
    | .error message => throw <| IO.userError s!"source-backend re-import semantic \
        projection failed for {label}: {message}"
  unless originalProjection == importedProjection do
    throw <| IO.userError s!"source-backend semantic projection changed {label}:\n\
      original: {repr originalProjection}\nre-import: {repr importedProjection}\n{source}"

private def checkCase (directory : System.FilePath) (case : RoundTripCase) : IO Unit := do
  let baselinePath : System.FilePath := "rust-exporter/tests/raw-unit" / case.baseline
  let original ← Benchmark.measure "baseline.decode_validate" case.baseline do
    match decodeAndValidate (← IO.FS.readFile baselinePath) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"source-backend input {case.baseline} does not validate: {repr diagnostics}"
  let renderedResult ← Benchmark.measureExcept "source.render_map" case.baseline fun _ =>
    Rust.Source.renderWithSourceMap original
  let rendered ← match renderedResult with
    | .ok rendered => pure rendered
    | .error message =>
        throw <| IO.userError s!"source-backend input {case.baseline} does not render: {message}"
  let renderedAgainResult ← Benchmark.measureExcept "source.render_map_repeat" case.baseline fun _ =>
    Rust.Source.renderWithSourceMap original
  let renderedAgain ← match renderedAgainResult with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError message
  unless renderedAgain == rendered do
    throw <| IO.userError s!"source-backend input {case.baseline} did not render deterministically"
  Benchmark.measure "source.map_validate" case.baseline <| checkSourceMap original rendered
  let source := rendered.text
  let sourceOnlyResult ← Benchmark.measureExcept "source.render_compat" case.baseline fun _ =>
    Rust.Source.render original
  let sourceOnly ← match sourceOnlyResult with
    | .ok source => pure source
    | .error message => throw <| IO.userError message
  unless sourceOnly == source do
    throw <| IO.userError "compatibility source renderer disagrees with mapped renderer"
  let sourcePath := directory / "roundtrip.rs"
  let outputPath := directory / "roundtrip.raw.json"
  IO.FS.writeFile sourcePath source
  let imported ← Benchmark.measure "roundtrip.import" case.baseline <|
    importRustFile { source := sourcePath, output := outputPath }
  Benchmark.measure "semantic.compare" case.baseline <|
    checkSemanticRoundTrip case.baseline source original imported.unit
  let commentShape (unit : ValidatedUnit) :=
    unit.namespaces.flatMap fun ns =>
      ns.comments.map fun comment => (comment.text, comment.isDoc, comment.ownLine)
  unless commentShape imported.unit == commentShape original do
    throw <| IO.userError s!"source-backend comment provenance changed {case.baseline}:\n{source}"
  let originalArguments ← match case.arguments original with
    | .ok arguments => pure arguments
    | .error message => throw <| IO.userError message
  let importedArguments ← match case.arguments imported.unit with
    | .ok arguments => pure arguments
    | .error message => throw <| IO.userError message
  let originalResult ← executableOutcome s!"{case.baseline} (original)" original case.functionId
    originalArguments case.initialState
  let importedResult ← executableOutcome s!"{case.baseline} (re-import)" imported.unit case.functionId
    importedArguments case.initialState
  unless outcomesEquivalent importedResult.2 originalResult.2 &&
      statesEquivalent importedResult.1 originalResult.1 do
    throw <| IO.userError s!"source-backend round trip changed {case.baseline}:\n\
      original: {repr originalResult}\nimported: {repr importedResult}\n{source}"

private def checkEnumShapeRoundTrip (directory : System.FilePath) (baseline : String) : IO Unit := do
  let baselinePath : System.FilePath := "rust-exporter/tests/raw-unit" / baseline
  let original ← match decodeAndValidate (← IO.FS.readFile baselinePath) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"enum source-backend input does not validate: {repr diagnostics}"
  let source ← match Rust.Source.render original with
    | .ok source => pure source
    | .error message => throw <| IO.userError s!"enum source-backend input does not render: {message}"
  let sourcePath := directory / "enum-roundtrip.rs"
  let outputPath := directory / "enum-roundtrip.raw.json"
  IO.FS.writeFile sourcePath source
  let imported ← importRustFile { source := sourcePath, output := outputPath }
  let originalShapes ← match enumShapes original with
    | .ok shapes => pure shapes
    | .error message => throw <| IO.userError message
  let importedShapes ← match enumShapes imported.unit with
    | .ok shapes => pure shapes
    | .error message => throw <| IO.userError message
  unless importedShapes == originalShapes do
    throw <| IO.userError s!"enum declaration shape changed after source re-import:\n{source}"

private def checkFunctionShapeRoundTrip (directory : System.FilePath) (baseline : String) :
    IO Unit := do
  let baselinePath : System.FilePath := "rust-exporter/tests/raw-unit" / baseline
  let original ← match decodeAndValidate (← IO.FS.readFile baselinePath) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"reference source-backend input does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.renderWithSourceMap original with
    | .ok rendered => pure rendered
    | .error message =>
        throw <| IO.userError s!"reference source-backend input does not render: {message}"
  checkSourceMap original rendered
  let sourcePath := directory / "reference-roundtrip.rs"
  let outputPath := directory / "reference-roundtrip.raw.json"
  IO.FS.writeFile sourcePath rendered.text
  let imported ← importRustFile { source := sourcePath, output := outputPath }
  let semanticBaselines := #["reference.exp.json", "mutable_reference.exp.json",
    "nested_reference.exp.json", "reference_composite.exp.json", "never.exp.json",
    "generic_adt.exp.json", "generic_lifetime_adt.exp.json",
    "generic_const_adt.exp.json", "nested_generic_adt.exp.json",
    "generic_identity.exp.json", "subslice.exp.json"]
  if semanticBaselines.contains baseline then
    checkSemanticRoundTrip baseline rendered.text original imported.unit
  let originalShapes ← match functionShapes original with
    | .ok shapes => pure shapes
    | .error message => throw <| IO.userError message
  let importedShapes ← match functionShapes imported.unit with
    | .ok shapes => pure shapes
    | .error message => throw <| IO.userError message
  unless importedShapes == originalShapes do
    throw <| IO.userError s!"reference function shape changed after source re-import:\n{rendered.text}"
  let originalNominals ← match nominalShapes original with
    | .ok shapes => pure shapes
    | .error message => throw <| IO.userError message
  let importedNominals ← match nominalShapes imported.unit with
    | .ok shapes => pure shapes
    | .error message => throw <| IO.userError message
  unless importedNominals == originalNominals do
    throw <| IO.userError s!"nominal declaration shape changed after source re-import:\n{rendered.text}"

private def checkMappedRendering (baseline : String) : IO Unit := do
  let baselinePath : System.FilePath := "rust-exporter/tests/raw-unit" / baseline
  let original ← match decodeAndValidate (← IO.FS.readFile baselinePath) with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"mapped source-backend input does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.renderWithSourceMap original with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError s!"mapped source input does not render: {message}"
  checkSourceMap original rendered
  let renderedAgain ← match Rust.Source.renderWithSourceMap original with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError message
  unless renderedAgain == rendered do
    throw <| IO.userError "mapped source rendering is not deterministic"

private def checkGenericFunctionRendering : IO Unit := do
  let baselinePath : System.FilePath := "rust-exporter/tests/raw-unit/basic.exp.json"
  let raw ← match Import.decodeJson (← IO.FS.readFile baselinePath) with
    | .ok unit => pure unit
    | .error message => throw <| IO.userError s!"generic function fixture does not decode: {message}"
  let function := raw.namespaces[0]!.functions[0]!
  let constTypeId : TypeId := ⟨raw.tables.types.size⟩
  let generics : Array GenericBinder := #[
    { name := "a", kind := .lifetime, loc := function.loc,
      predicates := #[.lifetimeOutlives ⟨0⟩ ⟨1⟩] },
    { name := "b", kind := .lifetime, loc := function.loc },
    { name := "T", kind := .typeArg, abilities := #[.copy], loc := function.loc },
    { name := "N", kind := .const,
      type := some { typeId := constTypeId, loc := function.loc }, loc := function.loc }
  ]
  let function := { function with
    signature := { function.signature with generics } }
  let namespaceValue := { raw.namespaces[0]! with
    functions := raw.namespaces[0]!.functions.set! 0 function }
  let raw := {
    raw with
    tables := { raw.tables with
      lifetimes := raw.tables.lifetimes ++ #[
        { kind := .parameter 0, loc := function.loc },
        { kind := .parameter 1, loc := function.loc }]
      types := raw.tables.types.push (.integer .pointer false) }
    namespaces := raw.namespaces.set! 0 namespaceValue }
  let unit ← match Rust.validate raw with
    | .ok unit => pure unit
    | .error diagnostics =>
        throw <| IO.userError s!"generic function fixture does not validate: {repr diagnostics}"
  let rendered ← match Rust.Source.renderWithSourceMap unit with
    | .ok rendered => pure rendered
    | .error message => throw <| IO.userError s!"generic function does not render: {message}"
  unless rendered.text.contains "pub fn answer<'a, 'b, T, const N: usize>()" &&
      rendered.text.contains "where 'a: 'b, T: Copy" do
    throw <| IO.userError s!"generic function binders were not rendered:\n{rendered.text}"
  checkSourceMap unit rendered

def checkCanonicalRustSourceRoundTrips : IO Unit :=
  Benchmark.measure "suite.total" "canonical-rust-source-roundtrips" <|
  IO.FS.withTempDir fun directory => do
    let selectedBaseline ← IO.getEnv "LEANER_RUST_SOURCE_BASELINE"
    let mut failures : Array (String × String) := #[]
    let fixed (arguments : Array RuntimeValue) :
        ValidatedUnit → Except String (Array RuntimeValue) := fun _ => pure arguments
    let pairArgument : ValidatedUnit → Except String (Array RuntimeValue) := fun unit => do
      let some pair := SemanticOperations.findStructHandle? unit "Pair"
        | throw "round-trip unit has no Pair declaration"
      pure #[.nominal pair none #[.integer 7, .integer 9]]
    let nestedPairArgument : ValidatedUnit → Except String (Array RuntimeValue) := fun unit => do
      let some leaf := SemanticOperations.findStructHandle? unit "Leaf"
        | throw "round-trip unit has no Leaf declaration"
      let some pair := SemanticOperations.findStructHandle? unit "Pair"
        | throw "round-trip unit has no Pair declaration"
      pure #[.nominal pair none #[
        .nominal leaf none #[.integer 7], .nominal leaf none #[.integer 9]]]
    let containerArgument : ValidatedUnit → Except String (Array RuntimeValue) := fun unit => do
      let some container := SemanticOperations.findStructHandle? unit "Container"
        | throw "round-trip unit has no Container declaration"
      let some leaf := SemanticOperations.findStructHandle? unit "Leaf"
        | throw "round-trip unit has no Leaf declaration"
      pure #[.nominal container (some "Item") #[.nominal leaf none #[.integer 7]]]
    let nominalArgument (name : String) (variant : Option String)
        (fields : Array RuntimeValue) :
        ValidatedUnit → Except String (Array RuntimeValue) := fun unit => do
      let some declaration := SemanticOperations.findStructHandle? unit name
        | throw s!"round-trip unit has no {name} declaration"
      pure #[.nominal declaration variant fields]
    let enumArgument (name variant : String) (fields : Array RuntimeValue) :=
      nominalArgument name (some variant) fields
    let guardedArgument (variant : String) (fields : Array RuntimeValue) :
        ValidatedUnit → Except String (Array RuntimeValue) := fun unit => do
      let values ← enumArgument "Maybe" variant fields unit
      pure (values.push (.integer 9))
    -- Under prophetic ownership a mutable reference argument owns its
    -- referent: the caller-side loan instance carries the current value
    -- directly. A shared reference is erased to the observed value itself.
    let rustReference (kind : ReferenceKind) (loan : Nat)
        (current : RuntimeValue) : RuntimeValue :=
      match kind with
      | .shared => current
      | .mutable => .borrow loan current
    let lending (loans : Nat) : RuntimeState := { nextLoan := loans }
    let cases : Array RoundTripCase := #[
      { baseline := "basic.exp.json" },
      { baseline := "comments.exp.json", arguments := fixed #[.integer 7] },
      { baseline := "string_slice.exp.json",
        arguments := fixed #[rustReference .shared 0 (.string "Leaner")],
        initialState := lending 1 },
      { baseline := "string_slice.exp.json", functionId := 1,
        arguments := fixed #[rustReference .mutable 0 (.string "Leaner")],
        initialState := lending 1 },
      { baseline := "string_length.exp.json",
        arguments := fixed #[rustReference .shared 0 (.string "Lean🦀")],
        initialState := lending 1 },
      { baseline := "generic_identity.exp.json", functionId := 1,
        arguments := fixed #[.integer 7] },
      { baseline := "generic_identity.exp.json", functionId := 2,
        arguments := fixed #[.integer 11] },
      { baseline := "scalar.exp.json", arguments := fixed #[.integer 42, .integer 15] },
      { baseline := "control.exp.json", arguments := fixed #[.bool true, .integer 7, .integer 9] },
      { baseline := "control.exp.json", arguments := fixed #[.bool false, .integer 7, .integer 9] },
      { baseline := "loop.exp.json", arguments := fixed #[.bool true] },
      { baseline := "loop.exp.json", arguments := fixed #[.bool false] },
      { baseline := "integer_widths.exp.json", arguments := fixed #[
          .integer 255, .integer 65535, .integer 4294967295,
          .integer 18446744073709551615, .integer 340282366920938463463374607431768211455,
          .integer (-128), .integer (-32768), .integer (-2147483648),
          .integer (-9223372036854775808),
          .integer (-170141183460469231731687303715884105728)] },
      { baseline := "aggregate.exp.json", functionId := 0,
        arguments := fixed #[.integer 7, .integer 9] },
      { baseline := "aggregate.exp.json", functionId := 1,
        arguments := fixed #[.tuple #[.integer 7, .bool true]] },
      { baseline := "aggregate.exp.json", functionId := 2,
        arguments := fixed #[.integer 7, .bool true] },
      { baseline := "array_repeat.exp.json", arguments := fixed #[.integer 7] },
      { baseline := "array_index.exp.json", functionId := 0,
        arguments := fixed #[.vector #[.integer 10, .integer 20, .integer 30, .integer 40]] },
      { baseline := "array_index.exp.json", functionId := 1,
        arguments := fixed #[.vector #[.integer 10, .integer 20, .integer 30, .integer 40],
          .integer 2] },
      { baseline := "array_index.exp.json", functionId := 1,
        arguments := fixed #[.vector #[.integer 10, .integer 20, .integer 30, .integer 40],
          .integer 4] },
      { baseline := "array_index.exp.json", functionId := 2,
        arguments := fixed #[.vector #[.integer 10, .integer 20, .integer 30, .integer 40]] },
      { baseline := "multi_call.exp.json", arguments := fixed #[.integer 41] },
      { baseline := "call.exp.json", arguments := fixed #[.bool true, .integer 41] },
      { baseline := "call.exp.json", arguments := fixed #[.bool false, .integer 41] },
      { baseline := "character.exp.json", functionId := 0,
        arguments := fixed #[.character 129408] },
      { baseline := "character.exp.json", functionId := 1,
        arguments := fixed #[.integer 65] },
      { baseline := "character.exp.json", functionId := 2 },
      { baseline := "character.exp.json", functionId := 3,
        arguments := fixed #[.character 97, .character 129408] },
      { baseline := "character.exp.json", functionId := 4,
        arguments := fixed #[.character 129408] },
      { baseline := "variant_move.exp.json", arguments := containerArgument },
      { baseline := "enum.exp.json", functionId := 1,
        arguments := enumArgument "Choice" "First" #[.integer 7] },
      { baseline := "enum.exp.json", functionId := 1,
        arguments := enumArgument "Choice" "Second" #[.integer 7] },
      { baseline := "match_guard.exp.json",
        arguments := guardedArgument "Some" #[.integer 7] },
      { baseline := "match_guard.exp.json",
        arguments := guardedArgument "Some" #[.integer 0] },
      { baseline := "match_guard.exp.json",
        arguments := guardedArgument "None" #[] },
      { baseline := "generic_adt.exp.json",
        arguments := nominalArgument "Wrapper" none #[.integer 7] },
      { baseline := "generic_lifetime_adt.exp.json",
        arguments := nominalArgument "Borrowed" none
          #[rustReference .shared 0 (.integer 7)],
        initialState := lending 1 },
      { baseline := "generic_const_adt.exp.json",
        arguments := nominalArgument "Tagged" none #[.integer 7] },
      { baseline := "nested_generic_adt.exp.json", arguments := fun unit => do
        let inner ← nominalArgument "Wrapper" none #[.integer 7] unit
        nominalArgument "Outer" none inner unit },
      { baseline := "generic_enum.exp.json",
        arguments := fun unit => do
          let values ← enumArgument "Maybe" "Some" #[.integer 7] unit
          pure (values.push (.integer 9)) },
      { baseline := "generic_enum.exp.json",
        arguments := fun unit => do
          let values ← enumArgument "Maybe" "None" #[] unit
          pure (values.push (.integer 9)) },
      { baseline := "function_pointer.exp.json", arguments := fixed #[.integer 41] },
      { baseline := "struct.exp.json", functionId := 0,
        arguments := fixed #[.integer 7, .integer 9] },
      { baseline := "struct.exp.json", functionId := 1, arguments := pairArgument },
      { baseline := "boolean_bitwise.exp.json", arguments := fixed #[.bool true, .bool false] },
      { baseline := "boolean_ordering.exp.json", arguments := fixed #[.bool false, .bool true] },
      { baseline := "bitwise_not.exp.json", functionId := 0, arguments := fixed #[.integer 7] },
      { baseline := "bitwise_not.exp.json", functionId := 1, arguments := fixed #[.integer (-7)] },
      { baseline := "overflowing.exp.json", functionId := 0,
        arguments := fixed #[.integer 250, .integer 10] },
      { baseline := "overflowing.exp.json", functionId := 1,
        arguments := fixed #[.integer (-120), .integer 20] },
      { baseline := "overflowing.exp.json", functionId := 2,
        arguments := fixed #[.integer 500, .integer 500] },
      { baseline := "integer_cast.exp.json", arguments := fixed #[.integer 70000, .integer (-7)] },
      { baseline := "integer_switch.exp.json",
        arguments := fixed #[.integer 0, .integer 7, .integer 9, .integer 11] },
      { baseline := "integer_switch.exp.json",
        arguments := fixed #[.integer 1, .integer 7, .integer 9, .integer 11] },
      { baseline := "integer_switch.exp.json",
        arguments := fixed #[.integer 2, .integer 7, .integer 9, .integer 11] },
      { baseline := "assert.exp.json", arguments := fixed #[.integer 17, .integer 5] },
      { baseline := "assert.exp.json", arguments := fixed #[.integer 17, .integer 0] },
      { baseline := "abort.exp.json" },
      { baseline := "u128_max.exp.json" },
      { baseline := "shift.exp.json", arguments := fixed #[.integer 305419896, .integer 4] },
      { baseline := "signed_division.exp.json",
        arguments := fixed #[.integer (-17), .integer 5] },
      { baseline := "signed_division.exp.json",
        arguments := fixed #[.integer (-128), .integer (-1)] },
      { baseline := "unary.exp.json", arguments := fixed #[.integer 7] },
      { baseline := "borrow_call.exp.json", arguments := fixed #[.integer 7] },
      { baseline := "reference.exp.json",
        arguments := fixed #[rustReference .shared 0 (.integer 7)],
        initialState := lending 1 },
      { baseline := "mutable_reference.exp.json",
        arguments := fixed #[rustReference .mutable 0 (.integer 7), .integer 9],
        initialState := lending 1 },
      { baseline := "nested_reference.exp.json",
        arguments := fixed
          #[rustReference .shared 1 (rustReference .shared 0 (.integer 7))],
        initialState := lending 2 },
      { baseline := "reference_composite.exp.json",
        arguments := fixed #[rustReference .shared 0 (.integer 7),
          rustReference .shared 1 (.integer 9)],
        initialState := lending 2 },
      { baseline := "slice.exp.json",
        arguments := fixed
          #[rustReference .shared 0 (.vector #[.integer 7, .integer 9])],
        initialState := lending 1 },
      { baseline := "slice_from_end.exp.json",
        arguments := fixed #[rustReference .shared 0 (.vector #[])],
        initialState := lending 1 },
      { baseline := "slice_from_end.exp.json",
        arguments := fixed
          #[rustReference .shared 0 (.vector #[.integer 7, .integer 9])],
        initialState := lending 1 },
      { baseline := "subslice.exp.json",
        arguments := fixed #[rustReference .shared 0 (.vector #[])],
        initialState := lending 1 },
      { baseline := "subslice.exp.json",
        arguments := fixed #[rustReference .shared 0
          (.vector #[.integer 7, .integer 9, .integer 11])],
        initialState := lending 1 },
      { baseline := "partial_move.exp.json", arguments := nestedPairArgument }
    ]
    if selectedBaseline.isNone then
      for baseline in #["enum.exp.json", "match_guard.exp.json", "generic_enum.exp.json"] do
        if let some failure ← captureFailure s!"enum shape {baseline}"
            (Benchmark.measure "shape.enum" baseline <|
              checkEnumShapeRoundTrip directory baseline) then
          failures := failures.push failure
      for baseline in #["reference.exp.json", "mutable_reference.exp.json",
          "nested_reference.exp.json", "reference_composite.exp.json", "never.exp.json",
          "match_guard.exp.json", "array_index.exp.json", "slice.exp.json",
          "slice_from_end.exp.json", "subslice.exp.json",
          "generic_adt.exp.json", "generic_lifetime_adt.exp.json",
          "generic_const_adt.exp.json", "nested_generic_adt.exp.json",
          "generic_enum.exp.json", "generic_identity.exp.json"] do
        if let some failure ← captureFailure s!"function shape {baseline}"
            (Benchmark.measure "shape.function" baseline <|
              checkFunctionShapeRoundTrip directory baseline) then
          failures := failures.push failure
      if let some failure ← captureFailure "generic function rendering"
          (Benchmark.measure "shape.generic_render" "basic.exp.json"
            checkGenericFunctionRendering) then
        failures := failures.push failure
    for index in [:cases.size] do
      let some case := cases[index]?
        | throw <| IO.userError s!"missing source round-trip case #{index}"
      if selectedBaseline.all (· == case.baseline) then
        if let some failure ← captureFailure
            s!"case #{index} {case.baseline} function {case.functionId}"
            (Benchmark.measure "case.total" case.baseline <| checkCase directory case) then
          failures := failures.push failure
    unless failures.isEmpty do
      let details := failures.map fun (label, message) =>
        if selectedBaseline.isSome then s!"- {label}:\n{message}"
        else s!"- {label}: {firstLine message}"
      let rerun := if selectedBaseline.isNone then
          "\nRerun a failure with LEANER_RUST_SOURCE_BASELINE=<baseline> for full diagnostics."
        else ""
      throw <| IO.userError s!"Rust source round-trip failures ({failures.size}):\n\
        {String.intercalate "\n" details.toList}{rerun}"

#guard_msgs in
#eval checkCanonicalRustSourceRoundTrips

end LeanerIR.Rust.Tests.Source
