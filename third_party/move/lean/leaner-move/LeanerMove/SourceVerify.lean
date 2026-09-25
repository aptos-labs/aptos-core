-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend
import LeanerLang.SourceVerify

/-!
# Verifying a Move source

A Move file or package is exported through compiler-v2, its modules rendered
as LeanerLang in dependency order and verified, and every message reported in the Move file's
coordinates. Its specifications are the Move `spec` blocks it declares; the
proofs its functions need beyond the automatic verification are LeanerLang
items — `verify f by …` and the lemmas they use — in the proof file beside
the Move file, `foo.proof.lean` for `foo.move`.
-/

namespace LeanerMove.SourceVerify

open LeanerLang.SourceVerify
open LeanerMove.Frontend.Xast (Module)
open LeanerMove.Frontend.Effects (Package)

/-- The proof file of a Move source file. -/
def proofFile (source : System.FilePath) : System.FilePath :=
  source.withExtension "proof.lean"

/-- The namespace of the unit that a module was encoded to. -/
private def namespaceOf? (unit : LeanerIR.Validation.ValidatedUnit) (module : Module) :
    Option LeanerIR.NamespaceId :=
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
def companions (package : Package) (unit : LeanerIR.Validation.ValidatedUnit) :
    IO (Array Companion) := do
  let mut found := #[]
  for module in package.modules do
    let some path := module.proofFile | continue
    let path : System.FilePath := path
    let some namespaceId := namespaceOf? unit module
      | throw <| IO.userError s!"{path}: the module `{module.name}` has no namespace"
    let text ← if ← path.pathExists then some <$> IO.FS.readFile path else pure none
    found := found.push { path, namespaceId, text }
  return found

/-- Verify a Move file, or the modules of a Move package directory, writing
the LeanerLang rendering to `output`. With `exported`, the package is read
from that existing `move exchange --format ast` export instead of being
exported here, as the Move CLI's `prove --lean` hands it over; `filter`
narrows the verified modules to those whose source file name contains it;
`heartbeats` is the budget of a function without `pragma heartbeats`. -/
def verifySource (environment : Lean.Environment) (source output : System.FilePath)
    (exported : Option System.FilePath := none) (filter : Option String := none)
    (renderOnly : Bool := false) (heartbeats : Option Nat := none) :
    IO (Array Report) := do
  let (unit, companions) ← LeanerLang.Perf.withPhase .frontend do
    let package ← match exported with
      | some exported =>
          LeanerMove.Frontend.Cli.readExportDir exported
            (if ← source.isDir then some source else none) filter
      | none =>
          if ← source.isDir then LeanerMove.Frontend.Cli.exportPackage source
          else LeanerMove.Frontend.Cli.exportMoveFiles [source]
    let package ← withProofFiles package
    match LeanerMove.Frontend.LIR.Backend.fromXast package with
    | .ok unit => pure (unit, ← companions package unit)
    | .error message => throw <| IO.userError s!"{source}: {message}"
  run environment { unit, companions, output, renderOnly, heartbeats }

end LeanerMove.SourceVerify
