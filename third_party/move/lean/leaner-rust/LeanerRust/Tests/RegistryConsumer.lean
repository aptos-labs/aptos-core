-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerRust.Tests.Registry

namespace LeanerIR.Rust.Tests.RegistryConsumer

open Lean Elab Command

syntax (name := checkImportedRustRegistrationCmd)
  "#check_imported_rust_registration " ident : command

@[command_elab checkImportedRustRegistrationCmd]
private def elabCheckImportedRustRegistration : CommandElab := fun stx => do
  let declaration := `LeanerIR.Rust.Tests.Registry ++ stx[1].getId
  let some unit := registeredUnit? (← getEnv) declaration
    | throwError "registered Rust unit did not survive the Lean module import boundary"
  let some receipt := registeredReceipt? (← getEnv) declaration
    | throwError "registered Rust receipt did not survive the Lean module import boundary"
  unless !receipt.cacheKey.isEmpty && !receipt.artifactHash.isEmpty do
    throwError "imported Rust registration lost its canonical-input cache key"
  unless unit.indexes.functionCounts == #[1] do
    throwError "imported Rust registration lost its validated declaration identity"

#check_imported_rust_registration registeredBasic
#check_imported_rust_registration registeredCargo

end LeanerIR.Rust.Tests.RegistryConsumer
