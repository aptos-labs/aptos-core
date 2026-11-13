-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexRust.SourceVerify

namespace VerexIR.Rust.Cli

open VerexIR.Rust.Driver

private def usage : String :=
  "usage:\n\
    verex-rust import-file <source.rs> [--output <unit.raw.json>] \
      [--target <triple>] [-- <additional rustc arguments>]\n\
    verex-rust import-crate <Cargo.toml> --package <name> \
      [--output <unit.raw.json>] [--features <a,b>] \
      [--no-default-features] [--target <triple>]\n\
    verex-rust verify <source.rs> [--spec <specs.lean>] [--output <generated.lean>] \
      [--ignore-proofs] [-- <additional rustc arguments>]\n\
    verex-rust verify <Cargo.toml> --package <name> [--features <a,b>] \
      [--no-default-features] [--target <triple>] [--output <generated.lean>] \
      [--ignore-proofs]"

private structure Options where
  request : ImportRequest

private structure CargoOptions where
  manifest : System.FilePath
  package : Option String := none
  output : Option System.FilePath := none
  features : Array String := #[]
  noDefaultFeatures : Bool := false
  target : Option String := none

private partial def parseImportOptions (source : System.FilePath)
    (arguments : List String) (output : Option System.FilePath := none)
    (rustcArgs : Array String := #[]) : Except String Options := do
  match arguments with
  | [] =>
      let output := output.getD (source.addExtension "raw.json")
      pure { request := { source, output, rustcArgs } }
  | "--output" :: path :: rest =>
      if output.isSome then
        throw "--output may be specified only once"
      parseImportOptions source rest (some (System.FilePath.mk path)) rustcArgs
  | "--target" :: target :: rest =>
      parseImportOptions source rest output (rustcArgs ++ #["--target", target])
  | "--" :: rest =>
      let rustcArgs := rustcArgs ++ rest.toArray
      parseImportOptions source [] output rustcArgs
  | argument :: _ => throw s!"unknown import-file argument `{argument}`"

/-- The leading Cargo argument and the arguments after it, or `none` when
the leading argument is not one. -/
private def cargoArgument (options : CargoOptions) :
    List String → Except String (Option (CargoOptions × List String))
  | "--package" :: package :: rest => do
      if options.package.isSome then
        throw "--package may be specified only once"
      pure (some ({ options with package := some package }, rest))
  | "--output" :: path :: rest => do
      if options.output.isSome then
        throw "--output may be specified only once"
      pure (some ({ options with output := some (System.FilePath.mk path) }, rest))
  | "--features" :: features :: rest =>
      let features := features.splitOn "," |>.filter (!·.isEmpty) |>.toArray
      pure (some ({ options with features := options.features ++ features }, rest))
  | "--no-default-features" :: rest => do
      if options.noDefaultFeatures then
        throw "--no-default-features may be specified only once"
      pure (some ({ options with noDefaultFeatures := true }, rest))
  | "--target" :: target :: rest => do
      if options.target.isSome then
        throw "--target may be specified only once"
      pure (some ({ options with target := some target }, rest))
  | _ => pure none

private partial def parseCargoOptions (options : CargoOptions) (arguments : List String) :
    Except String CargoImportRequest := do
  match arguments with
  | [] =>
      let some package := options.package
        | throw "import-crate requires --package <name>"
      let output := options.output.getD (options.manifest.addExtension "raw.json")
      pure {
        manifest := options.manifest
        package
        output
        features := options.features
        noDefaultFeatures := options.noDefaultFeatures
        target := options.target
      }
  | argument :: _ =>
      match ← cargoArgument options arguments with
      | some (options, rest) => parseCargoOptions options rest
      | none => throw s!"unknown import-crate argument `{argument}`"

private structure VerifyOptions where
  source : System.FilePath
  specFile : Option System.FilePath := none
  output : Option System.FilePath := none
  rustcArgs : Array String := #[]
  /-- Every function is verified automatically, its authored proof ignored. -/
  ignoreProofs : Bool := false

private partial def parseVerifyOptions (options : VerifyOptions) :
    List String → Except String VerifyOptions
  | [] => pure options
  | "--spec" :: path :: rest => do
      if options.specFile.isSome then throw "--spec may be specified only once"
      parseVerifyOptions { options with specFile := some path } rest
  | "--output" :: path :: rest => do
      if options.output.isSome then throw "--output may be specified only once"
      parseVerifyOptions { options with output := some path } rest
  | "--ignore-proofs" :: rest => parseVerifyOptions { options with ignoreProofs := true } rest
  | "--" :: rest => pure { options with rustcArgs := rest.toArray }
  | argument :: _ => throw s!"unknown verify argument `{argument}`"

private structure VerifyCrateOptions where
  cargo : CargoOptions
  ignoreProofs : Bool := false

/-- The `import-crate` arguments, `--output` naming the rendering, and
`--ignore-proofs`. -/
private partial def parseVerifyCrateOptions (options : VerifyCrateOptions) :
    List String → Except String VerifyCrateOptions
  | [] => pure options
  | "--ignore-proofs" :: rest =>
      parseVerifyCrateOptions { options with ignoreProofs := true } rest
  | arguments@(argument :: _) => do
      match ← cargoArgument options.cargo arguments with
      | some (cargo, rest) => parseVerifyCrateOptions { options with cargo } rest
      | none => throw s!"unknown verify argument `{argument}`"

/-- Report a verification's messages and phase times; the exit code fails on
an error. -/
private def reportVerification (reports : Array VerexSL.SourceVerify.Report) (output : System.FilePath)
    (start : Nat) : IO UInt32 := do
  for report in reports do IO.println report.render
  IO.println s!"verex-rust: generated {output}"
  -- The wall time per phase is a report on the run, not one of its messages.
  IO.eprintln s!"verex-rust: {← VerexSL.Perf.phaseSummary ((← IO.monoNanosNow) - start)}"
  pure <| if reports.any (·.severity == .error) then 1 else 0

/-- Verify a Rust file; the rendering is written to `--output`, or to
`<name>.rs.lean`. -/
private def verifySource (options : VerifyOptions) : IO UInt32 := do
  let start ← IO.monoNanosNow
  let output := options.output.getD (options.source.addExtension "lean")
  let environment ← VerexSL.Perf.withPhase .load VerexSL.SourceVerify.importVerexSL
  let reports ← SourceVerify.verifyFile environment options.source options.specFile output
    options.rustcArgs options.ignoreProofs
  reportVerification reports output start

/-- Verify a Cargo package's library crate; the rendering is written to
`--output`, or to `target/verex-verify.lean` beside the manifest. -/
private def verifyCargo (options : VerifyCrateOptions) : IO UInt32 := do
  let start ← IO.monoNanosNow
  let some package := options.cargo.package
    | throw <| IO.userError s!"verify of a Cargo package requires --package <name>\n{usage}"
  let manifest := options.cargo.manifest
  let output := options.cargo.output.getD
    ((manifest.parent.getD ".") / "target" / "verex-verify.lean")
  let request : CargoImportRequest := {
    manifest, package, output
    features := options.cargo.features
    noDefaultFeatures := options.cargo.noDefaultFeatures
    target := options.cargo.target }
  let environment ← VerexSL.Perf.withPhase .load VerexSL.SourceVerify.importVerexSL
  let reports ← SourceVerify.verifyCrate environment request output options.ignoreProofs
  reportVerification reports output start

private def report (result : ImportResult) (output : System.FilePath) : IO UInt32 := do
  let status := if result.cacheHit then "cache hit" else "exported and validated"
  IO.println s!"verex-rust: {status}: {output}"
  pure 0

def run (arguments : List String) : IO UInt32 := do
  let arguments := match arguments with
    | "--" :: rest => rest
    | arguments => arguments
  match arguments with
  | "import-file" :: source :: rest =>
      let options ← match parseImportOptions (System.FilePath.mk source) rest with
        | .ok options => pure options
        | .error message => throw <| IO.userError s!"{message}\n{usage}"
      report (← importRustFile options.request) options.request.output
  | "import-crate" :: manifest :: rest =>
      let request ← match parseCargoOptions { manifest := System.FilePath.mk manifest } rest with
        | .ok request => pure request
        | .error message => throw <| IO.userError s!"{message}\n{usage}"
      report (← importCargoCrate request) request.output
  | "verify" :: source :: rest =>
      if (System.FilePath.mk source).fileName == some "Cargo.toml" then
        match parseVerifyCrateOptions { cargo := { manifest := System.FilePath.mk source } } rest with
        | .ok options => verifyCargo options
        | .error message => throw <| IO.userError s!"{message}\n{usage}"
      else
        match parseVerifyOptions { source := System.FilePath.mk source } rest with
        | .ok options => verifySource options
        | .error message => throw <| IO.userError s!"{message}\n{usage}"
  | _ => throw <| IO.userError usage

end VerexIR.Rust.Cli
