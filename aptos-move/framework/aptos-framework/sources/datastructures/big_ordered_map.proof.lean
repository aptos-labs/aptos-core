-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "big_ordered_map.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

verify test_verify_borrow_front_key by
  verex_obligations
  verex_on
  | assert_5 =>
    verex_prepare only
    have read := congrArg RuntimeValue.asInt (show RuntimeValue.integer borrow_front_result_0.val = keyAt new_from_result 0 from by assumption)
    simpa [new_from_result_eq, keyAt, entriesOf_restamp, entriesOf_build] using read
  | assert_4 =>
    verex_prepare only
    rw [show RuntimeValue.integer borrow_front_result_0.val = keyAt new_from_result 0 from by assumption]
    exact congrArg RuntimeValue.asInt (show RuntimeValue.integer borrow_front_result_1.val = valueAt new_from_result (keyAt new_from_result 0) from by assumption) |>.symm
  | assert_2 =>
    verex_prepare only
    exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
  | _ =>
    first
    | verex_decide
    | (verex_prepare only; all_goals (first | assumption | verex_residual_budgeted omega | skip))

verify test_verify_borrow_back_key by
  verex_obligations
  verex_on
  | assert_2 =>
    verex_prepare only
    exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
  | _ =>
    first
    | verex_decide
    | (verex_prepare only; all_goals simp_all [vir_denote_norm])

verify test_verify_remove by
  verex_obligations
  verex_on
  | assert_2 =>
    verex_prepare only
    exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
  | _ =>
    first
    | verex_decide
    | (verex_prepare only; all_goals simp_all [vir_denote_norm])

-- Append the current key, then advance the iterator rank by one.
verify test_verify_iter_collect_symbolic by
  verex_obligations
  verex_on
  | invariant_5_step, invariant_4_step, invariant_4_step_1, invariant_4_step_2, invariant_2_step, invariant_1_step, ensures_2 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
    verex_prepare only
    all_goals verex_cases it
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | omega
    | assumption
    | (have step := iter_next_ensures_7 (by assumption); omega)
    | (have step := iter_next_ensures_6 (by assumption); omega)
    | (apply iter_next_ensures_3; assumption)
    | (apply iter_next_ensures_1; assumption)
    | (have below := rank_lt_size (show hasKey m (.integer it_Some_2.val) = true from by assumption); omega)
    | (apply invariant_3; omega; omega)
    | skip
  | invariant_1_entry, invariant_4_entry, invariant_4_entry_1, invariant_4_entry_2, invariant_5_entry =>
    verex_prepare only
    all_goals verex_cases internal_new_begin_iter_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | assumption
    | omega
    | exact (internal_new_begin_iter_ensures_5 internal_new_begin_iter_ensures_2).symm
    | exact internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_2
    | (apply internal_new_begin_iter_ensures_3; split; verex_normalize_context; omega)
  | ensures_1 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals try omega
  | aborts_if_1 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
    verex_prepare only
    all_goals try verex_cases it
    all_goals verex_normalize_context
    all_goals try omega
  | invariant_3_step =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals
      have located := keyAt_rank (show hasKey m (.integer it_Some_2.val) = true from by assumption)
      have position : (out.values.size : Int) = rank m (.integer it_Some_2.val) := by assumption
      have read : iter_borrow_key_result.val = it_Some_2.val := by assumption
      have upper : i ≤ (out.values.size : Int) := by omega
      rcases Decidable.em (i = out.values.size) with last | last
      all_goals first
      | (subst i
         simp only [Int.toNat_natCast, Array.getElem?_push_size, Option.getD_some]
         rw [position]
         simpa only [asInt_integer, read] using congrArg RuntimeValue.asInt located.symm)
      | (simpa only [Array.getElem?_push, if_neg (show i.toNat ≠ out.values.size by omega)] using
          invariant_3 i (by omega) (by omega))
  | _ => try verex_decide

verify test_aborts_if_new_from_1 by
  verex_obligations
  all_goals verex_decide

verify test_aborts_if_remove by
  verex_obligations
  all_goals try verex_decide
  all_goals verex_prepare only
  all_goals simp_all only [vir_denote_norm]

verify test_aborts_if_new_from_2 by
  verex_obligations
  all_goals verex_prepare only
  all_goals have duplicateLaw := integer_duplicate_indices 64 false keys
  all_goals simp only [asInt_getD_map_specInt 64 false (by decide)] at duplicateLaw
  all_goals simp only [Maps.AbortsNewFrom, Maps.elementsOf,
    Array.toList_map, List.length_map, Array.length_toList] at *
  all_goals grind only

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

verify test_verify_enumeration_view by
  verex_obligations
  verex_group initial := assert_1, assert_2, assert_3, assert_7, assert_8, assert_9, assert_10
  verex_group observations := assert_11, assert_12, assert_13
  all_goals verex_prepare only
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
  verex_group back := assert_18, assert_19, assert_20, assert_21, assert_22
  all_goals verex_prepare only
  all_goals first
    | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
    | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
    | (try simp_all [vir_denote_norm, Maps.Valid, Maps.Ascending,
        compare, compareOfLessAndEq])
  all_goals verex_canonical_families
  all_goals try simp_all [vir_denote_norm]
  all_goals simp [Maps.rank, Maps.remove, Maps.ordDel, Maps.indexOf]

