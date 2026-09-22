-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean.Data.Json
import LeanerRust.Benchmark
import LeanerRust.Profile

/-!
# Lean-owned Rust import driver

This is the first M1.5 build-integration slice. It locates or builds the
project-owned exporter, invokes it under its pinned Rust toolchain, validates
the detached RawUnit through the ordinary Rust profile, and caches only an
artifact whose producer inputs still match.
-/

namespace LeanerIR.Rust.Driver

open Lean (Json)
open LeanerIR

structure ImportRequest where
  source : System.FilePath
  output : System.FilePath
  rustcArgs : Array String := #[]
  deriving Inhabited

/-- Import path which produced a cache receipt. The mode is retained so equal
textual identities from file and Cargo imports cannot be conflated by proof or
source consumers. -/
inductive ImportMode where
  | file
  | cargo
  deriving Repr, BEq, Inhabited

/-- Lean-owned receipt for the canonical producer inputs admitted by the
import driver. `cacheKey` binds all inputs hashed by the selected import mode;
`inputIdentity` remains readable provenance rather than a replacement for the
key. -/
structure ImportCacheReceipt where
  private mk ::
  mode : ImportMode
  inputIdentity : String
  cacheKey : String
  artifactHash : String
  deriving Repr, BEq

structure ImportResult where
  private mk ::
  unit : LeanerIR.Validation.ValidatedUnit
  receipt : ImportCacheReceipt
  cacheHit : Bool

/-- An imported Rust unit admitted by the executable semantic boundary. The
validated unit is retained so source/proof consumers can share declaration and
location identities with the interpreter-facing view. -/
structure ExecutionImportResult where
  private mk ::
  validated : LeanerIR.Validation.ValidatedUnit
  executable : LeanerIR.Validation.ExecutableUnit
  receipt : ImportCacheReceipt
  cacheHit : Bool

/-- An imported Rust unit admitted by the verification semantic boundary.
The same validated declaration/location identities remain available to proof
and source consumers; the private wrapper records that every reachable
verification node has a registered meaning. -/
structure VerificationImportResult where
  private mk ::
  validated : LeanerIR.Validation.ValidatedUnit
  verifiable : LeanerIR.Validation.VerifiableUnit
  receipt : ImportCacheReceipt
  cacheHit : Bool

/-- Prepare an already imported Rust unit for semantic execution. Keeping this
as an explicit conversion prevents callers from treating structural validation
as proof that every reachable operation has executable meaning. -/
def ImportResult.prepareExecution (result : ImportResult) :
    Except (Array LeanerIR.Validation.Diagnostic) ExecutionImportResult := do
  let executable ← LeanerIR.Validation.prepareExecution #[LeanerIR.Rust.semantics]
    result.unit
  pure {
    validated := result.unit
    executable := executable
    receipt := result.receipt
    cacheHit := result.cacheHit }

/-- Prepare an already imported Rust unit for verification consumers. This is
kept separate from execution preparation because the supported logical and
runtime feature sets intentionally advance at different milestones. -/
def ImportResult.prepareVerification (result : ImportResult) :
    Except (Array LeanerIR.Validation.Diagnostic) VerificationImportResult := do
  let verifiable ← LeanerIR.Validation.prepareVerification #[LeanerIR.Rust.semantics]
    result.unit
  pure {
    validated := result.unit
    verifiable := verifiable
    receipt := result.receipt
    cacheHit := result.cacheHit }

structure CargoImportRequest where
  manifest : System.FilePath
  package : String
  output : System.FilePath
  features : Array String := #[]
  noDefaultFeatures : Bool := false
  target : Option String := none
  deriving Inhabited

private structure CargoMetadata where
  raw : String
  crateName : String
  localRoots : Array System.FilePath

private structure Exporter where
  binary : System.FilePath
  root : System.FilePath

private initialize managedExporterMemo : IO.Ref (Option Exporter) ← IO.mkRef none

