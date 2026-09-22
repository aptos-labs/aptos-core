-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lake
open Lake DSL

/-!
# Leaner end-to-end baselines

The `lakefile.lean` exists, rather than a `lakefile.toml`, because the test
driver links the MonoVM adapter: custom targets run Cargo and `leanc`, and
target-valued `moreLinkObjs` cannot be declared in TOML. Only the driver
executable gains the native dependency; the libraries and the language
server stay free of it.
-/

package "leaner-e2e-tests" where
  testDriver := "LeanerE2ETestDriver"

-- The Move exchange frontend and LIR adapter live in leaner-move.
require "leaner-move" from ".." / "leaner-move"
require "leaner-rust" from ".." / "leaner-rust"

@[default_target]
lean_lib LeanerE2ETests where
  roots := #[`LeanerE2ETests]

/-- The repository root, three directories above this package. -/
private def repoRoot (pkg : NPackage __name__) : System.FilePath :=
  pkg.dir.join ".." |>.join ".." |>.join ".." |>.join ".."

/-- Lists every file under `dir`, recursively, sorted by path for a stable
hash order. -/
private partial def walkFiles : System.FilePath → IO (Array System.FilePath)
  | dir => do
    let mut out : Array System.FilePath := #[]
    for entry in (← dir.readDir) do
      if ← entry.path.isDir then
        out := out ++ (← walkFiles entry.path)
      else
        out := out.push entry.path
    return out.qsort (fun a b => a.toString < b.toString)

/-- Mixes the hashes of the adapter crate's sources, the workspace lockfile,
and the toolchain and build-identity strings into the staticlib's dependency
trace, so a change on any of them relinks the driver. -/
private def staticlibTrace (pkg : NPackage __name__) : IO Hash := do
    let files :=
      (← walkFiles (repoRoot pkg |>.join "third_party" |>.join "move" |>.join "mono-move"
        |>.join "lean-link"))
        ++ #[repoRoot pkg |>.join "Cargo.lock", repoRoot pkg |>.join "rust-toolchain.toml"]
    let mut hash := Hash.ofString "mono-move-lean-link release abi-1"
    for file in files do
      let fileHash ← computeFileHash file
      hash := Hash.mk (mixHash hash.val fileHash.val)
    return hash

/-- Builds the MonoVM adapter staticlib with Cargo, in the explicitly
selected release profile, from the same checkout this package builds from.
Cargo's own target directory under `.lake` keeps its incremental cache. -/
target monovm_staticlib pkg : System.FilePath := Job.async do
  let targetDir := pkg.dir.join ".lake" |>.join "cargo-target"
  let archive := targetDir.join "release" |>.join "libmono_move_lean_link.a"
  let depTrace := BuildTrace.ofHash (← staticlibTrace pkg)
  let traceFile := System.FilePath.mk (archive.toString ++ ".trace")
  buildUnlessUpToDate archive depTrace traceFile do
    createParentDirs archive
    proc (quiet := false) {
      cmd := "cargo",
      args := #[
        "build", "-p", "mono-move-lean-link", "--profile", "release",
        "--manifest-path", (repoRoot pkg |>.join "Cargo.toml").toString,
        "--target-dir", targetDir.toString ],
      cwd := repoRoot pkg }
  return archive

/-- Compiles the C shim against the pinned Lean toolchain's `lean/lean.h`. -/
target monovm_shim pkg : System.FilePath := Job.async do
  let source := pkg.dir.join "shim" |>.join "monovm_shim.c"
  let object := pkg.buildDir.join "monovm_shim.o"
  let sourceTrace ← computeTrace source
  let traceFile := System.FilePath.mk (object.toString ++ ".trace")
  buildUnlessUpToDate object sourceTrace traceFile do
    createParentDirs object
    proc (quiet := false) {
      cmd := "leanc", args := #["-c", source.toString, "-o", object.toString] }
  return object

@[default_target]
lean_exe LeanerE2ETestDriver where
  root := `Main
  supportInterpreter := true
  moreLinkObjs := #[monovm_staticlib, monovm_shim]
  moreLinkArgs := #["-lm", "-ldl", "-lpthread"]
