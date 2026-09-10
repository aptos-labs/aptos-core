-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust

namespace LeanerIR.Rust.Tests

open LeanerIR
open LeanerIR.Import
open LeanerIR.Validation

def checkRustSemanticProjection : IO Unit := do
  let path : System.FilePath := "rust-exporter/tests/raw-unit/scalar.exp.json"
  let raw ← match Import.decodeJson (← IO.FS.readFile path) with
    | .ok raw => pure raw
    | .error message => throw <| IO.userError s!"semantic projection fixture does not decode: {message}"
  let original ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"semantic projection fixture does not validate: {repr diagnostics}"
  let ns := raw.namespaces[0]!
  let function := ns.functions[0]!
  let renamedLocals := function.locals.mapIdx fun index localDecl =>
    { localDecl with name := s!"alpha_{index}" }
  let renamedRaw := { raw with namespaces := raw.namespaces.set! 0 { ns with
    functions := ns.functions.set! 0 { function with locals := renamedLocals } } }
  let renamed ← match validate renamedRaw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"alpha-renamed fixture does not validate: {repr diagnostics}"
  unless (Rust.Equivalence.semanticallyAlphaEquivalent original renamed).toOption == some true do
    throw <| IO.userError "local spelling changed Rust semantic alpha-equivalence"
  let changedRaw := { raw with namespaces := raw.namespaces.set! 0 { ns with
    expressions := ns.expressions.set! 2 { ns.expressions[2]! with
      kind := .operation (.primitive .add) #[] #[⟨0⟩, ⟨1⟩] } } }
  let changed ← match validate changedRaw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"semantic-change fixture does not validate: {repr diagnostics}"
  unless (Rust.Equivalence.semanticallyAlphaEquivalent original changed).toOption == some false do
    throw <| IO.userError "semantic operation change survived Rust alpha-equivalence"
  let arrayPath : System.FilePath := "rust-exporter/tests/raw-unit/array_index.exp.json"
  let arrayUnit ← match decodeAndValidate (← IO.FS.readFile arrayPath) with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"array semantic fixture does not validate: {repr diagnostics}"
  let arrayProjection ← match Rust.Equivalence.semanticProjection arrayUnit with
    | .ok projection => pure projection
    | .error message => throw <| IO.userError s!"array semantic projection failed: {message}"
  let some semanticNamespace := arrayProjection.namespaces[0]?
    | throw <| IO.userError "array semantic projection lost its namespace"
  let some dynamicGet := semanticNamespace.functions.find? (·.name == "n0::get")
    | throw <| IO.userError "array semantic projection lost dynamic get"
  unless dynamicGet.body.contains "ThrowKind.panic" do
    throw <| IO.userError "array semantic projection erased a real dynamic bounds panic"
  for name in #["n0::destructure", "n0::get_third"] do
    let some function := semanticNamespace.functions.find? (·.name == name)
      | throw <| IO.userError s!"array semantic projection lost {name}"
    if function.body.contains "ThrowKind.panic" then
      throw <| IO.userError s!"array semantic projection retained a statically redundant panic in {name}"

#guard_msgs in
#eval checkRustSemanticProjection

private def tables : Tables where
  files := #[{ name := "fixture.rs", contentHash := "m0" }]
  locations := #[{ primary := some { file := ⟨0⟩, startByte := 0, endByte := 1 } }]
  origins := #[{ kind := .rustMir, location := ⟨0⟩ }]
  alignments := #[{ source := ⟨0⟩, trust := .checked, description := "M0 Rust fixture" }]
  types := #[.bool]
  namespaces := #[{ segments := #["fixture"] }]
  names := #[{ namespaceId := ⟨0⟩, name := "answer" }]

private def fixture : RawUnit where
  tables
  profiles := #[config]
  namespaces := #[{
    loc := ⟨0⟩
    identity := ⟨0⟩
    profile := some .rust
    expressions := #[{ loc := ⟨0⟩, typeId := ⟨0⟩, kind := .value (.bool true) }]
    functions := #[{
      loc := ⟨0⟩
      name := ⟨0⟩
      profile := .rust
      signature := { results := #[{ typeId := ⟨0⟩, loc := ⟨0⟩ }] }
      body := .structured ⟨0⟩
      origin := ⟨0⟩
      alignment := ⟨0⟩ }] }]

#guard match validate fixture with
  | .ok checked => checked.indexes.functionCounts == #[1]
  | .error _ => false

#guard match decodeAndValidate (encodeJson fixture) with
  | .ok checked => checked.indexes.functionCounts == #[1]
  | .error _ => false

#guard match validate fixture with
  | .ok checked => (prepareExecution #[semantics] checked).isOk
  | .error _ => false

#guard match validate { fixture with
    namespaces := #[{ fixture.namespaces[0]! with
      functions := #[{ fixture.namespaces[0]!.functions[0]! with
        profileData := #[{ profile := .rust, tag := "not-yet-supported" }] }] }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-RUST-TAG")
  | .ok _ => false

#guard match validate { fixture with
    profiles := #[{ config with options := #[
      ("panic", "unwind"),
      ("unsafe", "reject"),
      ("target_pointer_width", "64")
    ] }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-RUST-PANIC")
  | .ok _ => false

#guard match validate { fixture with
    profiles := #[{ config with options := #[
      ("panic", "abort"),
      ("unsafe", "allow"),
      ("target_pointer_width", "64")
    ] }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-RUST-UNSAFE")
  | .ok _ => false

#guard match validate { fixture with
    profiles := #[{ config with options := #[
      ("panic", "abort"),
      ("unsafe", "reject"),
      ("target_pointer_width", "host")
    ] }] } with
  | .error diagnostics => diagnostics.any (·.code == "LIR-RUST-TARGET-WIDTH")
  | .ok _ => false

