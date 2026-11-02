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

/-- Import what verifying a Move source elaborates: `VerexSL` and the header
of its proof files. -/
def importEnvironment : IO Lean.Environment :=
  importLibraries #[`VerexMove.Proofs]

/-- The namespace of the unit that a module was encoded to. -/
private def namespaceOf? (unit : VerexIR.Validation.ValidatedUnit) (module : Module) :
    Option VerexIR.NamespaceId :=
  unit.namespaces.findSome? fun ns => do
    let segments ← ns.tables.namespaces[ns.identity.index]? |>.map (·.segments)
    guard (segments.size == 2 && segments[1]! == module.name &&
      (segments[0]! == module.address || module.addressAlias == some segments[0]!))
    return ns.identity

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
    if !misplaced.isEmpty then return .error (Sum.inr misplaced)
    match VerexMove.Frontend.VIR.Backend.fromXast package with
    | .ok unit => pure (Except.ok (unit, ← companions package unit, ← omissions package))
    | .error message => pure (Except.error (Sum.inl message))
  match prepared with
  | .error (Sum.inr reports) => return reports
  | .error (Sum.inl message) =>
      return #[{
        file := source.toString, line := 1, column := 1
        severity := .error, text := message }]
  | .ok (unit, companions, omitted) =>
      let reports ← run environment
        { unit, companions, output, renderOnly, heartbeats, ignoreProofs, jobs }
      return (if renderOnly then #[] else omitted) ++ reports

end VerexMove.SourceVerify
