-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Frontend
import VerexMove.Proofs
import VerexSL.SourceVerify

/-!
# Verifying a Move source

A Move file or package is exported through compiler-v2, its modules rendered
as VerexSL in dependency order and verified, and every message reported in the Move file's
coordinates. Its specifications are the Move `spec` blocks it declares; the
proofs its functions need beyond the automatic verification are in the proof
file beside the Move file, `foo.proof.lean` for `foo.move`
(`VerexMove.Proofs`): `verify f by …` items and the lemmas they use.
-/

namespace VerexMove.SourceVerify

open VerexSL.SourceVerify
open VerexMove.Frontend.Xast (Module)
open VerexMove.Frontend.Effects (Package)

open VerexMove.Proofs (proofFile fileRendering packageRendering)

/-- Where verifying a Move file or a package directory writes its rendering:
where a proof file's header finds it. -/
def renderingOf (source : System.FilePath) : IO System.FilePath := do
  if ← source.isDir then return packageRendering source else fileRendering source

/-- The libraries a namespace is compiled over: those verifying its importers
loads. -/
def compilationLibraries : Array Lean.Name := #[`VerexMove.Proofs]

/-- Import what verifying a Move source elaborates: `VerexSL` and the header
of its proof files. -/
def importEnvironment : IO Lean.Environment :=
  importLibraries compilationLibraries

/-- The namespace of the unit that a module was encoded to. -/
private def namespaceOf? (unit : VerexIR.Validation.ValidatedUnit) (module : Module) :
    Option VerexIR.NamespaceId :=
  unit.namespaces.findSome? fun ns => do
    let segments ← ns.tables.namespaces[ns.unitIdentity.index]? |>.map (·.segments)
    guard (segments.size == 2 && segments[1]! == module.name &&
      (segments[0]! == module.address || module.addressAlias == some segments[0]!))
    return ns.unitIdentity

/-- The source file declaring a module; its specifications, and callees
read for inlining, may come from other files. -/
private def declaringSource? (module : Module) : Option String :=
  module.sources[module.loc.file]?

/-- The package with the proof file of every target module named on it. A
proof file accompanies a source file declaring one module, whose functions
its `verify` items name; a file declaring several has none. -/
def withProofFiles (package : Package) : IO Package := do
  let modules ← package.modules.mapM fun module => do
    unless module.isTarget do return module
    let some source := declaringSource? module | return module
    let sharing := package.modules.filter fun other =>
      other.isTarget && declaringSource? other == some source
    let path := proofFile source
    if sharing.length > 1 then
      if ← path.pathExists then
        throw <| IO.userError s!"{path}: a proof file accompanies a source file declaring one \
          module, but {source} declares {sharing.length}; move the modules to their own files"
      return module
    return { module with proofFile := some path.toString }
  return { package with modules }

/-- The proof files named on the package's modules, read when they exist. -/
def companions (package : Package) (unit : VerexIR.Validation.ValidatedUnit) :
    IO (Array Companion) := do
  let mut found := #[]
  for module in package.modules do
    let some path := module.proofFile | continue
    let path : System.FilePath := path
    let some namespaceId := namespaceOf? unit module
      | throw <| IO.userError s!"{path}: the module `{module.name}` has no namespace"
    let text ← if ← path.pathExists then some <$> IO.FS.readFile path else pure none
    found := found.push { path, namespaceId, text, role := .proofs }
  return found

/-- The declarations the export of a target module left out, as errors at
the module: what the export does not carry is not verified. -/
def omissions (package : Package) : IO (Array Report) := do
  let mut reports := #[]
  for module in package.modules do
    unless module.isTarget do continue
    let file := (module.sources[module.loc.file]?).getD "?"
    let position ← try
        pure <| some <| (Lean.FileMap.ofString (← IO.FS.readFile file)).toPosition
          ⟨module.loc.start⟩
      catch _ => pure none
    for skipped in module.omitted do
      reports := reports.push {
        file, line := position.map (·.line) |>.getD 1
        column := position.map (·.column + 1) |>.getD 1, severity := .error
        text := s!"unsupported Move declaration `{skipped.name}`: {skipped.reason}" }
  return reports

/-- The loop invariants of the target modules' functions that no loop claims
(`Frontend.LoopInvariants`), each an error at its location, in source order. -/
def misplacedLoopInvariants (package : Package) : IO (Array Report) := do
  let mut maps : Std.HashMap String (Option Lean.FileMap) := {}
  let mut reports := #[]
  for module in package.modules do
    unless module.isTarget do continue
    for function in module.functions do
      let some body := function.body | continue
      for loc in VerexMove.Frontend.LoopInvariants.misplaced body do
        let file := (module.sources[loc.file]?).getD "?"
        let map ← match maps[file]? with
          | some map => pure map
          | none => do
              let map ← try pure (some (Lean.FileMap.ofString (← IO.FS.readFile file)))
                catch _ => pure none
              maps := maps.insert file map
              pure map
        let position := map.map (·.toPosition ⟨loc.start⟩)
        reports := reports.push {
          file, line := position.map (·.line) |>.getD 1
          column := position.map (·.column + 1) |>.getD 1, severity := .error
          text := VerexMove.Frontend.LoopInvariants.message }
  return reports.qsort fun a b => a.file < b.file || a.file == b.file &&
    (a.line < b.line || a.line == b.line && a.column < b.column)

