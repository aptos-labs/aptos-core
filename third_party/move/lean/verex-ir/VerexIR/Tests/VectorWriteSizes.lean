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

end VerexIR.Tests.VectorWriteSizes