verify test_verify_iter_rank_loop by
  verex_obligations
  -- The loop invariants describe the current iterator; discard the initial facts.
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step,
    invariant_5_step, ensures_1, ensures_2, assert_1, assert_2, aborts_if_1 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
  | _ => skip
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | assumption
    | omega
    | (have step := iter_next_ensures_7 (by assumption); omega)
    | (have step := iter_next_ensures_6 (by assumption); omega)
    | (apply iter_next_ensures_3; assumption)
    | (apply iter_next_ensures_1; assumption)
    | skip
  | invariant_1_entry, invariant_2_entry, invariant_3_entry =>
    verex_prepare only
    all_goals verex_cases internal_new_begin_iter_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | assumption
    | omega
    | (apply internal_new_begin_iter_ensures_3; rfl)
    | (have first := internal_new_begin_iter_ensures_5 (by rfl); omega)
    | skip
  | assert_1, assert_2 =>
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals first
    | omega
    | (simp_all only [vir_denote_norm]; omega)
  | _ =>
    first
    | verex_decide
    | (verex_prepare only
       all_goals first | assumption | omega | simp_all only [vir_denote_norm])

verify test_verify_iter_sum_symbolic by
  verex_obligations
  -- The loop invariants describe the current iterator; discard the initial facts.
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step,
    invariant_5_step, ensures_1, ensures_2, assert_1, assert_2, aborts_if_1 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
  | _ => skip
  verex_group_contexts
  all_goals verex_prepare only
  all_goals first | verex_cases it | verex_cases internal_new_begin_iter_result | verex_cases internal_new_end_iter_result | skip
  all_goals try verex_cases iter_next_result
  all_goals try verex_cases iter_prev_result
  all_goals verex_normalize_context
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals repeat' apply And.intro
  all_goals verex_intro_any
  all_goals first
  | assumption
  | omega
  | (have step := iter_next_ensures_7 (by assumption); omega)
  | (have step := iter_next_ensures_6 (by assumption); omega)
  | (apply iter_next_ensures_3; assumption)
  | (apply iter_prev_ensures_3; assumption)
  | (apply iter_next_ensures_1; assumption)
  | (apply iter_prev_ensures_1; assumption)
  | (have step := iter_prev_ensures_7 (by assumption); omega)
  | (have step := iter_prev_ensures_6 (by assumption); omega)
  | (try simp_all only [vir_denote_norm])
  all_goals first
  | (rw [spec_test_sum_upto.spec.unfold]; simp; done)
  | (have low := rank_nonneg m (.integer it_Some_2.val)
     have located := keyAt_rank invariant_2_1
     have read := congrArg RuntimeValue.asInt iter_borrow_ensures_1
     conv => rhs; rw [spec_test_sum_upto.spec.unfold]
     dsimp only
     split
     all_goals first
     | omega
     | (try rw [← iter_next_ensures_7]
        simp only [Int.add_sub_cancel, located, asInt_integer]
        have value_eq : iter_borrow_result.val = (valueAt m (.integer it_Some_2.val)).asInt := by
          simpa only [asInt_integer, asInt_eraseGhosts] using read
        rw [value_eq]))

verify test_verify_iter_prev_loop_symbolic by
  verex_obligations preparing
    verex_prepare values
    all_goals first | verex_cases it | verex_cases internal_new_begin_iter_result | verex_cases internal_new_end_iter_result | skip
    all_goals try verex_cases iter_next_result
    all_goals try verex_cases iter_prev_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_type_bounds
    all_goals verex_lift_maps
  all_goals verex_paths
  all_goals repeat' apply And.intro
  all_goals verex_intro_any
  all_goals first
  | assumption
  | omega
  | (have step := iter_next_ensures_7 (by assumption); omega)
  | (have step := iter_next_ensures_6 (by assumption); omega)
  | (apply iter_next_ensures_3; assumption)
  | (apply iter_prev_ensures_3; assumption)
  | (apply iter_next_ensures_1; assumption)
  | (apply iter_prev_ensures_1; assumption)
  | (have step := iter_prev_ensures_7 (by assumption); omega)
  | (have step := iter_prev_ensures_6 (by assumption); omega)
  | (try simp_all only [vir_denote_norm])
  all_goals try omega
  all_goals try
    have lower := SpecMap.rank_nonneg m iter_prev_result_Some_2.val
    omega

verify test_verify_insert_shift_symbolic by
  verex_obligations
  verex_on
  | ensures_1, ensures_2, ensures_3, ensures_4, ensures_5 =>
    verex_prepare typed
    have missing : SpecMap.hasKey m k.val = false := by assumption
    have valid : SpecMap.Valid (.ordered SpecMap.intOrder) m := by assumption
    have equal : value = SpecMap.update (.ordered SpecMap.intOrder) m k.val v.val :=
      (show SpecMap.hasKey m k.val = false → _ from by assumption) missing
    simp only [equal] at *
    solve
    | simp only [SpecMap.hasKey_update, decide_true, Bool.true_or]
    | simp only [SpecMap.size_update valid, missing, Bool.false_eq_true, if_false]
    | rw [SpecMap.valueAt_update, if_pos rfl]
    | exact SpecMap.keyAt_update_before _ _ _ _ _ missing (by omega) (by omega)
    | exact SpecMap.keyAt_update_after _ _ _ _ _ missing (by omega)
  | _ => first | verex_decide | (verex_prepare only; all_goals first | assumption | verex_decide | grind only)

