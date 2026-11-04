-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.Driver
import VerexSL.SourceVerify

/-!
# Verifying a Rust source

A Rust file carries no specifications of its own: they are VerexSL items —
`spec` blocks, specification functions, invariants — in a file beside it,
`<name>.spec.lean` for `<name>.rs` unless another is named. The file is
exported through rustc, its crate rendered as VerexSL with those items
appended and verified, and every message reported in the coordinates of the
Rust file or of the specification file.

A Cargo package's library crate is one namespace whose modules are scopes.
Each file of the crate's modules has its specification file beside it, whose
items reopen the module's scope; an inline module's specifications nest in
those of the file it is declared in.
-/

namespace VerexIR.Rust.SourceVerify

open VerexSL.SourceVerify

/-- The specification file of a Rust source: the one named, or
`<name>.spec.lean` beside `<name>.rs`, read when it exists. Its items are
spliced into the crate's namespace, the unit's last. -/
def specsFor (unit : VerexIR.Validation.ValidatedUnit) (source : System.FilePath)
    (named : Option System.FilePath) : IO (Array Companion) := do
  let path := named.getD (source.withExtension "spec.lean")
  let some ns := unit.namespaces.back? | return #[]
  let text ← if ← path.pathExists then some <$> IO.FS.readFile path else pure none
  return #[{ path, namespaceId := ns.unitIdentity, text }]

/-- The files of a crate's modules: the root's and each module's declared in a
file of its own, with the module's scope. A scope's file holds its
declarations and those of its submodules; a module in its parent's file is
inline and has none of its own. -/
def moduleFiles (unit : VerexIR.Validation.ValidatedUnit)
    (ns : VerexIR.Validation.ValidatedNamespace) :
    Except String (Array (Array String × String)) := do
  let fileOf (loc : VerexIR.LocId) : Option String := do
    let range ← (← unit.tables.locations[loc.index]?).primary
    return (← unit.tables.files[range.file.index]?).name
  let scopeOf (name : VerexIR.NameId) : Array String :=
    (unit.tables.names[name.index]?.map (·.scope)).getD #[]
  let declarations := ns.functions.map (fun declaration => (scopeOf declaration.name, declaration.loc)) ++
    ns.structs.map (fun declaration => (scopeOf declaration.name, declaration.loc)) ++
    ns.constants.map (fun declaration => (scopeOf declaration.name, declaration.loc)) ++
    ns.scopes.map fun scope => (scope.path.pop, scope.loc)
  let text (scope : Array String) := if scope.isEmpty then "the crate root"
    else s!"module `{"::".intercalate scope.toList}`"
  -- Every scope's file, each after its parent's.
  let mut scopeFiles : Array (Array String × String) := #[]
  for scope in #[#[]] ++ ns.scopes.map (·.path) do
    let found := declarations.foldl (init := #[]) fun found (owner, loc) =>
      match fileOf loc with
      | some file => if owner == scope && !found.contains file then found.push file else found
      | none => found
    match found with
    | #[file] => scopeFiles := scopeFiles.push (scope, file)
    | #[] => throw s!"{text scope} declares nothing with a source location"
    | _ => throw s!"{text scope} declares items in several files: {", ".intercalate found.toList}"
  return scopeFiles.filter fun (scope, file) =>
    scope.isEmpty || (scopeFiles.find? (·.1 == scope.pop)).map (·.2) != some file

/-- The specification files of a crate: `<name>.spec.lean` beside each file of
its modules, read when it exists, its items in the module's scope. -/
def crateSpecs (unit : VerexIR.Validation.ValidatedUnit) (root : System.FilePath) :
    IO (Array Companion) := do
  let #[ns] := unit.namespaces
    | throw <| IO.userError "a Rust crate is exported as exactly one namespace"
  let files ← match moduleFiles unit ns with
    | .ok files => pure files
    | .error message => throw <| IO.userError message
  files.mapM fun (scope, file) => do
    let path := (root / file).withExtension "spec.lean"
    let text ← if ← path.pathExists then some <$> IO.FS.readFile path else pure none
    return { path, namespaceId := ns.identity, scope, text }

/-- Verify a Rust file against its specification file, writing the
VerexSL rendering to `output`. -/
def verifyFile (environment : Lean.Environment) (source : System.FilePath)
    (specFile : Option System.FilePath) (output : System.FilePath)
    (rustcArgs : Array String := #[]) (ignoreProofs : Bool := false) : IO (Array Report) := do
  let (unit, companions) ← VerexSL.Perf.withPhase .frontend do
    let unit ← IO.FS.withTempDir fun directory =>
      return (← Driver.importRustFile {
        source, output := directory / "unit.raw.json", rustcArgs }).unit
    pure (unit, ← specsFor unit source specFile)
  run environment { unit, companions, output, ignoreProofs }

/-- Verify the library crate of a Cargo package against the specification
files of its modules, writing the VerexSL rendering to `output`. -/
def verifyCrate (environment : Lean.Environment) (request : Driver.CargoImportRequest)
    (output : System.FilePath) (ignoreProofs : Bool := false) : IO (Array Report) := do
  let (unit, sourceRoot, companions) ← VerexSL.Perf.withPhase .frontend do
    let result ← IO.FS.withTempDir fun directory =>
      Driver.importCargoCrate { request with output := directory / "unit.raw.json" }
    let some root := result.sourceRoot
      | throw <| IO.userError "a Cargo import did not name its workspace root"
    pure (result.unit, root, ← crateSpecs result.unit root)
  run environment { unit, companions, output, ignoreProofs, sourceRoot }

end VerexIR.Rust.SourceVerify
