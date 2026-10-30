-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "big_ordered_map.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- Append the current key, then advance the iterator rank by one.

verify test_verify_iter_collect_symbolic by
  on invariant 2 step {
    split it
    apply lemma_rank_lt_len(m, it.key)
  }
  on invariant 4 step, invariant 5 step {
    split it
  }
  on invariant 3 step {
    split it
    assert len(out) == spec_rank(m, it.key)
    apply lemma_key_at_rank(m, it.key)
    split i == len(out)
    lean =>
      first
      | (rw [Array.getElem?_push, if_pos (by omega), Option.getD_some])
      | (rw [Array.getElem?_push, if_neg (by omega)])
  }
  on invariant 1 step {
    split iter_next_result
    split internal_new_begin_iter_result
  }
  on requires 1 of iter_next {
    split it
    split internal_new_begin_iter_result
  }
  on invariant 1 entry {
    split internal_new_begin_iter_result
  }
  on invariant 2 entry, invariant 3 entry, invariant 4 entry, invariant 5 entry, ensures 1, ensures 2, aborts_if 1,
    requires of internal_new_begin_iter { }

verify test_aborts_if_new_from_1 by
  on _ { }

verify test_aborts_if_remove by
  on _ { }

verify test_aborts_if_new_from_2 by
  on _ { }

verify test_verify_prev_key by
  on assert 1 {
    assert is_none(prev_key_result) by eval
  }
  on assert 3 {
    assert spec_contains_key(new_from_result, 1) by eval
  }
  on assert 5 {
    assert spec_contains_key(new_from_result, 2) by eval
  }
  on assert 6 {
    assert is_some(prev_key_result) by eval
  }
  on data invariant, requires of remove at call 1, requires of prev_key at call 1, requires of prev_key at call 2 { }

verify test_verify_next_key by
  on assert 1 {
    assert is_none(next_key_result) by eval
  }
  on assert 3 {
    assert spec_contains_key(new_from_result, 1) by eval
  }
  on assert 5 {
    assert spec_contains_key(new_from_result, 2) by eval
  }
  on assert 6, assert 7 {
    assert is_some(next_key_result) by eval
    assert 2 == std::option::borrow(next_key_result) by eval
    split next_key_result
  }
  on data invariant, requires of remove at call 1, requires of next_key at call 1, requires of next_key at call 2 { }

verify test_verify_iter_rank_loop by
  on invariant 2 step, invariant 3 step {
    assert spec_contains_key(map, it.key)
  }
  on assert 2 {
    assert spec_len(map) == 3 by eval
  }
  on invariant 1 step {
    split iter_next_result
    split internal_new_begin_iter_result
  }
  on requires 1 of iter_next {
    split it
    split internal_new_begin_iter_result
  }
  on invariant 1 entry {
    split internal_new_begin_iter_result
  }
  on invariant 2 entry, invariant 3 entry, assert 1, data invariant, requires of internal_new_begin_iter { }

verify test_verify_iter_prev_loop_symbolic by lean =>
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
  on ensures 1 {
    apply lemma_contains_key_set(m, k, v, k)
  }
  on ensures 2 {
    apply lemma_len_set(m, k, v)
  }
  on ensures 3 {
    apply lemma_get_set(m, k, v, k)
  }
  on ensures 4 {
    apply lemma_key_at_set_before(m, k, v, i)
  }
  on ensures 5 {
    apply lemma_key_at_set_after(m, k, v, i)
  }
  on _ { }

verify test_verify_keys_for_loop_symbolic by lean =>
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
    simp only [Int.add_sub_cancel] at *
    first | (simp only [*]; done) | omega
  | _ => first | verex_decide | (verex_prepare only; all_goals first | assumption | omega)

verify test_verify_iter_valid_after_insert_symbolic by
  on assert 1, assert 2, assert 3 {
    apply lemma_len_set(m, k, v)
    split internal_new_begin_iter_result
  }
  on ensures 1 {
    apply lemma_contains_key_set(m, k, v, k)
  }
  on ensures 2 {
    apply lemma_len_set(m, k, v)
  }
  on _ { }

verify test_verify_keys_sorted by
  on assert 1 {
    assert len(keys_result) == 3 by eval
  }
  on assert 2 {
    assert keys_result[0] < keys_result[1] by eval
  }
  on assert 3 {
    assert keys_result[1] < keys_result[2] by eval
  }
  on assert 4, assert 5, assert 6 {
    lean => (try subst new_from_result_eq); (try simp only [vir_denote_norm, vir_denote_eval] at *); (try subst_vars); all_goals first | decide | (simp only [vir_denote_norm, vir_denote_eval]; done)
  }
  on data invariant, requires of remove at call 1, requires of keys at call 1 { }