verify test_verify_keys_for_loop_symbolic by
  verex_obligations
  verex_on
  | invariant_3_entry =>
    verex_prepare values
    rw [spec_test_sum_upto.spec.unfold]
    simp
  | invariant_1_entry =>
    verex_prepare typed
    have keys : keys_result.values.toList.map SpecInt.val = m.map Prod.fst := by assumption
    simpa only [List.length_map, Array.length_toList] using SpecMap.size_of_keys keys
  | ensures_1 =>
    verex_prepare only
    have last : i.val = size m := by omega
    simpa only [last] using invariant_3
  | invariant_3_step =>
    verex_prepare values
    conv => rhs; rw [spec_test_sum_upto.spec.unfold]
    dsimp only
    rw [dif_neg (by omega)]
    verex_lift_maps
    obtain ⟨entry, readSlot, readValue⟩ : ∃ entry : SpecInt (.bits 64) false,
        keys_result.values[i.val.toNat]? = some entry ∧
          SpecMap.valueAt m entry.val = borrow_result.val := ⟨_, by assumption, consumed⟩
    have keys : keys_result.values.toList.map SpecInt.val = m.map Prod.fst := by assumption
    have located := SpecMap.keyAt_of_keys keys i.val (by omega)
    simp only [List.getElem?_map, Array.getElem?_toList, readSlot,
      Option.map_some, Option.getD_some] at located
    simpa only [Int.add_sub_cancel, ← invariant_3, located, readValue]
  | _ => first | verex_decide | (verex_prepare only; all_goals first | assumption | omega)

verify test_verify_iter_valid_after_insert_symbolic by
  verex_obligations
  verex_on
  | assert_1, assert_2, assert_3 =>
    verex_prepare only
    all_goals
      have missing : hasKey m (.integer k.val) = false := by assumption
      have length : size value = size m + 1 := by
        rw [value_eq, size_restamp, size_update _ m_valid, if_neg (by simp only [missing, Bool.false_eq_true, not_false_eq_true])]
      verex_cases internal_new_begin_iter_result
      all_goals first
      | omega
      | (simp only [vir_denote_norm] at *
         all_goals verex_canonical_families
         all_goals simp only [vir_denote_norm] at *
         all_goals first | assumption | omega)
  | ensures_1, ensures_2 =>
    verex_prepare only
    all_goals
      have missing : hasKey m (.integer k.val) = false := by assumption
      rw [value_eq]
      solve
      | simp only [hasKey_restamp, hasKey_update, decide_true, Bool.true_or]
      | simp only [size_restamp, size_update _ m_valid, missing, Bool.false_eq_true, if_false]
  | _ => first | verex_decide | (verex_prepare only; all_goals first | assumption | omega | grind only)

verify test_verify_keys_sorted by
  verex_obligations
  verex_group observations := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6
  all_goals verex_prepare typed
  all_goals try verex_decide
  all_goals
    have keys : keys_result.values.toList.map SpecInt.val = new_from_result.map Prod.fst := by assumption
    have length := SpecMap.size_of_keys keys
    have first := SpecMap.keyAt_of_keys keys 0 (by decide)
    have second := SpecMap.keyAt_of_keys keys 1 (by decide)
    have third := SpecMap.keyAt_of_keys keys 2 (by decide)
    simp [new_from_result_eq, SpecMap.keyAt, SpecMap.size] at length first second third
    try simp only [List.getElem?_map, Array.getElem?_toList] at first second third
    simp only [List.getElem?_map, Array.getElem?_toList,
      Option.getD_map SpecInt.val (⟨0, by decide⟩ : SpecInt (.bits 64) false)] at first second third
    simp [length, ← first, ← second, ← third, new_from_result_eq, SpecMap.hasKey, SpecMap.containsB]

verify test_verify_iter_across_upsert by
  verex_obligations
  all_goals verex_prepare only
  all_goals try
    have present : hasKey new_from_result (.integer 2) = true := by
      simp [new_from_result_eq, hasKey, containsB]
    obtain ⟨_, changed⟩ := (show _ ∨ _ from by assumption).resolve_right
      (by rintro ⟨missing, _⟩; simp only [present, Bool.true_eq_false] at missing)
    subst value
  all_goals try verex_cases internal_find_result
  all_goals try verex_cases upsert_result
  all_goals verex_normalize_context
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals try simp_all only [new_from_result_eq, vir_denote_norm, asInt_eraseGhosts]
  all_goals try omega

verify test_verify_iter_remove_shift_symbolic by
  verex_obligations
  all_goals verex_prepare values
  all_goals try verex_cases internal_find_with_path_result_0
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_lift_maps
  all_goals try simp_all only [vir_denote_norm]
  all_goals first
  | assumption
  | omega
  | exact iter_remove_ensures_1
  | (exact iter_remove_ensures_5 i (by omega) (by omega))
  | (exact iter_remove_ensures_6 i (by omega) (by omega))
  | (exact iter_remove_ensures_4_1 ⟨other, by constructor <;> omega⟩ (by assumption))
  | (have read := iter_remove_ensures_4_2 k ⟨other, by constructor <;> omega⟩ (by assumption) (by assumption)
      exact read)
  | skip