/-- Checked-in baselines use the exporter's human-readable whitespace. Compact
the parsed value before the existing canonical RawUnit spelling checks. -/
private def readBaselineJson (path : System.FilePath) : IO String := do
  let text ← IO.FS.readFile path
  match Import.decodeJson text with
  | .error message =>
      throw <| IO.userError s!"Rust exporter baseline {path} does not decode: {message}"
  | .ok _ => pure ()
  match Lean.Json.parse text with
  | .ok json => pure json.compress
  | .error message =>
      throw <| IO.userError s!"Rust exporter baseline {path} is invalid JSON: {message}"

def checkRustExporterBaseline : IO Unit := do
  let cases : Array (String × String × Nat) := #[
    ("basic.exp.json", "tests/raw-unit/basic.rs", 0),
    ("scalar.exp.json", "tests/raw-unit/scalar.rs", 2),
    ("signed_division.exp.json", "tests/raw-unit/signed_division.rs", 2),
    ("control.exp.json", "tests/raw-unit/control.rs", 3),
    ("loop.exp.json", "tests/raw-unit/loop.rs", 1),
    ("integer_widths.exp.json", "tests/raw-unit/integer_widths.rs", 10),
    ("integer_cast.exp.json", "tests/raw-unit/integer_cast.rs", 2),
    ("boolean_bitwise.exp.json", "tests/raw-unit/boolean_bitwise.rs", 2),
    ("u128_max.exp.json", "tests/raw-unit/u128_max.rs", 0),
    ("reference.exp.json", "tests/raw-unit/reference.rs", 1),
    ("mutable_reference.exp.json", "tests/raw-unit/mutable_reference.rs", 2)]
  for (file, source, parameterCount) in cases do
    let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
    let text ← readBaselineJson path
    let raw ← match Import.decodeJson text with
      | .ok raw => pure raw
      | .error message =>
          throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
    unless Import.encodeJson raw == text do
      throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
    let checked ← match validate raw with
      | .ok checked => pure checked
      | .error diagnostics =>
          throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
    unless checked.evidence == #[{
        producer := "leaner-rust-export via rustc Public"
        description :=
          "rustc accepted this crate through analysis and exposed optimized generic MIR for a 64-bit pointer target"
        trusted := true }] do
      throw <| IO.userError s!"Rust exporter baseline {file} lost its explicit rustc admission evidence"
    unless (profileConfig? checked.profiles .rust |>.bind
        (·.options.find? (·.1 == "target_pointer_width")) |>.map (·.2)) == some "64" do
      throw <| IO.userError s!"Rust exporter baseline {file} lost its target pointer width"
    let actualParameterCount := checked.namespaces[0]? |>.bind fun ns =>
      ns.functions[0]? |>.map (·.signature.parameters.size)
    unless checked.indexes.functionCounts == #[1] &&
        checked.tables.files[0]?.map (·.name) == some source &&
        actualParameterCount == some parameterCount do
      throw <| IO.userError s!"Rust exporter baseline {file} lost its function signature or source table"
    let executable ← match prepareExecution #[semantics] checked with
      | .ok executable => pure executable
      | .error diagnostics =>
          throw <| IO.userError s!"Rust exporter baseline {file} is not executable core LIR: \
            {repr diagnostics}"
    if file == "integer_widths.exp.json" then
      let arguments : Array RuntimeValue := #[
        .integer 255, .integer 65535, .integer 4294967295,
        .integer 18446744073709551615, .integer 340282366920938463463374607431768211455,
        .integer (-128), .integer (-32768), .integer (-2147483648),
        .integer (-9223372036854775808), .integer (-170141183460469231731687303715884105728)]
      match Interpreter.run executable 8 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } arguments with
      | .ok (_, { value := .returned #[.integer (-7)], .. }) => pure ()
      | result =>
          throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterBaseline

def checkRustExporterCallBaseline : IO Unit := do
  let file := "call.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let namespace0 : NamespaceId := ⟨0⟩
  let expression3 : ExprId := ⟨3⟩
  let returnLocal : LocalId := ⟨2⟩
  let block3 : BlockId := ⟨3⟩
  match ns.expressions[3]? with
    | some expression => match expression.kind with
        | .operation (.call (.function callee)) _ arguments _ =>
            unless arguments.size == 2 && callee.namespaceId == namespace0 &&
                raw.tables.names[callee.name.index]?.map (·.name) == some "recurse" do
              throw <| IO.userError s!"Rust exporter baseline {file} lost its direct callee identity"
        | _ => throw <| IO.userError s!"Rust exporter baseline {file} lost its direct call expression"
    | none => throw <| IO.userError s!"Rust exporter baseline {file} lost its direct call expression"
  match ns.functions[0]? with
    | some function => match function.body with
        | .cfg graph => match graph.blocks[1]? with
            | some block => match block.terminator with
                | .call call (some destination) .unreachable =>
                    let destinationIsReturn :=
                      ns.places[destination.place.index]? == some (.localVar returnLocal)
                    unless call == expression3 && destinationIsReturn && destination.target == block3 do
                      throw <| IO.userError s!"Rust exporter baseline {file} lost its call destination or unwind edge"
                | _ => throw <| IO.userError s!"Rust exporter baseline {file} lost its raw call terminator"
            | none => throw <| IO.userError s!"Rust exporter baseline {file} lost its call block"
        | _ => throw <| IO.userError s!"Rust exporter baseline {file} lost its raw CFG"
    | none => throw <| IO.userError s!"Rust exporter baseline {file} lost its function"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics => throw <| IO.userError s!"Rust exporter baseline {file} is not executable after call structurization: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterCallBaseline

def checkRustExporterMultiCallBaseline : IO Unit := do
  let file := "multi_call.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  unless ns.functions.size == 2 && raw.tables.names.map (·.name) == #["apply", "transform"] do
    throw <| IO.userError s!"Rust exporter baseline {file} lost deterministic function identities"
  match ns.expressions[1]? with
    | some expression => match expression.kind with
        | .operation (.call (.function callee)) _ arguments _ =>
            unless arguments.size == 1 && callee.namespaceId == ⟨0⟩ &&
                raw.tables.names[callee.name.index]?.map (·.name) == some "transform" do
              throw <| IO.userError s!"Rust exporter baseline {file} lost its non-recursive callee"
        | _ => throw <| IO.userError s!"Rust exporter baseline {file} lost its call expression"
    | none => throw <| IO.userError s!"Rust exporter baseline {file} lost its call expression"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics => throw <| IO.userError s!"Rust exporter baseline {file} is not executable after call structurization: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterMultiCallBaseline

def checkRustExporterFunctionPointerBaseline : IO Unit := do
  let file := "function_pointer.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasFunctionType := raw.tables.types.any fun
    | .function #[_] _ abilities => abilities.contains .copy && abilities.contains .drop
    | _ => false
  let hasClosure := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.call (.closure _)) _ #[] _
  let hasInvoke := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.call .invoke) _ #[_, _] _
  unless hasFunctionType && hasClosure && hasInvoke do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its function type, reification, or invocation"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  match Interpreter.run executable 64
      { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[.integer 41] with
  | .ok (_, { value := .returned #[.integer 42], .. }) => pure ()
  | result =>
      throw <| IO.userError s!"Rust exporter baseline {file} did not invoke its function pointer: {repr result}"

#guard_msgs in
#eval checkRustExporterFunctionPointerBaseline

def checkRustExporterIntegerSwitchBaseline : IO Unit := do
  let file := "integer_switch.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some function := raw.namespaces[0]?.bind (·.functions[0]?)
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its function"
  match function.body with
    | .cfg graph => match graph.blocks[0]? with
        | some block => match block.terminator with
            | .switch _ cases defaultTarget =>
                unless cases.map (·.1) == #[.integer 0, .integer 1] && defaultTarget == ⟨1⟩ do
                  throw <| IO.userError s!"Rust exporter baseline {file} lost its switch values or default edge"
            | _ => throw <| IO.userError s!"Rust exporter baseline {file} lost its raw switch"
        | none => throw <| IO.userError s!"Rust exporter baseline {file} lost its entry block"
    | _ => throw <| IO.userError s!"Rust exporter baseline {file} lost its raw CFG"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics => throw <| IO.userError s!"Rust exporter baseline {file} is not executable after switch structurization: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterIntegerSwitchBaseline

def checkRustExporterAggregateBaseline : IO Unit := do
  let file := "aggregate.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasTupleType := raw.tables.types.any fun type => type matches .tuple _
  let hasArrayType := raw.tables.types.any fun
    | .vector _ (some (.integer 2)) => true
    | _ => false
  let hasTupleConstructor := ns.expressions.any fun expression => match expression.kind with
    | .operation (.primitive .tuple) _ arguments _ => arguments.size == 2
    | _ => false
  let hasArrayConstructor := ns.expressions.any fun expression => match expression.kind with
    | .operation (.primitive .vector) _ arguments _ => arguments.size == 2
    | _ => false
  let hasTupleIndex := ns.places.any fun place => place matches .index _ _
  unless hasTupleType && hasArrayType && hasTupleConstructor && hasArrayConstructor &&
      hasTupleIndex do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its aggregate types, constructors, or tuple index"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics => throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterAggregateBaseline

def checkRustExporterStructBaseline : IO Unit := do
  let file := "struct.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let fields := ns.structs[0]?.map (·.fields.map fun field => raw.tables.names[field.name.index]!.name)
  let hasConstructor := ns.expressions.any fun expression => match expression.kind with
    | .operation (.call (.constructor reference none)) _ arguments _ =>
        arguments.size == 2 && raw.tables.names[reference.name.index]?.map (·.name) == some "Pair"
    | _ => false
  let hasFieldPlace := ns.places.any fun place => place matches .field ..
  unless ns.structs.size == 1 && fields == some #["first", "second"] &&
      ns.structs[0]!.variants.isEmpty && hasConstructor && hasFieldPlace do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its struct declaration, constructor, or field place"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics => throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterStructBaseline

def checkRustExporterPartialMoveBaseline : IO Unit := do
  let file := "partial_move.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasProjectedMove := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    match expression.kind with
    | .operation (.move place) _ _ _ => match ns.places[place.index]? with
        | some (.field ..) => true
        | _ => false
    | _ => false
  unless hasProjectedMove do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its partial field move"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not prepare: {repr diagnostics}"
  unless executable.initializationCertificates.size == 1 do
    throw <| IO.userError s!"Rust exporter baseline {file} lost initialization evidence"
  let some leafHandle := SemanticOperations.findStructHandle? checked "Leaf"
    | throw <| IO.userError "missing Leaf declaration"
  let some pairHandle := SemanticOperations.findStructHandle? checked "Pair"
    | throw <| IO.userError "missing Pair declaration"
  let first : RuntimeValue := .nominal leafHandle none #[.integer 7]
  let second : RuntimeValue := .nominal leafHandle none #[.integer 9]
  let pair : RuntimeValue := .nominal pairHandle none #[first, second]
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  match Interpreter.run executable 16 handle #[pair] with
  | .ok (_, outcome) => unless outcome.value == .returned #[first] do
      throw <| IO.userError s!"Rust exporter baseline {file} returned the wrong moved field"
  | .error error =>
      throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr error}"

#guard_msgs in
#eval checkRustExporterPartialMoveBaseline

private def isVariantQualifiedFieldPlace (ns : RawNamespace) (place : PlaceId) : Bool :=
  match ns.places[place.index]? with
  | some (.field base ..) => match ns.places[base.index]? with
      | some (.downcast _ _) => true
      | _ => false
  | _ => false

def checkRustExporterVariantMoveBaseline : IO Unit := do
  let file := "variant_move.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasVariantMove := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    match expression.kind with
    | .operation (.move place) _ _ _ => isVariantQualifiedFieldPlace ns place
    | _ => false
  unless hasVariantMove do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its variant-qualified field move"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not prepare: {repr diagnostics}"
  let some containerHandle := SemanticOperations.findStructHandle? checked "Container"
    | throw <| IO.userError "missing Container declaration"
  let some leafHandle := SemanticOperations.findStructHandle? checked "Leaf"
    | throw <| IO.userError "missing Leaf declaration"
  let leaf : RuntimeValue := .nominal leafHandle none #[.integer 7]
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  let input : RuntimeValue := .nominal containerHandle (some "Item") #[leaf]
  match Interpreter.run executable 16 handle #[input] with
  | .ok (_, outcome) => unless outcome.value == .returned #[leaf] do
      throw <| IO.userError s!"Rust exporter baseline {file} returned the wrong variant payload"
  | .error error =>
      throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr error}"

#guard_msgs in
#eval checkRustExporterVariantMoveBaseline

def checkRustExporterEnumBaseline : IO Unit := do
  let file := "enum.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasDiscriminant := ns.expressions.any fun expression => match expression.kind with
    | .operation (.data (.discriminant reference)) _ arguments _ =>
        arguments.size == 1 && raw.tables.names[reference.name.index]?.map (·.name) == some "Choice"
    | _ => false
  let hasProjectedPayload := (ns.places.any fun place => place matches .downcast _ _) &&
    (ns.places.any fun place => place matches .field ..)
  let hasConstructor := ns.expressions.any fun expression => match expression.kind with
    | .operation (.call (.constructor reference (some "First"))) _ arguments _ =>
        arguments.size == 1 && raw.tables.names[reference.name.index]?.map (·.name) == some "Choice"
    | _ => false
  let discriminants := ns.structs[0]?.map fun declaration =>
    declaration.variants.map (·.discriminant)
  unless hasDiscriminant && hasProjectedPayload && hasConstructor &&
      discriminants == some #[some 4, some 9] do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its enum declaration, constructor, discriminants, or payload places"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  let hasExhaustiveMatch := checked.namespaces[0]?.any fun checkedNs =>
    checkedNs.expressions.any fun expression => match expression.kind with
      | .match_ _ arms => arms.size == 2
      | _ => false
  unless hasExhaustiveMatch do
    throw <| IO.userError s!"Rust exporter baseline {file} did not eliminate its impossible default edge"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not target-width executable: {repr diagnostics}"
  let some choiceHandle := SemanticOperations.findStructHandle? checked "Choice"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its enum declaration"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨1⟩ }
  for variant in #["First", "Second"] do
    let input : RuntimeValue := .nominal choiceHandle (some variant) #[.integer 7]
    match Interpreter.run executable 32 handle #[input] with
    | .ok (_, { value := .returned #[.integer 7], .. }) => pure ()
    | result =>
        throw <| IO.userError s!"Rust exporter baseline {file} did not execute {variant}: {repr result}"

#guard_msgs in
#eval checkRustExporterEnumBaseline

def checkRustExporterDropBaseline : IO Unit := do
  let file := "drop.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasDrop := ns.functions.any fun function => match function.body with
    | .cfg graph => graph.blocks.any fun block => match block.terminator with
        | .drop _ _ _ => true
        | _ => false
    | .absent | .structured _ => false
  unless hasDrop do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its MIR drop terminator"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize its fixed panic=abort drop: {repr diagnostics}"
  let hasStructurizedDrop := checked.namespaces.any fun checkedNs =>
    checkedNs.expressions.any fun expression => match expression.kind with
      | .operation (.drop _) _ _ => true
      | _ => false
  unless hasStructurizedDrop do
    throw <| IO.userError s!"Rust exporter baseline {file} erased its structured drop operation"

#guard_msgs in
#eval checkRustExporterDropBaseline

private def hasStructurizedPanic (unit : ValidatedUnit) : Bool :=
  unit.namespaces.any fun ns =>
    (ns.functions.all fun function => function.body matches .structured _) &&
      ns.expressions.any (·.kind == .throw_ .panic #[])

def checkRustExporterAssertBaseline : IO Unit := do
  let file := "assert.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasAssert := raw.namespaces.any fun ns => ns.functions.any fun function =>
    match function.body with
    | .cfg graph => graph.blocks.any fun block => match block.terminator with
        | .assert _ false .divisionByZero _ .unreachable => true
        | _ => false
    | .absent | .structured _ => false
  unless hasAssert do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its MIR division assertion"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize its assertion: {repr diagnostics}"
  unless hasStructurizedPanic checked do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its structured panic branch"
  unless (prepareExecution #[semantics] checked).isOk do
    throw <| IO.userError s!"Rust exporter baseline {file} is not executable after assertion structurization"

#guard_msgs in
#eval checkRustExporterAssertBaseline

def checkRustExporterAbortBaseline : IO Unit := do
  let file := "abort.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasAbort := raw.namespaces.any fun ns => ns.functions.any fun function =>
    match function.body with
    | .cfg graph => graph.blocks.any fun block => block.terminator == .abort
    | .absent | .structured _ => false
  unless hasAbort do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its MIR abort terminator"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  unless hasStructurizedPanic checked do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its structured panic terminal"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not prepare: {repr diagnostics}"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  match Interpreter.run executable 8 handle #[] with
  | .ok (_, outcome) => unless outcome.value == .threw .panic #[] do
      throw <| IO.userError s!"Rust exporter baseline {file} did not execute as terminal panic"
  | .error error =>
      throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr error}"

#guard_msgs in
#eval checkRustExporterAbortBaseline

def checkRustExporterArrayIndexBaseline : IO Unit := do
  let file := "array_index.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasDynamicPointerIndex := ns.places.any fun place => match place with
    | .index _ index => match ns.expressions[index.index]? with
        | some expression =>
            raw.tables.types[expression.typeId.index]? == some (.integer .pointer false) &&
              (match expression.kind with
                | .operation (.copy _) _ arguments _ => arguments.isEmpty
                | _ => false)
        | none => false
    | _ => false
  let hasLiteralPointerIndex := ns.places.any fun place => match place with
    | .index _ index => match ns.expressions[index.index]? with
        | some expression =>
            raw.tables.types[expression.typeId.index]? == some (.integer .pointer false) &&
              expression.kind == .value (.integer 0)
        | none => false
    | _ => false
  let hasBoundsCheck := ns.functions.any fun function => match function.body with
    | .cfg graph => graph.blocks.any fun block => match block.terminator with
        | .assert _ true .boundsCheck _ .unreachable => true
        | _ => false
    | .absent | .structured _ => false
  unless ns.functions.size == 3 && hasDynamicPointerIndex && hasLiteralPointerIndex &&
      hasBoundsCheck do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its array indexes or bounds assertion"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize its bounds assertion: {repr diagnostics}"
  unless hasStructurizedPanic checked do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its structured bounds panic branch"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not target-width executable: {repr diagnostics}"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨1⟩ }
  let values := .vector #[.integer 10, .integer 20, .integer 30, .integer 40]
  match Interpreter.run executable 24 handle #[values, .integer 2] with
  | .ok (_, { value := .returned #[.integer 30], .. }) => pure ()
  | result =>
      throw <| IO.userError s!"Rust exporter baseline {file} did not execute a usize index: {repr result}"
  match Interpreter.run executable 24 handle #[values, .integer 4] with
  | .ok (_, { value := .threw .panic _, .. }) => pure ()
  | result =>
      throw <| IO.userError s!"Rust exporter baseline {file} did not preserve its bounds panic: {repr result}"

#guard_msgs in
#eval checkRustExporterArrayIndexBaseline

def checkRustExporterSliceBaseline : IO Unit := do
  let file := "slice.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  unless raw.tables.types.any (fun type => type matches .vector _ none) do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its dynamic slice type"
  let hasDereferenceAndLength := raw.namespaces.any fun ns =>
    (ns.expressions.any fun expression =>
      expression.kind matches .operation (.reference .dereference) _ _ _) &&
    (ns.expressions.any fun expression =>
      expression.kind matches .operation (.primitive .length) _ _ _) &&
    ns.places.any (· matches .index _ _)
  unless hasDereferenceAndLength do
    throw <| IO.userError s!"Rust exporter baseline {file} lost slice metadata or indexing"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  unless hasStructurizedPanic checked do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its structured bounds panic branch"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not target-width executable: {repr diagnostics}"
  let borrowedVector (elements : Array RuntimeValue) : RuntimeValue :=
    (.vector elements)
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  let lending : RuntimeState := { nextLoan := 1 }
  match Interpreter.run executable 32 handle
      #[borrowedVector #[.integer 7, .integer 9]] lending with
  | .ok (_, { value := .returned #[.integer 7], .. }) => pure ()
  | result => throw <| IO.userError s!"Rust exporter baseline {file} returned the wrong slice value: {repr result}"
  match Interpreter.run executable 32 handle #[borrowedVector #[]] lending with
  | .ok (_, { value := .threw .panic _, .. }) => pure ()
  | result => throw <| IO.userError s!"Rust exporter baseline {file} lost its bounds panic: {repr result}"

#guard_msgs in
#eval checkRustExporterSliceBaseline

def checkRustExporterSubsliceBaseline : IO Unit := do
  let file := "subslice.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  unless raw.namespaces.any fun ns =>
      ns.places.any (· matches .subslice _ 1 0 true) do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its from-end subslice place"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  unless checked.namespaces.all fun ns =>
      ns.functions.all fun function => function.body matches .structured _ do
    throw <| IO.userError s!"Rust exporter baseline {file} did not structurize"

#guard_msgs in
#eval checkRustExporterSubsliceBaseline

def checkRustExporterFromEndSliceBaseline : IO Unit := do
  let file := "slice_from_end.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasFromEndIndex := raw.namespaces.any fun ns =>
    ns.places.any fun place => match place with
      | .index _ index => (ns.expressions[index.index]?).any (fun expression =>
          expression.kind matches .operation (.primitive .subtract) _ _ _)
      | _ => false
  unless hasFromEndIndex do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its length-minus-offset index"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  unless checked.namespaces.all fun ns =>
      ns.functions.all fun function => function.body matches .structured _ do
    throw <| IO.userError s!"Rust exporter baseline {file} did not structurize"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not target-width executable: {repr diagnostics}"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  for (values, expected) in [
      (#[], 0),
      (#[RuntimeValue.integer 7, .integer 9], 9)] do
    let state : RuntimeState := { nextLoan := 1 }
    match Interpreter.run executable 64 handle
        #[(.vector values)] state with
    | .ok (_, { value := .returned #[.integer value], .. }) => unless value == expected do
        throw <| IO.userError s!"Rust exporter baseline {file} returned {value}, expected {expected}"
    | result => throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterFromEndSliceBaseline

def checkRustExporterArrayRepeatBaseline : IO Unit := do
  let file := "array_repeat.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasCompactRepeat := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    match expression.kind, raw.tables.types[expression.typeId.index]? with
    | .operation (.primitive .repeatVector) _ arguments _,
        some (.vector _ (some (.integer 4))) => arguments.size == 1
    | _, _ => false
  unless hasCompactRepeat do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its compact repeated-vector operation"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  match Interpreter.run executable 16
      { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[.integer 7] with
    | .ok (_, { value := .returned #[.vector values], .. }) =>
        unless values == #[.integer 7, .integer 7, .integer 7, .integer 7] do
          throw <| IO.userError s!"Rust exporter baseline {file} has incorrect repeat semantics"
    | result =>
        throw <| IO.userError s!"Rust exporter baseline {file} did not return a repeated vector: {repr result}"

#guard_msgs in
#eval checkRustExporterArrayRepeatBaseline

def checkRustExporterSignedDivisionBaseline : IO Unit := do
  let file := "signed_division.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasDivide := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .divide) _ #[_, _] _
  let hasModulo := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .modulo) _ #[_, _] _
  unless hasDivide && hasModulo do
    throw <| IO.userError s!"Rust exporter baseline {file} lost signed division or remainder"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  for (arguments, expected) in [
      (#[.integer (-7), .integer 3], #[.integer (-2), .integer (-1)]),
      (#[.integer 7, .integer (-3)], #[.integer (-2), .integer 1])] do
    match Interpreter.run executable 64 handle arguments with
      | .ok (_, { value := .returned #[.tuple values], .. }) =>
          unless values == expected do
            throw <| IO.userError s!"Rust exporter baseline {file} returned {repr values}, expected {repr expected}"
      | result =>
          throw <| IO.userError s!"Rust exporter baseline {file} did not execute signed division: {repr result}"

#guard_msgs in
#eval checkRustExporterSignedDivisionBaseline

def checkRustExporterUnaryBaseline : IO Unit := do
  let file := "unary.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasNegate := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    match expression.kind with
    | .operation (.primitive .negate) _ arguments _ => arguments.size == 1
    | _ => false
  let hasOverflowCheck := raw.namespaces.any fun ns => ns.functions.any fun function =>
    match function.body with
    | .cfg graph => graph.blocks.any fun block => match block.terminator with
        | .assert _ false .overflow _ .unreachable => true
        | _ => false
    | .absent | .structured _ => false
  unless hasNegate && hasOverflowCheck do
    throw <| IO.userError s!"Rust exporter baseline {file} lost negation or its overflow assertion"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize its overflow assertion: {repr diagnostics}"
  unless hasStructurizedPanic checked do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its structured overflow panic branch"
  unless (prepareExecution #[semantics] checked).isOk do
    throw <| IO.userError s!"Rust exporter baseline {file} is not executable after overflow assertion structurization"

#guard_msgs in
#eval checkRustExporterUnaryBaseline

def checkRustExporterBorrowCallBaseline : IO Unit := do
  let file := "borrow_call.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasReferenceType := raw.tables.types.any fun
    | .reference reference => reference.profile == .rust && reference.kind == .shared
    | _ => false
  let hasBorrow := ns.expressions.any fun expression => match expression.kind with
    | .operation (.borrow .immutable _) _ _ _ => true
    | _ => false
  let hasDeref := ns.places.any fun
    | .deref _ => true
    | _ => false
  unless hasReferenceType && hasBorrow && hasDeref && !raw.tables.lifetimes.isEmpty do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its reference, borrow, dereference, or lifetime"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics => throw <| IO.userError s!"Rust exporter baseline {file} is not executable after call structurization: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterBorrowCallBaseline

def checkRustExporterNeverBaseline : IO Unit := do
  let file := "never.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  unless raw.tables.types.any (fun type => type == .never) do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its never type"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not structurize: {repr diagnostics}"
  let some function := checked.namespaces[0]?.bind (fun ns => ns.functions[0]?)
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its function"
  unless function.body matches .structured _ do
    throw <| IO.userError s!"Rust exporter baseline {file} did not structurize its diverging loop"

#guard_msgs in
#eval checkRustExporterNeverBaseline

def checkRustExporterNestedReferenceBaseline : IO Unit := do
  let file := "nested_reference.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let referenceTypes := raw.tables.types.filter fun
    | .reference reference => reference.profile == .rust
    | _ => false
  let dereferencePlaces := raw.namespaces.foldl (init := 0) fun count ns =>
    count + ns.places.countP fun
      | .deref _ => true
      | _ => false
  unless referenceTypes.size == 2 && dereferencePlaces == 2 do
    throw <| IO.userError s!"Rust exporter baseline {file} lost a nested reference layer or dereference"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterNestedReferenceBaseline

def checkRustExporterReferenceCompositeBaseline : IO Unit := do
  let file := "reference_composite.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasReferenceTuple := raw.tables.types.any fun
    | .tuple elements => elements.size == 2 && elements.all fun typeId =>
        match raw.tables.types[typeId.index]? with
        | some (.reference reference) => reference.profile == .rust
        | _ => false
    | _ => false
  unless hasReferenceTuple do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its reference-bearing tuple"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterReferenceCompositeBaseline

def checkRustExporterGenericAdtBaseline : IO Unit := do
  let file := "generic_adt.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasGenericDeclaration := raw.namespaces.any fun ns => ns.structs.any fun declaration =>
    declaration.generics.size == 1 && declaration.generics[0]!.kind == .typeArg
  let hasConcreteNominal := raw.tables.types.any fun
    | .nominal _ #[.typeArg argument] =>
        raw.tables.types[argument.typeId.index]? == some (.integer (.bits 32) false)
    | _ => false
  unless hasGenericDeclaration && hasConcreteNominal do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its generic binder or concrete argument"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  let some wrapperHandle := SemanticOperations.findStructHandle? checked "Wrapper"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its nominal declaration"
  let input : RuntimeValue := .nominal wrapperHandle none #[.integer 7]
  match Interpreter.run executable 24 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[input] with
  | .ok (_, { value := .returned #[.integer 7], .. }) => pure ()
  | result => throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterGenericAdtBaseline

def checkRustExporterGenericLifetimeAdtBaseline : IO Unit := do
  let file := "generic_lifetime_adt.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasLifetimeDeclaration := raw.namespaces.any fun ns => ns.structs.any fun declaration =>
    declaration.generics.size == 2 &&
      declaration.generics[0]!.kind == .lifetime &&
      declaration.generics[1]!.kind == .typeArg
  let hasLifetimeNominal := raw.tables.types.any fun
    | .nominal _ #[.lifetime _, .typeArg _] => true
    | _ => false
  unless hasLifetimeDeclaration && hasLifetimeNominal && !raw.tables.lifetimes.isEmpty do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its lifetime binder or argument"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  let some borrowedName := SemanticOperations.findStructHandle? checked "Borrowed"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its nominal declaration"
  let reference : RuntimeValue := .integer 7
  let input : RuntimeValue := .nominal borrowedName none #[reference]
  let state : RuntimeState := { nextLoan := 1 }
  match Interpreter.run executable 24 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
      #[input] state with
  | .ok (finalState, { value := .returned #[.integer 7], .. }) =>
      unless finalState == state do
        throw <| IO.userError s!"Rust exporter baseline {file} changed inherited reference storage"
  | result => throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterGenericLifetimeAdtBaseline

def checkRustExporterGenericConstAdtBaseline : IO Unit := do
  let file := "generic_const_adt.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasConstDeclaration := raw.namespaces.any fun ns => ns.structs.any fun declaration =>
    declaration.generics.size == 3 &&
      declaration.generics[0]!.kind == .typeArg &&
      declaration.generics[1]!.kind == .const &&
      declaration.generics[2]!.kind == .const &&
      (match declaration.generics[1]!.type, declaration.generics[2]!.type with
       | some lengthType, some enabledType =>
           raw.tables.types[lengthType.typeId.index]? == some (.integer .pointer false) &&
             raw.tables.types[enabledType.typeId.index]? == some .bool
       | _, _ => false)
  let hasConstNominal := raw.tables.types.any fun
    | .nominal _ #[.typeArg _, .const (.integer 3), .const (.bool true)] => true
    | _ => false
  unless hasConstDeclaration && hasConstNominal do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its typed const binder or argument"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterGenericConstAdtBaseline

def checkRustExporterGenericEnumBaseline : IO Unit := do
  let file := "generic_enum.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasGenericEnum := ns.structs.any fun declaration =>
    declaration.generics.size == 1 && declaration.generics[0]!.kind == .typeArg &&
      declaration.variants.size == 2
  let hasConcreteNominal := raw.tables.types.any fun
    | .nominal _ #[.typeArg argument] =>
        raw.tables.types[argument.typeId.index]? == some (.integer (.bits 32) false)
    | _ => false
  let hasDiscriminant := ns.expressions.any fun expression =>
    expression.kind matches .operation (.data (.discriminant _)) _ #[_] _
  unless hasGenericEnum && hasConcreteNominal && hasDiscriminant do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its generic enum or discriminant"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  unless checked.namespaces[0]?.any fun checkedNs =>
      checkedNs.expressions.any fun expression => expression.kind matches .match_ _ _ do
    throw <| IO.userError s!"Rust exporter baseline {file} did not structurize its generic enum match"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  let some maybeName := SemanticOperations.findStructHandle? checked "Maybe"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its enum declaration"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  for (variant, fields, expected) in #[("Some", #[.integer 7], 7), ("None", #[], 9)] do
    let input : RuntimeValue := .nominal maybeName (some variant) fields
    match Interpreter.run executable 40 handle #[input, .integer 9] with
    | .ok (_, { value := .returned #[.integer actual], .. }) =>
        unless actual == expected do
          throw <| IO.userError s!"Rust exporter baseline {file} returned {actual}, expected {expected}"
    | result => throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterGenericEnumBaseline

def checkRustExporterMatchGuardBaseline : IO Unit := do
  let file := "match_guard.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasSwitchAndGuard := raw.namespaces.any fun ns => ns.functions.any fun function =>
    match function.body with
    | .cfg cfg =>
        cfg.blocks.any (·.terminator matches .switch ..) &&
          cfg.blocks.any (·.terminator matches .branch ..)
    | _ => false
  unless hasSwitchAndGuard do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its match or guard control"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let hasSwitchAndBranchWitness := checked.structurizationWitnesses.any fun witness =>
    witness.regions.any (·.kind == .switch) && witness.regions.any (·.kind == .branch)
  unless hasSwitchAndBranchWitness do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its checked control correspondence"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not target-width executable: {repr diagnostics}"
  let some maybeName := SemanticOperations.findStructHandle? checked "Maybe"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its enum declaration"
  let handle : FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
  let positive : RuntimeValue := .nominal maybeName (some "Some") #[.integer 7]
  let zero : RuntimeValue := .nominal maybeName (some "Some") #[.integer 0]
  let none : RuntimeValue := .nominal maybeName (some "None") #[]
  for (input, expected) in #[(positive, 7), (zero, 9), (none, 9)] do
    match Interpreter.run executable 48 handle #[input, .integer 9] with
    | .ok (_, { value := .returned #[.integer actual], .. }) =>
        unless actual == expected do
          throw <| IO.userError s!"Rust exporter baseline {file} returned {actual}, expected {expected}"
    | result =>
        throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterMatchGuardBaseline

def checkRustExporterShiftBaseline : IO Unit := do
  let file := "shift.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasShiftLeft := ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .shiftLeft) _ _ _
  let hasShiftRight := ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .shiftRight) _ _ _
  let hasOverflowAssertions := ns.functions.any fun function => match function.body with
    | .cfg graph => graph.blocks.countP (fun block =>
        block.terminator matches .assert _ true .overflow _ _) == 2
    | _ => false
  unless hasShiftLeft && hasShiftRight && hasOverflowAssertions do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its shifts or range assertions"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterShiftBaseline

def checkRustExporterBitwiseNotBaseline : IO Unit := do
  let file := "bitwise_not.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let bitwiseNots := raw.namespaces.foldl (init := 0) fun count ns =>
    count + ns.expressions.countP fun expression =>
      expression.kind matches .operation (.primitive .bitwiseNot) _ #[_] _
  unless bitwiseNots == 2 do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its signed or unsigned complement"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterBitwiseNotBaseline

def checkRustExporterNestedGenericAdtBaseline : IO Unit := do
  let file := "nested_generic_adt.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let concreteNominals := raw.tables.types.countP fun
    | .nominal _ #[.typeArg argument] =>
        raw.tables.types[argument.typeId.index]? == some (.integer (.bits 32) false)
    | _ => false
  unless 2 <= concreteNominals do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its nested concrete nominal type"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  let executable ← match prepareExecution #[semantics] checked with
    | .ok executable => pure executable
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"
  let some wrapperName := SemanticOperations.findStructHandle? checked "Wrapper"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its inner nominal declaration"
  let some outerName := SemanticOperations.findStructHandle? checked "Outer"
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its outer nominal declaration"
  let input : RuntimeValue := .nominal outerName none #[
    .nominal wrapperName none #[.integer 7]]
  match Interpreter.run executable 24 { namespaceId := ⟨0⟩, functionId := ⟨0⟩ } #[input] with
  | .ok (_, { value := .returned #[.integer 7], .. }) => pure ()
  | result => throw <| IO.userError s!"Rust exporter baseline {file} did not execute: {repr result}"

#guard_msgs in
#eval checkRustExporterNestedGenericAdtBaseline

def checkRustExporterOverflowingBaseline : IO Unit := do
  let file := "overflowing.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let hasAdd := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .overflowingAdd) _ #[_, _] _
  let hasSubtract := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .overflowingSubtract) _ #[_, _] _
  let hasMultiply := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .overflowingMultiply) _ #[_, _] _
  unless hasAdd && hasSubtract && hasMultiply do
    throw <| IO.userError s!"Rust exporter baseline {file} lost an overflowing operation"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterOverflowingBaseline

def checkRustExporterBooleanOrderingBaseline : IO Unit := do
  let file := "boolean_ordering.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  let some ns := raw.namespaces[0]?
    | throw <| IO.userError s!"Rust exporter baseline {file} lost its namespace"
  let hasLess := ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .less) _ #[_, _] _
  let hasLessEqual := ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .lessEqual) _ #[_, _] _
  let hasGreater := ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .greater) _ #[_, _] _
  let hasGreaterEqual := ns.expressions.any fun expression =>
    expression.kind matches .operation (.primitive .greaterEqual) _ #[_, _] _
  unless hasLess && hasLessEqual && hasGreater && hasGreaterEqual do
    throw <| IO.userError s!"Rust exporter baseline {file} lost a Boolean ordering operation"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterBooleanOrderingBaseline

def checkRustExporterCharacterBaseline : IO Unit := do
  let file := "character.exp.json"
  let path : System.FilePath := "rust-exporter/tests/raw-unit" / file
  let text ← readBaselineJson path
  let raw ← match Import.decodeJson text with
    | .ok raw => pure raw
    | .error message =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not decode: {message}"
  unless Import.encodeJson raw == text do
    throw <| IO.userError s!"Rust exporter baseline {file} is not canonical RawUnit JSON v1"
  unless raw.tables.types.contains .character do
    throw <| IO.userError s!"Rust exporter baseline {file} lost its character type"
  let hasCrab := raw.namespaces.any fun ns => ns.expressions.any fun expression =>
    expression.kind matches .value (.character 0x1f980) _
  let hasOrdering := raw.namespaces.any fun ns =>
    (ns.expressions.any fun expression =>
      expression.kind matches .operation (.primitive .less) _ #[_, _] _) &&
    (ns.expressions.any fun expression =>
      expression.kind matches .operation (.primitive .lessEqual) _ #[_, _] _) &&
    (ns.expressions.any fun expression =>
      expression.kind matches .operation (.primitive .greater) _ #[_, _] _) &&
    (ns.expressions.any fun expression =>
      expression.kind matches .operation (.primitive .greaterEqual) _ #[_, _] _)
  let hasSwitch := raw.namespaces.any fun ns => ns.functions.any fun function =>
    match function.body with
    | .cfg graph => graph.blocks.any fun block => match block.terminator with
        | .switch _ cases _ => cases.map (·.1) == #[.character 0x61, .character 0x1f980]
        | _ => false
    | _ => false
  let castCount := raw.namespaces.foldl (init := 0) fun count ns =>
    count + ns.expressions.countP (fun expression =>
      expression.kind matches .operation (.primitive .cast) _ #[_] _)
  unless hasCrab && hasOrdering && hasSwitch && 2 <= castCount do
    throw <| IO.userError s!"Rust exporter baseline {file} lost character literals, ordering, switch cases, or casts"
  let checked ← match validate raw with
    | .ok checked => pure checked
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} does not validate: {repr diagnostics}"
  match prepareExecution #[semantics] checked with
    | .ok _ => pure ()
    | .error diagnostics =>
        throw <| IO.userError s!"Rust exporter baseline {file} is not executable: {repr diagnostics}"

#guard_msgs in
#eval checkRustExporterCharacterBaseline

end LeanerIR.Rust.Tests
