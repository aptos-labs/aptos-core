-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide

namespace VerexIR.Proofs.Denote

-- A raw-only boundary keeps progress with no typed writeBack reads.
example (old exit : VerexIR.SpecEntries (Nat × Nat)) (k v : Nat) (e : Nat × Nat)
    (written : exit.entries = SpecMap.update .sequence old.entries k v)
    (member : e ∈ exit.entries) : e = (k, v) ∨ e ∈ old.entries := by
  verex_written_back
  all_goals first | exact Or.inl (by assumption) | exact Or.inr (by assumption)

-- A dependent hypothesis prevents clearing an update membership.
example (old exit : VerexIR.SpecEntries (Nat × Nat)) (k v : Nat) (e : Nat × Nat)
    (written : exit.entries = SpecMap.update .sequence old.entries k v)
    (member : e ∈ SpecMap.update .sequence old.entries k v)
    (dependent : member = member) : e = (k, v) ∨ e ∈ old.entries := by
  verex_written_back
  all_goals first | exact Or.inl (by assumption) | exact Or.inr (by assumption)

-- The same dependency can retain a removal membership.
example (old exit : VerexIR.SpecEntries (Nat × Nat)) (k : Nat) (e : Nat × Nat)
    (written : exit.entries = SpecMap.remove .sequence old.entries k)
    (member : e ∈ SpecMap.remove .sequence old.entries k)
    (dependent : member = member) : e ∈ old.entries := by
  verex_written_back
  assumption

end VerexIR.Proofs.Denote
