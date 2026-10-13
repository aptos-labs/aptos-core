-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "stored_fun_values.move"
verex proofs where

open VerexIR.Proofs.Denote

-- A frame can change a resource with a trivial invariant while preserving
-- the invariants of every other resource.
theorem stored_invariants_outside {unit : Validation.ValidatedUnit}
    {invariant : ResourceType → RuntimeValue → Prop}
    {initial final : Memory unit}
    {modified : ResourceType}
    (frame : ∀ resource, resource ≠ modified → final resource = initial resource)
    (trivial : ∀ value, invariant modified value)
    (holds : MemoryInvariants invariant initial) :
    MemoryInvariants invariant final := by
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
