-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.SourceVerify

namespace VerexIR.Rust.Tests

open VerexIR.Rust.Driver

def checkImportDriver : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let request : ImportRequest := {
      source := "rust-exporter/tests/raw-unit/basic.rs"
      output := directory / "basic.raw.json"
    }
    let first ← importRustFile request
    let second ← importRustFile request
    unless !first.cacheHit && second.cacheHit &&
        first.receipt == second.receipt && first.receipt.mode == .file &&
        !first.receipt.cacheKey.isEmpty && !first.receipt.artifactHash.isEmpty &&
        first.unit.indexes.functionCounts == #[1] &&
        second.unit.indexes.functionCounts == #[1] do
      throw <| IO.userError "Rust import driver did not export, validate, and reuse its fixture"
    unless (← request.output.pathExists) &&
        (← (request.output.addExtension "key").pathExists) do
      throw <| IO.userError "Rust import driver did not persist its artifact and cache receipt"
    IO.FS.writeFile request.output
      (← IO.FS.readFile "rust-exporter/tests/raw-unit/control.exp.json")
    let repaired ← importRustFile request
    unless !repaired.cacheHit && repaired.receipt == first.receipt &&
        repaired.unit.indexes.functionCounts == #[1] do
      throw <| IO.userError
        "Rust import driver accepted a valid artifact that disagreed with its receipt hash"
    let prepared ← match second.prepareExecution with
      | .ok prepared => pure prepared
      | .error diagnostics =>
          throw <| IO.userError s!"imported Rust fixture did not prepare: {repr diagnostics}"
    let handle : VerexIR.FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
    match VerexIR.Interpreter.run prepared.executable 16 handle #[] with
    | .ok (_, outcome) => unless outcome.value == .returned #[.bool true] do
        throw <| IO.userError "prepared Rust import returned the wrong semantic value"
    | .error error =>
        throw <| IO.userError s!"prepared Rust import did not execute: {repr error}"
    unless prepared.cacheHit && prepared.receipt == second.receipt &&
        prepared.validated.indexes.functionCounts == #[1] &&
        prepared.executable.initializationCertificates.size == 1 do
      throw <| IO.userError "execution preparation did not retain import identity or cache status"
    let verification ← match second.prepareVerification with
      | .ok verification => pure verification
      | .error diagnostics =>
          throw <| IO.userError s!"imported Rust fixture did not prepare for verification: {repr diagnostics}"
    unless verification.cacheHit && verification.receipt == second.receipt &&
        verification.validated.indexes.functionCounts == #[1] &&
        verification.validated.tables.locations == prepared.validated.tables.locations &&
        prepared.executable.initializationCertificates.size == 1 &&
        verification.verifiable.initializationCertificates ==
          prepared.executable.initializationCertificates do
      throw <| IO.userError
        "verification preparation did not retain import identity or initialization evidence"

