-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "ordered_map.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- The loop preserves the map's enumeration by removing its first entry.
-- Use the map position laws directly on the prepared loop obligations.
verify test_verify_drain_symbolic by
  verex_obligations
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare typed
    have present : SpecMap.hasKey m (SpecMap.keyAt m 0) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have length : SpecMap.size m' = SpecMap.size m - 1 := by
      rw [m'_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt m' i = SpecMap.keyAt m (i + 1) := by
         rw [m'_eq, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_neg (by omega)]
       rw [shift]
       first
       | simpa only [Int.add_assoc, Int.add_comm, Int.add_left_comm] using invariant_2 (i + 1) (by omega) (by omega)
       | (rw [m'_eq, SpecMap.valueAt_remove m_valid _ _ (by
            intro equal
            have := (SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal
            omega)]
          exact invariant_3 (i + 1) (by omega) (by omega)))
  | _ =>
    first
    | verex_decide
    | (verex_prepare typed; all_goals (first | assumption | omega))

-- The walk writes each value in place: the write hits the key at the walk's
-- position, so the keys and the size are unchanged, and the invariant over
-- the earlier positions carries over.
verify test_verify_iter_walk_mut_symbolic by
  -- Share iterator cases and map facts before the clauses split.
  verex_obligations preparing
    verex_prepare values
    all_goals first | verex_cases it | verex_cases internal_new_begin_iter_result | skip
    all_goals try verex_cases iter_next_result
    all_goals try simp only [vir_denote_norm] at *
    all_goals verex_type_bounds
    all_goals verex_name_maps
    all_goals try simp only [invariant_4_1] at m'_eq
    -- The write hits a present key, so the map keeps its size.
    all_goals try
      have present : SpecMap.hasKey m (SpecMap.keyAt m count.val) = true :=
        SpecMap.hasKey_keyAt (by omega) (by omega)
      have sizes : SpecMap.size m' = SpecMap.size m := by
        rw [m'_eq, SpecMap.size_update m_valid, if_pos present]
  verex_on
  | invariant_6_step =>
    verex_paths
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals (first
      | omega
      | (rw [m'_eq, SpecMap.keyAt_update_present m_valid present, SpecMap.valueAt_update]
         rcases Decidable.em (i = count.val) with same | same
         all_goals (first
           | (subst same; simp only [if_true])
           | (rw [if_neg (fun equal =>
                same ((SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal))]
              exact invariant_6 i (by omega) (by omega)))))
  | invariant_3_step =>
    verex_paths
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals (first
      | omega
      | (rw [m'_eq, SpecMap.keyAt_update_present m_valid present]
         exact invariant_3 i (by omega) (by omega)))
  | _ =>
    verex_paths
    all_goals verex_intro_any
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals try verex_decide
    all_goals try exact m'_valid
    all_goals try omega
    all_goals try (apply invariant_3; omega; omega)
    all_goals try exact invariant_6 i (by omega) (by omega)
    all_goals try
      rw [if_pos (show SpecMap.hasKey m (SpecMap.keyAt m count.val) = true from SpecMap.hasKey_keyAt (by omega) (by omega))]

-- The model uses Nodup; the Move contract states duplicate integer indices.

verify test_aborts_if_new_from_2 by
  verex_obligations
  all_goals verex_prepare typed
  all_goals simp only [SpecMap.AbortsNewFrom, duplicate_indices_map, List.length_map,
    Array.length_toList] at *
  all_goals first | omega | grind only

verify test_verify_iter_remove_shift_at_position by
  verex_obligations
  verex_on
  | ensures_5 =>
    verex_prepare typed
    all_goals exact iter_remove_ensures_5 j (by omega) (by omega)
  | ensures_1 =>
    verex_prepare typed
    all_goals exact iter_remove_ensures_1
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)

verify test_verify_iter_borrow_mut_at_position by
  verex_obligations
  verex_on
  | ensures_2, ensures_3 =>
    verex_prepare typed
    have present : SpecMap.hasKey m (SpecMap.keyAt m i.val) = true :=
      SpecMap.hasKey_keyAt (by omega) (by omega)
    have valid : SpecMap.Valid (.ordered SpecMap.intOrder) m := by assumption
    have sizes : SpecMap.size m' = SpecMap.size m := by
      rw [m'_eq, SpecMap.size_update valid, if_pos present]
    rw [m'_eq, SpecMap.keyAt_update_present valid present]
    all_goals first
    | rfl
    | (rw [SpecMap.valueAt_update, if_neg (fun equal => (by omega : ¬j = i.val)
        ((SpecMap.keyAt_eq_keyAt_iff valid (by omega) (by omega) (by omega) (by omega)).mp equal))])
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)

verify test_verify_iter_borrow_mut_symbolic by
  verex_obligations
  all_goals verex_prepare typed
  all_goals try verex_cases internal_find_result
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  all_goals try simp_all only [Bool.false_eq_true]
  all_goals try omega
  all_goals
    have present : SpecMap.hasKey m k.val = true := by assumption
    have high := SpecMap.rank_lt_size present
    have low := SpecMap.rank_nonneg m k.val
    try rw [internal_find_ensures_2]
    try rw [SpecMap.keyAt_rank present]
    first
    | omega
    | rfl
    | rw [if_pos present]
    | simp only [SpecMap.size_update m_valid, present, if_true]
    | rw [SpecMap.valueAt_update, if_pos rfl]
    | rw [SpecMap.keyAt_update_present m_valid present]
    | rw [SpecMap.valueAt_update, if_neg (by omega)]

verify test_verify_iter_add_middle_symbolic by
  verex_obligations
  verex_on
  | ensures_2 =>
    verex_prepare typed
    all_goals exact iter_add_ensures_2_2
  | ensures_4 =>
    verex_prepare typed
    all_goals exact iter_add_ensures_4 j (by omega) (by omega)
  | ensures_5 =>
    verex_prepare typed
    all_goals exact iter_add_ensures_5 j (by omega) (by omega)
  | aborts_if_1 =>
    verex_prepare typed
    omega
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)

verify test_verify_iter_add_append_symbolic by
  verex_obligations
  all_goals verex_prepare typed
  all_goals try verex_cases internal_new_end_iter_result
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  all_goals try assumption
  all_goals try omega
  all_goals first
    | assumption
    | omega
    | exact value_valid
    | exact iter_add_ensures_1
    | exact iter_add_ensures_3
    | exact iter_add_ensures_4 j (by omega) (by omega)
    | exact iter_add_ensures_2_2

-- Typed map arguments; encoding is handled by preparation.

verify ground_enum_123 by
  verex_obligations
  verex_group facts := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6, assert_7, assert_8, assert_9, assert_10, assert_11
  all_goals verex_prepare typed
  all_goals
    have h1 : SpecMap.hasKey map 1 = true := by assumption
    have h2 : SpecMap.hasKey map 2 = true := by assumption
    have h3 : SpecMap.hasKey map 3 = true := by assumption
    have first := SpecMap.rank_nonneg map 1
    have last := SpecMap.rank_lt_size h3
    have lt12 := SpecMap.rank_lt_rank (by assumption) h1 h2 (by decide)
    have lt23 := SpecMap.rank_lt_rank (by assumption) h2 h3 (by decide)
    have positions : SpecMap.rank map 1 = 0 ∧ SpecMap.rank map 2 = 1 ∧
        SpecMap.rank map 3 = 2 := by omega
    have key1 := SpecMap.keyAt_rank h1
    have key2 := SpecMap.keyAt_rank h2
    have key3 := SpecMap.keyAt_rank h3
    simp_all only [Int.reduceLT, Int.reduceLE, and_self]
    try omega
