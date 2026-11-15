-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Decide
open VerexIR VerexIR.Proofs.Denote

namespace VerexIR.Tests.VectorWriteSizes

-- Certification and a preceding swap preserve the size used by the removal guard.

theorem erase_after_swap [Carriers] (v : (NTy.vector (.param 0)).plain)
    (held : NTy.deepHolds (.vector (.param 0)) v) (i j : Nat)
    (within : j < v.values.size) :
    (((NTy.ofDeepHolds (.vector (.param 0)) v held).values.swapIfInBounds i j).eraseIdxIfInBounds j).size + 1 =
      v.values.size := by
  verex_lookups_after_writes
  omega

set_option linter.unusedVariables false in
example [Carriers] (v : (NTy.vector (.param 0)).plain)
    (held : NTy.deepHolds (.vector (.param 0)) v) (i j : Nat) : True := by
  fail_if_success
    have : (((NTy.ofDeepHolds (.vector (.param 0)) v held).values.swapIfInBounds i j).eraseIdxIfInBounds j).size + 1 = v.values.size := by
      verex_lookups_after_writes
      omega
  trivial

-- Untouched slice-reference elements after `setIfInBounds` discharge both
-- `i ≠ k` (via `Ne` in `arithmeticProposition`, with no `ne_eq` rewrite of the
-- goal) and `k < slice.currentVector.values.size` (via `currentVector_size` /
-- `finalVector_size`), and fail when either hypothesis is missing.

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

end VerexIR.Tests.VectorWriteSizes