verify test_verify_iter_remove_symbolic by
  verex_obligations
  all_goals verex_prepare values
  all_goals try verex_cases internal_find_with_path_result_0
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_lift_maps
  all_goals try simp_all only [vir_denote_norm]
  all_goals first
  | assumption
  | omega
  | exact iter_remove_ensures_1
  | (exact iter_remove_ensures_5 i (by omega) (by omega))
  | (exact iter_remove_ensures_6 i (by omega) (by omega))
  | (exact iter_remove_ensures_4_1 ⟨other, by simp only [IntegerValueFits_unsigned_succ]; omega⟩ (by assumption))
  | (have read := iter_remove_ensures_4_2 k ⟨other, by simp only [IntegerValueFits_unsigned_succ]; omega⟩ (by assumption) (by assumption)
      exact read)
  | skip
  all_goals
    let otherValue : SpecInt (.bits 64) false := ⟨other, by simp only [IntegerValueFits_unsigned_succ]; omega⟩
    have read := iter_remove_ensures_4_2 k otherValue (by simpa only [otherValue] using (show other ≠ k.val from by assumption)) (by simpa only [otherValue] using (show SpecMap.hasKey m other = true from by assumption))
    exact read

verify test_verify_lower_bound_scan_start_symbolic by
  verex_obligations
  all_goals try verex_decide
  all_goals verex_prepare only
  all_goals try verex_split_variants
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals first
  | (constructor
     all_goals intro present
     all_goals have impossible := internal_lower_bound_ensures_2 k present
     all_goals omega)
  | (intro present
     have least := internal_lower_bound_ensures_5 k present (by omega)
     omega)
  | (have bound := rank_lt_size internal_lower_bound_ensures_3
     obtain ⟨key, encoded⟩ := encoded_keyAt m_keys_encoded (index := i) (by omega) (by omega)
     have present : hasKey m (.integer key.val) = true := by
       rw [show RuntimeValue.integer key.val = keyAt m i from encoded]
       exact hasKey_keyAt (by omega) (by omega)
     have ordered := order_keyAt_lt permitted (show 0 ≤ i from by omega)
       (show i < rank m (.integer x_Some_2.val) from by omega) bound
     rw [keyAt_rank internal_lower_bound_ensures_3, ← encoded, NTy.encode_int,
       RuntimeValue.order_integer] at ordered
     have earlier := Int.compare_eq_lt.mp ordered
     have keyRead := congrArg RuntimeValue.asInt encoded
     simp only [NTy.encode_int, asInt_integer] at keyRead
     apply Classical.byContradiction
     intro wrong
     have least := internal_lower_bound_ensures_5 key present (by omega)
     omega)

verify test_verify_bounded_walk_symbolic by
  verex_obligations
  -- The loop invariants describe the current iterator; discard the initial facts.
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step,
    invariant_5_step, ensures_1, ensures_2, assert_1, assert_2, aborts_if_1 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_new_begin_iter_ensures_1
    all_goals try clear internal_new_begin_iter_ensures_2
    all_goals try clear internal_new_begin_iter_ensures_3
    all_goals try clear internal_new_begin_iter_ensures_4
    all_goals try clear internal_new_begin_iter_ensures_5
  | _ => skip
  verex_on
  | invariant_5_step, invariant_4_step, invariant_4_step_1, invariant_4_step_2, invariant_2_step, invariant_1_step, ensures_2 =>
    verex_prepare only
    all_goals verex_cases it
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | omega
    | assumption
    | (have step := iter_next_ensures_7 (by assumption); omega)
    | (have step := iter_next_ensures_6 (by assumption); omega)
    | (apply iter_next_ensures_3; assumption)
    | (apply iter_next_ensures_1; assumption)
    | (have below := rank_lt_size (show hasKey m (.integer it_Some_2.val) = true from by assumption); omega)
    | (apply invariant_3; omega; omega)
    | skip
  | invariant_1_entry, invariant_4_entry, invariant_4_entry_1, invariant_4_entry_2, invariant_5_entry =>
    verex_prepare only
    all_goals verex_cases internal_new_begin_iter_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | assumption
    | omega
    | exact (internal_new_begin_iter_ensures_5 internal_new_begin_iter_ensures_2).symm
    | exact internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_2
    | (apply internal_new_begin_iter_ensures_3; split; verex_normalize_context; omega)
  | ensures_1 =>
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals try omega
  | aborts_if_1 =>
    verex_prepare only
    all_goals try verex_cases it
    all_goals verex_normalize_context
    all_goals try omega
  | invariant_3_step =>
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals
      have located := keyAt_rank (show hasKey m (.integer it_Some_2.val) = true from by assumption)
      have position : (out.values.size : Int) = rank m (.integer it_Some_2.val) := by omega
      have read : iter_borrow_key_result.val = it_Some_2.val := by assumption
      have upper : i ≤ (out.values.size : Int) := by omega
      rcases Decidable.em (i = out.values.size) with last | last
      all_goals first
      | (subst i
         simp only [Int.toNat_natCast, Array.getElem?_push_size, Option.getD_some]
         rw [position]
         simpa only [asInt_integer, read] using congrArg RuntimeValue.asInt located.symm)
      | (simpa only [Array.getElem?_push, if_neg (show i.toNat ≠ out.values.size by omega)] using
          invariant_3 i (by omega) (by omega))
  | _ => try verex_decide

