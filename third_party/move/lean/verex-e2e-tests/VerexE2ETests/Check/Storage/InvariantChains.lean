-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR.Proofs.Denote

variable {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
  (inv other : (resource : ResourceType) → resource.carrier unit → Prop)
  (m0 m1 m2 m3 m4 m5 m6 m7 m8 m9 : Memory unit)

-- Nine opaque calls preserve an invariant without exposing any memory write.
-- Conjoined facts and a cyclic alternative must not hide the valid chain.
example (base : DataInvariants inv m0)
    (_cycle : DataInvariants inv m9 → DataInvariants inv m8)
    (h01 : DataInvariants inv m0 → DataInvariants inv m1)
    (h12 : DataInvariants inv m1 → DataInvariants inv m2)
    (h23 : DataInvariants inv m2 → DataInvariants inv m3)
    (h34 : DataInvariants inv m3 → DataInvariants inv m4)
    (h45 : DataInvariants inv m4 → DataInvariants inv m5)
    (h56 : DataInvariants inv m5 → DataInvariants inv m6)
    (h67 : DataInvariants inv m6 → DataInvariants inv m7)
    (last : (DataInvariants inv m7 → DataInvariants inv m8) ∧
            (DataInvariants inv m8 → DataInvariants inv m9)) :
    DataInvariants inv m9 := by
  verex_data_invariants

-- A cycle without a base proves nothing. Nor may a base for a different
-- invariant predicate at the same memory satisfy the chain's premise.
example (_base : DataInvariants other m0)
    (_h01 : DataInvariants inv m0 → DataInvariants inv m1)
    (_h10 : DataInvariants inv m1 → DataInvariants inv m0)
    (wanted : DataInvariants inv m1) : DataInvariants inv m1 := by
  fail_if_success (clear wanted; verex_data_invariants)
  exact wanted
