-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "ordered_map.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- The loop preserves the map's enumeration by removing its first entry.
-- Use the map position lemmas directly on the prepared loop obligations.
verify test_verify_drain_symbolic by
  on invariant 1 step {
    apply lemma_contains_key_key_at(m, 0)
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, spec_key_at(m, 0))
  }
  on invariant 2 step {
    let k0 := spec_key_at(m, 0)
    apply lemma_contains_key_key_at(m, 0)
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    assert spec_key_at(m', i) == spec_key_at(m, i + 1) by apply lemma_key_at_remove(m, k0, i)
    assert spec_key_at(m, i + 1) == spec_key_at(old(m), i + 1 + count)
    assert i + 1 + count == i + (count + 1) by arith
  }
  on invariant 3 step {
    let k0 := spec_key_at(m, 0)
    let k := spec_key_at(m, i + 1)
    apply lemma_contains_key_key_at(m, 0)
    assert spec_len(m') == spec_len(m) - 1 by apply lemma_len_remove(m, k0)
    apply lemma_rank_key_at(m, 0)
    assert spec_key_at(m', i) == k by apply lemma_key_at_remove(m, k0, i)
    assert k != k0 by apply lemma_key_at_eq_iff(m, i + 1, 0)
    assert spec_get(m', k) == spec_get(m, k) by apply lemma_get_remove(m, k0, k)
    assert spec_get(m, k) == spec_get(old(m), k)
  }

  on aborts_if 1, ensures 1, ensures 2, invariant 2 entry { }

-- The walk writes each value in place: the write hits the key at the walk's
-- position, so the keys and the size are unchanged, and the invariant over
-- the earlier positions carries over.
verify test_verify_iter_walk_mut_symbolic by
  on invariant 1 step, invariant 2 step, invariant 4 step, invariant 5 step {
    split it
    split iter_next_result
    let k := spec_key_at(m, count)
    apply lemma_contains_key_key_at(m, count)
    apply lemma_len_set(m, k, 7)
  }
  on invariant 3 step {
    split it
    split iter_next_result
    let k := spec_key_at(m, count)
    apply lemma_contains_key_key_at(m, count)
    apply lemma_len_set(m, k, 7)
    apply lemma_key_at_set_present(m, k, 7, i)
  }
  on invariant 6 step {
    split it
    split iter_next_result
    let k := spec_key_at(m, count)
    apply lemma_contains_key_key_at(m, count)
    apply lemma_len_set(m, k, 7)
    apply lemma_key_at_set_present(m, k, 7, i)
    apply lemma_get_set(m, k, 7, spec_key_at(m, i))
    apply lemma_key_at_eq_iff(m, i, count)
    split i == count
  }
  on invariant 4 entry, invariant 5 entry {
    split internal_new_begin_iter_result
  }
  on ensures 3, aborts_if 1 {
    split it
  }
  on _ { }

verify test_aborts_if_new_from_2 by
  on _ { }

verify test_verify_iter_remove_shift_at_position by
  on ensures 5, ensures 1, aborts_if 1 { }

verify test_verify_iter_borrow_mut_at_position by
  on ensures 2 {
    let k := spec_key_at(m, i)
    apply lemma_contains_key_key_at(m, i)
    apply lemma_key_at_set_present(m, k, 7, j)
  }
  on ensures 3 {
    let k := spec_key_at(m, i)
    let kj := spec_key_at(m, j)
    apply lemma_contains_key_key_at(m, i)
    apply lemma_len_set(m, k, 7)
    apply lemma_key_at_set_present(m, k, 7, j)
    apply lemma_get_set(m, k, 7, kj)
    apply lemma_key_at_eq_iff(m, j, i)
  }

  on ensures 1 {
    let k := spec_key_at(m, i)
    apply lemma_get_set(m, k, 7, k)
  }

verify test_verify_iter_borrow_mut_symbolic by
  on ensures 1 {
    split internal_find_result
    apply lemma_key_at_rank(m, k)
    apply lemma_get_set(m, k, 7, k)
  }
  on ensures 2 {
    split internal_find_result
    apply lemma_key_at_rank(m, k)
    apply lemma_len_set(m, k, 7)
  }
  on ensures 3 {
    split internal_find_result
    apply lemma_key_at_rank(m, k)
    apply lemma_key_at_set_present(m, k, 7, i)
  }
  on ensures 4 {
    split internal_find_result
    apply lemma_key_at_rank(m, k)
    apply lemma_get_set(m, k, 7, k2)
  }
  on aborts_if 1 {
    split internal_find_result
    apply lemma_rank_lt_len(m, k)
    apply lemma_rank_nonneg(m, k)
  }

verify test_verify_iter_add_middle_symbolic by
  on ensures 2, ensures 4, ensures 5, aborts_if 1 { }

verify test_verify_iter_add_append_symbolic by
  on _ {
    split internal_new_end_iter_result
  }

-- Typed map arguments; encoding is handled by preparation.

verify ground_enum_123 by
  on assert 1, assert 2, assert 3, assert 4, assert 5, assert 6, assert 7, assert 8, assert 9, assert 10, assert 11 {
    apply lemma_rank_nonneg(map, 1)
    apply lemma_rank_lt_len(map, 3)
    apply lemma_rank_lt_rank(map, 1, 2)
    apply lemma_rank_lt_rank(map, 2, 3)
    assert spec_rank(map, 1) == 0 && spec_rank(map, 2) == 1 && spec_rank(map, 3) == 2 by arith
    apply lemma_key_at_rank(map, 1)
    apply lemma_key_at_rank(map, 2)
    apply lemma_key_at_rank(map, 3)
  }
