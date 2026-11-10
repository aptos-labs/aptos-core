-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexIR.Proofs.Denote

-- Optional reads agree with reads of the integer values at every index,
-- including missing entries. The fallback need not be zero or unsigned.
example (index : Nat) :
    ((([⟨1, by decide⟩, ⟨2, by decide⟩] : List (SpecInt (.bits 8) false))[index]?).getD
      ⟨9, by decide⟩).val = ([1, 2][index]?).getD (9 : Int) := by
  verex_normalize_context

example (index : Nat) :
    ((([⟨-3, by decide⟩, ⟨7, by decide⟩] : List (SpecInt (.bits 8) true))[index]?).getD
      ⟨-7, by decide⟩).val = ([-3, 7][index]?).getD (-7 : Int) := by
  verex_normalize_context

-- A symbolic stored value keeps its certificate and its value, rather than
-- being treated as a literal or replaced by the fallback.
example (value : SpecInt (.bits 8) true) :
    (([value][0]?).getD (⟨-7, by decide⟩ : SpecInt (.bits 8) true)).val = value.val := by
  verex_normalize_context

-- A stated read supplies an array witness even when the context contains many
-- unrelated certified integers. Their speculative witnesses must not exhaust
-- the bounded search before it reaches the position actually read.
set_option linter.unusedVariables false in
example (values : Array (SpecInt (.bits 32) false)) (endpoint : SpecInt (.bits 64) false)
    (target : SpecInt (.bits 32) false)
    (noise0 noise1 noise2 noise3 noise4 noise5 noise6 noise7
     noise8 noise9 noise10 noise11 noise12 noise13 noise14 noise15 : SpecInt (.bits 64) false)
    (positive : 0 < endpoint.val) (bound : endpoint.val ≤ values.size)
    (read : (values[(endpoint.val - 1).toNat]?.getD default).val = target.val) :
    ∃ i : Int, 0 ≤ i ∧ i < values.size ∧ values[i.toNat]? = some target := by
  verex_cheap_witness
