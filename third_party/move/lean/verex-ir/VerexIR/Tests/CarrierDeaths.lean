-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Compile

namespace VerexIR.Tests.CarrierDeaths
open VerexIR VerexIR.Validation VerexIR.Proofs.Denote

-- Replacing the reference under `selected : &mut &mut [u32]` retires the
-- inner slice reference, while keeping the outer loan to `selected` live.
example (unit : ValidatedUnit) (ns : CompileNamespace) :
    holderDeaths unit ns .none
      (Γ := .cons (.ref (.sliceRef (.int 32 false))) .nil) 0 .here
      [#[.dereference]] =
      .ok [Term.resolveSliceAt .here (.deref .nil)] := rfl

-- The same path rule applies to an ordinary nested mutable reference.
example (unit : ValidatedUnit) (ns : CompileNamespace) :
    holderDeaths unit ns .none
      (Γ := .cons (.ref (.ref (.int 32 false))) .nil) 0 .here
      [#[.dereference]] =
      .ok [Term.resolveAt 0 .here (.deref .nil) .slot] := rfl

-- Explicit paths take precedence over a holder's root when the runtime
-- receives the retirement annotation, matching the compiler interpretation.
private def nestedDeath : LoanDeath := {
  anchor := ⟨3⟩, holders := #[⟨0⟩]
  holderPaths := #[(⟨0⟩, #[.dereference])] }
example : nestedDeath.places = #[(⟨0⟩, [.dereference])] := rfl
example : (carrierDeathsIn #[nestedDeath] ⟨3⟩).after =
    #[(⟨0⟩, [.dereference])] := rfl
example : (carrierDeathsIn #[nestedDeath] ⟨3⟩).before = #[] := rfl
example : (carrierDeathsIn #[nestedDeath] ⟨4⟩).after = #[] := rfl

end VerexIR.Tests.CarrierDeaths