def checkCargoImportDriver : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let request : CargoImportRequest := {
      manifest := "Tests/CargoFixture/Cargo.toml"
      package := "verex-rust-cargo-fixture"
      output := directory / "cargo.raw.json"
      features := #["extra"]
      noDefaultFeatures := true
    }
    let first ← importCargoCrate request
    let second ← importCargoCrate request
    unless !first.cacheHit && second.cacheHit &&
        first.receipt == second.receipt && first.receipt.mode == .cargo &&
        !first.receipt.cacheKey.isEmpty && !first.receipt.artifactHash.isEmpty &&
        first.unit.indexes.functionCounts == #[4] &&
        second.unit.indexes.functionCounts == #[4] do
      throw <| IO.userError
        "Cargo import driver did not export only the root library and reuse its fixture"
    let prepared ← match second.prepareExecution with
      | .ok prepared => pure prepared
      | .error diagnostics =>
          throw <| IO.userError s!"Cargo Rust fixture did not prepare: {repr diagnostics}"
    let handle : VerexIR.FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
    match VerexIR.Interpreter.run prepared.executable 64 handle #[] with
    | .ok (_, outcome) => unless outcome.value == .returned #[.bool true] do
        throw <| IO.userError "prepared Cargo import returned the wrong semantic value"
    | .error error =>
        throw <| IO.userError s!"prepared Cargo import did not execute: {repr error}"
    let verification ← match second.prepareVerification with
      | .ok verification => pure verification
      | .error diagnostics =>
          throw <| IO.userError s!"Cargo Rust fixture did not prepare for verification: {repr diagnostics}"
    unless verification.cacheHit && prepared.receipt == second.receipt &&
        verification.receipt == second.receipt &&
        verification.validated.indexes.functionCounts == #[4] &&
        verification.validated.tables.locations == prepared.validated.tables.locations &&
        prepared.executable.initializationCertificates.size == 4 &&
        verification.verifiable.initializationCertificates ==
          prepared.executable.initializationCertificates do
      throw <| IO.userError
        "Cargo verification preparation did not retain import identity or initialization evidence"
    -- The root and the module file `bounds.rs` have specification files; the
    -- inline module `bounds::compare` has none of its own.
    let some root := second.sourceRoot
      | throw <| IO.userError "Cargo import did not name its workspace root"
    let specs ← VerexIR.Rust.SourceVerify.crateSpecs second.unit root
    let expected : Array (Array String × System.FilePath) := #[
      (#[], root / "src" / "lib.spec.lean"), (#["bounds"], root / "src" / "bounds.spec.lean")]
    unless specs.map (fun companion => (companion.scope, companion.path)) == expected &&
        specs.all (·.text.isSome) do
      throw <| IO.userError s!"Cargo specification files were not found by module: \
        {specs.map fun companion => (companion.scope, companion.path)}"

/-- Both receiver adapters are covered by synthesized, proved specifications;
these items do not pretend to come from a companion file on disk. -/
def checkAdapterSpecs (environment : Lean.Environment) : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let source : System.FilePath := "rust-exporter/tests/raw-unit/tuple_callbacks.rs"
    let imported ← importRustFile { source, output := directory / "adapters.raw.json" }
    let companions ← VerexIR.Rust.SourceVerify.specsFor imported.unit source none
    let #[generated] := companions.filter (·.generated)
      | throw <| IO.userError "expected one generated adapter companion"
    unless generated.path == source do
      throw <| IO.userError "adapter contracts must retain generated-file diagnostics"
    let text := generated.text.getD ""
    unless (text.splitOn "ensures ensures_of<function[").length == 3 &&
        (text.splitOn "== old(").length == 3 do
      throw <| IO.userError "both closure adapters need behavior and receiver-preservation contracts"
    let reimported ← match VerexSL.Print.reimportUnit environment imported.unit with
      | .ok unit => pure unit
      | .error error => throw <| IO.userError s!"adapter round-trip failed: {error}"
    let roundTrip ← VerexIR.Rust.SourceVerify.specsFor reimported source none
    let contracts (text : String) :=
      ((text.splitOn "\n\n").toArray.map (·.trimAscii.toString)).qsort (· < ·)
    unless (roundTrip.filter (·.generated)).map (fun c => contracts (c.text.getD "")) ==
        #[contracts text] do
      throw <| IO.userError "adapter target changed when the rendering was reimported"
    let some ns := imported.unit.namespaces[0]?
      | throw <| IO.userError "adapter fixture has no namespace"
    match VerexIR.Rust.SourceVerify.moduleFiles imported.unit ns with
    | .ok files => unless files.size == 1 && files[0]!.1.isEmpty do
        throw <| IO.userError "generated closure adapters changed module discovery"
    | .error error => throw <| IO.userError error

/-- An error in synthesized text names the persisted rendering, with its real
line numbers, rather than the nominal companion path. -/
def checkGeneratedDiagnostics (environment : Lean.Environment) : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let imported ← importRustFile {
      source := "rust-exporter/tests/raw-unit/basic.rs"
      output := directory / "basic.raw.json" }
    let some ns := imported.unit.namespaces[0]?
      | throw <| IO.userError "diagnostic fixture has no namespace"
    let output := directory / "generated.lean"
    let reports ← VerexSL.SourceVerify.run environment {
      unit := imported.unit, output
      companions := #[{
        path := directory / "not-a-real-companion.spec.lean"
        namespaceId := ns.unitIdentity, generated := true
        text := some "spec answer where\n  ensures false\n" }] }
    unless reports.any (fun report => report.file == output.toString &&
        report.text.contains "not established") do
      throw <| IO.userError s!"a generated specification error did not name its rendering: {reports.map (·.render)}"
    unless ← output.pathExists do
      throw <| IO.userError "generated diagnostic target does not exist"

