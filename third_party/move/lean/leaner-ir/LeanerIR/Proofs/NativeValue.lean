-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Computation

namespace LeanerIR.Proofs

/-- Name an owned local without losing its exact value at a modular boundary. -/
theorem wp_pure_value (value : Result) (ensures : Result → State → Prop)
    (aborts : Error → Prop) (initial : State) :
    wp (Spec.pure value) ensures aborts initial ↔
      ∀ result, result = value → ensures result initial := by
  simp only [wp_pure]
  exact ⟨fun holds _ equal => equal ▸ holds, fun holds => holds value rfl⟩

end LeanerIR.Proofs
