-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
open Lean Meta Elab Command VerexSL VerexSL.Contract VerexSL.Verify
open VerexIR.Proofs VerexIR.Proofs.Denote
verex module 0x42::given_production where
  pragma verify = false
  fun identity {T}(value : T) -> T := value
  spec identity where
    aborts_if false
    ensures result == value
  fun caller {T}(value : T) -> T := identity(value)
  spec caller where
    aborts_if false
    ensures result == value
verify 0x42::given_production::identity by
  on _ { compute }
verify 0x42::given_production::caller by
  on _ { compute }

-- The ordinary verifier must retain the actual Given proof and state the
-- concrete adapter's evidence as a named proposition.
run_cmd do
  let env ← getEnv
  for base in [`«0x42».given_production.identity, `«0x42».given_production.caller] do
    unless (givenProofArtifacts.getState env).find? base == some (base ++ `givenTypedVerified) do
      throwError "missing actual Given body artifact"
    let some transfer := env.find? (base ++ `givenTransfer)
      | throwError "missing explicit concrete transfer proposition"
    unless transfer.type == mkSort .zero do
      throwError "concrete transfer evidence is not a proposition"

-- Ordinary normalization must retain the declaration table as one named
-- application, including in the family used by every dependent binder.
run_cmd liftTermElabM do
  let table := mkConst `«0x42».given_production.givenOwnDeclarations
  withLocalDeclD `unit (mkConst ``VerexIR.Validation.ValidatedUnit) fun unit =>
    withLocalDeclD `functions (mkApp (mkConst ``FunctionCarriers) unit) fun functions =>
      withLocalDeclD `given (mkConst ``GivenFrame) fun given => do
        let application := mkApp3 table unit functions given
        unless (← withReducible (whnf application)) == application do
          throwError "ordinary normalization expanded the nominal declaration table"