verify test_verify_find_started_walk_symbolic by
  verex_obligations
  -- The loop invariants describe the current iterator; discard the initial facts.
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step,
    invariant_5_step, ensures_1, ensures_2, assert_1, assert_2, aborts_if_1 =>
    verex_paths
    all_goals verex_split_hypotheses
    all_goals try clear internal_find_ensures_1
    all_goals try clear internal_find_ensures_2
    all_goals try clear internal_find_ensures_3
    all_goals try clear internal_find_ensures_4
    all_goals try clear internal_find_ensures_5
  | _ => skip
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step =>
    verex_prepare only
    all_goals verex_cases it
    all_goals verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | assumption
    | omega
    | (have step := iter_next_ensures_7 (by assumption); omega)
    | (have step := iter_next_ensures_6 (by assumption); omega)
    | (apply iter_next_ensures_3; assumption)
    | (apply iter_next_ensures_1; assumption)
    | (have bound := rank_lt_size (show hasKey m (.integer it_Some_2.val) = true from by assumption); omega)
    | (constructor <;> first | assumption | omega | (have step := iter_next_ensures_6 (by assumption); omega))
    | skip
  | invariant_1_entry, invariant_2_entry, invariant_3_entry, invariant_4_entry =>
    verex_prepare only
    all_goals verex_cases internal_find_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | assumption
    | omega
    | (solve | rw [internal_find_ensures_3])
    | (rw [internal_find_ensures_3]; assumption)
    | (have present : hasKey m (.integer k.val) = true := by assumption
       exact False.elim (Bool.false_ne_true (internal_find_ensures_2.symm.trans present)))
    | (have bound := rank_lt_size (show hasKey m (.integer k.val) = true from by assumption); omega)
    | skip
  | ensures_1, aborts_if_1 =>
    verex_prepare only
    all_goals try verex_cases it
    all_goals verex_normalize_context
    all_goals first
    | omega
    | (have bound := rank_lt_size (show hasKey m (.integer it_Some_2.val) = true from by assumption)
       have low := rank_nonneg m (.integer k.val)
       omega)
  | _ =>
    first
    | verex_decide
    | (verex_prepare only; all_goals first | assumption | omega)

verify test_verify_early_exit_walk_symbolic by
  verex_obligations
  verex_on
  | invariant_4_entry, invariant_3_entry, invariant_1_entry =>
    verex_prepare only
    all_goals verex_cases internal_new_begin_iter_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first | assumption | omega
  | invariant_4_step, invariant_3_step, invariant_2_step, invariant_1_step =>
    verex_prepare only
    all_goals clear internal_new_begin_iter_ensures_1 internal_new_begin_iter_ensures_2 internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_4 internal_new_begin_iter_ensures_5
    all_goals verex_cases it
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals first
    | omega
    | assumption
    | (have step := iter_next_ensures_7 (by assumption); omega)
    | (have step := iter_next_ensures_6 (by assumption); omega)
    | (apply iter_next_ensures_3; assumption)
    | (apply iter_next_ensures_1; assumption)
    | (have below := rank_lt_size (show hasKey m (.integer it_Some_2.val) = true from by assumption); omega)
  | invariant_5_step =>
    verex_prepare only
    all_goals clear internal_new_begin_iter_ensures_1 internal_new_begin_iter_ensures_2 internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_4 internal_new_begin_iter_ensures_5
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals
      have located := keyAt_rank invariant_3_1
      rw [invariant_3_2] at located
      by_cases last : i = seen.val
      · rw [last, located]
        simp only [asInt_integer]
        have read := congrArg RuntimeValue.asInt iter_borrow_ensures_1
        simp only [asInt_integer, asInt_eraseGhosts] at read
        rw [← read]
        assumption
      · exact invariant_5 i (by omega) (by omega)
  | ensures_1 =>
    verex_prepare only
    all_goals clear internal_new_begin_iter_ensures_1 internal_new_begin_iter_ensures_2 internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_4 internal_new_begin_iter_ensures_5
    all_goals verex_cases it
    all_goals verex_normalize_context
    all_goals
      have checked :=  invariant_5 i (by omega) (by omega)
      simpa only [integer_asInt_keyAt m_keys (by omega) (by omega)] using checked
  | aborts_if_1 =>
    verex_prepare only
    all_goals try verex_cases it
    all_goals verex_normalize_context
    all_goals first
    | omega
    | (have below := rank_lt_size (show hasKey m (.integer it_Some_2.val) = true from by assumption); omega)
  | _ => verex_decide

verify test_verify_next_key_scan_symbolic by
  verex_obligations
  all_goals verex_prepare values
  all_goals try verex_cases nxt
  all_goals try verex_cases next_key_result
  all_goals verex_normalize_context
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_type_bounds
  all_goals verex_lift_maps
  all_goals try omega
  all_goals try assumption
  all_goals try
    have front_present : SpecMap.hasKey m front_key_result.val = true := by
      rw [← consumed]
      exact SpecMap.hasKey_keyAt (by omega) (by omega)
    have front_position : SpecMap.rank m front_key_result.val = 0 := by
      rw [← consumed]
      exact SpecMap.rank_keyAt m_valid (by omega) (by omega)
  all_goals try
    have step_present : SpecMap.hasKey m borrow_result.val = true := by
      rw [borrow_ensures_1]
      exact invariant_4_1
    have step_position : SpecMap.rank m borrow_result.val = count.val := by
      rw [borrow_ensures_1]
      exact invariant_4_2
    have step_high := SpecMap.rank_lt_size step_present
    first
    | omega
    | assumption
    | (have last := SpecMap.nextKey_none m_valid step_present (by assumption); omega)
    | (have next := SpecMap.nextKey_some (next := next_key_result_Some.val)
          m_valid step_present (by assumption)
       first | exact next.1 | omega)
  all_goals first
  | omega
  | assumption
  | (have last := SpecMap.nextKey_none m_valid front_present (by assumption); omega)
  | (have next := SpecMap.nextKey_some (next := next_key_result_Some.val)
        m_valid front_present (by assumption)
     first | exact next.1 | omega)

