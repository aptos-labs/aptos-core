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
      rw [consumed, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt m' i = SpecMap.keyAt m (i + 1) := by
         rw [consumed, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_neg (by omega)]
       rw [shift]
       first
       | simpa only [Int.add_assoc, Int.add_comm, Int.add_left_comm] using invariant_2 (i + 1) (by omega) (by omega)
       | (rw [consumed, SpecMap.valueAt_remove m_valid _ _ (by
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
    all_goals verex_lift_maps
    all_goals try simp only [invariant_4_1] at m'_eq
    -- The write hits a present key, so the map keeps its size.
    all_goals try
      have present : SpecMap.hasKey m (SpecMap.keyAt m count.val) = true :=
        SpecMap.hasKey_keyAt (by omega) (by omega)
      have sizes : SpecMap.size m' = SpecMap.size m := by
        rw [m'_eq (by omega), SpecMap.size_update m_valid, if_pos present]
  verex_on
  | invariant_6_step =>
    verex_paths
    all_goals (first
      | omega
      | (rw [m'_eq (by omega), SpecMap.keyAt_update_present m_valid present, SpecMap.valueAt_update]
         rcases Decidable.em (i = count.val) with same | same
         all_goals (first
           | (subst same; simp only [if_pos rfl, if_true])
           | (rw [if_neg (fun equal =>
                same ((SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal))]
              exact invariant_6 i (by omega) (by omega)))))
  | invariant_3_step =>
    verex_paths
    all_goals (first
      | omega
      | (rw [m'_eq (by omega), SpecMap.keyAt_update_present m_valid present]
         exact invariant_3 i (by omega) (by omega)))
  | _ =>
    verex_paths
    all_goals verex_intro_any
    all_goals try exact m'_valid (by omega)
    all_goals try omega
    all_goals try (apply invariant_3; omega; omega)
    all_goals try exact invariant_6 i (by omega) (by omega)
    all_goals try
      rw [if_pos (show SpecMap.hasKey m (SpecMap.keyAt m count.val) = true from SpecMap.hasKey_keyAt (by omega) (by omega))]

verify test_aborts_if_new_from_1 by
  verex_obligations
  all_goals (try verex_decide)
  all_goals simp_all [Maps.AbortsNewFrom, Maps.elementsOf]

verify test_verify_borrow_front_key by
  verex_obligations
  all_goals (try verex_decide)
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [vir_denote_norm, Maps.Valid, Maps.Ascending,
        compare, compareOfLessAndEq]
  all_goals verex_canonical_families
  all_goals simp_all [vir_denote_norm]

verify test_verify_borrow_back_key by
  verex_obligations
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [vir_denote_norm, Maps.Valid, Maps.Ascending,
        compare, compareOfLessAndEq]
  all_goals verex_canonical_families
  all_goals simp_all [vir_denote_norm]

-- The model uses Nodup; the Move contract states duplicate integer indices.

verify test_aborts_if_new_from_2 by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
  all_goals have duplicateLaw := integer_duplicate_indices 64 false keys
  all_goals simp only [asInt_getD_map_specInt 64 false (by decide)] at duplicateLaw
  all_goals simp only [Maps.AbortsNewFrom, Maps.elementsOf,
    Array.toList_map, List.length_map, Array.length_toList] at *
  all_goals grind only

verify test_verify_next_key by
  verex_obligations
  verex_group successor := assert_2, assert_3, assert_4, assert_5, assert_6, assert_7
  all_goals verex_prepare values
  all_goals try verex_cases next_key_result
  all_goals try verex_cases old_next_key_result
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_lift_maps
  all_goals simp_all [SpecMap.nextKey?, SpecMap.prevKey?, SpecMap.intOrder,
    SpecMap.hasKey, SpecMap.containsB, SpecMap.keyAt, SpecMap.rank, SpecMap.indexOf,
    variantName, variantPayload, variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
    NTy.encode_enum_payload, compare, compareOfLessAndEq, vir_denote_norm]

verify test_verify_prev_key by
  verex_obligations
  verex_group predecessor := assert_2, assert_3, assert_4, assert_5, assert_6
  all_goals verex_prepare values
  all_goals try verex_cases prev_key_result
  all_goals try verex_cases old_prev_key_result
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_lift_maps
  all_goals simp_all [SpecMap.nextKey?, SpecMap.prevKey?, SpecMap.intOrder,
    SpecMap.hasKey, SpecMap.containsB, SpecMap.keyAt, SpecMap.rank, SpecMap.indexOf,
    variantName, variantPayload, variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
    NTy.encode_enum_payload, compare, compareOfLessAndEq, vir_denote_norm]

verify test_verify_enumeration_view by
  verex_obligations
  all_goals (try verex_decide)
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [vir_denote_norm, Maps.Valid, Maps.Ascending,
        compare, compareOfLessAndEq]
  all_goals verex_canonical_families
  all_goals simp_all [vir_denote_norm]

verify test_verify_pop_rank by
  verex_obligations
  verex_group initial := assert_1, assert_2, assert_3, assert_7, assert_8, assert_9, assert_10
  verex_group front := assert_11, assert_12, assert_13, assert_14, assert_15, assert_16, assert_17
  verex_group back := assert_18, assert_19, assert_20, assert_21
  all_goals verex_prepare typed
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | simp_all [SpecMap.size, SpecMap.hasKey, SpecMap.containsB,
        SpecMap.valueAt, SpecMap.get, SpecMap.keyAt, SpecMap.rank, SpecMap.indexOf,
        SpecMap.remove, SpecMap.Discipline.del, SpecMap.ordDel, SpecMap.Valid,
        SpecMap.Ascending, SpecMap.intOrder, compare, compareOfLessAndEq, vir_denote_norm]

verify test_verify_upsert by
  verex_obligations
  verex_group inserted := assert_2, assert_3, assert_4, assert_5
  verex_group replaced := assert_6, assert_7, assert_8, assert_9
  verex_group preserved := assert_10, assert_11, assert_12
  verex_group removed := assert_14, assert_15, assert_16, assert_17, assert_18
  all_goals verex_prepare values
  all_goals try verex_cases upsert_result
  all_goals try verex_cases old_upsert_result
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_normalize_dependent
  all_goals try simp only [variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
    variantPayload, variantName, NTy.encode_enum_payload, vir_denote_norm] at *
  all_goals verex_lift_maps
  all_goals simp_all [SpecMap.size, SpecMap.hasKey, SpecMap.containsB,
    SpecMap.valueAt, SpecMap.get, SpecMap.update, SpecMap.Discipline.set, SpecMap.ordSet,
    SpecMap.remove, SpecMap.Discipline.del, SpecMap.ordDel, SpecMap.Valid,
    SpecMap.Ascending, SpecMap.intOrder, compare, compareOfLessAndEq]
  all_goals try verex_decide


verify test_verify_iter_remove_shift_at_position by
  verex_obligations
  verex_on
  | ensures_5 =>
    verex_prepare typed
    exact iter_remove_ensures_5 permitted_1 j (by omega) (by omega)
  | ensures_1 =>
    verex_prepare typed
    exact iter_remove_ensures_1 permitted_1
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
      rw [consumed, SpecMap.size_update valid, if_pos present]
    rw [consumed, SpecMap.keyAt_update_present valid present]
    all_goals first
    | rfl
    | (rw [SpecMap.valueAt_update, if_neg (fun equal => (by omega : ¬j = i.val)
        ((SpecMap.keyAt_eq_keyAt_iff valid (by omega) (by omega) (by omega) (by omega)).mp equal))])
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)


verify test_verify_iter_borrow_mut_symbolic by
  verex_obligations
  all_goals verex_prepare values
  all_goals try verex_cases internal_find_result
  all_goals verex_normalize_context
  all_goals verex_normalize_dependent
  all_goals verex_type_bounds
  all_goals verex_lift_maps
  all_goals try simp_all only [Bool.false_eq_true]
  all_goals try assumption
  all_goals try omega
  all_goals
    have present : SpecMap.hasKey m k.val = true := by assumption
    have high := SpecMap.rank_lt_size present
    have low := SpecMap.rank_nonneg m k.val
    first
    | omega
    | (try rw [consumed]
       try rw [internal_find_ensures_2]
       try rw [SpecMap.keyAt_rank present]
       first
       | rw [SpecMap.keyAt_update_present m_valid present]
       | (simp only [SpecMap.size_update m_valid, present, if_true])
       | (rw [SpecMap.valueAt_update]; simp only [if_true])
       | (rw [SpecMap.valueAt_update, if_neg (by omega)]))

verify test_verify_iter_add_middle_symbolic by
  verex_obligations
  verex_on
  | ensures_2 =>
    verex_prepare values
    have read := iter_add_ensures_2_2 (by assumption)
    verex_lift_maps only [read]
    exact read
  | ensures_4 =>
    verex_prepare values
    have read := iter_add_ensures_4 (by assumption) j (by verex_arithmetic_only) (by verex_arithmetic_only)
    verex_lift_maps only [read]
    exact read
  | ensures_5 =>
    verex_prepare values
    have read := iter_add_ensures_5 (by assumption) j (by verex_arithmetic_only) (by verex_arithmetic_only)
    verex_lift_maps only [read]
    exact read
  | aborts_if_1 =>
    verex_prepare typed
    omega
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)

verify test_verify_iter_add_append_symbolic by
  verex_obligations
  all_goals verex_prepare values
  all_goals try verex_cases internal_new_end_iter_result
  all_goals verex_normalize_context
  all_goals verex_normalize_dependent
  all_goals try simp_all only [Bool.false_eq_true]
  all_goals try assumption
  all_goals try omega
  all_goals verex_lift_maps
  all_goals first
    | assumption
    | omega
    | exact value_valid (by simp)
    | exact iter_add_ensures_1 (by simp)
    | exact iter_add_ensures_3 (by simp)
    | exact iter_add_ensures_4 (by simp) j (by verex_arithmetic_only) (by verex_arithmetic_only)
    | exact iter_add_ensures_2_2 (by simp)

verify test_verify_remove_or_none by
  verex_obligations
  verex_group initial := assert_2, assert_3, assert_4, assert_5
  verex_group present := assert_6, assert_7, assert_8, assert_9, assert_10, assert_11, assert_12
  verex_group absent := assert_13, assert_14, assert_15, assert_16, assert_17
  verex_group empty := assert_18, assert_19, assert_20, assert_21
  all_goals verex_prepare only
  all_goals try verex_cases remove_or_none_result
  all_goals try verex_cases old_remove_or_none_result
  all_goals verex_canonical_families
  -- Normalize the syntax once before rewriting by the map equations.
  all_goals try simp only [variantName, variantPayload, NTy.encode_enum_payload,
    variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem, vir_denote_norm] at *
  all_goals try simp_all [Maps.update, Maps.Discipline.set, Maps.ordSet, Maps.Valid,
    Maps.Ascending, compare, compareOfLessAndEq,
    Maps.remove, Maps.Discipline.del, Maps.ordDel, vir_denote_norm]
  all_goals try verex_decide

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
    simp_all only [Int.reduceLT, Int.reduceLE, and_self, true_and]
    try omega
