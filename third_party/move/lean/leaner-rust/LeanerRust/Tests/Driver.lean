-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Driver

namespace LeanerIR.Rust.Tests

open LeanerIR.Rust.Driver

def checkImportDriver : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let request : ImportRequest := {
      source := "rust-exporter/tests/raw-unit/basic.rs"
      output := directory / "basic.raw.json"
    }
    let first ← importRustFile request
    let second ← importRustFile request
    unless !first.cacheHit && second.cacheHit &&
        first.receipt == second.receipt && first.receipt.mode == .file &&
        !first.receipt.cacheKey.isEmpty && !first.receipt.artifactHash.isEmpty &&
        first.unit.indexes.functionCounts == #[1] &&
        second.unit.indexes.functionCounts == #[1] do
      throw <| IO.userError "Rust import driver did not export, validate, and reuse its fixture"
    unless (← request.output.pathExists) &&
        (← (request.output.addExtension "key").pathExists) do
      throw <| IO.userError "Rust import driver did not persist its artifact and cache receipt"
    IO.FS.writeFile request.output
      (← IO.FS.readFile "rust-exporter/tests/raw-unit/control.exp.json")
    let repaired ← importRustFile request
    unless !repaired.cacheHit && repaired.receipt == first.receipt &&
        repaired.unit.indexes.functionCounts == #[1] do
      throw <| IO.userError
        "Rust import driver accepted a valid artifact that disagreed with its receipt hash"
    let prepared ← match second.prepareExecution with
      | .ok prepared => pure prepared
      | .error diagnostics =>
          throw <| IO.userError s!"imported Rust fixture did not prepare: {repr diagnostics}"
    let handle : LeanerIR.FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
    match LeanerIR.Interpreter.run prepared.executable 16 handle #[] with
    | .ok (_, outcome) => unless outcome.value == .returned #[.bool true] do
        throw <| IO.userError "prepared Rust import returned the wrong semantic value"
    | .error error =>
        throw <| IO.userError s!"prepared Rust import did not execute: {repr error}"
    unless prepared.cacheHit && prepared.receipt == second.receipt &&
        prepared.validated.indexes.functionCounts == #[1] &&
        prepared.executable.initializationCertificates.size == 1 do
      throw <| IO.userError "execution preparation did not retain import identity or cache status"
    let verification ← match second.prepareVerification with
      | .ok verification => pure verification
      | .error diagnostics =>
          throw <| IO.userError s!"imported Rust fixture did not prepare for verification: {repr diagnostics}"
    unless verification.cacheHit && verification.receipt == second.receipt &&
        verification.validated.indexes.functionCounts == #[1] &&
        verification.validated.tables.locations == prepared.validated.tables.locations &&
        prepared.executable.initializationCertificates.size == 1 &&
        verification.verifiable.initializationCertificates ==
          prepared.executable.initializationCertificates do
      throw <| IO.userError
        "verification preparation did not retain import identity or initialization evidence"

def checkCargoImportDriver : IO Unit := do
  IO.FS.withTempDir fun directory => do
    let request : CargoImportRequest := {
      manifest := "Tests/CargoFixture/Cargo.toml"
      package := "leaner-rust-cargo-fixture"
      output := directory / "cargo.raw.json"
      features := #["extra"]
      noDefaultFeatures := true
    }
    let first ← importCargoCrate request
    let second ← importCargoCrate request
    unless !first.cacheHit && second.cacheHit &&
        first.receipt == second.receipt && first.receipt.mode == .cargo &&
        !first.receipt.cacheKey.isEmpty && !first.receipt.artifactHash.isEmpty &&
        first.unit.indexes.functionCounts == #[1] &&
        second.unit.indexes.functionCounts == #[1] do
      throw <| IO.userError
        "Cargo import driver did not export only the root library and reuse its fixture"
    let prepared ← match second.prepareExecution with
      | .ok prepared => pure prepared
      | .error diagnostics =>
          throw <| IO.userError s!"Cargo Rust fixture did not prepare: {repr diagnostics}"
    let handle : LeanerIR.FunctionHandle := { namespaceId := ⟨0⟩, functionId := ⟨0⟩ }
    match LeanerIR.Interpreter.run prepared.executable 16 handle #[] with
    | .ok (_, outcome) => unless outcome.value == .returned #[.bool true] do
        throw <| IO.userError "prepared Cargo import returned the wrong semantic value"
    | .error error =>
        throw <| IO.userError s!"prepared Cargo import did not execute: {repr error}"
    let verification ← match second.prepareVerification with
      | .ok verification => pure verification
      | .error diagnostics =>
          throw <| IO.userError s!"Cargo Rust fixture did not prepare for verification: {repr diagnostics}"
    unless verification.cacheHit && prepared.receipt == second.receipt &&
        verification.receipt == second.receipt &&
        verification.validated.indexes.functionCounts == #[1] &&
        verification.validated.tables.locations == prepared.validated.tables.locations &&
        prepared.executable.initializationCertificates.size == 1 &&
        verification.verifiable.initializationCertificates ==
          prepared.executable.initializationCertificates do
      throw <| IO.userError
        "Cargo verification preparation did not retain import identity or initialization evidence"

#guard_msgs in
#eval checkImportDriver *> checkCargoImportDriver

end LeanerIR.Rust.Tests