private initialize exporterLibraryPathMemo : IO.Ref (Option (System.FilePath × String)) ←
  IO.mkRef none

private def run (command : IO.Process.SpawnArgs) : IO IO.Process.Output := do
  let result ← IO.Process.output command
  if result.exitCode != 0 then
    throw <| IO.userError s!"`{command.cmd} {" ".intercalate command.args.toList}` failed:\n\
      {result.stderr}{result.stdout}"
  pure result

private partial def findExporterRoot (directory : System.FilePath) : IO (Option System.FilePath) := do
  let direct := directory / "rust-exporter"
  if ← (direct / "Cargo.toml").pathExists then
    return some direct
  let nested := directory / "leaner-rust" / "rust-exporter"
  if ← (nested / "Cargo.toml").pathExists then
    return some nested
  match directory.parent with
  | some parent => findExporterRoot parent
  | none => pure none

private def exporterRoot : IO System.FilePath := do
  let some root ← findExporterRoot (← IO.currentDir)
    | throw <| IO.userError "cannot locate leaner-rust/rust-exporter from the current directory"
  pure root

private def ensureExporter : IO Exporter := do
  let root ← exporterRoot
  if let some override ← IO.getEnv "LEANER_RUST_EXPORTER" then
    let binary := System.FilePath.mk override
    unless ← binary.pathExists do
      throw <| IO.userError s!"LEANER_RUST_EXPORTER does not exist: {binary}"
    return { binary, root }
  if let some exporter ← managedExporterMemo.get then
    if exporter.root == root && (← exporter.binary.pathExists) then
      return exporter
  let _ ← Benchmark.measure "exporter.ensure" root.toString <| run {
      cmd := "cargo"
      -- The source round-trip suites invoke the exporter once per fixture. The
      -- rustc driver itself is prebuilt, but our mapper and JSON encoder are not;
      -- using Cargo's development profile here made the managed path needlessly
      -- expensive. The binary bytes already participate in every cache key.
      args := #["build", "--release", "--features", "rustc-public"]
      cwd := some root
    }
  let binary := root / "target" / "release" / "leaner-rust-export"
  unless ← binary.pathExists do
    throw <| IO.userError s!"cargo succeeded but did not produce {binary}"
  let exporter := { binary, root }
  managedExporterMemo.set (some exporter)
  pure exporter

private def exporterLibraryPath (root : System.FilePath) : IO String := do
  if let some override ← IO.getEnv "LEANER_RUST_SYSROOT" then
    return (System.FilePath.mk override / "lib").toString
  if let some (cachedRoot, path) ← exporterLibraryPathMemo.get then
    if cachedRoot == root then
      return path
  let result ← Benchmark.measure "exporter.sysroot" root.toString <| run {
      cmd := "rustc"
      args := #["--print", "sysroot"]
      cwd := some root
    }
  let path := (System.FilePath.mk result.stdout.trimAscii.toString / "lib").toString
  exporterLibraryPathMemo.set (some (root, path))
  pure path

private def appendHash (state : UInt64) (bytes : ByteArray) : UInt64 :=
  bytes.foldl (fun hash byte => (hash ^^^ byte.toUInt64) * 1099511628211) state

private def appendString (state : UInt64) (value : String) : UInt64 :=
  appendHash (appendHash state value.toUTF8) (ByteArray.empty.push 0)

private structure BaseCacheIdentity where
  binary : System.FilePath
  binaryModified : IO.FS.SystemTime
  binarySize : UInt64
  toolchainModified : IO.FS.SystemTime
  toolchainSize : UInt64
  deriving BEq

private initialize baseCacheHashMemo : IO.Ref (Option (BaseCacheIdentity × UInt64)) ←
  IO.mkRef none

