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

/-- A pushed vector's reads are injective when the vector's are and the pushed
element is none of them; the pushed reads are in range, so their default is
arbitrary (`VerexIR.getElem?_getD_in_bounds`). -/
theorem PoolU64Proofs.push_read_injective {α : Type} {xs : Array α} {x d e : α} {i j : Int}
    (same : (xs.push x)[i.toNat]?.getD e = (xs.push x)[j.toNat]?.getD e)
    (inj : ∀ i : Int, 0 ≤ i → i < ↑xs.size → ∀ j : Int, 0 ≤ j → j < ↑xs.size →
      xs[i.toNat]?.getD d = xs[j.toNat]?.getD d → i = j)
    (absent : ∀ i : Int, 0 ≤ i → i < ↑xs.size → xs[i.toNat]?.getD d ≠ x)
    (ilo : 0 ≤ i) (ihi : i < ↑(xs.size + 1))
    (jlo : 0 ≤ j) (jhi : j < ↑(xs.size + 1)) : i = j := by
  by_cases hi : i.toNat = xs.size
  · by_cases hj : j.toNat = xs.size
    · omega
    · simp only [Array.getElem?_push, hi, hj, ite_true, ite_false, Option.getD_some] at same
      rw [VerexIR.getElem?_getD_in_bounds (values := xs) (index := j.toNat) e d (by omega)] at same
      exact absurd same.symm (absent j jlo (by omega))
  · by_cases hj : j.toNat = xs.size
    · simp only [Array.getElem?_push, hi, hj, ite_true, ite_false, Option.getD_some] at same
      rw [VerexIR.getElem?_getD_in_bounds (values := xs) (index := i.toNat) e d (by omega)] at same
      exact absurd same (absent i ilo (by omega))
    · simp only [Array.getElem?_push, hi, hj, ite_false] at same
      rw [VerexIR.getElem?_getD_in_bounds (values := xs) (index := i.toNat) e d (by omega),
        VerexIR.getElem?_getD_in_bounds (values := xs) (index := j.toNat) e d (by omega)] at same
      exact inj i ilo (by omega) j jlo (by omega) same

/-- No element of a vector whose entries the map holds equals a key the map
does not hold. -/
theorem PoolU64Proofs.absent_of_member {K V : Type} [DecidableEq K] {m : VerexIR.SpecMap K V}
    {xs : Array K} {x d : K}
    (member : ∀ i : Int, 0 ≤ i → i < ↑xs.size → m.hasKey (xs[i.toNat]?.getD d) = true)
    (missing : m.hasKey x = false) :
    ∀ i : Int, 0 ≤ i → i < ↑xs.size → xs[i.toNat]?.getD d ≠ x := by
  intro i ilo ihi equal
  have held := member i ilo ihi
  rw [equal, missing] at held
  exact Bool.noConfusion held

/-- A key the map already holds stays held after an update of any key. -/
theorem PoolU64Proofs.hasKey_update_of_member {K V : Type} [DecidableEq K]
    {m : VerexIR.SpecMap K V} {xs : Array K} {d key : K} {value : V} {i : Int}
    (member : ∀ i : Int, 0 ≤ i → i < ↑xs.size → m.hasKey (xs[i.toNat]?.getD d) = true)
    (ilo : 0 ≤ i) (ihi : i < ↑xs.size) :
    (VerexIR.SpecMap.update .sequence m key value).hasKey (xs[i.toNat]?.getD d) = true := by
  rw [VerexIR.SpecMap.hasKey_update, member i ilo ihi]
  exact Bool.or_true _

/-- The typed update a raw entries update states: the entries of the written
back maps, read back through the table's own reads (`ofEntries`), are the
typed update of the read maps. -/
theorem PoolU64Proofs.typed_update_of_entries {K V V' : Type} [DecidableEq K]
    {unread : K × V' → K × V} {value : V → V'} {updated source : VerexIR.SpecMap K V'}
    {probe : K} {written : V}
    (raw : (VerexIR.SpecMap.writeBack unread updated).entries =
      VerexIR.SpecMap.update .sequence (VerexIR.SpecMap.writeBack unread source).entries probe
        written)
    (readUpdated : VerexIR.SpecMap.ofEntries (fun x => x) value
      (VerexIR.SpecMap.writeBack unread updated) = updated)
    (readSource : VerexIR.SpecMap.ofEntries (fun x => x) value
      (VerexIR.SpecMap.writeBack unread source) = source) :
    updated = VerexIR.SpecMap.update .sequence source probe (value written) := by
  have mapped := congrArg (List.map (fun entry => (entry.1, value entry.2))) raw
  simp only [PoolU64Proofs.pool_map_update] at mapped
  rw [← readUpdated, ← readSource]
  exact mapped

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
      first
      | rw (transparency := .default) [add_ensures]
      | have typedUpdate := PoolU64Proofs.typed_update_of_entries
          (by assumption) (by assumption) (by assumption)
        rw (transparency := .default) [typedUpdate]
      first
      | rw (transparency := .default) [VerexIR.SpecMap.size_update_sequence_present (by assumption)]
      | rw (transparency := .default) [VerexIR.SpecMap.size_update_sequence_absent (by assumption)]
      omega
  }
  on data invariant 1 of Pool {
    lean =>
      first
      | rw (transparency := .default) [add_ensures]
      | have typedUpdate := PoolU64Proofs.typed_update_of_entries
          (by assumption) (by assumption) (by assumption)
        rw (transparency := .default) [typedUpdate]
      intros
      first
      | apply PoolU64Proofs.pool_update_push_member
        all_goals assumption
      | exact PoolU64Proofs.hasKey_update_of_member (by assumption) (by assumption) (by assumption)
  }
  on data invariant 4 of Pool {
    lean =>
      try
        intros
        exact PoolU64Proofs.push_read_injective (by assumption) (by assumption)
          (PoolU64Proofs.absent_of_member (by assumption) (by assumption))
          (by assumption) (by assumption) (by assumption) (by assumption)
  }
  on ensures 6 {
    lean =>
      intros
      -- Optional: this clause's leaves reach the script in both the `SpecVector.push`
      -- and the raw `Array.push` spelling, and one has no push at all; where the
      -- conversion applies its room premise is proved, and the no-push leaf is left
      -- to the block's closer.
      try rw (transparency := .default) [VerexIR.SpecVector.values_push _ (by omega)]
      try (rw (transparency := .default) [Array.getElem?_push_size]; rfl)
  }
  on ensures 7 {
    lean =>
      intros
      -- Optional wrapper conversion only; the `Array` rewrites below stay required.
      try rw (transparency := .default) [VerexIR.SpecVector.values_push _ (by omega)]
      rw (transparency := .default) [Array.getElem?_push_lt (by omega),
        Array.getElem?_eq_getElem (by omega)]
  }
  on _ { }
