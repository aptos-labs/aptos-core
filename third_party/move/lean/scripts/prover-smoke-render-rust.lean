-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.SourceVerify

/-!
Writes the VerexSL rendering of Rust source fixtures without elaborating it:
what the verification of the source renders and then elaborates, its
specification file and the library's contracts spliced in. Run by
`scripts/prover-smoke-renderings.sh` in `verex-e2e-tests`, with the fixture stems in
`PROVER_SMOKE_RENDER_NAMES` (separated by commas) and the output directory in
`PROVER_SMOKE_RENDER_OUT`.
-/

open VerexIR.Rust.SourceVerify VerexSL.SourceVerify

def renderRustSource (environment : Lean.Environment) (out : System.FilePath) (stem : String) :
    IO Unit := do
  let source : System.FilePath := s!"VerexE2ETests/SourceVerify/{stem}.rs"
  let unit ← IO.FS.withTempDir fun directory =>
    return (← VerexIR.Rust.Driver.importRustFile
      { source, output := directory / "unit.raw.json" }).unit
  let specs ← specsFor unit source none
  let companions := specs ++ (← libraryCompanions unit (specs.filterMap (·.text)))
  discard <| VerexSL.SourceVerify.run environment
    { unit, companions, output := out / s!"{stem}.rs.lean", renderOnly := true }

run_cmd do
  let environment ← Lean.getEnv
  let some names ← IO.getEnv "PROVER_SMOKE_RENDER_NAMES" | throwError "PROVER_SMOKE_RENDER_NAMES is not set"
  let some out ← IO.getEnv "PROVER_SMOKE_RENDER_OUT" | throwError "PROVER_SMOKE_RENDER_OUT is not set"
  IO.FS.createDirAll out
  for name in names.splitOn "," do
    let name := name.trimAscii.copy
    if name.isEmpty then continue
    renderRustSource environment out name
