-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "big_ordered_map.move"
verex proofs where

-- Append the current key, then advance the iterator rank by one.

verify test_verify_iter_collect_symbolic by
  on invariant 2 step {
    split it
    apply lemma_rank_lt_len(m, it.key)
  }
  on invariant 4 step, invariant 5 step {
    split it
    assert spec_contains_key(m, it.key)
    split iter_next_result
  }
  on invariant 3 step {
    split it
    assert len(out) == spec_rank(m, it.key)
    apply lemma_key_at_rank(m, it.key)
    split i == len(out)
    apply lemma_vector_index_push_back(out, iter_borrow_key_result, i)
    if i < len(out) {
      assert out[i] == spec_key_at(m, i)
    }
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
    split it
    assert spec_contains_key(map, it.key)
    split iter_next_result
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

verify test_verify_iter_prev_loop_symbolic by
  on invariant 1 entry, invariant 2 entry, invariant 3 entry, invariant 4 entry {
    split internal_new_end_iter_result
  }
  on invariant 1 step, invariant 2 step, invariant 3 step, invariant 4 step {
    split it
    split iter_prev_result
    apply !(iter_prev_result is End) ==> lemma_rank_nonneg(m, iter_prev_result.key)
  }
  on aborts_if 1 when aborts at op 1 {
    split it
    split iter_prev_result
    apply !(iter_prev_result is End) ==> lemma_rank_nonneg(m, iter_prev_result.key)
  }
  on ensures 1, aborts_if 1 {
    split it
  }
  on requires 1 of iter_prev, requires 1 of iter_is_begin { split it }
  on _ { }

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

verify test_verify_keys_for_loop_symbolic by
  on invariant 1 entry { assert len(keys_result) == spec_len(m) by eval }
  on ensures 1 {
    assert i == spec_len(m)
    assert sum == spec_test_sum_upto(m, i) by eval
    assert sum == spec_test_sum_upto(m, spec_len(m)) by congr
  }
  on invariant 3 entry {
    apply spec_test_sum_upto.unfold(m, 0)
  }
  on invariant 3 step {
    assert sum == spec_test_sum_upto(m, i) by eval
    apply spec_test_sum_upto.unfold(m, i + 1)
    assert borrow_result == spec_get(m, spec_key_at(m, i)) by congr
    assert sum + borrow_result == spec_test_sum_upto(m, i + 1) by congr
  }
  on _ { }

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
  on assert 4 { assert keys_result[0] == 1 by eval
    assert spec_contains_key(new_from_result, 1) by eval }
  on assert 5 { assert keys_result[1] == 2 by eval
    assert spec_contains_key(new_from_result, 2) by eval }
  on assert 6 { assert keys_result[2] == 3 by eval
    assert spec_contains_key(new_from_result, 3) by eval }
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

verify test_verify_lower_bound_scan_start_symbolic by
  on ensures 1 {
    split internal_lower_bound_result
    apply lemma_rank_lt_len(m, internal_lower_bound_result.key)
    apply lemma_contains_key_key_at(m, i)
    apply lemma_key_at_lt(m, i, spec_rank(m, internal_lower_bound_result.key))
    apply lemma_key_at_rank(m, internal_lower_bound_result.key)
    assert spec_key_at(m, i) < k by contradiction {
      assert k <= spec_key_at(m, i)
      assert internal_lower_bound_result.key <= spec_key_at(m, i)
    }
  }
  on ensures 2 {
    split internal_lower_bound_result
    if spec_contains_key(m, k) {
      if internal_lower_bound_result is End {
        assert k < k
      } else {
        assert internal_lower_bound_result.key <= k
      }
    }
  }
  on _ { }

verify test_verify_bounded_walk_symbolic by
  on invariant 1 entry, invariant 4 entry, invariant 5 entry {
    split internal_new_begin_iter_result
  }
  on invariant 2 step, invariant 4 step, invariant 5 step {
    split it
    apply !(it is End) ==> lemma_rank_lt_len(m, it.key)
    split iter_next_result
  }
  on invariant 3 step {
    split it
    assert len(out) == spec_rank(m, it.key)
    apply lemma_key_at_rank(m, it.key)
    apply lemma_vector_index_push_back(out, iter_borrow_key_result, i)
    if i < len(out) {
      assert out[i] == spec_key_at(m, i)
    }
    split i == len(out)
  }
  on ensures 1, ensures 2, aborts_if 1 {
    split it
  }
  on requires 1 of iter_next {
    split it
    split internal_new_begin_iter_result
  }
  on invariant 1 step {
    split iter_next_result
    split internal_new_begin_iter_result
  }
  on _ { }

verify test_verify_find_started_walk_symbolic by
  on invariant 1 entry, invariant 2 entry, invariant 3 entry, invariant 4 entry {
    split internal_find_result
    apply lemma_rank_lt_len(m, k)
  }
  on invariant 2 step, invariant 3 step, invariant 4 step {
    split it
    apply !(it is End) ==> lemma_rank_lt_len(m, it.key)
    split iter_next_result
  }
  on ensures 1, aborts_if 1 {
    split it
    apply lemma_rank_nonneg(m, k)
    apply !(it is End) ==> lemma_rank_lt_len(m, it.key)
  }
  on requires 1 of iter_next {
    split it
    split internal_find_result
  }
  on invariant 1 step {
    split iter_next_result
    split internal_find_result
  }
  on _ { }

verify test_verify_early_exit_walk_symbolic by
  on invariant 1 entry, invariant 3 entry, invariant 4 entry {
    split internal_new_begin_iter_result
  }
  on invariant 2 step, invariant 3 step, invariant 4 step when if 1 else {
    split it
    apply !(it is End) ==> lemma_rank_lt_len(m, it.key)
    split iter_next_result
  }
  on invariant 1 step, invariant 2 step, invariant 3 step, invariant 4 step when if 1 then { split it }
  on invariant 5 step {
    split it
    apply lemma_key_at_rank(m, it.key)
    split i == seen
  }
  on aborts_if 1 when aborts at op 1 {
    split it
    apply lemma_rank_lt_len(m, it.key)
    split iter_next_result
  }
  on ensures 1, aborts_if 1 {
    split it
  }
  on requires 1 of iter_next, requires 1 of iter_borrow {
    split it
    split internal_new_begin_iter_result
  }
  on invariant 1 step when if 1 else {
    split iter_next_result
    split internal_new_begin_iter_result
  }
  on _ { }

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
  on assert 3 { witness 1 }
  on requires 1 of iter_prev at call 1, requires 1 of iter_prev at call 2 { }
  on data invariant, requires of remove at call 1, requires of iter_prev at call 1, requires of iter_prev at call 2,
    requires of internal_new_end_iter at call 1 { }

verify test_verify_iter_next by
  on assert 7 { assert spec_contains_key(new_from_result, 1) by eval }
  on assert 8 { assert spec_contains_key(new_from_result, 2) by eval }
  on assert 9 { assert spec_contains_key(new_from_result, 3) by eval }
  on assert 10 { assert spec_len(new_from_result) == 3 by eval }
  on assert 4 { witness 0 }
  on assert 5 { witness 1 }
  on assert 6 { witness 2 }
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

verify test_verify_leaf_walk_sum_symbolic by
  on requires of iter_next, requires of iter_borrow, requires of iter_is_end,
    requires of internal_new_begin_iter {
    split it
    forall j : Int where 0 <= j && j < aptos_framework::ordered_map::spec_len(entries) {
      assert spec_key_at(m, spec_leaf_offset(it, m) + j) == aptos_framework::ordered_map::spec_key_at(entries, j)
      assert aptos_framework::ordered_map::spec_key_at(entries, j) == spec_key_at(m, spec_leaf_offset(it, m) + j)
    }
    assert 0 <= spec_leaf_offset(it, m)
    apply lemma_valid_slice(m, spec_leaf_offset(it, m))
  }
  on invariant 2 step {
    split it
    split oit
    assert 0 <= oit.index && oit.index < aptos_framework::ordered_map::spec_len(entries)
    assert aptos_framework::ordered_map::spec_get(entries, aptos_framework::ordered_map::spec_key_at(entries, oit.index)).value == spec_get(m, spec_key_at(m, spec_leaf_offset(it, m) + oit.index))
    let Leaf { value } := iter_borrow_result
    assert value == spec_get(m, spec_key_at(m, spec_leaf_offset(it, m) + oit.index)) by congr
    assert sum == spec_test_sum_upto(m, spec_leaf_offset(it, m) + oit.index) by eval
    assert 0 <= spec_leaf_offset(it, m)
    assert 0 < spec_leaf_offset(it, m) + (oit.index + 1) by arith
    assert spec_leaf_offset(it, m) + (oit.index + 1) - 1 == spec_leaf_offset(it, m) + oit.index by arith
    apply spec_test_sum_upto.unfold(m, spec_leaf_offset(it, m) + (oit.index + 1))
    assert sum + internal_leaf_borrow_value_result == spec_test_sum_upto(m, spec_leaf_offset(it, m) + (oit.index + 1)) by congr
    split iter_next_result
    assert (if iter_next_result is End then aptos_framework::ordered_map::spec_len(entries) else iter_next_result.index) == oit.index + 1 by arith
  }
  on invariant 4 of loop 1 entry {
    split internal_leaf_new_begin_iter_result
    assert spec_leaf_offset(internal_leaf_new_begin_iter_result, m) == 0
    assert 0 <= spec_len(m) by arith
  }
  on invariant 6 of loop 1 entry {
    split internal_leaf_new_begin_iter_result
    apply spec_test_sum_upto.unfold(m, 0)
  }
  on invariant 1 entry, invariant 2 entry {
    split it
    split internal_new_begin_iter_result
  }
  on invariant 1 step {
    split oit
    split iter_next_result
  }
  on invariant 4 of loop 1 step {
    split it
    split next_it
    assert 0 <= spec_leaf_offset(next_it, m)
    assert spec_leaf_offset(next_it, m) == spec_leaf_offset(it, m) + aptos_framework::ordered_map::spec_len(entries)
    assert spec_leaf_offset(it, m) + aptos_framework::ordered_map::spec_len(entries) <= spec_len(m)
    assert 0 <= spec_leaf_offset(next_it, m) && spec_leaf_offset(next_it, m) <= spec_len(m) by arith
  }
  on invariant 6 of loop 1 step {
    split it
    split next_it
    split oit
    assert spec_leaf_offset(next_it, m) == spec_leaf_offset(it, m) + aptos_framework::ordered_map::spec_len(entries)
    assert sum == spec_test_sum_upto(m, spec_leaf_offset(it, m) + aptos_framework::ordered_map::spec_len(entries)) by eval
    assert sum == spec_test_sum_upto(m, spec_leaf_offset(next_it, m)) by congr
    finish by congr
  }
  on ensures 1 {
    split it
    assert spec_leaf_offset(it, m) == spec_len(m)
    assert sum == spec_test_sum_upto(m, spec_leaf_offset(it, m)) by eval
    assert sum == spec_test_sum_upto(m, spec_len(m)) by congr
  }
  on _ { }

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
    assert spec_get(map, spec_key_at(map, 0)) == 10 * spec_key_at(map, 0)
    assert spec_key_at(map, 0) == 0 + 4 - spec_len(map) by solve
    assert spec_len(map) == 1 || spec_len(map) == 2 || spec_len(map) == 3 by arith
    split spec_len(map) == 1
    split spec_len(map) == 2
    split spec_len(map) == 3
    assert sum + v == 10 * (3 - spec_len(map')) * (4 - spec_len(map')) / 2 by eval
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
  on assert 14 { assert sum == 60 by eval }
  on assert 5 { witness 1 }
  on requires of ground_enum_123 at call 1 { }

verify test_verify_leaf_iter by
  on requires of borrow at call 1, requires of keys at call 1 {
    assert spec_leaf_offset(internal_leaf_new_begin_iter_result, new_from_result) == 0
    forall j : Int where 0 <= j && j < aptos_framework::ordered_map::spec_len(entries) {
      assert spec_key_at(new_from_result, j) == aptos_framework::ordered_map::spec_key_at(entries, j) by solve
      assert aptos_framework::ordered_map::spec_key_at(entries, j) == spec_key_at(new_from_result, j)
    }
    apply lemma_valid_slice(new_from_result, 0)
  }
  on assert 2 {
    let Leaf { value } := borrow_result
    assert value == spec_get(new_from_result, k0)
    finish by eval

  }
  on assert 1 { }
  on _ { }

verify ground_enum_123 by
  on assert 1, assert 2, assert 3, assert 4, assert 5, assert 6, assert 7, assert 8, assert 9, assert 10, assert 11 {
    apply lemma_rank_nonneg(map, 1)
    apply lemma_rank_lt_len(map, 3)
    apply lemma_rank_lt_rank(map, 1, 2)
    apply lemma_rank_lt_rank(map, 2, 3)
    assert spec_rank(map, 1) == 0
    assert spec_rank(map, 2) == 1
    assert spec_rank(map, 3) == 2
    apply lemma_key_at_rank(map, 1)
    apply lemma_key_at_rank(map, 2)
    apply lemma_key_at_rank(map, 3)
  }
  on _ { }

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

verify test_verify_back_key_rank_symbolic by
  on ensures 1, ensures 2 {
    apply lemma_contains_key_key_at(m, spec_len(m) - 1)
    apply lemma_rank_key_at(m, spec_len(m) - 1)
  }
  on _ { }

-- A found iterator's key is the key looked up, so each closure's postcondition speaks
-- of key 1; the first write leaves the key present, so the second search finds it too.

verify test_verify_modify by
  on requires 1 of iter_modify at call 2, requires 2 of iter_modify at call 1, requires 2 of iter_modify at call 2 {
    split internal_find_result
  }
  after call 2 of internal_find {
    assert old_internal_find_result.key == 1
    assert spec_contains_key(map', 1) by congr
  }
  after call 2 of iter_modify {
    split internal_find_result
    assert internal_find_result.key == 1
    assert spec_get(map', 1) == 12 by congr
  }
  on assert 1 {
    assert internal_find_result.key == 1
    assert spec_get(map', 1) == 11 by congr
  }
  on _ { }