private def baseCacheHash (exporter : Exporter) : IO UInt64 := do
  let toolchainPath := exporter.root / "rust-toolchain.toml"
  let binaryMetadata ← exporter.binary.metadata
  let toolchainMetadata ← toolchainPath.metadata
  let identity : BaseCacheIdentity := {
    binary := exporter.binary
    binaryModified := binaryMetadata.modified
    binarySize := binaryMetadata.byteSize
    toolchainModified := toolchainMetadata.modified
    toolchainSize := toolchainMetadata.byteSize }
  if let some (cachedIdentity, hash) ← baseCacheHashMemo.get then
    if cachedIdentity == identity then
      return hash
  let binary ← IO.FS.readBinFile exporter.binary
  let toolchain ← IO.FS.readFile toolchainPath
  let mut hash : UInt64 := 14695981039346656037
  hash := appendHash hash binary
  hash := appendString hash toolchain
  hash := appendString hash profileName
  hash := appendString hash (toString profileVersion)
  for (name, value) in config.options do
    hash := appendString hash name
    hash := appendString hash value
  baseCacheHashMemo.set (some (identity, hash))
  pure hash

private def cacheKey (request : ImportRequest) (exporter : Exporter) : IO String := do
  let source ← IO.FS.readBinFile request.source
  let mut hash ← baseCacheHash exporter
  hash := appendString hash "leaner-rust-import-file-v1"
  hash := appendString hash request.source.toString
  hash := appendHash hash source
  for argument in request.rustcArgs do
    hash := appendString hash argument
  pure (toString hash)

private def receiptPath (output : System.FilePath) : System.FilePath :=
  output.addExtension "key"

private def artifactHash (path : System.FilePath) : IO String := do
  let bytes ← IO.FS.readBinFile path
  pure <| toString (appendHash 14695981039346656037 bytes)

private def decodeValidated (path : System.FilePath)
    (displayPath : System.FilePath := path) : IO LeanerIR.Validation.ValidatedUnit := do
  let text ← IO.FS.readFile path
  match decodeAndValidate text with
  | .ok unit => pure unit
  | .error diagnostics =>
      throw <| IO.userError s!"Rust RawUnit for `{displayPath}` did not validate: {repr diagnostics}"

private def cached? (output : System.FilePath) (key : String) :
    IO (Option (LeanerIR.Validation.ValidatedUnit × String)) := do
  let receipt := receiptPath output
  unless (← output.pathExists) && (← receipt.pathExists) do
    return none
  let receiptLines := (← IO.FS.readFile receipt).trimAscii.toString.splitOn "\n"
  let [recordedKey, recordedArtifactHash] := receiptLines | return none
  unless recordedKey == key && (← artifactHash output) == recordedArtifactHash do
    return none
  try
    return some (← decodeValidated output, recordedArtifactHash)
  catch _ =>
    pure none

private def publish (output artifact : System.FilePath) (key : String) : IO String := do
  if let some parent := output.parent then
    IO.FS.createDirAll parent
  IO.FS.writeFile output (← IO.FS.readFile artifact)
  let hash ← artifactHash output
  IO.FS.writeFile (receiptPath output) (key ++ "\n" ++ hash ++ "\n")
  pure hash

