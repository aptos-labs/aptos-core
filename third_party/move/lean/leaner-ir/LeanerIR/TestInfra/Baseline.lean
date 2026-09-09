-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Position
import Lean.Util.Diff

/-!
# Runtime baseline support

Baseline checks run from a package test executable rather than while its
library is compiled. This makes a normal test command check source fixtures
even when Lake can reuse every `.olean`, and lets `UB=1` update expectations.
-/

namespace LeanerIR.TestInfra.Baseline

open Lean

private def readBoolEnv (name : String) : IO Bool := do
  let some value ← IO.getEnv name
    | return false
  return value.toLower == "true" || value == "1"

def updateRequested : IO Bool := do
  for name in #["UPDATE_BASELINE", "UPBL", "UB"] do
    if ← readBoolEnv name then
      return true
  return false

/-- Discover baseline inputs in stable filename order. A new source file is
therefore included without editing a Lean driver or case table. -/
def sourceFiles (directory : System.FilePath) (suffix : String) : IO (Array System.FilePath) := do
  let entries ← directory.readDir
  pure <| entries.filter (·.fileName.endsWith suffix)
    |>.qsort (·.fileName < ·.fileName)
    |>.map (·.path)

/-- Discover immediate child directories containing `marker` in stable order.
This lets a baseline driver pick up a newly dropped package without a case
table or driver registration. -/
def markedDirectories (directory : System.FilePath) (marker : String) : IO (Array System.FilePath) := do
  let entries ← directory.readDir
  let mut directories := #[]
  for entry in entries do
    if !entry.fileName.startsWith "." && (← (entry.path / marker).pathExists) then
      directories := directories.push entry.path
  pure <| directories.qsort (toString · < toString ·)

private partial def copySelectedFiles (source destination : System.FilePath)
    (includeFile : System.FilePath → Bool) : IO Unit := do
  for entry in ← source.readDir do
    let metadata ← entry.path.symlinkMetadata
    match metadata.type with
    | .dir =>
        copySelectedFiles entry.path (destination / entry.fileName) includeFile
    | .file =>
        if includeFile entry.path then
          IO.FS.createDirAll destination
          IO.FS.writeBinFile (destination / entry.fileName) (← IO.FS.readBinFile entry.path)
    | _ => pure ()

/-- Run an action on a temporary sibling copy containing only selected files.
Keeping the copy beside the source preserves relative paths in package manifests.
The caller owns the destination name; an existing path is never overwritten. -/
def withStagedDirectory (source destination : System.FilePath)
    (includeFile : System.FilePath → Bool) (action : System.FilePath → IO α) : IO α := do
  if ← destination.pathExists then
    throw <| IO.userError s!"refusing to overwrite baseline staging directory {destination}"
  IO.FS.createDirAll destination
  try
    copySelectedFiles source destination includeFile
    action destination
  finally
    IO.FS.removeDirAll destination

/-- Discover inputs below `directory` at any depth, in stable path order,
so a newly dropped file or subdirectory is included without registration. -/
partial def sourceFilesRecursive (directory : System.FilePath) (suffix : String) :
    IO (Array System.FilePath) := do
  let mut found : Array System.FilePath := #[]
  for entry in ← directory.readDir do
    if entry.fileName.startsWith "." then continue
    if ← entry.path.isDir then
      found := found ++ (← sourceFilesRecursive entry.path suffix)
    else if entry.fileName.endsWith suffix then
      found := found.push entry.path
  pure <| found.qsort (toString · < toString ·)

def expectationPath (source : System.FilePath) (extension : String) : System.FilePath :=
  source.withExtension s!"exp.{extension}"

/-- Attach a one-based source line and column to a diagnostic whose producer
reported a UTF-8 byte offset. -/
def locatedMessage (source : System.FilePath) (startByte : Nat) (message : String) : IO String := do
  let contents ← IO.FS.readFile source
  let position := (Lean.FileMap.ofString contents).toPosition ⟨startByte⟩
  pure s!"{source}:{position.line}:{position.column + 1}: {message}"

/-- Canonical Lean spelling for a supported input whose current outcome is an
error. Prefixing every line keeps the expectation valid Lean source even for
multiline compiler diagnostics. Errors and successful generated sources use
the same side-by-side expectation path. -/
def errorOutput (message : String) : String :=
  let lines := s!"error: {message.trimAscii}" |>.splitOn "\n"
  "\n".intercalate (lines.map fun line =>
    let line := line.trimAsciiEnd.toString
    if line.isEmpty then "--" else "-- " ++ line) ++ "\n"

def formatDiff (expectation : System.FilePath) (expected actual : String) : String :=
  let changes := Diff.diff
    (expected.split '\n').toStringArray
    (actual.split '\n').toStringArray
  s!"--- {expectation}\n+++ actual\n{Diff.linesToString changes}"

/-- Compare generated output with an expectation, or replace the expectation
when one of the standard baseline-update environment variables is enabled. -/
def check (expectation : System.FilePath) (actual : String) : IO Unit := do
  if ← updateRequested then
    if let some parent := expectation.parent then
      IO.FS.createDirAll parent
    IO.FS.writeFile expectation actual
    return
  let expected ←
    if ← expectation.pathExists then
      IO.FS.readFile expectation
    else
      pure ""
  unless expected == actual do
    throw <| IO.userError s!"baseline {expectation} did not match:\n\
      {formatDiff expectation expected actual}\n\
      Run with `UB=1 lake test` (or `UPDATE_BASELINE=1 lake test`) to save the \
      current output as the new expectation"

/-- A tool's output as a baseline records: every line's trailing
whitespace trimmed, trailing empty lines dropped, one trailing newline —
and nothing at all for empty output.  This is the cleaning compiler-v2's
baseline harness applies (`move-prover/test-utils`, `clean_for_baseline`),
so its `.exp` files and these read the same way. -/
def cleanOutput (output : String) : String :=
  let lines := (output.splitOn "\n").map (·.trimAsciiEnd.toString)
  let trimmed := (lines.reverse.dropWhile (·.isEmpty)).reverse
  if trimmed.isEmpty then "" else "\n".intercalate trimmed ++ "\n"

/-- Compare a tool's output with an expectation that exists only when the
output does: a check that prints nothing has no `.exp` file.  Updating
writes the cleaned output when there is any and removes the file when
there is none; verifying reads a missing file as empty.  These are the
semantics of compiler-v2's `verify_or_update_baseline`. -/
def checkOutput (expectation : System.FilePath) (output : String) : IO Unit := do
  let actual := cleanOutput output
  if ← updateRequested then
    if actual.isEmpty then
      if ← expectation.pathExists then
        IO.FS.removeFile expectation
    else
      if let some parent := expectation.parent then
        IO.FS.createDirAll parent
      IO.FS.writeFile expectation actual
    return
  let expected ←
    if ← expectation.pathExists then
      IO.FS.readFile expectation
    else
      pure ""
  unless expected == actual do
    throw <| IO.userError s!"baseline {expectation} did not match:\n\
      {formatDiff expectation expected actual}\n\
      Run with `UB=1 lake test` (or `UPDATE_BASELINE=1 lake test`) to save the \
      current output as the new expectation (an empty output removes the file)"

end LeanerIR.TestInfra.Baseline