/-- The import rewrites a type the library models to its VIR type, and the
verification keeps the library's contracts of the functions a unit declares. -/
def checkLibrary : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let types ← importRustFile {
      source := "rust-exporter/tests/raw-unit/dependency_types.rs"
      output := directory / "types.raw.json" }
    let paths := types.unit.tables.namespaces.map (·.segments)
    -- `Global` is reached only as an argument of `Vec`, so nothing of `alloc` is left.
    unless paths == #[#["dependency_types"], #["core"]] do
      throw <| IO.userError s!"a unit keeps what only a modeled type reached: {paths}"
    unless types.unit.tables.names.all (·.name != "Vec") do
      throw <| IO.userError "a modeled type's name outlives its declaration"
    let vectors := types.unit.tables.types.filter (· matches .vector _ none)
    unless vectors.size == 1 do
      throw <| IO.userError s!"the type table repeats a modeled type: {repr vectors}"
    let calls ← importRustFile {
      source := "rust-exporter/tests/raw-unit/dependency_calls.rs"
      output := directory / "calls.raw.json" }
    let companions ← VerexIR.Rust.SourceVerify.libraryCompanions calls.unit
    let spliced := (companions.filterMap (·.text)).toList
    let holds (text : String) := spliced.any fun item => (item.splitOn text).length > 1
    unless holds "spec len where" && holds "spec checked_add where" &&
        !holds "spec pop where" && !holds "spec checked_sub where" do
      throw <| IO.userError s!"the library's reached items are wrong:\n{"\n".intercalate spliced}"
    let text := "mod m where\n  spec f where\n    ensures true\n\n  spec g where\n    ensures true\n"
    let file ← match Library.File.read "test.spec.lean" "core" text with
      | .ok file => pure file
      | .error message => throw <| IO.userError message
    unless file.reachedItems (fun contract => contract.scope == #["m"] && contract.target == "g") ==
        some "mod m where\n\n\n\n  spec g where\n    ensures true\n" do
      throw <| IO.userError "a library file's unreached item keeps its lines"
    let functions ← match Library.File.read "test.spec.lean" "core"
        ("mod m where\n  spec fun h(x : Int) : Int := x\n\n" ++
          "  spec fun k(x : Int) : Int := h(x)\n\n  spec f where\n    ensures k(1) == 1\n") with
      | .ok file => pure file
      | .error message => throw <| IO.userError message
    let kept (keep : Library.ContractItem → Bool) (clauses : Array String) :=
      (functions.keptSpecFunctions keep clauses).map (·.name)
    unless kept (fun _ => true) #[] == #["k", "h"] && kept (fun _ => false) #[] == #[] &&
        kept (fun _ => false) #["ensures m::h(x) == 1"] == #["h"] &&
        kept (fun _ => false) #["ensures h(x) == 1", "ensures mh(x) == 1"] == #[] do
      throw <| IO.userError "a library file keeps the wrong specification functions"
    unless (file.reachedItems fun _ => false) == none do
      throw <| IO.userError "a library file reaching nothing is spliced"
    -- A model with fields holds them, and is opaque no more.
    let model := "mod m where\n  @[model] struct It {a : lifetime} {T} where\n    \
      slice : &[a] Vector<T>\n    pos : usize\n"
    let held : Library.Model := .fields #[.lifetime, .typeArg]
      #[("slice", .shared 0 (.vector (.parameter 1))), ("pos", .closed (.integer .pointer false))]
    let models := (Library.File.read "test.spec.lean" "core" model).toOption.map (·.models)
    unless models == some #[{ crate := "core", scope := #["m"], name := "It", generics := 2,
                              model := held }] do
      throw <| IO.userError s!"a library model's fields are misread: {repr models}"
    -- A model naming a binder is that argument.
    let identity := (Library.File.read "test.spec.lean" "alloc"
      "mod m where\n  @[model(T)] opaque struct It {T} {A} where\n").toOption.map (·.models)
    unless identity == some #[{ crate := "alloc", scope := #["m"], name := "It", generics := 2,
                                model := .identity 0 }] do
      throw <| IO.userError s!"an identity model is misread: {repr identity}"
    unless (Library.File.read "test.spec.lean" "core"
        "@[model] struct It {T} where\n  pos : Frob\n") matches .error _ do
      throw <| IO.userError "a library model's unknown field type is accepted"
    IO.FS.writeFile (directory / "iterate.rs")
      "pub fn f(v: &[u32]) -> usize { let mut n = 0; for _x in v.iter() { n += 1; } n }\n"
    let iterate ← importRustFile {
      source := directory / "iterate.rs"
      output := directory / "iterate.raw.json" }
    let iter? := iterate.unit.namespaces.findSome? fun ns => ns.structs.find? fun declaration =>
      (iterate.unit.tables.names[declaration.name.index]?).any (·.name == "Iter")
    unless iter?.any fun declaration =>
        declaration.fields.size == 3 && declaration.properties.isEmpty do
      throw <| IO.userError "a type modeled with fields does not hold them"
    unless (Library.File.read "test.spec.lean" "core" "fun f() -> Unit\n") matches .error _ do
      throw <| IO.userError "a library file's unknown item is accepted"
    -- A contract whose target no function has would be dropped from every unit, and a
    -- call of the function rejected, so each is found on a unit calling them all.
    let targets ← importRustFile {
      source := "Tests/LibraryTargets/targets.rs"
      output := directory / "targets.raw.json" }
    let mut unresolved : Array String := #[]
    for file in ← Library.readFiles (← libraryRoot) do
      for contract in file.contracts do
        let declared := targets.unit.namespaces.any fun ns =>
          (targets.unit.tables.namespaces[ns.identity.index]?.map (·.segments)) ==
              some #[file.crate] &&
            ns.functions.any fun function =>
              (targets.unit.tables.names[function.name.index]?).any fun name =>
                name.scope == contract.scope && name.name == contract.target
        unless declared do
          unresolved := unresolved.push s!"{file.crate}: {contract.scope} {contract.target}"
    unless unresolved.isEmpty do
      throw <| IO.userError s!"the library specifies functions no unit declares: {unresolved}"

#guard_msgs in
#eval checkImportDriver *> checkCargoImportDriver *> checkLibrary

end VerexIR.Rust.Tests

open Lean Elab Command in
run_cmd do
  let environment ← getEnv
  liftIO <| VerexIR.Rust.Tests.checkAdapterSpecs environment
  liftIO <| VerexIR.Rust.Tests.checkGeneratedDiagnostics environment