/-- Export and validate one self-contained Rust library file. A matching,
successfully revalidated artifact is reused; cache corruption or staleness
causes a fresh export. -/
def importRustFile (request : ImportRequest) : IO ImportResult := do
  let source ← IO.FS.realPath request.source
  let request := { request with source }
  let exporter ← Benchmark.measure "import.ensure_exporter" source.toString ensureExporter
  let key ← Benchmark.measure "import.cache_key" source.toString <| cacheKey request exporter
  if let some (unit, hash) ← Benchmark.measure "import.cache_lookup" source.toString <|
      cached? request.output key then
    let receipt : ImportCacheReceipt := {
      mode := .file, inputIdentity := source.toString, cacheKey := key,
      artifactHash := hash }
    return { unit, receipt, cacheHit := true }
  let libraryPath ← Benchmark.measure "import.sysroot" source.toString <|
    exporterLibraryPath exporter.root
  let inheritedLibraryPath ← IO.getEnv "LD_LIBRARY_PATH"
  let libraryPath := inheritedLibraryPath.map (libraryPath ++ ":" ++ ·) |>.getD libraryPath
  IO.FS.withTempDir fun temporary => do
    let artifact := temporary / "unit.raw.json"
    let result ← Benchmark.measure "rust.export" source.toString <| IO.Process.output {
      cmd := exporter.binary.toString
      args := #["--output", artifact.toString, "--", source.toString,
        "--crate-type=lib", "--edition=2024"] ++ request.rustcArgs
      env := #[(("LD_LIBRARY_PATH"), some libraryPath)]
    }
    if result.exitCode != 0 then
      throw <| IO.userError s!"Rust exporter failed:\n{result.stderr}{result.stdout}"
    let unit ← Benchmark.measure "lir.decode_validate" source.toString <|
      decodeValidated artifact source
    let hash ← Benchmark.measure "artifact.publish" source.toString <|
      publish request.output artifact key
    let receipt : ImportCacheReceipt := {
      mode := .file, inputIdentity := source.toString, cacheKey := key,
      artifactHash := hash }
    pure { unit, receipt, cacheHit := false }

private def cargoFeatureArgs (request : CargoImportRequest) : Array String := Id.run do
  let mut arguments := #[]
  if request.noDefaultFeatures then
    arguments := arguments.push "--no-default-features"
  if !request.features.isEmpty then
    arguments := arguments ++ #["--features", ",".intercalate request.features.toList]
  arguments

private def parseCargoMetadata (text package : String) : Except String CargoMetadata := do
  let json ← Json.parse text
  let packages ← (← json.getObjVal? "packages").getArr?
  let mut crateName : Option String := none
  let mut localRoots := #[]
  for packageJson in packages do
    let name ← (← packageJson.getObjVal? "name").getStr?
    match ← packageJson.getObjVal? "source" with
    | .null =>
        let manifest ← (← packageJson.getObjVal? "manifest_path").getStr?
        if let some root := (System.FilePath.mk manifest).parent then
          localRoots := localRoots.push root
    | _ => pure ()
    if name == package then
      let targets ← (← packageJson.getObjVal? "targets").getArr?
      for targetJson in targets do
        let kinds ← (← targetJson.getObjVal? "kind").getArr?
        let isLibrary := kinds.any fun
          | .str "lib" | .str "rlib" => true
          | _ => false
        if isLibrary then
          if crateName.isSome then
            throw s!"Cargo package `{package}` has more than one library target"
          crateName := some (← (← targetJson.getObjVal? "name").getStr?)
  let some selectedCrateName := crateName
    | throw s!"Cargo metadata has no library target for package `{package}`"
  pure { raw := text, crateName := selectedCrateName, localRoots }

private def cargoMetadata (request : CargoImportRequest) (exporter : Exporter) : IO CargoMetadata := do
  let mut arguments := #["metadata", "--format-version=1", "--locked",
    "--manifest-path", request.manifest.toString] ++ cargoFeatureArgs request
  if let some target := request.target then
    arguments := arguments ++ #["--filter-platform", target]
  let result ← run { cmd := "cargo", args := arguments, cwd := some exporter.root }
  match parseCargoMetadata result.stdout request.package with
  | .ok metadata => pure metadata
  | .error message => throw <| IO.userError s!"invalid Cargo metadata: {message}"

private def isIgnoredDirectory (path : System.FilePath) : Bool :=
  path.fileName.any fun name => name == ".git" || name == ".lake" || name == "target"

