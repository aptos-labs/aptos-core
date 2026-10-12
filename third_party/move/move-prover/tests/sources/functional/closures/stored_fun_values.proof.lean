-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- A frame can change a resource with a trivial invariant while preserving
-- the invariants of every other resource.
theorem stored_invariants_outside {unit : VerexIR.Validation.ValidatedUnit}
    {invariant : VerexIR.Proofs.Denote.ResourceType → VerexIR.RuntimeValue → Prop}
    {initial final : VerexIR.Proofs.Denote.Memory unit}
    {modified : VerexIR.Proofs.Denote.ResourceType}
    (frame : ∀ resource, resource ≠ modified → final resource = initial resource)
    (trivial : ∀ value, invariant modified value)
    (holds : VerexIR.Proofs.Denote.MemoryInvariants invariant initial) :
    VerexIR.Proofs.Denote.MemoryInvariants invariant final := by
  intro resource key value read
  by_cases same : resource = modified
  · subst resource
    exact trivial _
  · apply holds resource key value
    simpa only [frame resource same] using read

verify use_counter_modifier by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals first
  | verex_leaf
  | (apply stored_invariants_outside
     · first | assumption | exact And.right ‹_ ∧ _›
     · intro value
       simp only [vir_denote, vir_denote_norm, vir_denote_eval]
     · assumption)

verify use_config_aware_modifier by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals first
  | verex_leaf
  | (apply stored_invariants_outside
     · first | assumption | exact And.right ‹_ ∧ _›
     · intro value
       simp only [vir_denote, vir_denote_norm, vir_denote_eval]
     · assumption)
