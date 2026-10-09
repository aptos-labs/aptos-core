-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A frame can change a resource with a trivial invariant while preserving
-- the invariants of every other resource.
theorem stored_invariants_outside {unit : LeanerIR.Validation.ValidatedUnit}
    {invariant : LeanerIR.Proofs.Denote.ResourceType → LeanerIR.RuntimeValue → Prop}
    {initial final : LeanerIR.Proofs.Denote.Memory unit}
    {modified : LeanerIR.Proofs.Denote.ResourceType}
    (frame : ∀ resource, resource ≠ modified → final resource = initial resource)
    (trivial : ∀ value, invariant modified value)
    (holds : LeanerIR.Proofs.Denote.MemoryInvariants invariant initial) :
    LeanerIR.Proofs.Denote.MemoryInvariants invariant final := by
  intro resource key value read
  by_cases same : resource = modified
  · subst resource
    exact trivial _
  · apply holds resource key value
    simpa only [frame resource same] using read

verify use_counter_modifier by
  all_goals first
  | leaner_denote_leaf
  | (apply stored_invariants_outside
     · first | assumption | exact And.right ‹_ ∧ _›
     · intro value
       simp only [lir_denote, lir_denote_norm, lir_denote_eval]
     · assumption)

verify use_config_aware_modifier by
  all_goals first
  | leaner_denote_leaf
  | (apply stored_invariants_outside
     · first | assumption | exact And.right ‹_ ∧ _›
     · intro value
       simp only [lir_denote, lir_denote_norm, lir_denote_eval]
     · assumption)
