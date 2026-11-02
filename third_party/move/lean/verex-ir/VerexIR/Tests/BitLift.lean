-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.BitLift

/-! A bit identity can be decided independently of source hypotheses. When
the hypotheses are necessary, a failed target-only attempt retains the full
request's context and proof destination. -/

set_option verex.bitVectors true

open VerexIR VerexIR.Proofs.IntegerArithmetic

example (value mask : SpecInt (.bits 8) false) :
    bitwiseAnd mask.val (bitwiseAnd value.val (↑((255 : Nat) ^^^ mask.val.toNat))) = 0 := by
  verex_bv_target

example (byte : SpecInt (.bits 8) false) (known : byte.val = 7) :
    bitwiseAnd byte.val 3 = 3 := by
  fail_if_success verex_bv_target
  first | verex_bv_target | verex_bv
