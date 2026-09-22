-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Operations
import Lean.Elab.Tactic.Omega

/-! Arithmetic evaluation facts shared with native agreement. They must not
require the retiring weakest-precondition engine. -/

namespace LeanerIR.Proofs

theorem truncatingQuotient?_eq (left right : Int) :
    SemanticOperations.truncatingQuotient? left right =
      if right = 0 then none else some (left.tdiv right) := by
  unfold SemanticOperations.truncatingQuotient?
  rcases left with m | m <;> rcases right with n | n <;>
    simp [Int.tdiv, Int.natAbs, beq_iff_eq]
  · by_cases h : n = 0 <;> simp [h]
    omega
  · rintro rfl; simp
  · by_cases h : n = 0 <;> simp [h]

theorem truncatingRemainder?_eq (left right : Int) :
    SemanticOperations.truncatingRemainder? left right =
      if right = 0 then none else some (left.tmod right) := by
  unfold SemanticOperations.truncatingRemainder?
  simp only [truncatingQuotient?_eq, Int.tmod_def]
  by_cases h : right = 0 <;> simp [h, Option.bind, Int.mul_comm]


end LeanerIR.Proofs
