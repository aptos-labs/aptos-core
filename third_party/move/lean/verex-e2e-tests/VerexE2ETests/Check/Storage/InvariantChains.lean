-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR.Proofs.Denote

variable {unit : VerexIR.Validation.ValidatedUnit} [FunctionCarriers unit]
  (inv other : (resource : ResourceType) → resource.carrier unit → Prop)
  (m0 m1 m2 m3 m4 m5 m6 m7 m8 m9 : Memory unit)

-- Nine opaque calls preserve an invariant without exposing any memory write.
-- Conjoined facts and a cyclic alternative must not hide the valid chain.
example (base : MemoryInvariants inv m0)
    (_cycle : MemoryInvariants inv m9 → MemoryInvariants inv m8)
    (h01 : MemoryInvariants inv m0 → MemoryInvariants inv m1)
    (h12 : MemoryInvariants inv m1 → MemoryInvariants inv m2)
    (h23 : MemoryInvariants inv m2 → MemoryInvariants inv m3)
    (h34 : MemoryInvariants inv m3 → MemoryInvariants inv m4)
    (h45 : MemoryInvariants inv m4 → MemoryInvariants inv m5)
    (h56 : MemoryInvariants inv m5 → MemoryInvariants inv m6)
    (h67 : MemoryInvariants inv m6 → MemoryInvariants inv m7)
    (last : (MemoryInvariants inv m7 → MemoryInvariants inv m8) ∧
            (MemoryInvariants inv m8 → MemoryInvariants inv m9)) :
    MemoryInvariants inv m9 := by
  verex_memory_invariants

-- A cycle without a base proves nothing. Nor may a base for a different
-- invariant predicate at the same memory satisfy the chain's premise.
example (_base : MemoryInvariants other m0)
    (_h01 : MemoryInvariants inv m0 → MemoryInvariants inv m1)
    (_h10 : MemoryInvariants inv m1 → MemoryInvariants inv m0)
    (wanted : MemoryInvariants inv m1) : MemoryInvariants inv m1 := by
  fail_if_success (clear wanted; verex_memory_invariants)
  exact wanted
