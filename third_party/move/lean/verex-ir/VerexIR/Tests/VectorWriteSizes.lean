-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide
open VerexIR VerexIR.Proofs.Denote

namespace VerexIR.Tests.VectorWriteSizes

-- Negative (first): Without `j < v.values.size`, `eraseIdxIfInBounds` need not shrink
-- the swapped vector even when `NTy.Holds` holds on `v`.
set_option linter.unusedVariables false in
example [Carriers] (v : (NTy.vector (.param 0)).carrier)
    (held : NTy.Holds (.vector (.param 0)) v) (i j : Nat) : True := by
  fail_if_success
    have : ((v.values.swapIfInBounds i j).eraseIdxIfInBounds j).size + 1 = v.values.size := by
      verex_lookups_after_writes
      omega
  trivial

-- Positive: Under `j < v.values.size`, a preceding swap preserves the size used by the removal guard.
set_option linter.unusedVariables false in
theorem erase_after_swap [Carriers] (v : (NTy.vector (.param 0)).carrier)
    (held : NTy.Holds (.vector (.param 0)) v) (i j : Nat)
    (within : j < v.values.size) :
    ((v.values.swapIfInBounds i j).eraseIdxIfInBounds j).size + 1 =
      v.values.size := by
  verex_lookups_after_writes
  omega

-- Untouched slice-reference elements after `setIfInBounds` fail when either `k ≠ i`
-- or `k < slice.extent` is missing (negatives first), and discharge both when present.

set_option linter.unusedVariables false in
example (slice : SpecSliceRef Nat) (i k v : Nat)
    (within : k < slice.extent)
    (bound : k < (slice.currentVector.values.setIfInBounds i v).size) : True := by
  fail_if_success
    have : (slice.currentVector.values.setIfInBounds i v)[k]'bound =
        slice.currentVector.values[k]'(by simpa [SpecSliceRef.currentVector_size] using within) := by
      verex_lookups_after_writes
  trivial

set_option linter.unusedVariables false in
example (slice : SpecSliceRef Nat) (i k v : Nat)
    (distinct : k ≠ i)
    (bound : k < (slice.currentVector.values.setIfInBounds i v).size) : True := by
  fail_if_success
    have : ∃ rightBound : k < slice.currentVector.values.size,
        (slice.currentVector.values.setIfInBounds i v)[k]'bound =
          slice.currentVector.values[k]'rightBound := by
      verex_lookups_after_writes
      exact ⟨_, rfl⟩
  trivial

theorem slice_current_lookup_after_write_ne (slice : SpecSliceRef Nat)
    (i k v : Nat) (within : k < slice.extent) (distinct : k ≠ i)
    (bound : k < (slice.currentVector.values.setIfInBounds i v).size) :
    (slice.currentVector.values.setIfInBounds i v)[k]'bound =
      slice.currentVector.values[k]'(by simpa [SpecSliceRef.currentVector_size] using within) ∧
      k ≠ i := by
  verex_lookups_after_writes
  exact ⟨trivial, distinct⟩

theorem slice_final_lookup_after_write_ne (slice : SpecSliceRef Nat)
    (i k v : Nat) (within : k < slice.extent) (distinct : k ≠ i)
    (bound : k < (slice.finalVector.values.setIfInBounds i v).size) :
    (slice.finalVector.values.setIfInBounds i v)[k]'bound =
      slice.finalVector.values[k]'(by simpa [SpecSliceRef.finalVector_size] using within) ∧
      k ≠ i := by
  verex_lookups_after_writes
  exact ⟨trivial, distinct⟩

-- Naming a read states it as `some element` and collapses the fallback it
-- was read with. The position is within the size by the same size rules the
-- lookup rules' side conditions use.

-- Negative (first): without a bound on the position, a read of a slice
-- reference's final vector is not named and the goal stays as it is.
set_option linter.unusedVariables false in
example (slice : SpecSliceRef Nat) (k fallback : Nat) :
    slice.finalVector.values[k]?.getD fallback = slice.finalVector.values[k]?.getD fallback := by
  fail_if_success verex_name_lookups
  rfl

