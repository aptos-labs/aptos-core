-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Transpiler
import LeanerIR.TestInfra

/-! Printer baselines.  Every `*.move` under `Transpiler/Tests/Programs` is
exported by the Aptos CLI at elaboration time, transpiled, and compared byte
for byte with the generated file next to it (`account.move` ↔ `Account.lean`).
The generated files double as the elaboration gate: `Transpiler/Tests.lean`
imports them, so `lake test` builds them against `Move`.

To update after a printer change:

```
UB=1 lake build Transpiler.Tests.Baseline
```

Every generated module is imported, `BasicCoin` included (its `pragma
aborts_if_is_partial` clause and two-directional `aborts_if` are Leaner
features now); `AptosFramework/Counter.lean` exercises the root namespace of
an aliased module (E1). -/

open Transpiler Transpiler.Driver
open LeanerIR.TestInfra

def programsDir : System.FilePath := "Transpiler/Tests/Programs"

def checkBaselines : IO Unit := do
  let entries ← programsDir.readDir
  let moves := entries.filter (·.fileName.endsWith ".move") |>.qsort (·.fileName < ·.fileName)
  let pkg ← Cli.exportMoveFiles (moves.toList.map (·.path))
  let results := transpilePackage pkg (emitVerify := true)
  for (o, text?) in results do
    match text? with
    | some text =>
      Baseline.check (programsDir / o.path) text
    | none => throw (IO.userError s!"{o.path}: {o.error.getD "no output"}")
  IO.print (renderReport (results.map (·.1)))

/--
info: 0x42::account: clean -> Account.lean
aptos_framework::counter: clean -> AptosFramework/Counter.lean
  relies on E1 nested namespace
0x42::basic_coin: clean -> BasicCoin.lean
0x42::constants: clean -> Constants.lean
0x42::enums: clean -> Enums.lean
0x42::function_values: clean -> FunctionValues.lean
0x42::generics: clean -> Generics.lean
0x42::loops: clean -> Loops.lean
0x42::ordered_map: clean -> OrderedMap.lean
0x42::vectors: clean -> Vectors.lean
-/
#guard_msgs in
#eval checkBaselines

/-! The E2E-owned move-stdlib source package (spec files included) is exported
by the CLI at elaboration time, transpiled under the Lean root
`Transpiler.Tests.Programs.MoveStdlib`, and compared byte for byte with this
package's legacy generated `Std/*.lean` fixtures and checked-in report. To update:

```
UB=1 lake build Transpiler.Tests.Baseline
```

`Transpiler/Tests.lean` imports every generated module (all 14 transpiled
stdlib modules elaborate; `vector` is curated), so the suite is also the
elaboration gate of the stdlib output — Leaner's derivation of the
specification versions of the Move functions the specifications apply
included. -/

def stdlibSourceDir : System.FilePath :=
  "../leaner-e2e-tests/LeanerE2ETests/MoveToLeanerLang/MoveStdlib"

def stdlibExpectedDir : System.FilePath := "Transpiler/Tests/Programs/MoveStdlib"

private def stdlibInput (path : System.FilePath) : Bool :=
  path.fileName.any fun name =>
    name == "Move.toml" || name == "Move.lock" || name.endsWith ".move"

def checkStdlibBaselines : IO Unit := do
  let some parent := stdlibSourceDir.parent
    | throw <| IO.userError s!"Move package {stdlibSourceDir} has no parent directory"
  let staged := parent / ".MoveStdlib.transpiler-input"
  Baseline.withStagedDirectory stdlibSourceDir staged stdlibInput fun staged => do
    let pkg ← Cli.exportPackage staged
    let results := transpilePackage pkg (leanRoot := "Transpiler.Tests.Programs.MoveStdlib")
    for (o, text?) in results do
      match text?, o.curated with
      | some text, _ =>
        Baseline.check (stdlibExpectedDir / o.path) text
      | none, some _ => pure ()
      | none, none => throw (IO.userError s!"{o.path}: {o.error.getD "no output"}")
    let report := renderReport (results.map (·.1))
    Baseline.check (stdlibExpectedDir / "transpile-report.txt") report
    IO.println s!"move-stdlib: {results.length} modules checked"

/-- info: move-stdlib: 15 modules checked -/
#guard_msgs in
#eval checkStdlibBaselines
