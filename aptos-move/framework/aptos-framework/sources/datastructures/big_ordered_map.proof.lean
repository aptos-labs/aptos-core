-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "big_ordered_map.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- Append the current key, then advance the iterator rank by one.

verify test_verify_iter_collect_symbolic by
  verex_obligations preparing
    verex_prepare only
    all_goals first
    | (verex_cases it
       all_goals try clear internal_new_begin_iter_ensures_1
       all_goals try clear internal_new_begin_iter_ensures_2
       all_goals try clear internal_new_begin_iter_ensures_3
       all_goals try clear internal_new_begin_iter_ensures_4
       all_goals try clear internal_new_begin_iter_ensures_5)
    | verex_cases internal_new_begin_iter_result
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_type_bounds
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_finish_prepare
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  verex_on
  | invariant_5_step, invariant_4_step, invariant_4_step_1, invariant_4_step_2, invariant_2_step, invariant_1_step, ensures_2 =>
    all_goals first
    | omega
    | verex_assumption
    | (have step := iter_next_ensures_7 (by verex_assumption); omega)
    | (have step := iter_next_ensures_6 (by verex_assumption); omega)
    | (apply iter_next_ensures_3; verex_assumption)
    | (apply iter_next_ensures_1; verex_assumption)
    | (have below := SpecMap.rank_lt_size (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption); omega)
    | (apply invariant_3; omega; omega)
  | invariant_1_entry, invariant_4_entry, invariant_4_entry_1, invariant_4_entry_2, invariant_5_entry =>
    all_goals first
    | verex_assumption
    | omega
    | exact (internal_new_begin_iter_ensures_5 internal_new_begin_iter_ensures_2).symm
    | exact internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_2
    | (apply internal_new_begin_iter_ensures_3; split; verex_normalize_context; omega)
  | ensures_1 =>
    all_goals try omega
  | aborts_if_1 =>
    all_goals try omega
  | invariant_3_step =>
    all_goals
      have located := SpecMap.keyAt_rank (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption)
      have position : (out.values.size : Int) = SpecMap.rank m it_Some_2.val := by
        first | verex_assumption | (symm; verex_assumption) | omega
      have read : iter_borrow_key_result.val = it_Some_2.val := by verex_assumption
      have upper : i ≤ (out.values.size : Int) := by omega
      rcases Decidable.em (i = out.values.size) with last | last
      all_goals first
      | (subst i
         simp only [Int.toNat_natCast, Array.getElem?_push_size, Option.getD_some]
         rw [position, located, read])
      | (simpa only [Array.getElem?_push, if_neg (show i.toNat ≠ out.values.size by omega)] using
          invariant_3 i (by omega) (by omega))
  | _ => try verex_decide

verify test_aborts_if_new_from_1 by
  verex_obligations
  all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))

verify test_aborts_if_remove by
  verex_obligations
  all_goals try verex_decide
  all_goals verex_prepare only
  all_goals simp_all only [vir_denote_norm]

verify test_aborts_if_new_from_2 by
  verex_obligations
  all_goals verex_prepare typed
  all_goals simp only [SpecMap.AbortsNewFrom, duplicate_indices_map, List.length_map,
    Array.length_toList] at *
  all_goals first | omega | grind only

verify test_verify_prev_key by
  verex_obligations
  verex_group predecessor := assert_2, assert_3, assert_4, assert_5, assert_6
  all_goals verex_prepare typed
  all_goals try verex_cases prev_key_result
  all_goals try verex_cases old_prev_key_result
  all_goals try subst new_from_result_eq
  all_goals first
    | (simp only [variantName, variantPayload,
        vir_denote_norm, vir_denote_eval] at *; done)
    | (simp_all [variantName, variantPayload, variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
        vir_denote_norm, vir_denote_eval]; done)

verify test_verify_next_key by
  verex_obligations
  verex_group successor := assert_2, assert_3, assert_4, assert_5, assert_6, assert_7
  all_goals verex_prepare typed
  all_goals try verex_cases next_key_result
  all_goals try verex_cases old_next_key_result
  all_goals try subst new_from_result_eq
  all_goals first
    | (simp only [variantName, variantPayload, variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
        vir_denote_norm, vir_denote_eval] at *; done)
    | (simp_all [variantName, variantPayload, variantCarrier.ofSkolem, HList.ofSkolem, NTy.ofSkolem,
        vir_denote_norm, vir_denote_eval]; done)

