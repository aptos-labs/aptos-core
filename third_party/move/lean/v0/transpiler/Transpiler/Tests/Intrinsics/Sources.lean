-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler
import LeanerIR.TestInfra

/-!
The E16 source fixtures are reduced, faithful copies of the framework packages:

* `Programs/Intrinsics/AptosStdlib` contains the definitions and specifications of
  `Table`, `TableWithLength`, `SimpleMap`, and `SmartTable`, plus the smallest
  source closure needed to compile them independently.
* `Programs/Intrinsics/AptosFramework` contains the definitions and specifications of
  `OrderedMap` and `BigOrderedMap` and depends on the reduced stdlib fixture.

This test carries all six map-intrinsic declarations through XAST, raw LIR,
and the Move-profile graph/signature validator. It then ensures that the
transpiler rejects their owning modules explicitly at the still-suspended E16
semantic interpretation boundary.
-/

open Transpiler Transpiler.Xast Transpiler.Effects Transpiler.Driver Transpiler.Print
open LeanerIR.TestInfra

namespace Transpiler.Tests.Intrinsics.Sources

private def aptosStdlibDir : System.FilePath :=
  "Transpiler/Tests/Programs/Intrinsics/AptosStdlib"

private def aptosFrameworkDir : System.FilePath :=
  "Transpiler/Tests/Programs/Intrinsics/AptosFramework"

private def moveStdlibDir : System.FilePath :=
  "../../leaner-e2e-tests/LeanerE2ETests/MoveToLeanerLang/MoveStdlib"

private def moveStdlibInput (path : System.FilePath) : Bool :=
  path.fileName.any fun name =>
    name == "Move.toml" || name == "Move.lock" || name.endsWith ".move"

private def isMapIntrinsic (s : Struct) : Bool :=
  s.spec.pragmas.any fun p => p.name == "intrinsic" && p.value == .name "map"

private def qualifiedName (name : QualifiedName) : String :=
  s!"{name.module.addressAlias.getD name.module.address}::{name.module.name}::{name.name}"

private def bindingTarget (role : String) (bindings : List IntrinsicBinding) : String :=
  (bindings.find? (·.role == role)).map (qualifiedName ·.target) |>.getD "missing"

private def describeMapIntrinsic (m : Module) (s : Struct) : String :=
  let ty := s!"{m.addressAlias.getD m.address}::{m.name}::{s.name}"
  match s.intrinsic with
  | none => s!"{ty}: missing resolved intrinsic"
  | some intrinsic =>
    s!"{ty}: {intrinsic.name} move={intrinsic.moveFunctions.length} spec={intrinsic.specFunctions.length} " ++
      s!"map_new={bindingTarget "map_new" intrinsic.moveFunctions} " ++
      s!"map_spec_get={bindingTarget "map_spec_get" intrinsic.specFunctions}"

private def mapIntrinsicSummaries (pkg : Package) : List String :=
  pkg.modules.flatMap (fun m =>
    m.structs.filterMap fun s =>
      if isMapIntrinsic s then
        some (describeMapIntrinsic m s)
      else
        none)

private def mapIntrinsicBindingCount (pkg : Package) : Nat :=
  pkg.modules.foldl (fun count m => m.structs.foldl (fun count s =>
    match s.intrinsic with
    | some intrinsic => count + intrinsic.moveFunctions.length + intrinsic.specFunctions.length
    | none => count) count) 0

private def expectedMapIntrinsicSummaries : List String := [
  "aptos_framework::big_ordered_map::BigOrderedMap: map move=26 spec=20 map_new=aptos_framework::big_ordered_map::new map_spec_get=aptos_framework::big_ordered_map::spec_get",
  "aptos_framework::ordered_map::OrderedMap: map move=29 spec=19 map_new=aptos_framework::ordered_map::new map_spec_get=aptos_framework::ordered_map::spec_get",
  "aptos_std::simple_map::SimpleMap: map move=8 spec=10 map_new=aptos_std::simple_map::create map_spec_get=aptos_std::simple_map::spec_get",
  "aptos_std::smart_table::SmartTable: map move=11 spec=5 map_new=aptos_std::smart_table::new map_spec_get=aptos_std::smart_table::spec_get",
  "aptos_std::table::Table: map move=10 spec=4 map_new=aptos_std::table::new map_spec_get=aptos_std::table::spec_get",
  "aptos_std::table_with_length::TableWithLength: map move=11 spec=5 map_new=aptos_std::table_with_length::new map_spec_get=aptos_std::table_with_length::spec_get",
]

private def requireIntrinsicRejection (pkg : Package) : IO Unit := do
  let outputs := transpilePackage pkg
  for m in pkg.modules do
    let some owner := m.structs.find? fun s => s.intrinsic.isSome | continue
    let some (outcome, source?) := outputs.find? fun (outcome, _) => outcome.module == m.ref
      | throw (IO.userError s!"missing transpiler outcome for {m.name}")
    if source?.isSome then
      throw (IO.userError s!"intrinsic module {m.name} unexpectedly produced Leaner source")
    let expected := s!"intrinsic type `{owner.name}` uses model `map`; E16 is suspended"
    match outcome.error with
    | some error =>
        unless error.contains expected do
          throw (IO.userError
            s!"unexpected intrinsic rejection for {m.name}\nexpected to contain: {expected}\nactual: {error}")
    | none => throw (IO.userError s!"intrinsic module {m.name} was not rejected")

private def checkIntrinsicSources : IO Unit := do
  let some parent := moveStdlibDir.parent
    | throw <| IO.userError s!"Move package {moveStdlibDir} has no parent directory"
  let staged := parent / ".MoveStdlib.intrinsics-input"
  Baseline.withStagedDirectory moveStdlibDir staged moveStdlibInput fun _ => do
    let aptosStdlib ← Cli.exportPackage aptosStdlibDir
    let aptosFramework ← Cli.exportPackage aptosFrameworkDir
    let actual := (mapIntrinsicSummaries aptosStdlib ++ mapIntrinsicSummaries aptosFramework).toArray
      |>.qsort (· < ·) |>.toList
    if actual != expectedMapIntrinsicSummaries then
      throw (IO.userError
        s!"intrinsic source fixture mismatch\nexpected: {expectedMapIntrinsicSummaries}\nactual:   {actual}")
    let bindingCount := mapIntrinsicBindingCount aptosStdlib +
      mapIntrinsicBindingCount aptosFramework
    if bindingCount != 158 then
      throw (IO.userError s!"intrinsic binding count mismatch: expected 158, found {bindingCount}")
    requireIntrinsicRejection aptosStdlib
    requireIntrinsicRejection aptosFramework
    let moduleCount := aptosStdlib.modules.length + aptosFramework.modules.length
    IO.println s!"intrinsic source fixtures: {actual.length} map types and {bindingCount} bindings across {moduleCount} modules"

/-- info: intrinsic source fixtures: 6 map types and 158 bindings across 10 modules -/
#guard_msgs in
#eval checkIntrinsicSources

end Transpiler.Tests.Intrinsics.Sources