verify test_verify_iter_prev by
  verex_obligations
  verex_group assertions := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6, assert_7, assert_8
  case assertions =>
    verex_prepare only
    verex_cases internal_new_end_iter_result
    all_goals verex_cases old_iter_prev_result
    all_goals verex_cases iter_prev_result
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals
      have present : hasKey new_from_result (.integer old_iter_prev_result_Some_2.val) = true := by assumption
      have position : rank new_from_result (.integer old_iter_prev_result_Some_2.val) = size new_from_result - 1 := by assumption
      have located := keyAt_rank present
      rw [position] at located
      have size3 : size new_from_result = 3 := by simp [new_from_result_eq, vir_denote_norm]
      rw [size3] at located
      have key3 : keyAt new_from_result (3 - 1) = .integer 3 := by simp [new_from_result_eq, vir_denote_norm]
      rw [key3] at located
      have : old_iter_prev_result_Some_2.val = 3 := (RuntimeValue.integer.inj located).symm
      have present2 : hasKey new_from_result (.integer iter_prev_result_Some_2.val) = true := by assumption
      have shift : rank new_from_result (.integer iter_prev_result_Some_2.val) =
          rank new_from_result (.integer old_iter_prev_result_Some_2.val) - 1 :=
        (show hasKey new_from_result (.integer old_iter_prev_result_Some_2.val) = true → _ from by assumption) present
      have located2 := keyAt_rank present2
      rw [shift, position, size3] at located2
      have key2 : keyAt new_from_result (3 - 1 - 1) = .integer 2 := by simp [new_from_result_eq, vir_denote_norm]
      rw [key2] at located2
      have second : iter_prev_result_Some_2.val = 2 := (RuntimeValue.integer.inj located2).symm
      have read1 : old_iter_borrow_key_result.val = old_iter_prev_result_Some_2.val := by assumption
      have read2 : iter_borrow_key_result.val = iter_prev_result_Some_2.val := by assumption
      simp [read1, read2, this, second, new_from_result_eq, vir_denote_norm]
      exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, rfl⟩, rfl⟩
  all_goals first | verex_decide | (verex_prepare only; all_goals try verex_decide)

verify test_verify_iter_next by
  verex_obligations
  verex_group initial := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6, assert_7, assert_8, assert_9, assert_10
  verex_group assertions := assert_11, assert_12, assert_13, assert_14, assert_15, assert_16, assert_17
  case assertions =>
    verex_prepare only
    verex_cases internal_find_result
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_cases old_iter_next_result
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_cases iter_next_result_1
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_cases iter_next_result
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals try simp_all only [vir_denote_norm]
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals try omega
    all_goals simp only [variantName, Int.reduceAdd, true_and]
    all_goals simp only [beq_self_eq_true, true_and]
    all_goals omega
  case initial =>
    verex_prepare only
    simp [new_from_result_eq, vir_denote_norm]
    exact ⟨⟨⟨3, by decide⟩, ⟨2, by decide, by decide, rfl⟩, rfl⟩,
      ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, rfl⟩, rfl⟩,
      ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, rfl⟩, rfl⟩⟩
  -- These clauses need normalization; skip the decider that fails beforehand.
  verex_on
  | data_invariant, requires_of_iter_next, requires_of_internal_find,
    requires_of_remove_3, requires_of_ground_enum_123_1 =>
    verex_prepare only
    all_goals verex_decide
  | _ => verex_decide