/-- What verifying a source needs of its export: the unit, its target's
namespace, the companions, and the reports on what the export omits. -/
private structure PreparedSource where
  unit : VerexIR.Validation.ValidatedUnit
  target : Option VerexIR.NamespaceId
  companions : Array Companion
  omitted : Array Report

/-- Export a source and encode it (`verifySource`): the reports where that ends
the run. -/
private def prepareSource (source : System.FilePath) (exported : Option System.FilePath)
    (filter : Option String) (modules : Array String) (dev : Bool) :
    IO (Except (Array Report) PreparedSource) := do
  let prepared ← VerexSL.Perf.withPhase .frontend do
    let package ← match exported with
      | some exported =>
          VerexMove.Frontend.Cli.readExportDir exported source filter
      | none =>
          if ← source.isDir then
            if modules.isEmpty then
              VerexMove.Frontend.Cli.packageTargets
                (← VerexMove.Frontend.Cli.exportPackage source (includeDeps := true) dev)
                source filter
            else VerexMove.Frontend.Cli.exportModules source modules dev
          else VerexMove.Frontend.Cli.exportMoveFiles [source]
    let package ← withProofFiles package
    -- A misplaced loop invariant rejects the run, as in the Move Prover.
    let misplaced ← misplacedLoopInvariants package
    if !misplaced.isEmpty then return .error misplaced
    match VerexMove.Frontend.VIR.Backend.fromXast package with
    | .ok unit =>
        let target := package.modules.filter (·.isTarget) |>.head?
        pure (Except.ok { unit, target := target.bind (namespaceOf? unit)
                          companions := ← companions package unit
                          omitted := ← omissions package })
    | .error message =>
        pure (Except.error #[{
          file := source.toString, line := 1, column := 1
          severity := .error, text := message }])
  return prepared

/-- Verify a Move file, or the modules of a Move package directory, writing
the VerexSL rendering to `output`. With `exported`, the source and its
dependencies are read from that existing `move exchange --format ast` export
instead of being exported here, as the Move CLI's `prove --lean` and the Move
Prover's `--lean` hand them over; `filter`
narrows the verified modules to those whose source file name contains it;
`heartbeats` is the budget of a function without `pragma heartbeats`;
`ignoreProofs` verifies every function automatically. With
`modules`, a package directory is exported with only the named modules and
what verifying them reads, and the named modules are verified. `dev` exports
a package directory in dev mode. -/
def verifySource (environment : Lean.Environment) (source output : System.FilePath)
    (exported : Option System.FilePath := none) (filter : Option String := none)
    (renderOnly : Bool := false) (heartbeats : Option Nat := none)
    (modules : Array String := #[]) (dev : Bool := false) (ignoreProofs : Bool := false)
    (jobs : Option Nat := none) :
    IO (Array Report) := do
  match ← prepareSource source exported filter modules dev with
  | .error reports => return reports
  | .ok { unit, companions, omitted, .. } =>
      let reports ← run environment
        { unit, companions, output, renderOnly, heartbeats, ignoreProofs, jobs }
      return (if renderOnly then #[] else omitted) ++ reports

/-- Verify one module of a package directory over the namespaces compiled under
`root`, which its rendering imports instead of elaborating them. -/
def verifyOverNamespaces (source output root : System.FilePath) (module : String)
    (renderOnly : Bool := false) (heartbeats : Option Nat := none) (dev : Bool := false)
    (ignoreProofs : Bool := false) (jobs : Option Nat := none) :
    IO (Array Report) := do
  match ← prepareSource source none none #[module] dev with
  | .error reports => return reports
  | .ok { unit, target, companions, omitted } =>
      let some target := target
        | throw <| IO.userError s!"`{module}` has no namespace in the export of {source}"
      let compiled : CompiledNamespaces := { root, target }
      let request : Request :=
        { unit, companions, output, renderOnly, heartbeats, ignoreProofs, jobs
          compiled := some compiled }
      -- Rendering says what it imports, which need not be compiled yet.
      let environment ← VerexSL.Perf.withPhase .load <|
        if renderOnly then importEnvironment
        else importCompiled compilationLibraries request compiled
      let reports ← run environment request
      return (if renderOnly then #[] else omitted) ++ reports

/-- Render every module of a package, each to a file of its own under `root`,
as a dependency of what is verified over it. -/
def renderPackageNamespaces (environment : Lean.Environment) (source root : System.FilePath)
    (dev : Bool := false) :
    IO (Except String (Array RenderedNamespace)) := do
  let split ← VerexMove.Frontend.Cli.exportPackage source (includeDeps := true) dev
  let package : Package :=
    { modules := (split.modules ++ split.dependencies).map ({ · with isTarget := false }) }
  match VerexMove.Frontend.VIR.Backend.fromXast package with
  | .error message => return .error message
  | .ok unit =>
      return (← renderNamespaces environment unit #[] root).mapError (·.message)

end VerexMove.SourceVerify