verify test_verify_iter_rank_loop by
  verex_obligations preparing
    verex_prepare only
    all_goals first
    | (verex_cases it
       all_goals try clear internal_new_begin_iter_ensures_1
       all_goals try clear internal_new_begin_iter_ensures_2
       all_goals try clear internal_new_begin_iter_ensures_3
       all_goals try clear internal_new_begin_iter_ensures_4
       all_goals try clear internal_new_begin_iter_ensures_5)
    | verex_cases internal_new_begin_iter_result
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_type_bounds
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_finish_prepare
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    all_goals first
    | verex_assumption
    | omega
    | (have step := iter_next_ensures_7 (by verex_assumption); omega)
    | (have step := iter_next_ensures_6 (by verex_assumption); omega)
    | (apply iter_next_ensures_3; verex_assumption)
    | (apply iter_next_ensures_1; verex_assumption)
  | invariant_1_entry, invariant_2_entry, invariant_3_entry =>
    all_goals first
    | verex_assumption
    | omega
    | (apply internal_new_begin_iter_ensures_3; rfl)
    | (have first := internal_new_begin_iter_ensures_5 (by rfl); omega)
  | assert_1, assert_2 =>
    all_goals first
    | omega
    | (simp_all only [vir_denote_norm]; omega)
  | _ =>
    first
    | verex_decide
    | (verex_prepare only
       all_goals first | verex_assumption | omega | simp_all only [vir_denote_norm])

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
    all_goals verex_name_maps
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
    subst m'_eq
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
    rfl
  | ensures_1 =>
    verex_prepare typed
    have last : i.val = SpecMap.size m := by omega
    simpa only [last] using invariant_3
  | invariant_3_step =>
    verex_prepare typed
    conv => rhs; rw [spec_test_sum_upto.spec.unfold]
    dsimp only
    try verex_name_maps
    rw [dif_neg (by omega)]
    -- The key the loop reads, the map's at its position, fits its carrier.
    have bound : i.val.toNat < (SpecMap.keys m).length := by
      simp only [SpecMap.keys, List.length_map]; omega
    have located := SpecMap.keyAt_of_keys (map := m) (keys := SpecMap.keys m) rfl i.val (by omega)
    rw [List.getElem?_eq_getElem bound, Option.getD_some] at located
    have fits := SpecMap.keyAt_satisfies_all m_keys
      (SpecMap.IntegerValueFits.zero_of (⟨0, by decide⟩ : SpecInt (.bits 64) false)) i.val
    rw [located] at fits
    simp only [SpecInt.val_ofInt _ fits, Int.add_sub_cancel, located] at *
    first | (simp only [*]; done) | omega
  | _ => first | verex_decide | (verex_prepare only; all_goals first | assumption | omega)

