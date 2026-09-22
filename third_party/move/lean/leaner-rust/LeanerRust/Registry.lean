-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean
import LeanerRust.Driver

/-!
# Registered Rust imports

Validated Rust imports are retained in the Lean environment under an explicit
declaration name. Source backends and proof commands therefore consume the
same checked unit without re-reading an artifact or constructing a parallel
frontend representation.
-/

namespace LeanerIR.Rust

open Lean Elab Command
open LeanerIR.Validation
open LeanerIR.Rust.Driver

private initialize importedUnitExt :
    SimplePersistentEnvExtension (Name × ValidatedUnit) (NameMap ValidatedUnit) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun units (name, unit) => units.insert name unit
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun units (name, unit) => units.insert name unit) {} entries
  }

private initialize importedReceiptExt :
    SimplePersistentEnvExtension (Name × ImportCacheReceipt)
      (NameMap ImportCacheReceipt) ←
  registerSimplePersistentEnvExtension {
    addEntryFn := fun receipts (name, receipt) => receipts.insert name receipt
    addImportedFn := fun entries =>
      mkStateFromImportedEntries
        (fun receipts (name, receipt) => receipts.insert name receipt) {} entries
  }

/-- Retrieve a validated Rust unit registered in this or an imported Lean
module. The result still requires the explicit execution or verification
preparation boundary before use by those consumers. -/
def registeredUnit? (env : Environment) (name : Name) : Option ValidatedUnit :=
  (importedUnitExt.getState env).find? name

/-- Retrieve the canonical-input cache receipt retained with a registered Rust
import. Manually registered validated units have no receipt. -/
def registeredReceipt? (env : Environment) (name : Name) : Option ImportCacheReceipt :=
  (importedReceiptExt.getState env).find? name

/-- Register one validated Rust unit. Repeating an identical registration is
idempotent; assigning different semantics to an existing name is rejected. -/
def registerUnit (env : Environment) (name : Name) (unit : ValidatedUnit) :
    Except String Environment :=
  match registeredUnit? env name with
  | none => .ok (importedUnitExt.addEntry env (name, unit))
  | some previous =>
      if previous == unit then .ok env
      else .error s!"Rust import name `{name.toString}` is already registered with a different validated unit"

private def registerImportResult (name : Syntax) (result : ImportResult) :
    CommandElabM Unit := do
  let declaration := (← getCurrNamespace) ++ name.getId
  let environment ← match registerUnit (← getEnv) declaration result.unit with
    | .ok environment => pure environment
    | .error message => throwErrorAt name message
  let environment ← match registeredReceipt? environment declaration with
    | none => pure (importedReceiptExt.addEntry environment (declaration, result.receipt))
    | some previous =>
        if previous == result.receipt then pure environment
        else throwErrorAt name
          s!"Rust import name `{declaration.toString}` is already registered with a different input receipt"
  setEnv environment
  let status := if result.cacheHit then "cache hit" else "exported and validated"
  logInfoAt name s!"registered Rust import `{declaration.toString}` ({status})"

syntax (name := importRustFileCmd)
  "#import_rust_file " str " => " str " as " ident : command

/-- Explicitly import, validate, and persist one self-contained Rust library
file in the current Lean environment. Merely placing a Rust file beside Lean
source never triggers this command. -/
@[command_elab importRustFileCmd]
def elabImportRustFile : CommandElab := fun stx => do
  let source := stx[1]
  let outputStx := stx[3]
  let name := stx[5]
  let some sourceValue := source.isStrLit?
    | throwErrorAt source "expected a Rust source path string"
  let some outputValue := outputStx.isStrLit?
    | throwErrorAt outputStx "expected a RawUnit output path string"
  let result ← liftIO <| importRustFile {
    source := System.FilePath.mk sourceValue
    output := System.FilePath.mk outputValue
  }
  registerImportResult name result

syntax (name := importRustCrateCmd)
  "#import_rust_crate " str ident str ident str ident ident ident str
  " => " str " as " ident : command

/-- Explicitly import, validate, and persist one Cargo library target in the
current Lean environment. An empty target string selects Cargo's host target;
all other request fields are included in the import cache identity. -/
@[command_elab importRustCrateCmd]
def elabImportRustCrate : CommandElab := fun stx => do
  let manifest := stx[1]
  let packageLabel := stx[2]
  let packageStx := stx[3]
  let featureLabel := stx[4]
  let featureStx := stx[5]
  let noDefaultsLabel := stx[6]
  let noDefaults := stx[7]
  let targetLabel := stx[8]
  let targetStx := stx[9]
  let outputStx := stx[11]
  let name := stx[13]
  unless packageLabel.getId == `package do
    throwErrorAt packageLabel "expected `package`"
  unless featureLabel.getId == `features do
    throwErrorAt featureLabel "expected `features`"
  unless noDefaultsLabel.getId == `no_default_features do
    throwErrorAt noDefaultsLabel "expected `no_default_features`"
  unless targetLabel.getId == `target do
    throwErrorAt targetLabel "expected `target`"
  let some manifestValue := manifest.isStrLit?
    | throwErrorAt manifest "expected a Cargo manifest path string"
  let some packageValue := packageStx.isStrLit?
    | throwErrorAt packageStx "expected a Cargo package string"
  let some featuresValue := featureStx.isStrLit?
    | throwErrorAt featureStx "expected a comma-separated feature string"
  let noDefaultFeatures ← match noDefaults.getId with
    | `true => pure true
    | `false => pure false
    | _ => throwErrorAt noDefaults "expected `true` or `false`"
  let some targetValue := targetStx.isStrLit?
    | throwErrorAt targetStx "expected a target triple string"
  let targetValue := if targetValue.isEmpty then none else some targetValue
  let some outputValue := outputStx.isStrLit?
    | throwErrorAt outputStx "expected a RawUnit output path string"
  let featureValues := featuresValue.splitOn "," |>.filter (!·.isEmpty) |>.toArray
  let result ← liftIO <| importCargoCrate {
    manifest := System.FilePath.mk manifestValue
    package := packageValue
    output := System.FilePath.mk outputValue
    features := featureValues
    noDefaultFeatures
    target := targetValue
  }
  registerImportResult name result

end LeanerIR.Rust
