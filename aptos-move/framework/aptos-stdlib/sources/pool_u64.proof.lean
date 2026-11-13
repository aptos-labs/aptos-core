-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove.Proofs



verex proofs for "pool_u64.move"
verex proofs where

theorem PoolU64Proofs.pool_map_contains {K V W : Type} [DecidableEq K] (f : V → W) (xs : List (K × V)) (k : K) :
    SpecMap.containsB (xs.map fun e => (e.1, f e.2)) k = SpecMap.containsB xs k := by
  induction xs with
  | nil => rfl
  | cons e xs ih => simp only [List.map_cons, SpecMap.containsB, ih]

theorem PoolU64Proofs.pool_map_replace {K V W : Type} [DecidableEq K] (f : V → W) (xs : List (K × V)) (k : K) (v : V) :
    (SpecMap.replaceFirst k v xs).map (fun e => (e.1, f e.2)) =
      SpecMap.replaceFirst k (f v) (xs.map fun e => (e.1, f e.2)) := by
  induction xs with
  | nil => rfl
  | cons e xs ih =>
    by_cases h : e.1 = k <;> simp [SpecMap.replaceFirst, h, ih]

theorem PoolU64Proofs.pool_map_update {K V W : Type} [DecidableEq K] (f : V → W) (xs : List (K × V)) (k : K) (v : V) :
    (SpecMap.update .sequence xs k v).map (fun e => (e.1, f e.2)) =
      SpecMap.update .sequence (xs.map fun e => (e.1, f e.2)) k (f v) := by
  simp only [SpecMap.update, SpecMap.Discipline.set, SpecMap.seqSet, PoolU64Proofs.pool_map_contains]
  split <;> simp_all [PoolU64Proofs.pool_map_replace]


theorem PoolU64Proofs.pool_update_push_member {K V : Type} [DecidableEq K] (m : SpecMap K V)
    (xs : Array K) (x d : K) (v : V)
    (member : ∀ i : Int, 0 ≤ i → i < ↑xs.size → m.hasKey (xs[i.toNat]?.getD d) = true)
    (i : Int) (ilo : 0 ≤ i) (ihi : i < ↑(xs.size + 1)) :
    (SpecMap.update .sequence m x v).hasKey ((xs.push x)[i.toNat]?.getD d) = true := by
  rw [SpecMap.hasKey_update]
  by_cases h : i.toNat = xs.size
  · simp [h]
  · simp only [Array.getElem?_push, h, ite_false]
    rw [member i ilo (by omega)]
    exact Bool.or_true _

theorem PoolU64Proofs.pool_entries_update_read {K V W : Type} [DecidableEq K] (f : V → W)
    (before after : SpecEntries (K × V)) (key : K) (value : V)
    (updated : after.entries = SpecMap.update .sequence before.entries key value) :
    SpecMap.ofEntries (fun x => x) f after = SpecMap.update .sequence (SpecMap.ofEntries (fun x => x) f before) key (f value) := by
  unfold SpecMap.ofEntries
  rw [updated, PoolU64Proofs.pool_map_update]

theorem PoolU64Proofs.pool_entries_update_read_eq {K V W : Type} [DecidableEq K] (f : V → W)
    (before after : SpecEntries (K × V)) (key : K) (value : V) (typed : SpecMap K W) (typedValue : W)
    (updated : after.entries = SpecMap.update .sequence before.entries key value)
    (read : SpecMap.ofEntries (fun x => x) f before = typed) (valueRead : f value = typedValue) :
    SpecMap.ofEntries (fun x => x) f after = SpecMap.update .sequence typed key typedValue := by
  rw [PoolU64Proofs.pool_entries_update_read f before after key value updated, read, valueRead]

theorem PoolU64Proofs.pool_push_index_injective {α : Type} (xs : Array α) (x d : α)
    (inj : ∀ i : Int, 0 ≤ i → i < ↑xs.size → ∀ j : Int, 0 ≤ j → j < ↑xs.size →
      xs[i.toNat]?.getD d = xs[j.toNat]?.getD d → i = j)
    (absent : ∀ i : Int, 0 ≤ i → i < ↑xs.size → xs[i.toNat]?.getD d ≠ x)
    (i : Int) (ilo : 0 ≤ i) (ihi : i < ↑(xs.size + 1))
    (j : Int) (jlo : 0 ≤ j) (jhi : j < ↑(xs.size + 1))
    (same : (xs.push x)[i.toNat]?.getD d = (xs.push x)[j.toNat]?.getD d) : i = j := by
  by_cases hi : i.toNat = xs.size
  · by_cases hj : j.toNat = xs.size
    · omega
    · simp only [Array.getElem?_push, hi, hj, ite_true, ite_false, Option.getD_some] at same
      exact False.elim (absent j jlo (by omega) same.symm)
  · by_cases hj : j.toNat = xs.size
    · simp only [Array.getElem?_push, hi, hj, ite_true, ite_false, Option.getD_some] at same
      exact False.elim (absent i ilo (by omega) same)
    · apply inj i ilo (by omega) j jlo (by omega)
      simpa only [Array.getElem?_push, hi, hj, ite_false] using same