-- Negative: a position the context puts beyond the extent is out of bounds,
-- so the read stays optional.
set_option linter.unusedVariables false in
example (slice : SpecSliceRef Nat) (k fallback : Nat) (beyond : slice.extent ≤ k) :
    slice.finalVector.values[k]?.getD fallback = slice.finalVector.values[k]?.getD fallback := by
  fail_if_success verex_name_lookups
  rfl

-- Negative: the same for an array whose size bounds nothing.
set_option linter.unusedVariables false in
example (values : Array Nat) (k fallback : Nat) :
    values[k]?.getD fallback = values[k]?.getD fallback := by
  fail_if_success verex_name_lookups
  rfl

-- Positive: the extent bounds the position, so the read of the final vector
-- is named through `SpecSliceRef.finalVector_size` and its fallback goes.
theorem slice_final_read_named (slice : SpecSliceRef Nat) (k fallback : Nat)
    (within : k < slice.extent) :
    ∃ element, slice.finalVector.values[k]? = some element ∧
      slice.finalVector.values[k]?.getD fallback = element := by
  verex_name_lookups

-- Positive: and of the current vector, through `currentVector_size`.
theorem slice_current_read_named (slice : SpecSliceRef Nat) (k fallback : Nat)
    (within : k < slice.extent) :
    ∃ element, slice.currentVector.values[k]? = some element ∧
      slice.currentVector.values[k]?.getD fallback = element := by
  verex_name_lookups

-- Positive: a plain vector read, whose size the context states as it stands,
-- is named as before.
theorem vector_read_named (values : Array Nat) (k fallback : Nat)
    (within : k < values.size) :
    ∃ element, values[k]? = some element ∧ values[k]?.getD fallback = element := by
  verex_name_lookups

-- The gate of the prepared deciders (`verex_prepared_applies`) and the lookup
-- stage recognize one set of operations (`isLookupOperation`): a read over
-- each reaches the prepared deciders and is decided by the lookup laws.

-- Negative (first): a read of an array no operation produced is not the
-- prepared deciders'.
set_option linter.unusedVariables false in
example (xs : Array Int) (k : Nat) (read : xs[k]? = some 1) : True := by
  fail_if_success
    have : xs[k]? = some 1 := by
      verex_prepared_applies
  trivial

-- Negative: a read the gate hands on is decided only where the context
-- places its position against the operation's.
set_option linter.unusedVariables false in
example (xs : Array Int) (i j k : Nat) (bound : k < (xs.swapIfInBounds i j).size)
    (within : k < xs.size) : True := by
  fail_if_success
    have : (xs.swapIfInBounds i j)[k]'bound = xs[k]'within := by
      verex_prepared_applies
      verex_decide_written
  trivial

set_option linter.unusedVariables false in
example (xs : Array Int) (pos : Nat) (valid : pos < xs.size) (i : Int) : True := by
  fail_if_success
    have : (xs.eraseIdx pos valid)[i.toNat]? = xs[i.toNat]? := by
      verex_prepared_applies
      verex_decide_written
  trivial

-- Negative: an optional read over a swap is decided only at one of the two
-- positions the swap moves, both within the array; any other is handed on by
-- the gate and stays, with no case analysis of its position.
set_option linter.unusedVariables false in
example (xs : Array Int) (i j : Nat) : True := by
  fail_if_success
    have : (xs.swapIfInBounds i j)[i]? = xs[j]? := by
      verex_prepared_applies
      verex_decide_written
  trivial

set_option linter.unusedVariables false in
example (xs : Array Int) (i j k : Nat) (hi : i < xs.size) (hj : j < xs.size) : True := by
  fail_if_success
    have : (xs.swapIfInBounds i j)[k]? = xs[k]? := by
      verex_prepared_applies
      verex_decide_written
  trivial

set_option linter.unusedVariables false in
example (xs : Array Int) (i j k : Nat) (x : Int) (hi : i < xs.size) (hj : j < xs.size)
    (read : (xs.swapIfInBounds i j)[k]? = some x) : (xs.swapIfInBounds i j)[k]? = some x := by
  verex_lookups_after_writes
  guard_hyp read : (xs.swapIfInBounds i j)[k]? = some x
  exact read

-- Positive: an optional read over a swap at the left and at the right
-- position, of the array and of a bounded vector, in the goal and in a
-- hypothesis.
set_option linter.unusedVariables false in
theorem optional_read_over_swap_left (xs : Array Int) (i j : Nat) (hi : i < xs.size)
    (hj : j < xs.size) : (xs.swapIfInBounds i j)[i]? = xs[j]? := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