verify test_verify_iter_across_upsert by
  on assert 1 {
    assert spec_contains_key(new_from_result, 2) by eval
    split upsert_result
  }
  on assert 2 {
    split internal_find_result
    split upsert_result
    assert spec_iter_valid(internal_find_result, map') by solve
  }
  on requires 1 of iter_borrow at call 1 {
    assert spec_contains_key(new_from_result, 2) by eval
    split internal_find_result
    split upsert_result
  }
  on assert 3 {
    assert spec_contains_key(new_from_result, 1) by eval
    assert spec_contains_key(new_from_result, 2) by eval
    assert spec_get(new_from_result, 1) == 10 by eval
    apply lemma_get_set(new_from_result, 2, 21, 1)
    split internal_find_result
    split upsert_result
    assert iter_borrow_result == 10 by congr
  }
  on data invariant, requires of upsert at call 1, requires of internal_find at call 1 { }

verify test_verify_iter_remove_shift_symbolic by
  on ensures 1, ensures 2, ensures 4, ensures 5 {
    assert internal_find_with_path_result.iterator.key == k
  }
  on aborts_if 1, requires 1 of iter_remove at call 1 {
    split internal_find_with_path_result.iterator
  }
  on _ { }

verify test_verify_iter_remove_symbolic by
  on ensures 3, ensures 4, ensures 5 {
    assert internal_find_with_path_result.iterator.key == k
  }
  on ensures 2 { }
  on aborts_if 1, requires 1 of iter_remove at call 1 { }

verify test_verify_lower_bound_scan_start_symbolic by lean =>
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
      have least := internal_lower_bound_ensures_5 (SpecMap.keyAt m i) (by omega) (by omega) present
        (show k.val ≤ SpecMap.keyAt m i by omega)
      omega
  | _ => first | verex_decide | (verex_prepare typed; all_goals first | verex_decide)

verify test_verify_bounded_walk_symbolic by lean =>
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

verify test_verify_find_started_walk_symbolic by lean =>
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

verify test_verify_early_exit_walk_symbolic by lean =>
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
  after call 1 of front_key {
    assert spec_contains_key(m, front_key_result) by apply lemma_contains_key_key_at(m, 0)
    assert spec_rank(m, front_key_result) == 0 by apply lemma_rank_key_at(m, 0)
  }
  on invariant 4 entry, invariant 5 entry {
    split next_key_result
  }
  on invariant 1 step, invariant 2 step, invariant 3 step, invariant 4 step, invariant 5 step {
    split nxt
    assert spec_contains_key(m, borrow_result)
    assert spec_rank(m, borrow_result) == count
    split next_key_result
  }
  on ensures 1 {
    split nxt
  }
  on aborts_if 1 when loop 1 continues {
    split nxt
    apply lemma_rank_lt_len(m, borrow_result)
  }

verify test_verify_iter_prev by
  on assert 1, assert 4, assert 5, assert 6, assert 7, assert 8 {
    assert spec_key_at(new_from_result, 2) == 3 by eval
    assert spec_key_at(new_from_result, 1) == 2 by eval
    assert spec_len(new_from_result) == 3 by eval
    assert spec_contains_key(new_from_result, 2) by eval
    split old_iter_prev_result
    split iter_prev_result
    assert spec_rank(new_from_result, old_iter_borrow_key_result) == 2
    assert spec_rank(new_from_result, iter_borrow_key_result) == 1
    apply lemma_key_at_rank(new_from_result, old_iter_borrow_key_result)
    apply lemma_key_at_rank(new_from_result, iter_borrow_key_result)
  }
  on assert 3 {
    lean => verex_goal_witnesses
  }
  on requires 1 of iter_prev at call 1, requires 1 of iter_prev at call 2 { }
  on data invariant, requires of remove at call 1, requires of iter_prev at call 1, requires of iter_prev at call 2,
    requires of internal_new_end_iter at call 1 { }

verify test_verify_iter_next by
  on assert 7 { assert spec_contains_key(new_from_result, 1) by eval }
  on assert 8 { assert spec_contains_key(new_from_result, 2) by eval }
  on assert 9 { assert spec_contains_key(new_from_result, 3) by eval }
  on assert 10 { assert spec_len(new_from_result) == 3 by eval }
  on assert 4, assert 5, assert 6 {
    lean => verex_goal_witnesses
  }
  on assert 11, assert 12, assert 13, assert 14, assert 15, assert 16, assert 17 {
    split internal_find_result
    split old_iter_next_result
    split old_iter_next_result_1
    assert old_iter_borrow_key_result == 1
    assert spec_rank(new_from_result, old_iter_borrow_key_result_1) == 1
    apply lemma_key_at_rank(new_from_result, old_iter_borrow_key_result_1)
    assert old_iter_borrow_key_result_1 == 2
    assert spec_rank(new_from_result, iter_borrow_key_result) == 2
    apply lemma_key_at_rank(new_from_result, iter_borrow_key_result)
    assert iter_borrow_key_result == 3
    split iter_next_result
    apply !(iter_next_result is End) ==> lemma_rank_lt_len(new_from_result, iter_next_result.key)
  }
  on requires 1 of iter_next at call 1 {
    split internal_find_result
  }
  on requires 1 of iter_next at call 2, requires 1 of iter_next at call 3, data invariant,
    requires of iter_next at call 1, requires of iter_next at call 2, requires of iter_next at call 3,
    requires of internal_find at call 1, requires of remove at call 1, requires of ground_enum_123 at call 1 { }

verify test_verify_leaf_walk_sum_symbolic by lean =>
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
        omega
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
  on assert 3 {
    split upsert_result
  }
  on assert 6, assert 7 {
    split upsert_result
  }
  on data invariant, requires of upsert at call 1, requires of borrow_back at call 1 { }

verify test_verify_drain_loop by
  on assert 7 { assert spec_contains_key(new_from_result, 1) by eval }
  on assert 8 { assert spec_contains_key(new_from_result, 2) by eval }
  on assert 9 { assert spec_contains_key(new_from_result, 3) by eval }
  on assert 10 { assert spec_len(new_from_result) == 3 by eval }
  on assert 11 { assert spec_get(new_from_result, 1) == 10 by eval }
  on assert 12 { assert spec_get(new_from_result, 2) == 20 by eval }
  on assert 13 { assert spec_get(new_from_result, 3) == 30 by eval }
  on invariant 1 step {
    apply lemma_contains_key_key_at(map, 0)
    apply lemma_len_remove(map, spec_key_at(map, 0))
  }
  on invariant 2 step {
    apply lemma_contains_key_key_at(map, 0)
    apply lemma_len_remove(map, spec_key_at(map, 0))
    apply lemma_rank_key_at(map, 0)
    apply lemma_key_at_remove(map, spec_key_at(map, 0), i)
    assert spec_key_at(map, i + 1) == i + 1 + 4 - spec_len(map)
  }
  on invariant 3 step {
    apply lemma_contains_key_key_at(map, 0)
    apply lemma_len_remove(map, spec_key_at(map, 0))
    apply lemma_rank_key_at(map, 0)
    apply lemma_key_at_remove(map, spec_key_at(map, 0), i)
    apply lemma_key_at_eq_iff(map, i + 1, 0)
    apply lemma_get_remove(map, spec_key_at(map, 0), spec_key_at(map, i + 1))
  }
  on invariant 4 step {
    apply lemma_contains_key_key_at(map, 0)
    apply lemma_len_remove(map, spec_key_at(map, 0))
    lean =>
      have val := invariant_3 0 (by omega) (by omega)
      have key := invariant_2 0 (by omega) (by omega)
      have sizes : SpecMap.size map' = SpecMap.size map - 1 := by omega
      rcases (show SpecMap.size map = 1 ∨ SpecMap.size map = 2 ∨ SpecMap.size map = 3 by omega)
        with len | len | len <;>
      simp only [sizes, len, pop_front_result_1_eq, pop_front_result_0_eq, Int.reduceSub, Int.reduceMul,
        Int.reduceTDiv, Int.reduceAdd] at * <;> omega
  }
  on invariant 2 entry {
    split i in 0..3
  }
  on invariant 3 entry {
    assert spec_key_at(new_from_result, 0) == 1
    assert spec_key_at(new_from_result, 1) == 2
    assert spec_key_at(new_from_result, 2) == 3
    split i in 0..3
  }
  on invariant 4 entry {
    assert 10 * (3 - spec_len(new_from_result)) * (4 - spec_len(new_from_result)) / 2 == 0 by eval
  }
  on assert 14 {
    lean => simpa only [x_eq, Int.reduceSub, Int.reduceMul, Int.reduceDiv] using invariant_4
  }
  on assert 5 {
    lean => verex_goal_witnesses
  }
  on requires of ground_enum_123 at call 1 { }

verify test_verify_leaf_iter by
  on requires of borrow at call 1, requires of keys at call 1 {
    lean =>
      refine SpecMap.valid_slice new_from_result_valid ?low
        internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_5 ?same
      case same =>
        intro j low high
        exact (internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_6 j low high).symm
      case low =>
        first
        | omega
        | (rw [internal_leaf_new_begin_iter_ensures_3]; omega)
        | (simp only [internal_leaf_new_begin_iter_ensures_3]; omega)
  }
  on assert 2 {
    lean =>
      rw [internal_leaf_borrow_value_ensures_1]
      exact internal_leaf_iter_borrow_entries_and_next_leaf_index_ensures_3_2 _ (by omega) (by omega)
        (by assumption)
  }
  on assert 1 { }
  on data invariant, requires of remove at call 1, requires of internal_leaf_new_begin_iter at call 1,
    requires of internal_leaf_iter_borrow_entries_and_next_leaf_index at call 1,
    requires 1 of internal_leaf_iter_borrow_entries_and_next_leaf_index at call 1 { }

verify ground_enum_123 by lean =>
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
  on ensures 1 {
    apply lemma_len_remove(m, k)
  }
  on ensures 2 {
    apply lemma_contains_key_remove(m, k, k)
  }
  on ensures 3 {
    apply lemma_key_at_remove(m, k, i)
  }
  on ensures 4 {
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, k)
    apply lemma_contains_key_key_at(m', i)
    apply lemma_contains_key_remove(m, k, spec_key_at(m', i))
    apply lemma_get_remove(m, k, spec_key_at(m', i))
  }
  on _ { }

verify test_verify_drain_symbolic by
  on invariant 1 step {
    apply lemma_contains_key_key_at(m, 0)
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, spec_key_at(m, 0))
  }
  on invariant 2 step {
    let k0 := spec_key_at(m, 0)
    apply lemma_contains_key_key_at(m, 0)
    apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    apply lemma_key_at_remove(m, k0, i)
    assert spec_key_at(m, i + 1) == spec_key_at(old(m), i + 1 + count)
    assert spec_key_at(m', i) == spec_key_at(old(m), i + (count + 1)) by congr
  }
  on invariant 3 step {
    let k0 := spec_key_at(m, 0)
    let k := spec_key_at(m, i + 1)
    apply lemma_contains_key_key_at(m, 0)
    apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    apply lemma_key_at_remove(m, k0, i)
    assert k != k0 by apply lemma_key_at_eq_iff(m, i + 1, 0)
    apply lemma_get_remove(m, k0, k)
  }
  on aborts_if 1, ensures 1, ensures 2, invariant 2 entry { }

verify test_verify_pop_back_drain_symbolic by
  on invariant 1 step {
    apply lemma_contains_key_key_at(m, spec_len(m) - 1)
    apply lemma_len_remove(m, spec_key_at(m, spec_len(m) - 1))
  }
  on invariant 2 step {
    let kl := spec_key_at(m, spec_len(m) - 1)
    apply lemma_contains_key_key_at(m, spec_len(m) - 1)
    apply lemma_len_remove(m, kl)
    apply lemma_rank_key_at(m, spec_len(m) - 1)
    assert i < spec_rank(m, kl)
    apply lemma_key_at_remove(m, kl, i)
  }
  on invariant 3 step {
    let kl := spec_key_at(m, spec_len(m) - 1)
    let k := spec_key_at(m, i)
    apply lemma_contains_key_key_at(m, spec_len(m) - 1)
    apply lemma_len_remove(m, kl)
    apply lemma_rank_key_at(m, spec_len(m) - 1)
    assert i < spec_rank(m, kl)
    apply lemma_key_at_remove(m, kl, i)
    assert k != kl by apply lemma_key_at_eq_iff(m, i, spec_len(m) - 1)
    apply lemma_get_remove(m, kl, k)
  }
  on aborts_if 1, ensures 1, ensures 2, invariant 2 entry { }

verify test_verify_front_remove_drain_symbolic by
  on aborts_if 1, invariant 1 step {
    apply lemma_contains_key_key_at(m, 0)
    apply lemma_len_remove(m, spec_key_at(m, 0))
  }
  on invariant 2 step {
    let k0 := spec_key_at(m, 0)
    apply lemma_contains_key_key_at(m, 0)
    apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    apply lemma_key_at_remove(m, k0, i)
    assert spec_key_at(m, i + 1) == spec_key_at(old(m), i + 1 + count)
    assert spec_key_at(m', i) == spec_key_at(old(m), i + (count + 1)) by congr
  }
  on invariant 3 step {
    let k0 := spec_key_at(m, 0)
    let k := spec_key_at(m, i + 1)
    apply lemma_contains_key_key_at(m, 0)
    apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    apply lemma_key_at_remove(m, k0, i)
    assert k != k0 by apply lemma_key_at_eq_iff(m, i + 1, 0)
    apply lemma_get_remove(m, k0, k)
  }
  on ensures 1, ensures 2, invariant 2 entry { }

verify test_verify_back_key_rank_symbolic by lean =>
  verex_obligations
  verex_on
  | ensures_1, ensures_2 =>
    verex_prepare typed
    rw [← x]
    first
    | exact SpecMap.hasKey_keyAt (by omega) (by omega)
    | exact SpecMap.rank_keyAt (by assumption) (by omega) (by omega)
  | _ => verex_decide