private def cargoCacheKey (request : CargoImportRequest) (metadata : CargoMetadata)
    (exporter : Exporter) : IO String := do
  let mut hash ← baseCacheHash exporter
  hash := appendString hash "leaner-rust-import-cargo-v1"
  hash := appendString hash request.manifest.toString
  hash := appendString hash request.package
  hash := appendString hash metadata.raw
  hash := appendString hash metadata.crateName
  hash := appendString hash (toString request.noDefaultFeatures)
  for feature in request.features do
    hash := appendString hash feature
  if let some target := request.target then
    hash := appendString hash target
  for root in metadata.localRoots do
    let paths ← root.walkDir fun path => pure !isIgnoredDirectory path
    let paths := paths.qsort fun left right => left.toString < right.toString
    for path in paths do
      let info ← path.metadata
      if info.type == .file && path != request.output && path != receiptPath request.output then
        hash := appendString hash path.toString
        hash := appendHash hash (← IO.FS.readBinFile path)
  pure (toString hash)

private def cargoTargetDirectory (exporter : Exporter) (key : String) : IO System.FilePath := do
  if let some override ← IO.getEnv "LEANER_RUST_CACHE" then
    pure (System.FilePath.mk override / "cargo" / key)
  else
    pure (exporter.root / "target" / "leaner-rust-import" / key)

/-- Import the selected library target of a Cargo package. Cargo prepares
dependencies normally, while the exporter wrapper intercepts only this root
target and stops it after analysis. -/
def importCargoCrate (request : CargoImportRequest) : IO ImportResult := do
  let manifest ← IO.FS.realPath request.manifest
  let currentDirectory ← IO.currentDir
  let output := if request.output.isAbsolute then request.output else currentDirectory / request.output
  let request := { request with manifest, output }
  let exporter ← ensureExporter
  let metadata ← cargoMetadata request exporter
  let key ← cargoCacheKey request metadata exporter
  if let some (unit, hash) ← cached? request.output key then
    let receipt : ImportCacheReceipt := {
      mode := .cargo
      inputIdentity := s!"{manifest}#{request.package}"
      cacheKey := key
      artifactHash := hash }
    return { unit, receipt, cacheHit := true }
  let libraryPath ← exporterLibraryPath exporter.root
  let inheritedLibraryPath ← IO.getEnv "LD_LIBRARY_PATH"
  let libraryPath := inheritedLibraryPath.map (libraryPath ++ ":" ++ ·) |>.getD libraryPath
  let targetDirectory ← cargoTargetDirectory exporter key
  IO.FS.createDirAll targetDirectory
  IO.FS.withTempDir fun temporary => do
    let artifact := temporary / "unit.raw.json"
    let mut arguments := #["check", "--locked", "--manifest-path", manifest.toString,
      "--package", request.package, "--lib", "--target-dir", targetDirectory.toString] ++
      cargoFeatureArgs request
    if let some target := request.target then
      arguments := arguments ++ #["--target", target]
    let result ← IO.Process.output {
      cmd := "cargo"
      args := arguments
      cwd := some exporter.root
      env := #[
        ("LD_LIBRARY_PATH", some libraryPath),
        ("RUSTC_WRAPPER", some exporter.binary.toString),
        ("LEANER_RUST_CARGO_OUTPUT", some artifact.toString),
        ("LEANER_RUST_CARGO_PACKAGE", some request.package),
        ("LEANER_RUST_CARGO_CRATE", some metadata.crateName)
      ]
    }
    if result.exitCode != 0 then
      throw <| IO.userError s!"Cargo Rust import failed:\n{result.stderr}{result.stdout}"
    unless ← artifact.pathExists do
      throw <| IO.userError s!"Cargo completed without exporting root crate `{request.package}`"
    let unit ← decodeValidated artifact request.manifest
    let hash ← publish request.output artifact key
    let receipt : ImportCacheReceipt := {
      mode := .cargo
      inputIdentity := s!"{manifest}#{request.package}"
      cacheKey := key
      artifactHash := hash }
    pure { unit, receipt, cacheHit := false }

end LeanerIR.Rust.Driver