theorem optional_read_over_swap_right (xs : Array Int) (i j : Nat) (hi : i < xs.size)
    (hj : j < xs.size) : (xs.swapIfInBounds i j)[j]? = xs[i]? := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
theorem optional_read_over_bounded_swap (v : SpecVector Int) (i j : Nat) (hi : i < v.values.size)
    (hj : j < v.values.size) : (swapBounded v i j).values[j]? = v.values[i]? := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
example (xs : Array Int) (i j : Nat) (x : Int) (hi : i < xs.size) (hj : j < xs.size)
    (read : (xs.swapIfInBounds i j)[j]? = some x) : xs[i]? = some x := by
  verex_lookups_after_writes
  guard_hyp read : xs[i]? = some x
  exact read

-- Positive: a read over a swap, of the array and of a bounded vector.
set_option linter.unusedVariables false in
theorem read_over_swap (xs : Array Int) (i j : Nat) (hi : i < xs.size) (hj : j < xs.size)
    (bound : i < (xs.swapIfInBounds i j).size) :
    (xs.swapIfInBounds i j)[i]'bound = xs[j]'hj := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
theorem read_over_bounded_swap (v : SpecVector Int) (i j : Nat) (hi : i < v.values.size)
    (hj : j < v.values.size) (bound : i < (swapBounded v i j).values.size) :
    (swapBounded v i j).values[i]'bound = v.values[j]'hj := by
  verex_prepared_applies
  verex_decide_written

-- Positive: a read over a removal, as the array's own operation, as the one
-- guarded by its bounds, and as a bounded vector's.
set_option linter.unusedVariables false in
theorem read_over_erase (xs : Array Int) (pos : Nat) (valid : pos < xs.size)
    (i : Int) (nonnegative : 0 ≤ i) (before : i < pos) :
    (xs.eraseIdx pos valid)[i.toNat]? = xs[i.toNat]? := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
theorem read_over_guarded_erase (xs : Array Int) (pos : Nat) (valid : pos < xs.size)
    (i : Int) (after : pos ≤ i) :
    (xs.eraseIdxIfInBounds pos)[i.toNat]? = xs[(i + 1).toNat]? := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
theorem read_over_bounded_erase (v : SpecVector Int) (pos : Nat) (valid : pos < v.values.size)
    (i : Int) (nonnegative : 0 ≤ i) (before : i < pos) :
    (eraseBounded v pos).values[i.toNat]? = v.values[i.toNat]? := by
  verex_prepared_applies
  verex_decide_written

-- Negative: a read over a bounded insertion whose position the context does
-- not place against the inserted one is handed on by the gate and stays.
set_option linter.unusedVariables false in
example (v : SpecVector Int) (x : Int) (pos : Nat)
    (bounded : (v.values.insertIdxIfInBounds pos x).size < 2 ^ 64) (i : Int) : True := by
  fail_if_success
    have : (insertBounded v pos x bounded).values[i.toNat]? = v.values[i.toNat]? := by
      verex_prepared_applies
      verex_decide_written
  trivial

-- Positive: a read over an insertion as the array's own operation, and as a
-- bounded vector's.
set_option linter.unusedVariables false in
theorem read_over_insert (xs : Array Int) (x : Int) (pos : Nat) (valid : pos ≤ xs.size)
    (i : Int) (nonnegative : 0 ≤ i) (before : i < pos) :
    (xs.insertIdx pos x valid)[i.toNat]? = xs[i.toNat]? := by
  verex_prepared_applies
  verex_decide_written

set_option linter.unusedVariables false in
theorem read_over_bounded_insert (v : SpecVector Int) (x : Int) (pos : Nat)
    (valid : pos ≤ v.values.size)
    (bounded : (v.values.insertIdxIfInBounds pos x).size < 2 ^ 64)
    (i : Int) (nonnegative : 0 ≤ i) (before : i < pos) :
    (insertBounded v pos x bounded).values[i.toNat]? = v.values[i.toNat]? := by
  verex_prepared_applies
  verex_decide_written

end VerexIR.Tests.VectorWriteSizes
