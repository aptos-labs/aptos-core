-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs

verex proofs for "acl.move"
verex proofs where

open VerexIR VerexIR.Maps VerexIR.Proofs VerexIR.Proofs.Denote

-- Removal preserves index uniqueness: each surviving index refers to its
-- old position before the removal, or one position further after it.
private theorem erase_unique {α β : Type} (xs : Array α) (read : Option α → β)
    (pos : Nat) (hp : pos < xs.size)
    (unique : ∀ i : Int, 0 ≤ i → i < xs.size → ∀ j : Int, 0 ≤ j → j < xs.size →
      read xs[i.toNat]? = read xs[j.toNat]? → i = j)
    (i j : Int) (hi : 0 ≤ i) (hj : 0 ≤ j)
    (hi' : i < (xs.eraseIdx pos hp).size) (hj' : j < (xs.eraseIdx pos hp).size)
    (same : read (xs.eraseIdx pos hp)[i.toNat]? = read (xs.eraseIdx pos hp)[j.toNat]?) : i = j := by
  simp only [Array.size_eraseIdx] at hi' hj'
  by_cases left : i < (pos : Int) <;> by_cases right : j < (pos : Int)
  · rw [getElem?_eraseIdx_toNat_of_lt xs pos hp i hi left,
      getElem?_eraseIdx_toNat_of_lt xs pos hp j hj right] at same
    exact unique i hi (by omega) j hj (by omega) same
  · rw [getElem?_eraseIdx_toNat_of_lt xs pos hp i hi left,
      getElem?_eraseIdx_toNat_of_ge xs pos hp j (by omega)] at same
    have := unique i hi (by omega) (j + 1) (by omega) (by omega) same
    omega
  · rw [getElem?_eraseIdx_toNat_of_ge xs pos hp i (by omega),
      getElem?_eraseIdx_toNat_of_lt xs pos hp j hj right] at same
    have := unique (i + 1) (by omega) (by omega) j hj (by omega) same
    omega
  · rw [getElem?_eraseIdx_toNat_of_ge xs pos hp i (by omega),
      getElem?_eraseIdx_toNat_of_ge xs pos hp j (by omega)] at same
    have := unique (i + 1) (by omega) (by omega) (j + 1) (by omega) (by omega) same
    omega

verify remove by
  verex_obligations
  all_goals verex_prepare only
  all_goals try verex_decide
  all_goals verex_clear_computations
  all_goals verex_lookups_after_writes
  all_goals try verex_decide
  case leaf_2 =>
    rename_i same
    refine erase_unique self_list.values (fun x => ((x.map Codec.address.encode).getD .unit).asString)
      _ ?_ permitted found index (by omega) (by omega) ?_ ?_ same
    · omega
    · simp only [Array.size_eraseIdx]; omega
    · simp only [Array.size_eraseIdx]; omega
  all_goals verex_split_search
  all_goals verex_split_hypotheses
  all_goals try verex_normalize
  all_goals try verex_decide
  case ensures_1.inl => grind only [NTy.eqb_address]
  case ensures_1.inr =>
    have selected : self_list.values[found]? = some addr := by grind only [NTy.eqb_address]
    by_cases before : i < (found : Int)
    · verex_lookups_after_writes
      intro remaining
      have impossible := search_2_2 i (by omega) before addr remaining
      simp only [NTy.eqb_address, decide_true, Bool.true_eq_false] at impossible
    · verex_lookups_after_writes
      intro remaining
      have same := permitted (↑found) (by omega) (by omega) (i + 1) (by omega) (by omega)
        (by simp only [Int.toNat_natCast, selected, remaining])
      omega