verify shares by
  on _ { }

verify balance by
  on ensures 1, aborts_if 1 {
    apply lemma_mul_nonneg(num_shares, self.total_coins)
  }
  on _ { }

verify buy_in by
  on aborts_if 1 when aborts at call 1 of add_shares {
    split spec_contains(self, shareholder)
    assert coins_amount > 0
  }
  on aborts_if 1, aborts_if 2, aborts_if 3, aborts_if 4 when returns {
    split spec_contains(self, shareholder)
  }
  on ensures 1 {
    assert self'.shares == spec_set(self.shares, shareholder,
      spec_get(self.shares, shareholder) +
        spec_amount_to_shares_with_total_coins(self, coins_amount, self.total_coins)) by congr
  }
  on _ { }
verify redeem_shares by
  after call 1 of deduct_shares {
    assert self'.total_coins == self.total_coins - redeemed_coins by congr
    assert self'.total_shares == self.total_shares - shares_to_redeem by congr
  }
  on _ { }

verify add_shares by
  on ensures 1 {
    lean =>
      apply VerexIR.SpecMap.eq_writeBack_of_ofEntries
        (key := fun key => key) (value := VerexIR.SpecInt.val)
        (fun _ => rfl) (fun value => VerexIR.SpecInt.ofInt_val _ value)
      apply PoolU64Proofs.pool_entries_update_read_eq
      all_goals try assumption
  }
  on ensures 2 {
    lean =>
      apply VerexIR.SpecMap.eq_writeBack_of_ofEntries
        (key := fun key => key) (value := VerexIR.SpecInt.val)
        (fun _ => rfl) (fun value => VerexIR.SpecInt.ofInt_val _ value)
      try assumption
  }
  on data invariant 2 of Pool {
    lean =>
      try
        first
        | rw (transparency := .default) [add_ensures]
        | have rawUpdate : old_value_0.val.1.entries =
              VerexIR.SpecMap.update .sequence self_shares.entries shareholder named := by assumption
          have readUpdate := congrArg
            (List.map (fun entry => (entry.1, VerexIR.SpecInt.val entry.2))) rawUpdate
          simp only [PoolU64Proofs.pool_map_update] at readUpdate
          change VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val old_value_0.val.1 =
            VerexIR.SpecMap.update .sequence
              (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares)
              shareholder named.val at readUpdate
          rw (transparency := .default) [readUpdate]
          have present : (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares).hasKey shareholder = true := by assumption
          rw (transparency := .default) [VerexIR.SpecMap.size_update_sequence, present, if_pos rfl]
          assumption
  }
  on data invariant 1 of Pool {
    lean =>
      try
        first
        | rw (transparency := .default) [add_ensures]
        | have rawUpdate : old_value_0.val.1.entries =
              VerexIR.SpecMap.update .sequence self_shares.entries shareholder named := by assumption
          have readUpdate := congrArg
            (List.map (fun entry => (entry.1, VerexIR.SpecInt.val entry.2))) rawUpdate
          simp only [PoolU64Proofs.pool_map_update] at readUpdate
          change VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val old_value_0.val.1 =
            VerexIR.SpecMap.update .sequence
              (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares)
              shareholder named.val at readUpdate
          rw (transparency := .default) [readUpdate]
        intros
        first
        | apply PoolU64Proofs.pool_update_push_member
          all_goals assumption
        | rw (transparency := .default) [VerexIR.SpecMap.hasKey_update]
          have oldMember : ∀ k : Int, 0 ≤ k → k < ↑value_4.values.size →
              (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares).hasKey
                (value_4.values[k.toNat]?.getD "") = true := by assumption
          rw (transparency := .default) [oldMember _ (by assumption) (by assumption)]
          exact Bool.or_true _
  }
  on data invariant 4 of Pool {
    lean =>
      try
        intros
        apply PoolU64Proofs.pool_push_index_injective self_shareholders.values shareholder ""
        · assumption
        · intro k klo khi equal
          have oldMember : ∀ k : Int, 0 ≤ k → k < ↑self_shareholders.values.size →
              (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares).hasKey
                (self_shareholders.values[k.toNat]?.getD "") = true := by assumption
          have member : (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares).hasKey
              (self_shareholders.values[k.toNat]?.getD "") = true := by
            exact oldMember k klo khi
          have absent : (VerexIR.SpecMap.ofEntries (fun x => x) VerexIR.SpecInt.val self_shares).hasKey shareholder = false := by assumption
          rw [equal, absent] at member
          contradiction
        all_goals assumption
  }
  on ensures 6 {
    lean =>
      intros
      try (rw (transparency := .default) [Array.getElem?_push_size]; rfl)
  }
  on ensures 7 {
    lean =>
      intros
      rw (transparency := .default) [Array.getElem?_push_lt (by omega),
        Array.getElem?_eq_getElem (by omega)]
  }
  on _ { }
