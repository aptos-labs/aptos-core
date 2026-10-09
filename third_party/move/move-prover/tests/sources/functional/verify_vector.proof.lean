-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

-- Swapping with the final entry and removing it is an update followed by truncation.
theorem swap_erase_last {α : Type} (xs : Array α) (i : Nat) (within : i < xs.size) :
    (xs.swapIfInBounds i (xs.size - 1)).eraseIdxIfInBounds (xs.size - 1) =
      (xs.setIfInBounds i (xs[xs.size - 1]'(by omega))).extract 0 (xs.size - 1) := by
  have last : xs.size - 1 < xs.size := by omega
  rw [Array.eraseIdxIfInBounds_eq, dif_pos (by simpa using last)]
  apply Array.ext
  · simp only [Array.size_eraseIdx, Array.size_swapIfInBounds, Array.size_extract,
      Array.size_setIfInBounds, Nat.sub_zero, Nat.min_eq_left (Nat.sub_le _ _)]
  · intro k hkleft hkright
    have hk : k < xs.size - 1 := by
      simpa only [Array.size_eraseIdx, Array.size_swapIfInBounds] using hkleft
    rw [Array.getElem_eraseIdx_of_lt (by simpa using last) hkleft hk,
      Array.getElem_swapIfInBounds, Array.getElem_extract]
    simp only [Nat.zero_add]
    rw [Array.getElem_setIfInBounds (by omega : k < xs.size)]
    simp only [last, within, and_true]
    by_cases same : k = i
    · simp [same]
    · simp [same, Nat.ne_of_lt hk, Ne.symm same]

verify verify_swap_remove by
  all_goals have within : i.val.toNat < v.values.size := by omega
  all_goals have last : ((v.values.size : Int) - 1).toNat = v.values.size - 1 := by omega
  case leaf_1 =>
    rw [Array.getElem_swapIfInBounds_right within,
      Array.getElem?_eq_getElem within]
    rfl
  case leaf_2 =>
    simp only [last]
    rw [swap_erase_last v.values i.val.toNat within, Array.map_extract,
      Array.map_setIfInBounds, Array.getElem?_eq_getElem (by omega)]
    rfl
  case leaf_3 =>
    have bounded : ((v.values.size : Int) - 1).toNat < v.values.size := by omega
    rw [Array.eraseIdxIfInBounds_eq, dif_pos (by simpa using bounded)]
    simp only [Array.size_eraseIdx, Array.size_swapIfInBounds]
    omega

verify verify_model_swap_remove by
  all_goals have within : i.val.toNat < v.values.size := by omega
  all_goals have last : ((v.values.size : Int) - 1).toNat = v.values.size - 1 := by omega
  case leaf_1 =>
    rw [Array.getElem_swapIfInBounds_right within,
      Array.getElem?_eq_getElem within]
    rfl
  case leaf_2 =>
    simp only [last]
    rw [swap_erase_last v.values i.val.toNat within, Array.map_extract,
      Array.map_setIfInBounds, Array.getElem?_eq_getElem (by omega)]
    rfl
  case leaf_3 =>
    have bounded : ((v.values.size : Int) - 1).toNat < v.values.size := by omega
    rw [Array.eraseIdxIfInBounds_eq, dif_pos (by simpa using bounded)]
    simp only [Array.size_eraseIdx, Array.size_swapIfInBounds]
    omega

-- Instantiate the no-match prefix invariant at the observed element.
verify verify_index_of by
  case leaf_1 =>
    expose_names
    rintro ⟨x, ⟨j, low, high, read⟩, equal⟩
    have absent := left j low (by omega)
    rw [read] at absent
    exact absent (by simpa only [Option.map_some, Option.getD_some] using equal)
  case leaf_2 =>
    expose_names
    by_cases previous : i < i_1.val
    · exact left i (by omega) previous
    · have same : i = i_1.val := by omega
      subst i
      rw [heq]
      simp only [Option.map_some, Option.getD_some]
      intro equal
      exact symmetric ((LeanerIR.Proofs.Denote.Carriers.codec 0).encode_injective equal).symm
