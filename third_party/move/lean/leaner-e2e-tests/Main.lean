-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests

/-- Walk up to the enclosing Cargo workspace that owns `aptos-move`. -/
private partial def findWorkspaceRoot (dir : System.FilePath) :
    IO (Option System.FilePath) := do
  if (← (dir / "Cargo.toml").pathExists) && (← (dir / "aptos-move").pathExists) then
    return some dir
  match dir.parent with
  | some parent => findWorkspaceRoot parent
  | none => return none

/-- Build the checkout's standalone `move` CLI with Cargo's optimized `ci`
profile. A debug exchange export dominates suite time, so the managed binary
deliberately uses the optimized profile; Cargo's own freshness check keeps
the repeat build cheap. -/
private def buildManagedMoveCli : IO (Option System.FilePath) := do
  let some root ← findWorkspaceRoot (← IO.currentDir) | return none
  let out ← IO.Process.output {
    cmd := "cargo"
    args := #["build", "--locked", "--profile", "ci", "-p", "aptos-move-cli",
      "--features", "binary", "--bin", "move"]
    cwd := some root }
  if out.exitCode != 0 then
    throw <| IO.userError s!"building the managed move CLI failed:\n{out.stderr}"
  let binary := root / "target" / "ci" / "move"
  unless ← binary.pathExists do
    throw <| IO.userError s!"cargo succeeded but did not produce {binary}"
  return some binary

unsafe def runSuites : IO Unit := do
  let environment ← LeanerLang.Print.loadEnvironment
  match ← IO.getEnv "LEANER_E2E_SUITE" with
  | none =>
      -- `lake test` is the full package suite: selecting one suite is a
      -- development convenience, never the default coverage.
      LeanerE2ETests.MoveToLeanerLang.testBaselines environment
      LeanerE2ETests.RustToLeanerLang.testBaselines environment
      LeanerE2ETests.Check.testBaselines
      LeanerE2ETests.MonoVM.testSmoke
      LeanerE2ETests.MonoDifferential.testBaselines environment
  | some "move" => LeanerE2ETests.MoveToLeanerLang.testBaselines environment
  | some "check" => LeanerE2ETests.Check.testBaselines
  | some "rust" => LeanerE2ETests.RustToLeanerLang.testBaselines environment
  | some "monovm" => LeanerE2ETests.MonoVM.testSmoke
  | some "monodiff" => LeanerE2ETests.MonoDifferential.testBaselines environment
  | some suite =>
      throw <| IO.userError
        s!"unknown LEANER_E2E_SUITE `{suite}`; expected `move`, `rust`, `check`, `monovm`, or `monodiff`"

unsafe def main : IO UInt32 := do
  -- Freshness contract: inside a checkout the driver always runs the locked
  -- ci-profile Cargo build of the standalone CLI, so the frontend is current
  -- with this tree. `APTOS_MOVE_CLI`, when set, must name a Cargo ci-profile
  -- build of the CLI - normally the checkout binary this build just
  -- freshened. `APTOS_CLI` selects the full Aptos CLI as an unmanaged escape
  -- hatch whose freshness its setter owns.
  match ← IO.getEnv "APTOS_MOVE_CLI" with
  | some override =>
      -- The re-executed child skips the check its parent just ran; the
      -- freshness build happens once per suite invocation.
      if (← IO.getEnv "LEANER_E2E_MANAGED").isNone then
        if let some managed ← buildManagedMoveCli then
          let sameBinary ← try
              pure ((← IO.FS.realPath override) == (← IO.FS.realPath managed))
            catch _ => pure false
          unless sameBinary do
            IO.eprintln <| s!"warning: APTOS_MOVE_CLI={override} is not this checkout's "
              ++ s!"managed frontend {managed}; the contract requires a Cargo ci-profile "
              ++ "build of the standalone CLI, and this run cannot ensure its freshness"
  | none =>
      if (← IO.getEnv "APTOS_CLI").isNone then
        if let some binary ← buildManagedMoveCli then
          let child ← IO.Process.spawn {
            cmd := (← IO.appPath).toString
            env := #[("APTOS_MOVE_CLI", some binary.toString),
              ("LEANER_E2E_MANAGED", some "1")] }
          return (← child.wait)
  runSuites
  return 0
