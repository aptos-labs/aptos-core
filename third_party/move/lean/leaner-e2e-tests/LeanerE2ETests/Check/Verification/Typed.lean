-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# V3 native verification boundaries

This fixture pins the composition of the shallow denotation with generated
struct twins.  The verified function takes and returns the native `Amount`
structure; no `Array RuntimeValue` appears in its proof-facing signature.
-/

namespace LeanerLang.Tests.VerificationTyped

leaner module 0x42::verification_typed where
  struct Amount has Copy, Drop, Store where
    value : u64

  public fun carry_amount(value : Amount) -> Amount := value

  spec carry_amount where
    pragma aborts_if_is_strict
    ensures result.value == value.value

  verify carry_amount

open LeanerIR
open LeanerIR.Proofs

#check («0x42».verification_typed.carry_amount.typedDenotation :
  LeanerIR.Validation.ExecutableUnit →
    «0x42».verification_typed.carry_amount.Arguments →
      Spec RuntimeState Failure «0x42».verification_typed.Amount)

#check («0x42».verification_typed.carry_amount.typedContract :
  Contract RuntimeState Failure
    «0x42».verification_typed.carry_amount.Arguments
    «0x42».verification_typed.Amount)

end LeanerLang.Tests.VerificationTyped
