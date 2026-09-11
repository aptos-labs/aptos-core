-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Registry

namespace LeanerIR.Rust.Tests.Registry

open Lean Elab Command
open LeanerIR.Validation

#import_rust_file "rust-exporter/tests/raw-unit/basic.rs" =>
  ".lake/build/registered-basic.raw.json" as registeredBasic

#import_rust_crate "Tests/CargoFixture/Cargo.toml"
  package "leaner-rust-cargo-fixture"
  features "extra" no_default_features true target ""
  => ".lake/build/registered-cargo.raw.json" as registeredCargo

syntax (name := checkRegisteredRustCmd) "#check_registered_rust " ident : command

@[command_elab checkRegisteredRustCmd]
private def elabCheckRegisteredRust : CommandElab := fun stx => do
  let name := stx[1]
  let declaration := (← getCurrNamespace) ++ name.getId
  let some unit := registeredUnit? (← getEnv) declaration
    | throwErrorAt name s!"Rust import `{declaration.toString}` was not registered"
  let some receipt := registeredReceipt? (← getEnv) declaration
    | throwErrorAt name s!"Rust import `{declaration.toString}` lost its input receipt"
  unless !receipt.inputIdentity.isEmpty && !receipt.cacheKey.isEmpty &&
      !receipt.artifactHash.isEmpty do
    throwErrorAt name "registered Rust import has an empty input receipt"
  unless unit.indexes.functionCounts == #[1] do
    throwErrorAt name "registered Rust import lost its declaration identity"
  unless (prepareExecution #[semantics] unit).isOk do
    throwErrorAt name "registered Rust import does not prepare for execution"
  unless (prepareVerification #[semantics] unit).isOk do
    throwErrorAt name "registered Rust import does not prepare for verification"

#check_registered_rust registeredBasic
#check_registered_rust registeredCargo

end LeanerIR.Rust.Tests.Registry
