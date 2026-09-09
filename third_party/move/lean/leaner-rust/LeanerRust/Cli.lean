-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Driver

namespace LeanerIR.Rust.Cli

open LeanerIR.Rust.Driver

private def usage : String :=
  "usage:\n\
    leaner-rust import-file <source.rs> [--output <unit.raw.json>] \
      [--target <triple>] [-- <additional rustc arguments>]\n\
    leaner-rust import-crate <Cargo.toml> --package <name> \
      [--output <unit.raw.json>] [--features <a,b>] \
      [--no-default-features] [--target <triple>]"

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
  | "--package" :: package :: rest =>
      if options.package.isSome then
        throw "--package may be specified only once"
      parseCargoOptions { options with package := some package } rest
  | "--output" :: path :: rest =>
      if options.output.isSome then
        throw "--output may be specified only once"
      parseCargoOptions { options with output := some (System.FilePath.mk path) } rest
  | "--features" :: features :: rest =>
      let features := features.splitOn "," |>.filter (!·.isEmpty) |>.toArray
      parseCargoOptions { options with features := options.features ++ features } rest
  | "--no-default-features" :: rest =>
      if options.noDefaultFeatures then
        throw "--no-default-features may be specified only once"
      parseCargoOptions { options with noDefaultFeatures := true } rest
  | "--target" :: target :: rest =>
      if options.target.isSome then
        throw "--target may be specified only once"
      parseCargoOptions { options with target := some target } rest
  | argument :: _ => throw s!"unknown import-crate argument `{argument}`"

private def report (result : ImportResult) (output : System.FilePath) : IO UInt32 := do
  let status := if result.cacheHit then "cache hit" else "exported and validated"
  IO.println s!"leaner-rust: {status}: {output}"
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
  | _ => throw <| IO.userError usage

end LeanerIR.Rust.Cli
