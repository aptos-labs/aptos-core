-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove.Frontend.Xast
import LeanerMove.Frontend.Decode
import LeanerMove.Frontend.Effects

/-!
# The Move CLI bridge

The transpiler's input is Move source; the XAST export is an internal
exchange step. This module runs standalone `move exchange --format ast` (or
the backward-compatible `aptos move exchange`) on a Move file or package and
decodes the result, so neither the tool nor the tests store XAST documents.

The frontend is located as the `MoveModel` frontend does:
`APTOS_MOVE_CLI` names the lightweight standalone `move` CLI;
`APTOS_CLI` selects the backward-compatible `aptos move exchange` frontend;
otherwise a checkout-local debug binary (`target/debug/aptos` up the directory
tree), then `aptos` on `PATH`, is used. Preferring the checkout binary keeps
the producer and this decoder on the same schema while developing inside the
repository.
-/

namespace LeanerMove.Frontend.Cli

open LeanerMove.Frontend.Xast LeanerMove.Frontend.Effects

private partial def findCheckoutCli (dir : System.FilePath) : IO (Option String) := do
  let candidate := dir / "target" / "debug" / "aptos"
  if ← candidate.pathExists then
    return some candidate.toString
  match dir.parent with
  | some parent => findCheckoutCli parent
  | none => return none

/-- Locates the exchange frontend. `APTOS_MOVE_CLI` takes precedence and
names the standalone `move` CLI, whose exchange subcommand is invoked directly.
`APTOS_CLI` selects the backward-compatible `aptos move exchange` frontend. -/
def findFrontend : IO (String × Array String) := do
  if let some p ← IO.getEnv "APTOS_MOVE_CLI" then
    return (p, #["exchange"])
  if let some p ← IO.getEnv "APTOS_CLI" then
    return (p, #["move", "exchange"])
  if let some p ← findCheckoutCli (← IO.currentDir) then
    return (p, #["move", "exchange"])
  return ("aptos", #["move", "exchange"])

private def run (exe : String) (args : Array String) : IO Unit := do
  let out ← IO.Process.output { cmd := exe, args }
  if out.exitCode != 0 then
    throw (IO.userError s!"`{exe} {" ".intercalate args.toList}` failed:\n{out.stderr}{out.stdout}")

private def decodeFile (path : System.FilePath) : IO Module := do
  let text ← IO.FS.readFile path
  match LeanerMove.Frontend.Decode.parseModule text with
  | .ok m => pure m
  | .error e => throw (IO.userError s!"cannot decode the XAST export of {path}: {e}")

/-- Exports one self-contained Move module file (the standard library is its
only dependency) and decodes it. -/
def exportMoveFile (moveFile : System.FilePath) : IO Module := do
  let (exe, commandArgs) ← findFrontend
  IO.FS.withTempDir fun dir => do
    let out := dir / "module.xast.json"
    run exe (commandArgs ++ #["--format", "ast", "--move-file", moveFile.toString,
      "--out-file", out.toString]
    )
    decodeFile out

/-- Exports and decodes several self-contained Move module files as one
package. -/
def exportMoveFiles (files : List System.FilePath) : IO Package := do
  let modules ← files.mapM exportMoveFile
  pure { modules }

/-- Reads every `*.xast.json` of a directory (an existing export). -/
def readXastDir (dir : System.FilePath) : IO Package := do
  let entries ← dir.readDir
  let files := entries.filter (fun e => e.fileName.endsWith ".xast.json")
    |>.qsort (·.fileName < ·.fileName)
  let modules ← files.toList.mapM fun e => decodeFile e.path
  pure { modules }

/-- Exports a Move package (`--package-dir`) and decodes its modules;
`includeDeps` also exports the dependency modules with source. -/
def exportPackage (dir : System.FilePath) (includeDeps : Bool := false) : IO Package := do
  let (exe, commandArgs) ← findFrontend
  IO.FS.withTempDir fun tmp => do
    let args := commandArgs ++ #["--format", "ast", "--package-dir", dir.toString,
      "--export-dir", tmp.toString] ++ (if includeDeps then #["--include-deps"] else #[])
    run exe args
    let package ← readXastDir tmp
    if !includeDeps then return package
    -- A package's own modules are the ones under its `sources` directory;
    -- every other exported module came in through a dependency.
    let ownedPrefix := (dir / "sources").toString
    let owned := package.modules.filter fun candidate =>
      candidate.sources.any (·.startsWith ownedPrefix)
    let dependencies := package.modules.filter fun candidate =>
      !candidate.sources.any (·.startsWith ownedPrefix)
    return { modules := owned, dependencies }

end LeanerMove.Frontend.Cli