verify test_verify_iter_valid_after_insert_symbolic by
  verex_obligations
  verex_on
  | assert_1, assert_2, assert_3 =>
    verex_prepare typed
    all_goals
      have missing : SpecMap.hasKey m k.val = false := by assumption
      have length : SpecMap.size m' = SpecMap.size m + 1 := by
        rw [m'_eq, SpecMap.size_update m_valid, if_neg (by simp only [missing, Bool.false_eq_true, not_false_eq_true])]
      verex_cases internal_new_begin_iter_result
      all_goals first
      | omega
      | (simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
         all_goals first | assumption | omega)
  | ensures_1, ensures_2 =>
    verex_prepare typed
    all_goals
      have missing : SpecMap.hasKey m k.val = false := by assumption
      rw [m'_eq]
      solve
      | simp only [SpecMap.hasKey_update, decide_true, Bool.true_or]
      | simp only [SpecMap.size_update m_valid, missing, Bool.false_eq_true, if_false]
  | _ => first | verex_decide | (verex_prepare only; all_goals first | assumption | omega | grind only)

verify test_verify_keys_sorted by
  verex_obligations
  verex_group observations := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6
  all_goals verex_prepare typed
  all_goals try verex_decide
  all_goals try subst new_from_result_eq
  all_goals try simp only [vir_denote_norm, vir_denote_eval] at *
  all_goals try subst_vars
  all_goals first | decide | (simp only [vir_denote_norm, vir_denote_eval]; done)

verify test_verify_iter_across_upsert by
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_cases internal_find_result
  all_goals try verex_cases upsert_result
  all_goals verex_normalize_context
  all_goals verex_canonical_families
  all_goals verex_normalize_context
  all_goals try simp_all only [vir_denote_norm]
  all_goals try omega

verify test_verify_iter_remove_shift_symbolic by
  verex_obligations preparing
    verex_prepare values
    all_goals try verex_cases internal_find_with_path_result_0
    all_goals verex_canonical_families
    all_goals verex_normalize_context
  -- Link the returned state before naming its maps. The clauses introduce
  -- the equality between the callee's output and the caller's final state.
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_name_maps
  all_goals try simp_all only [vir_denote_norm]
  all_goals verex_finish_prepare
  all_goals first
  | assumption
  | omega
  | exact iter_remove_ensures_1
  | (exact iter_remove_ensures_5 i (by omega) (by omega))
  | (exact iter_remove_ensures_6 i (by omega) (by omega))
  | skip

verify test_verify_iter_remove_symbolic by
  verex_obligations preparing
    verex_prepare values
    all_goals try verex_cases internal_find_with_path_result_0
    all_goals verex_canonical_families
    all_goals verex_normalize_context
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_name_maps
  all_goals try simp_all only [vir_denote_norm]
  all_goals verex_finish_prepare
  all_goals first
  | assumption
  | omega
  | exact iter_remove_ensures_1
  | (exact iter_remove_ensures_5 i (by omega) (by omega))
  | (exact iter_remove_ensures_6 i (by omega) (by omega))
  | (exact iter_remove_ensures_4_1 other (by omega) (by omega) (by assumption))
  | exact iter_remove_ensures_4_2 0 (by omega) (by omega) other (by omega) (by omega)
      (by assumption) (by assumption)

verify test_verify_lower_bound_scan_start_symbolic by
  verex_obligations
  verex_on
  | ensures_1, ensures_2 =>
    verex_prepare typed
    all_goals try verex_split_variants
    all_goals simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals first
    | omega
    | (constructor
       all_goals intro present
       all_goals have impossible := internal_lower_bound_ensures_2 k.val (by omega) (by omega) present
       all_goals omega)
    | (intro present
       have least := internal_lower_bound_ensures_5 k.val (by omega) (by omega) present (by omega)
       omega)
    | skip
    all_goals
      have bound := SpecMap.rank_lt_size internal_lower_bound_ensures_3
      have valid : SpecMap.Valid (.ordered SpecMap.intOrder) m := by assumption
      have present : SpecMap.hasKey m (SpecMap.keyAt m i) = true :=
        SpecMap.hasKey_keyAt (by omega) (by omega)
      have ordered := SpecMap.keyAt_lt_of_lt valid i (SpecMap.rank m x_Some_2.val)
        (by omega) (by omega) bound
      rw [SpecMap.keyAt_rank internal_lower_bound_ensures_3] at ordered
      apply Classical.byContradiction
      intro wrong
      have fits := SpecMap.keyAt_satisfies m_keys (index := i) (by omega) (by omega)
      simp only [IntegerValueFits_unsigned_succ] at fits
      have least := internal_lower_bound_ensures_5 (SpecMap.keyAt m i) (by omega) (by omega) present
        (show k.val ≤ SpecMap.keyAt m i by omega)
      omega
  | _ => first | verex_decide | (verex_prepare typed; all_goals first | verex_decide)

verify test_verify_bounded_walk_symbolic by
  verex_obligations preparing
    verex_prepare only
    all_goals first
    | (verex_cases it
       all_goals try clear internal_new_begin_iter_ensures_1
       all_goals try clear internal_new_begin_iter_ensures_2
       all_goals try clear internal_new_begin_iter_ensures_3
       all_goals try clear internal_new_begin_iter_ensures_4
       all_goals try clear internal_new_begin_iter_ensures_5)
    | verex_cases internal_new_begin_iter_result
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_type_bounds
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_finish_prepare
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  verex_on
  | invariant_5_step, invariant_4_step, invariant_4_step_1, invariant_4_step_2, invariant_2_step, invariant_1_step, ensures_2 =>
    all_goals first
    | omega
    | verex_assumption
    | (have step := iter_next_ensures_7 (by verex_assumption); omega)
    | (have step := iter_next_ensures_6 (by verex_assumption); omega)
    | (apply iter_next_ensures_3; verex_assumption)
    | (apply iter_next_ensures_1; verex_assumption)
    | (have below := SpecMap.rank_lt_size (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption); omega)
    | (apply invariant_3; omega; omega)
  | invariant_1_entry, invariant_4_entry, invariant_4_entry_1, invariant_4_entry_2, invariant_5_entry =>
    all_goals first
    | verex_assumption
    | omega
    | exact (internal_new_begin_iter_ensures_5 internal_new_begin_iter_ensures_2).symm
    | exact internal_new_begin_iter_ensures_3 internal_new_begin_iter_ensures_2
    | (apply internal_new_begin_iter_ensures_3; split; verex_normalize_context; omega)
  | ensures_1 =>
    all_goals try omega
  | aborts_if_1 =>
    all_goals try omega
  | invariant_3_step =>
    all_goals
      have located := SpecMap.keyAt_rank (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption)
      have position : (out.values.size : Int) = SpecMap.rank m it_Some_2.val := by
        first | verex_assumption | (symm; verex_assumption) | omega
      have read : iter_borrow_key_result.val = it_Some_2.val := by verex_assumption
      have upper : i ≤ (out.values.size : Int) := by omega
      rcases Decidable.em (i = out.values.size) with last | last
      all_goals first
      | (subst i
         simp only [Int.toNat_natCast, Array.getElem?_push_size, Option.getD_some]
         rw [position, located, read])
      | (simpa only [Array.getElem?_push, if_neg (show i.toNat ≠ out.values.size by omega)] using
          invariant_3 i (by omega) (by omega))
  | _ => try verex_decide

verify test_verify_find_started_walk_symbolic by
  verex_obligations preparing
    verex_prepare only
    all_goals first
    | (verex_cases it
       all_goals try clear internal_find_ensures_1
       all_goals try clear internal_find_ensures_2
       all_goals try clear internal_find_ensures_3
       all_goals try clear internal_find_ensures_4
       all_goals try clear internal_find_ensures_5)
    | verex_cases internal_find_result
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_type_bounds
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_finish_prepare
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step =>
    all_goals first
    | verex_assumption
    | omega
    | (have step := iter_next_ensures_7 (by verex_assumption); omega)
    | (have step := iter_next_ensures_6 (by verex_assumption); omega)
    | (apply iter_next_ensures_3; verex_assumption)
    | (apply iter_next_ensures_1; verex_assumption)
    | (have bound := SpecMap.rank_lt_size (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption); omega)
    | (constructor <;> first | verex_assumption | omega | (have step := iter_next_ensures_6 (by verex_assumption); omega))
  | invariant_1_entry, invariant_2_entry, invariant_3_entry, invariant_4_entry =>
    all_goals first
    | verex_assumption
    | omega
    | (solve | rw [internal_find_ensures_3])
    | (rw [internal_find_ensures_3]; verex_assumption)
    | (have present : SpecMap.hasKey m k.val = true := by verex_assumption
       exact False.elim (Bool.false_ne_true (internal_find_ensures_2.symm.trans present)))
    | (have bound := SpecMap.rank_lt_size (show SpecMap.hasKey m k.val = true from by verex_assumption); omega)
  | ensures_1, aborts_if_1 =>
    all_goals first
    | omega
    | (have bound := SpecMap.rank_lt_size (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption)
       have low := SpecMap.rank_nonneg m k.val
       omega)
  | _ =>
    first
    | verex_decide
    | (verex_prepare only; all_goals first | verex_assumption | omega)

verify test_verify_early_exit_walk_symbolic by
  verex_obligations preparing
    verex_prepare only
    all_goals first
    | (verex_cases it
       all_goals try clear internal_new_begin_iter_ensures_1
       all_goals try clear internal_new_begin_iter_ensures_2
       all_goals try clear internal_new_begin_iter_ensures_3
       all_goals try clear internal_new_begin_iter_ensures_4
       all_goals try clear internal_new_begin_iter_ensures_5)
    | verex_cases internal_new_begin_iter_result
    all_goals try verex_cases iter_next_result
    all_goals verex_normalize_context
    all_goals verex_canonical_families
    all_goals verex_normalize_context
    all_goals verex_type_bounds
  all_goals verex_paths
  all_goals verex_intro_any
  all_goals verex_normalize_context
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals verex_reduce_projections
  all_goals verex_finish_prepare
  all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
  verex_on
  | invariant_4_entry, invariant_3_entry, invariant_1_entry =>
    all_goals first | verex_assumption | omega
  | invariant_4_step, invariant_3_step, invariant_2_step, invariant_1_step =>
    all_goals first
    | omega
    | verex_assumption
    | (have step := iter_next_ensures_7 (by verex_assumption); omega)
    | (have step := iter_next_ensures_6 (by verex_assumption); omega)
    | (apply iter_next_ensures_3; verex_assumption)
    | (apply iter_next_ensures_1; verex_assumption)
    | (have below := SpecMap.rank_lt_size (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption); omega)
  | invariant_5_step =>
    all_goals
      have located := SpecMap.keyAt_rank invariant_3_1
      rw [invariant_3_2] at located
      by_cases last : i = seen.val
      · first
        | (rw [last, located]
           have read : iter_borrow_result.val = SpecMap.valueAt m it_Some_2.val := by verex_assumption
           rw [← read]
           verex_assumption)
      · first
        | exact invariant_5 i (by omega) (by omega)
  | ensures_1 =>
    all_goals
      exact invariant_5 i (by omega) (by omega)
  | aborts_if_1 =>
    all_goals first
    | omega
    | (have below := SpecMap.rank_lt_size (show SpecMap.hasKey m it_Some_2.val = true from by verex_assumption); omega)
  | _ => verex_decide

verify test_verify_next_key_scan_symbolic by
  verex_obligations
  all_goals verex_prepare typed
  all_goals try verex_decide
  all_goals try verex_cases nxt
  -- The neighbours the contracts of `next_key` read, case by case.
  all_goals try
    generalize next_front : SpecMap.nextKey? SpecMap.intOrder m front_key_result.val = front_next at *
    rcases front_next with _ | next_front_key
  all_goals try
    generalize next_step : SpecMap.nextKey? SpecMap.intOrder m borrow_result.val = step_next at *
    rcases step_next with _ | next_step_key
  all_goals try simp only [variantName, variantPayload, Option.elim_none, Option.elim_some,
    vir_denote_norm, vir_denote_eval] at *
  all_goals verex_split_hypotheses
  all_goals verex_subst_vars
  all_goals try omega
  all_goals try verex_assumption
  all_goals try
    have front_read : SpecMap.keyAt m 0 = front_key_result.val := by assumption
    have front_present : SpecMap.hasKey m front_key_result.val = true := by
      rw [← front_read]
      exact SpecMap.hasKey_keyAt (by omega) (by omega)
    have front_position : SpecMap.rank m front_key_result.val = 0 := by
      rw [← front_read]
      exact SpecMap.rank_keyAt m_valid (by omega) (by omega)
    first
    | omega
    | verex_assumption
    | (have last := SpecMap.nextKey_none m_valid front_present next_front; omega)
    | (have next := SpecMap.nextKey_some m_valid front_present next_front
       rw [SpecInt.val_ofInt _ (SpecMap.hasKey_satisfies m_keys next.1)]
       first | exact next.1 | omega)
  all_goals
    have step_present : SpecMap.hasKey m borrow_result.val = true := by
      rw [borrow_ensures_1]
      exact invariant_4_1
    have step_position : SpecMap.rank m borrow_result.val = count.val := by
      rw [borrow_ensures_1]
      exact invariant_4_2
    have step_high := SpecMap.rank_lt_size step_present
    first
    | omega
    | verex_assumption
    | (have last := SpecMap.nextKey_none m_valid step_present next_step; omega)
    | (have next := SpecMap.nextKey_some m_valid step_present next_step
       rw [SpecInt.val_ofInt _ (SpecMap.hasKey_satisfies m_keys next.1)]
       first | exact next.1 | omega)

verify test_verify_iter_prev by
  verex_obligations preparing
    verex_prepare typed
  verex_group assertions := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6, assert_7, assert_8
  case assertions =>
    verex_cases internal_new_end_iter_result
    all_goals verex_cases old_iter_prev_result
    all_goals verex_cases iter_prev_result
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals
      have present : SpecMap.hasKey new_from_result old_iter_prev_result_Some_2.val = true := by assumption
      have position : SpecMap.rank new_from_result old_iter_prev_result_Some_2.val =
          SpecMap.size new_from_result - 1 := by assumption
      have present2 : SpecMap.hasKey new_from_result iter_prev_result_Some_2.val = true := by assumption
      have shift : SpecMap.rank new_from_result iter_prev_result_Some_2.val =
          SpecMap.rank new_from_result old_iter_prev_result_Some_2.val - 1 := by
        first | assumption | exact iter_prev_ensures_8 present
      have read1 : old_iter_borrow_key_result.val = old_iter_prev_result_Some_2.val := by assumption
      have read2 : iter_borrow_key_result.val = iter_prev_result_Some_2.val := by assumption
      have located := SpecMap.keyAt_rank present
      have located2 := SpecMap.keyAt_rank present2
      rw [position] at located
      rw [shift, position] at located2
      subst new_from_result_eq
      simp only [vir_denote_norm, vir_denote_eval] at located located2
      simp only [read1, read2, ← located, ← located2, vir_denote_norm, vir_denote_eval]
      first
      | verex_goal_witnesses
      | (refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩ <;> first | decide | verex_goal_witnesses)
  all_goals first | verex_decide | (verex_prepare typed; all_goals try verex_decide)
  -- The iterator the precondition admits: the end iterator, as constructed.
  all_goals first | (left; assumption)

verify test_verify_iter_next by
  verex_obligations preparing
    verex_prepare typed
  verex_group initial := assert_1, assert_2, assert_3, assert_4, assert_5, assert_6, assert_7, assert_8, assert_9, assert_10
  verex_group assertions := assert_11, assert_12, assert_13, assert_14, assert_15, assert_16, assert_17
  case assertions =>
    verex_prepare cases [internal_find_result, old_iter_next_result, iter_next_result_1, iter_next_result]
    all_goals try subst new_from_result_eq
    all_goals simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals try simp only [internal_find_ensures_3] at *
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals try verex_consume
    all_goals try simp only [vir_denote_norm, vir_denote_eval] at *
    all_goals try omega
    all_goals try verex_decide
    -- The keys the iterators reach, among the map's three.
    all_goals
      obtain o | o | o := (show 1 = old_iter_next_result_Some_2.val ∨ 2 = old_iter_next_result_Some_2.val ∨
        3 = old_iter_next_result_Some_2.val by assumption) <;>
      obtain n | n | n := (show 1 = iter_next_result_1_Some_2.val ∨ 2 = iter_next_result_1_Some_2.val ∨
        3 = iter_next_result_1_Some_2.val by assumption) <;>
      simp only [← o, ← n] at * <;>
      simp only [vir_denote_norm, vir_denote_eval] at * <;>
      first
      | omega
      | (have b : iter_borrow_key_result.val = 3 := by omega
         have b1 : iter_borrow_key_result_1.val = 2 := by omega
         have b0 : old_iter_borrow_key_result.val = 1 := by omega
         simp only [b, b1, b0, vir_denote_norm, vir_denote_eval]
         done)
  case initial =>
    simp only [new_from_result_eq, vir_denote_norm, vir_denote_eval]
    first
    | verex_goal_witnesses
    | (refine ⟨?_, ?_, ?_⟩ <;> first | verex_goal_witnesses | decide)
  verex_on
  | data_invariant, requires of iter_next, requires of internal_find =>
    verex_prepare typed
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals first | verex_decide | (right; assumption) | (left; assumption)
  | _ => first | verex_decide

verify test_verify_leaf_walk_sum_symbolic by
  verex_obligations
  verex_on
  | requires of iter_next, requires of iter_borrow, requires of iter_is_end,
    requires of internal_new_begin_iter =>
    verex_prepare typed
    all_goals first
    | (refine SpecMap.valid_slice permitted ?low
        internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5 ?same
       case same =>
         intro j low high
         exact (internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6 j low high).symm
       case low =>
         first
         | omega
         | (rw [internal_leaf_new_begin_iter_ensures_3]; omega)
         | (simp only [internal_leaf_new_begin_iter_ensures_3]; omega))
  | invariant_2_step =>
    verex_prepare typed
    all_goals try verex_cases it
    all_goals try verex_cases internal_leaf_new_begin_iter_result
    all_goals try verex_cases internal_leaf_iter_borrow_entries_and_next_leaf_index_result_1
    all_goals try verex_cases oit
    all_goals try verex_cases internal_new_begin_iter_result
    all_goals try verex_cases iter_next_result
    all_goals try verex_cases iter_borrow_result
    all_goals verex_canonical_families
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals try simp only [internal_leaf_new_begin_iter_ensures_3] at *
    all_goals try omega
    all_goals (first | (exfalso; omega) | skip)
    all_goals
      -- The entry the inner walk reads is the leaf's at the iterator's
      -- position, which the leaf reads as the map's value at its offset.
      have inside : oit_Position.val < entries.size := by
        assumption
      have low : 0 ≤ oit_Position.val := by
        first | omega | (have fits := oit_Position.fits; simp only [IntegerValueFits_unsigned_succ] at fits; omega)
      have read := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_8 oit_Position.val low inside
      have leaf := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_7 oit_Position.val low inside
      verex_canonical_families
      try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at read leaf iter_borrow_ensures_1
      first
      | obtain ⟨payload, node⟩ := variantCarrier.second_of_name "Inner" "Leaf" (by decide) leaf
      | obtain ⟨payload, node⟩ := variantCarrier.second_of_name "Inner" "Leaf" (by decide) leaf.symm
      first
      | rw [node] at read iter_borrow_ensures_1
      simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval,
        Option.some.injEq, String.reduceEq, reduceCtorEq,
        and_false, false_and] at read iter_borrow_ensures_1
      all_goals first
      | (exfalso; exact iter_borrow_ensures_1)
      | (exfalso; simp at iter_borrow_ensures_1; done)
      | (first
         | (have size_eq : entries.size = oit_Position.val + 1 := by omega
            rw [size_eq])
         | rw [iter_next_ensures_2]
         conv => rhs; rw [spec_test_sum_upto.spec.unfold]
         dsimp only
         try verex_name_maps
         rw [dif_neg (by omega)]
         simp only [← Int.add_assoc, Int.add_sub_cancel]
         omega)
  | _ =>
    verex_prepare typed
    all_goals try verex_cases it
    all_goals try verex_cases internal_leaf_new_begin_iter_result
    all_goals try verex_cases internal_leaf_iter_borrow_entries_and_next_leaf_index_result_1
    all_goals try verex_cases oit
    all_goals try verex_cases internal_new_begin_iter_result
    all_goals try verex_cases iter_next_result
    all_goals try verex_cases iter_borrow_result
    all_goals verex_canonical_families
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals try simp only [internal_leaf_new_begin_iter_ensures_3] at *
    all_goals first
    | verex_decide
    | omega
    | (exfalso; simp_all; done)
    | (rw [spec_test_sum_upto.spec.unfold]; simp; done)
    | (have offset := invariant_5 (by assumption)
       rw [offset] at invariant_6
       exact invariant_6)
    | (simp only [Int.add_zero, Int.zero_add] at *; assumption)
    | (have empty : SpecMap.size internal_leaf_iter_borrow_entries_and_next_leaf_index_result_0 = 0 := by omega
       simp only [empty, Int.add_zero] at *
       assumption)
    | (simp only [internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_9,
         internal_new_begin_iter_ensures_1, Int.add_zero, Int.zero_add] at *
       first | assumption | omega)
    | (simp only [internal_new_begin_iter_ensures_2, Int.add_zero, Int.zero_add] at *
       first | assumption | omega)

verify test_verify_upsert by
  verex_proof preparing
    verex_prepare typed splitting [upsert_result, old_upsert_result]
    verex_map_simp
    try verex_decide

verify test_verify_drain_loop by
  verex_obligations
  verex_group initial := assert_1, assert_2, assert_3, assert_7, assert_8, assert_9, assert_10, assert_11, assert_12, assert_13
  verex_on
  | invariant_2_entry, invariant_3_entry =>
    verex_prepare typed
    have indices : i = 0 ∨ i = 1 ∨ i = 2 := by omega
    first
    | (rcases indices with rfl | rfl | rfl <;> simp_all [vir_denote_norm, vir_denote_eval])
  | assert_14 =>
    verex_prepare typed
    first
    | (simpa only [x_eq, Int.reduceSub, Int.reduceMul, Int.reduceTDiv] using invariant_4)
  | invariant_1_step, invariant_2_step, invariant_3_step, invariant_4_step =>
    verex_prepare typed
    have present : SpecMap.hasKey map (SpecMap.keyAt map 0) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have sizes : SpecMap.size map' = SpecMap.size map - 1 := by
      rw [map'_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt map' i = SpecMap.keyAt map (i + 1) := by
         rw [map'_eq, SpecMap.keyAt_remove_ordered, SpecMap.rank_keyAt map_valid (by omega) (by omega),
           if_neg (by omega)]
       rw [shift]
       first
       | (have previous := invariant_2 (i + 1) (by omega) (by omega); omega)
       | (have different : SpecMap.keyAt map (i + 1) ≠ SpecMap.keyAt map 0 := fun equal =>
            (by omega : ¬i + 1 = 0) ((SpecMap.keyAt_eq_keyAt_iff map_valid (by omega) (by omega) (by omega) (by omega)).mp equal)
          rw [map'_eq, SpecMap.valueAt_remove map_valid _ _ different]
          exact invariant_3 (i + 1) (by omega) (by omega)))
    | (have read : pop_front_result_1.val = SpecMap.valueAt map (SpecMap.keyAt map 0) := by assumption
       have val := invariant_3 0 (by omega) (by omega)
       have key := invariant_2 0 (by omega) (by omega)
       rw [read, val, key, invariant_4, sizes]
       rcases (show SpecMap.size map = 1 ∨ SpecMap.size map = 2 ∨ SpecMap.size map = 3 from by omega)
         with len | len | len <;> simp [len])
  | _ =>
    first | verex_decide | (
      verex_prepare typed
      all_goals first
      | verex_goal_witnesses
      | (simp_all [vir_denote_norm, vir_denote_eval]; done)
      | omega)

verify test_verify_leaf_iter by
  verex_obligations
  verex_on
  | requires of borrow, requires of keys =>
    verex_prepare typed
    all_goals first
    | (refine SpecMap.valid_slice new_from_result_valid ?low
        internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5 ?same
       case same =>
         intro j low high
         exact (internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6 j low high).symm
       case low =>
         first
         | omega
         | (rw [internal_leaf_new_begin_iter_ensures_3]; omega)
         | (simp only [internal_leaf_new_begin_iter_ensures_3]; omega))
  | assert_2 =>
    verex_prepare typed
    all_goals try verex_cases internal_leaf_new_begin_iter_result
    all_goals try verex_cases internal_leaf_borrow_value_result
    all_goals verex_canonical_families
    all_goals try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at *
    all_goals try simp only [internal_leaf_new_begin_iter_ensures_3] at *
    all_goals first
    | verex_decide
    | assumption
    | omega
    | skip
    all_goals
      have related := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_3_2 _ ?low ?high
        (by assumption)
      case low => omega
      case high => omega
      have leaf := internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_3_1 _ ?low ?high
        (by assumption)
      case low => omega
      case high => omega
      verex_canonical_families
      try simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval] at related leaf internal_leaf_borrow_value_ensures_1
      first
      | obtain ⟨payload, node⟩ := variantCarrier.second_of_name "Inner" "Leaf" (by decide) leaf
      | obtain ⟨payload, node⟩ := variantCarrier.second_of_name "Inner" "Leaf" (by decide) leaf.symm
      first
      | rw [node] at related internal_leaf_borrow_value_ensures_1
      simp only [variantName, variantPayload, vir_denote_norm, vir_denote_eval]
        at related internal_leaf_borrow_value_ensures_1
      first | omega
  | _ =>
    first
    | verex_decide
    | (verex_prepare typed
       -- A key the leaf holds is the map's.
       all_goals first
       | verex_decide
       | exact internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_2 _
           (by verex_type_bounds; omega) (by verex_type_bounds; omega) (by assumption))

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

verify test_verify_remove_shift_symbolic by
  verex_obligations
  verex_on
  | ensures_1, ensures_2, ensures_3, ensures_4 =>
    verex_prepare typed
    have present : SpecMap.hasKey m k.val = true := by assumption
    have valid : SpecMap.Valid (.ordered SpecMap.intOrder) m := by assumption
    solve
    | simp only [m'_eq, SpecMap.size_remove, present, if_true]
    | simp only [m'_eq, SpecMap.hasKey_remove valid, decide_true, Bool.not_true, Bool.false_and]
    | simp only [m'_eq, SpecMap.keyAt_remove_ordered]
    | (have key_present : SpecMap.hasKey m' (SpecMap.keyAt m' i) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
       have different : SpecMap.keyAt m' i ≠ k.val := by
         intro same
         rw [same, m'_eq, SpecMap.hasKey_remove valid] at key_present
         simp at key_present
       exact (congrArg (fun map => SpecMap.valueAt map (SpecMap.keyAt m' i)) m'_eq).trans
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

verify test_verify_pop_back_drain_symbolic by
  verex_obligations
  verex_on
  | invariant_1_step, invariant_2_step, invariant_3_step =>
    verex_prepare typed
    have present : SpecMap.hasKey m (SpecMap.keyAt m (SpecMap.size m - 1)) = true := SpecMap.hasKey_keyAt (by omega) (by omega)
    have length : SpecMap.size m' = SpecMap.size m - 1 := by
      rw [m'_eq, SpecMap.size_remove, if_pos present]
    first
    | omega
    | (have shift : SpecMap.keyAt m' i = SpecMap.keyAt m i := by
         rw [m'_eq, SpecMap.keyAt_remove_ordered,
           SpecMap.rank_keyAt m_valid (by omega) (by omega), if_pos (by omega)]
       rw [shift]
       first
       | exact invariant_2 i (by omega) (by omega)
       | (rw [m'_eq, SpecMap.valueAt_remove m_valid _ _ (by
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
    rw [front] at m'_eq
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
    rw [← x]
    first
    | exact SpecMap.hasKey_keyAt (by omega) (by omega)
    | exact SpecMap.rank_keyAt (by assumption) (by omega) (by omega)
  | _ => verex_decide
