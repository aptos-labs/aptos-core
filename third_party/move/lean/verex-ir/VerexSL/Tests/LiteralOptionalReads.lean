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
