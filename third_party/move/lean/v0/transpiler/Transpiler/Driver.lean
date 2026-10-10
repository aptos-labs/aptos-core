-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler.Xast
import Transpiler.Decode
import Transpiler.Names
import Transpiler.Effects
import Transpiler.Print
import Transpiler.Cli
import Transpiler.LIR.Backend
import Transpiler.LIR.Effects
import Transpiler.LIR.MoveNames
import Transpiler.LIR.Report

/-!
# Driver

The batch transpiler's checked entry point consumes `ValidatedUnit`. The Aptos
CLI/XAST route is retained as a compatibility wrapper which constructs and
validates LIR before entering that boundary. Module identity, package-wide
effect facts, and source omissions come from validated LIR. The transitional
printer view contributes only observations made while rendering.

The driver prints one `.lean` file per module (under the module's Lean path,
`AptosFramework/Coin.lean`) and writes a transpilation report
(`transpile-report.txt`) listing, per module, the Leaner extensions the output
relies on, the constructs dropped with their disposition, and the declarations
emitted commented out. Nothing is dropped silently.
-/

namespace Transpiler.Driver

open Transpiler.Xast Transpiler.Names Transpiler.Effects Transpiler.Print

/-- The outcome of transpiling one module. -/
structure Outcome where
  module : ModuleRef
  /-- The generated file, relative to the output directory. -/
  path : String
  report : Report
  /-- A fatal error (the module could not be printed at all). -/
  error : Option String := none
  /-- The module is not transpiled: Leaner provides it (`std::vector` is
  `Move.Vector`); the Lean name it maps onto. -/
  curated : Option String := none

/-- Relative path of a module's generated file. -/
def outputPath (m : ModuleRef) : String :=
  (leanModulePath m).replace "." "/" ++ ".lean"

private def transpilePrinterPackage (unit : LeanerIR.Validation.ValidatedUnit)
    (pkg : Package) (effects : Table)
    (emitVerify : Bool) (leanRoot : String) :
    Except String (List (Outcome × Option String)) := do
  unless unit.namespaces.size == pkg.modules.length do
    throw "validated LIR and transitional printer package have different module counts"
  (unit.namespaces.toList.zip pkg.modules).mapM fun (ns, m) => do
    let moduleRef ← Transpiler.LIR.MoveNames.moduleRef unit.tables ns.identity
    unless moduleRef == m.ref do
      throw "validated LIR and transitional printer package module identities disagree"
    let path := outputPath moduleRef
    if let some lean := curatedModule? moduleRef then
      pure ({ module := moduleRef, path, report := {}, curated := some lean }, none)
    else
      let initialReport ← Transpiler.LIR.Report.initial ns
      match printModule pkg m effects (emitVerify := emitVerify) (leanRoot := leanRoot)
          (initialReport := initialReport) with
      | .ok (text, report) => pure ({ module := moduleRef, path, report }, some text)
      | .error e =>
          pure ({ module := moduleRef, path, report := initialReport, error := some e }, none)

/-- Transpiles a checked LIR unit. `emitVerify` appends `verify f` to every
`spec f`; `leanRoot` is the Lean module prefix of the output directory, used by
generated `import`s. The `Except` accounts only for failure to construct the
transitional printer view; per-module printer failures remain in `Outcome`. -/
def transpileValidated (unit : LeanerIR.Validation.ValidatedUnit)
    (emitVerify : Bool := false) (leanRoot : String := "") :
    Except String (List (Outcome × Option String)) := do
  let effects ← Transpiler.LIR.Effects.computePrinterTable unit
  let pkg ← Transpiler.LIR.Backend.toPrinterPackage .move unit
  transpilePrinterPackage unit pkg effects emitVerify leanRoot

/-- Compatibility entry point for compiler-v2 XAST. New backend callers should
construct a `ValidatedUnit` and call `transpileValidated`. -/
def transpilePackage (pkg : Package) (emitVerify : Bool := false) (leanRoot : String := "") :
    List (Outcome × Option String) :=
  -- Intrinsic-owning modules are refused by the suspended E16 capability gate
  -- before semantic validation: the refusal is a frontend capability
  -- statement, not a judgment about the module's semantics.
  let (intrinsicModules, ordinaryModules) := pkg.modules.partition fun m =>
    m.structs.any (·.intrinsic.isSome)
  let suspended := intrinsicModules.filterMap fun m => do
    let owner ← m.structs.find? (·.intrinsic.isSome)
    let model := owner.intrinsic.map (·.name) |>.getD "map"
    let outcome : Outcome := {
      module := m.ref
      path := outputPath m.ref
      report := {}
      error := some s!"unsupported: intrinsic type `{owner.name}` uses model `{model}`; E16 is suspended until intrinsic validation is implemented on the unified LIR" }
    some (outcome, (none : Option String))
  let ordinary := match Transpiler.LIR.Backend.fromXast { pkg with modules := ordinaryModules } >>=
      fun unit => transpileValidated unit (emitVerify := emitVerify) (leanRoot := leanRoot) with
  | .error error => ordinaryModules.map fun m =>
      ({ module := m.ref, path := outputPath m.ref, report := {}, error := some error }, none)
  | .ok outcomes => outcomes
  suspended ++ ordinary

/-- The textual report. -/
def renderReport (outcomes : List Outcome) : String :=
  let lines := outcomes.flatMap fun o =>
    let name := s!"{o.module.addressAlias.getD o.module.address}::{o.module.name}"
    let status := match o.error, o.curated with
      | some e, _ => s!"ERROR {e}"
      | none, some lean => s!"curated ({lean})"
      | none, none => if o.report.unsupported.isEmpty then "clean" else s!"partial ({o.report.unsupported.length} declaration(s) not transpiled)"
    [s!"{name}: {status}" ++ (if o.curated.isSome then "" else s!" -> {o.path}")] ++
      (o.report.extensions.map fun e => s!"  relies on {e.describe}") ++
      (o.report.unsupported.map fun u => s!"  not transpiled: {u}") ++
      (o.report.axioms.map fun a => s!"  axiom: {a}") ++
      (o.report.dropped.map fun d => s!"  dropped: {d}")
  String.intercalate "\n" lines ++ "\n"

/-- Writes the generated files and the report to `out`. -/
def writeOutputs (out : System.FilePath) (results : List (Outcome × Option String)) : IO Unit := do
  IO.FS.createDirAll out
  for (o, text?) in results do
    if let some text := text? then
      let path := out / o.path
      if let some parent := path.parent then IO.FS.createDirAll parent
      IO.FS.writeFile path text
  IO.FS.writeFile (out / "transpile-report.txt") (renderReport (results.map (·.1)))

end Transpiler.Driver