verify test_verify_leaf_walk_sum_symbolic by
  verex_obligations
  -- Iterator preconditions need only the leaf's ordered slice of the parent.
  -- Prove them before splitting iterator results irrelevant to that argument.
  verex_on
  | requires_of_iter_next, requires_of_iter_borrow, requires_of_iter_is_end, requires_of_internal_new_begin_iter =>
    verex_prepare values
    all_goals
      have sliceSize := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5 (by assumption)
      have sliceKeys := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6 (by assumption)
      verex_lift_maps only [permitted, invariant_4_1, sliceSize, sliceKeys]
      apply SpecMap.valid_slice permitted invariant_4_1 sliceSize
      intro i low high
      exact (sliceKeys i low high).symm
  | _ => skip
  verex_group_contexts
  all_goals verex_paths
  all_goals try verex_cases it
  all_goals try verex_cases internal_leaf_new_begin_iter_result
  all_goals verex_prepare only
  all_goals try verex_cases internal_leaf_iter_borrow_entries_and_next_leaf_index_result_1
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals try verex_cases oit
  all_goals try verex_cases internal_new_begin_iter_result
  all_goals try verex_cases iter_next_result
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals verex_normalize_dependent
  all_goals try simp_all only [variantCarrier.ofSkolem, HList.ofSkolem,
    NTy.ofSkolem, variantName, variantPayload, vir_denote_norm]
  all_goals repeat' apply And.intro
  all_goals verex_intro_any
  all_goals try assumption
  all_goals try omega
  all_goals try (rw [spec_test_sum_upto.spec.unfold]; simp; done)
  all_goals try
    verex_lift_maps only [permitted, invariant_4_1,
      internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5,
      internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6]
    apply SpecMap.valid_slice permitted invariant_4_1
      internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5
    intro i low high
    exact (internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6 i low high).symm
  all_goals try
    verex_lift_maps only [internal_new_begin_iter_ensures_1]
    apply SpecMap.valid_of_keyAt_lt
    intro i j low before high
    simp only [internal_new_begin_iter_ensures_1] at high
    omega
  all_goals try simp only [Int.add_zero]
  all_goals try verex_cases iter_borrow_result
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals try cases internal_leaf_borrow_value_ensures_1
  all_goals verex_type_bounds
  all_goals
    have inside : oit_Position.val < size internal_leaf_iter_borrow_entries_and_next_leaf_index_result_0 := by assumption
    have low : 0 ≤ oit_Position.val := by omega
    have leaf := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_7 oit_Position.val low inside
    have read := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_8 oit_Position.val low inside
    have selected := congrArg (fun v => (VerexSL.Contract.selectVariantField v
      ⟨⟨0⟩, 1⟩ #[ ("Leaf", 0) ]).asInt) iter_borrow_ensures_1
    rw [VerexSL.Contract.asInt_select_eraseGhosts] at selected
    have global := congrArg RuntimeValue.asInt read
    simp only [asInt_eraseGhosts] at global
    have returned := internal_leaf_borrow_value_ensures_1
    have value := returned.trans (selected.trans global)
    try rw [show size internal_leaf_iter_borrow_entries_and_next_leaf_index_result_0 =
      oit_Position.val + 1 by omega]
    conv => rhs; rw [spec_test_sum_upto.spec.unfold]
    dsimp only
    split
    · omega
    · simp only [Int.add_sub_assoc, Int.sub_self, Int.add_zero]
      rw [integer_asInt_keyAt m_keys (by omega) (by omega), value]

verify test_verify_upsert by
  verex_obligations
  verex_group inserted := assert_1, assert_2, assert_3
  verex_group replaced := assert_4, assert_5, assert_6, assert_7, assert_8
  verex_group preserved := assert_9, assert_10, assert_11
  verex_group removed := assert_13, assert_14, assert_15, assert_16, assert_17
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


verify test_verify_drain_loop by
  verex_obligations
  verex_group initial := assert_1, assert_2, assert_3, assert_7, assert_8, assert_9, assert_10, assert_11, assert_12, assert_13
  verex_on
  | invariant_2_entry, invariant_3_entry =>
    verex_prepare only
    have indices : i = 0 ∨ i = 1 ∨ i = 2 := by omega
    rcases indices with rfl | rfl | rfl <;> simp_all [new_from_result_eq, vir_denote_norm]
  | assert_14 =>
    verex_prepare only
    simpa only [x_eq, Int.reduceSub, Int.reduceMul, Int.reduceTDiv] using invariant_4
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step =>
    verex_prepare only
    have present : hasKey map (keyAt map 0) = true := hasKey_keyAt (by omega) (by omega)
    have sizes : size value = size map - 1 := by
      simp only [value_eq, size_restamp, size_remove, present, if_true]
    first
    | omega
    | (
      have read := congrArg RuntimeValue.asInt consumed
      simp only [asInt_integer] at read
      have val := invariant_3 0 (by omega) (by omega)
      rw [integer_asInt_keyAt map_keys (by omega) (by omega)] at val
      have key := invariant_2 0 (by omega) (by omega)
      rw [read, val, key, invariant_4, sizes]
      rcases (show size map = 1 ∨ size map = 2 ∨ size map = 3 from by omega) with len | len | len <;> simp [len])
    | (
      have shift : keyAt value i = keyAt map (i + 1) := by
        rw [value_eq, keyAt_restamp, keyAt_remove_ordered]
        rw [rank_keyAt map_valid (by omega) (by omega)]
        simp only [if_neg (by omega : ¬i < 0)]
      rw [shift]
      first
      | (have previous := invariant_2 (i + 1) (by omega) (by omega); omega)
      | (
        have different : keyAt map (i + 1) ≠ keyAt map 0 := fun equal =>
          (by omega : ¬i + 1 = 0) ((keyAt_eq_keyAt_iff map_valid (by omega) (by omega) (by omega) (by omega)).mp equal)
        rw [value_eq, valueAt_restamp, valueAt_remove _ map_valid _ _ different]
        simpa only [integer_asInt_keyAt (index := i + 1) map_keys (by omega) (by omega)] using
          invariant_3 (i + 1) (by omega) (by omega)))
  | _ =>
    first | verex_decide | (
      verex_prepare only
      all_goals first
      | exact ⟨⟨1, by decide⟩, ⟨0, by decide, by decide, by decide, rfl⟩, rfl⟩
      | exact ⟨⟨2, by decide⟩, ⟨1, by decide, by decide, by decide, rfl⟩, rfl⟩
      | exact ⟨⟨3, by decide⟩, ⟨2, by decide, by decide, by decide, rfl⟩, rfl⟩
      | simp_all [vir_denote_norm]
      | omega)

verify test_verify_leaf_iter by
  verex_obligations
  verex_on
  | requires_of_borrow, requires_of_keys =>
    verex_prepare values
    verex_lift_maps only [new_from_result_valid, internal_leaf_new_begin_iter_ensures_3,
      internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5,
      internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6]
    apply SpecMap.valid_slice new_from_result_valid
      (by rw [internal_leaf_new_begin_iter_ensures_3]; omega)
      internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5
    intro i low high
    exact (internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6 i low high).symm
  | assert_2 =>
    verex_prepare only
    have related := congrArg RuntimeValue.asInt
      (internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_3_2 _ (by assumption))
    verex_cases borrow_result
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_normalize_dependent
    all_goals try cases internal_leaf_borrow_value_ensures_1
    all_goals try simp only [consumed, vir_denote_norm, asInt_eraseGhosts] at related
    all_goals try simpa only [internal_leaf_borrow_value_ensures_1] using related
  | _ => first | verex_decide | (verex_prepare only; all_goals verex_decide)

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

verify test_verify_remove_shift_symbolic by
  verex_obligations
  verex_on
  | ensures_1, ensures_2, ensures_3, ensures_4 =>
    verex_prepare typed
    have present : SpecMap.hasKey m k.val = true := by assumption
    have valid : SpecMap.Valid (.ordered SpecMap.intOrder) m := by assumption
    have equal : value = SpecMap.remove (.ordered SpecMap.intOrder) m k.val :=
      (show SpecMap.hasKey m k.val = true → _ from by assumption) present
    solve
    | simp only [equal, SpecMap.size_remove, present, if_true]
    | simp only [equal, SpecMap.hasKey_remove valid, decide_true, Bool.not_true, Bool.false_and]
    | simp only [equal, SpecMap.keyAt_remove_ordered]
    | (have key_present : SpecMap.hasKey value (SpecMap.keyAt value i) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
       have different : SpecMap.keyAt value i ≠ k.val := by
         intro same
         rw [same, equal, SpecMap.hasKey_remove valid] at key_present
         simp at key_present
       exact (congrArg (fun map => SpecMap.valueAt map (SpecMap.keyAt value i)) equal).trans
         (SpecMap.valueAt_remove valid _ _ different))
  | _ =>
    verex_prepare typed
    all_goals simp_all

verify test_verify_drain_symbolic by
  verex_obligations
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare typed
    have present : SpecMap.hasKey m (SpecMap.keyAt m 0) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have length : SpecMap.size value = SpecMap.size m - 1 := by
      rw [value_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt value i = SpecMap.keyAt m (i + 1) := by
         rw [value_eq, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_neg (by omega)]
       rw [shift]
       first
       | simpa only [Int.add_assoc, Int.add_comm, Int.add_left_comm] using invariant_2 (i + 1) (by omega) (by omega)
       | (rw [value_eq, SpecMap.valueAt_remove m_valid _ _ (by
            intro equal
            have := (SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal
            omega)]
          exact invariant_3 (i + 1) (by omega) (by omega)))
  | _ =>
    first
    | verex_decide
    | (verex_prepare typed; all_goals (first | assumption | omega))

verify test_verify_pop_back_drain_symbolic by
  verex_obligations
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare typed
    have present : SpecMap.hasKey m (SpecMap.keyAt m (SpecMap.size m - 1)) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have length : SpecMap.size value = SpecMap.size m - 1 := by
      rw [value_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt value i = SpecMap.keyAt m i := by
         rw [value_eq, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_pos (by omega)]
       rw [shift]
       first
       | exact invariant_2 i (by omega) (by omega)
       | (rw [value_eq, SpecMap.valueAt_remove m_valid _ _ (by
            intro equal
            have := (SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal
            omega)]
          exact invariant_3 i (by omega) (by omega)))
  | _ =>
    first
    | verex_decide
    | (verex_prepare typed; all_goals (first | assumption | omega))

verify test_verify_front_remove_drain_symbolic by
  verex_obligations
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare typed
    have front : front_key_result.val = SpecMap.keyAt m 0 := by
      symm
      assumption
    rw [front] at value_eq
    have present : SpecMap.hasKey m (SpecMap.keyAt m 0) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have length : SpecMap.size value = SpecMap.size m - 1 := by
      rw [value_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt value i = SpecMap.keyAt m (i + 1) := by
         rw [value_eq, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_neg (by omega)]
       rw [shift]
       first
       | simpa only [Int.add_assoc, Int.add_comm, Int.add_left_comm] using invariant_2 (i + 1) (by omega) (by omega)
       | (rw [value_eq, SpecMap.valueAt_remove m_valid _ _ (by
            intro equal
            have := (SpecMap.keyAt_eq_keyAt_iff m_valid (by omega) (by omega) (by omega) (by omega)).mp equal
            omega)]
          exact invariant_3 (i + 1) (by omega) (by omega)))
  | aborts_if_1 =>
    verex_prepare typed
    all_goals first
    | omega
    | (have present : SpecMap.hasKey m (SpecMap.keyAt m 0) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
       rw [show SpecMap.keyAt m 0 = front_key_result.val from by assumption] at present
       simp_all only [Bool.true_eq_false])
  | _ =>
    first
    | verex_decide
    | (verex_prepare typed; all_goals (first | assumption | omega | skip))

verify test_verify_back_key_rank_symbolic by
  verex_obligations
  verex_on
  | ensures_1, ensures_2 =>
    verex_prepare typed
    rw [← x (by omega)]
    first
    | exact SpecMap.hasKey_keyAt (by omega) (by omega)
    | exact SpecMap.rank_keyAt (by assumption) (by omega) (by omega)
  | _ => verex_decide
