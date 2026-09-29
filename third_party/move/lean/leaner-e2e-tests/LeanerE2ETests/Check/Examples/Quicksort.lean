-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Quicksort

A generic in-place quicksort: a Lomuto `partition` over a
type parameter, the recursive `quick_sort_range` calling itself at its own
type parameter, and `quick_sort`. Sortedness is stated by the structural
order and the permutation of each range by counts, a recursive
specification function. The closer leaves the effect of a swap on each
partition invariant and the composition of the two recursive sorts with
the partition; authored proofs discharge them with count lemmas proved
through the generated definition's unfolding theorem. `quick_sort` closes
automatically.
-/

namespace LeanerLang.Tests.Check.Examples.Quicksort

set_option leaner.verifyHeartbeats 400000

leaner module 0x42::quicksort where
  -- ## Partition

  struct PartitionResult {T has Copy, Drop, Store} has Copy, Drop, Store where
    values : Vector<T>
    pivot : u64

  -- Lomuto partition of `[store, pivot_index]`.
  fun partition {T has Copy, Drop, Store}(values : Vector<T>, pivot_index : u64, store : u64) ->
      PartitionResult<T> := do
    let original := values
    let len := values.length
    let pivot := values[pivot_index]
    let mut values := values
    let mut scan := store
    let mut lo := store
    while scan < pivot_index do
      if core.prim.compare(&values[scan], &pivot) < 0 then
        values := core.prim.swapVector(values, lo, scan)
        lo := lo + 1
      scan := scan + 1
    where
      invariant values.length == len && pivot_index < len && store <= lo && lo <= scan &&
        scan <= pivot_index
      invariant values[pivot_index] == pivot
      invariant forall (i : Int), store <= i && i < lo ==> core.prim.compare(values[i], pivot) < 0
      invariant forall (i : Int), lo <= i && i < scan ==> core.prim.compare(values[i], pivot) >= 0
      invariant unchanged_outside(original, values, store, pivot_index + 1)
      invariant same_counts(original, values, store, pivot_index + 1)
    values := core.prim.swapVector(values, lo, pivot_index)
    new PartitionResult<T> { values := values, pivot := lo }
  spec partition where
    requires store <= pivot_index && pivot_index < values.length
    ensures result.values.length == values.length
    ensures store <= result.pivot && result.pivot <= pivot_index
    ensures result.values[result.pivot] == values[pivot_index]
    ensures forall (i : Int), store <= i && i < result.pivot ==>
      core.prim.compare(result.values[i], values[pivot_index]) < 0
    ensures forall (i : Int), result.pivot < i && i <= pivot_index ==>
      core.prim.compare(result.values[i], values[pivot_index]) >= 0
    ensures unchanged_outside(values, result.values, store, pivot_index + 1)
    ensures same_counts(values, result.values, store, pivot_index + 1)
    aborts_if false

  spec fun unchanged_outside {T}(before : Vector<T>, after : Vector<T>, lo : Int, hi : Int) : Bool :=
    forall (i : Int), 0 <= i && i < before.length && (i < lo || hi <= i) ==> after[i] == before[i]

  spec fun same_counts {T}(before : Vector<T>, after : Vector<T>, lo : Int, hi : Int) : Bool :=
    forall (x : T), count(before, x, lo, hi) == count(after, x, lo, hi)

  -- Occurrences of `needle` among the positions `[lo, hi)`.
  spec fun count {T}(values : Vector<T>, needle : T, lo : Int, hi : Int) : Int decreases hi - lo :=
    if hi <= lo then 0
    else count(values, needle, lo, hi - 1) + (if values[hi - 1] == needle then 1 else 0)

  open LeanerIR Classical in
  /-- The generated definition of `count`, at a vector of elements. -/
  theorem count_unfold (a : Array RuntimeValue) (x : RuntimeValue) (lo hi : Int) :
      count.spec (.vector a, x, lo, hi, ()) =
        if hi ≤ lo then 0 else
          count.spec (.vector a, x, lo, hi - 1, ()) +
            if a[(hi - 1).toNat]?.getD .unit = x then 1 else 0 := by
    rw [count.spec.unfold]
    simp only [RuntimeValue.field, dite_eq_ite]
    congr

  open LeanerIR Classical in
  /-- An update outside `[lo, hi)` leaves the count over it unchanged. -/
  theorem count_set_outside (a : Array RuntimeValue) (x y : RuntimeValue) (i : Nat) :
      ∀ (k : Nat) (lo hi : Int), (hi - lo).toNat = k → 0 ≤ lo → ((i : Int) < lo ∨ hi ≤ i) →
        count.spec (.vector (a.setIfInBounds i y), x, lo, hi, ()) =
          count.spec (.vector a, x, lo, hi, ()) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro lo hi hk hlo hout
      rw [count_unfold (a.setIfInBounds i y), count_unfold a]
      by_cases hle : hi ≤ lo
      · rw [if_pos hle, if_pos hle]
      · rw [if_neg hle, if_neg hle,
          ih (hi - 1 - lo).toNat (by omega) lo (hi - 1) rfl hlo (by omega),
          Array.getElem?_setIfInBounds_ne (by omega)]

  open LeanerIR Classical in
  /-- An update inside `[lo, hi)` trades the old element's count for the new one's. -/
  theorem count_set_inside (a : Array RuntimeValue) (x y : RuntimeValue) (i : Nat)
      (bound : i < a.size) :
      ∀ (k : Nat) (lo hi : Int), (hi - lo).toNat = k → 0 ≤ lo → lo ≤ i → (i : Int) < hi →
        count.spec (.vector (a.setIfInBounds i y), x, lo, hi, ()) =
          count.spec (.vector a, x, lo, hi, ()) -
            (if a[i] = x then 1 else 0) + (if y = x then 1 else 0) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro lo hi hk hlo hlow hhigh
      rw [count_unfold (a.setIfInBounds i y), count_unfold a]
      have hlt : ¬hi ≤ lo := by omega
      rw [if_neg hlt]
      by_cases hlast : (hi - 1).toNat = i
      · rw [count_set_outside a x y i (hi - 1 - lo).toNat lo (hi - 1) rfl hlo (by omega), hlast,
          Array.getElem?_setIfInBounds_self, if_pos bound, Option.getD_some,
          Array.getElem?_eq_getElem bound, Option.getD_some]
        omega
      · rw [ih (hi - 1 - lo).toNat (by omega) lo (hi - 1) rfl hlo hlow (by omega),
          Array.getElem?_setIfInBounds_ne (fun h => hlast h.symm)]
        omega

  open LeanerIR Classical in
  /-- Swapping two positions inside `[lo, hi)` preserves every count over it. -/
  theorem count_swap (a : Array RuntimeValue) (x : RuntimeValue) (i j : Nat) (hi : i < a.size)
      (hj : j < a.size) (lo high : Int) (hlo : 0 ≤ lo) (hi1 : lo ≤ i) (hi2 : (i : Int) < high)
      (hj1 : lo ≤ j) (hj2 : (j : Int) < high) :
      count.spec (.vector ((a.setIfInBounds i a[j]).setIfInBounds j a[i]), x, lo,
          high, ()) =
        count.spec (.vector a, x, lo, high, ()) := by
    rw [count_set_inside (a.setIfInBounds i a[j]) x a[i] j (by simpa using hj) _ lo high rfl hlo hj1
        hj2,
      count_set_inside a x a[j] i hi _ lo high rfl hlo hi1 hi2]
    rw [Array.getElem_setIfInBounds (by simpa using hj), ite_self]
    omega

  open LeanerIR Classical in
  /-- The same, for a vector of encoded elements. -/
  theorem count_swap_map {α : Type} (f : α → RuntimeValue) (a : Array α) (x : RuntimeValue)
      (i j : Nat) (hi : i < a.size) (hj : j < a.size) (lo high : Int) (hlo : 0 ≤ lo)
      (hi1 : lo ≤ i) (hi2 : (i : Int) < high) (hj1 : lo ≤ j) (hj2 : (j : Int) < high) :
      count.spec
          (.vector (Array.map f ((a.setIfInBounds i a[j]).setIfInBounds j a[i])), x, lo, high, ()) =
        count.spec (.vector (Array.map f a), x, lo, high, ()) := by
    rw [Array.map_setIfInBounds, Array.map_setIfInBounds]
    have e1 : f a[j] = (Array.map f a)[j]'(by simpa using hj) := by simp
    have e2 : f a[i] = (Array.map f a)[i]'(by simpa using hi) := by simp
    rw [e1, e2]
    exact count_swap (Array.map f a) x i j (by simpa using hi) (by simpa using hj) lo high hlo hi1 hi2
      hj1 hj2

  open LeanerIR in
  /-- An element found at a position is the lookup there. -/
  theorem lookup_eq {α : Type} (f : α → RuntimeValue) (xs : Array α) (k : Int) (a : α)
      (found : xs[k.toNat]? = some a) : f a = (Option.map f xs[k.toNat]?).getD .unit := by
    rw [found]; rfl

  open LeanerIR in
  /-- A lookup in a vector after swapping two of its positions. -/
  theorem swap_lookup {α : Type} (f : α → RuntimeValue) (xs : Array α) (i j : Int) (a b : α)
      (ha : xs[i.toNat]? = some a) (hb : xs[j.toNat]? = some b) (k : Int) (hi : 0 ≤ i)
      (hj : 0 ≤ j) (hk : 0 ≤ k) :
      (Option.map f ((xs.setIfInBounds i.toNat b).setIfInBounds j.toNat a)[k.toNat]?).getD .unit =
        if j = k then (Option.map f xs[i.toNat]?).getD .unit
        else if i = k then (Option.map f xs[j.toNat]?).getD .unit
        else (Option.map f xs[k.toNat]?).getD .unit := by
    have ib := (Array.getElem?_eq_some_iff.mp ha).1
    have jb := (Array.getElem?_eq_some_iff.mp hb).1
    simp only [Array.getElem?_setIfInBounds, Array.size_setIfInBounds, ib, jb, if_true, ha, hb]
    by_cases e1 : j = k
    · have : j.toNat = k.toNat := by omega
      simp [e1]
    · have : ¬j.toNat = k.toNat := by omega
      simp only [this, e1, if_false]
      by_cases e2 : i = k
      · have : i.toNat = k.toNat := by omega
        simp [e2]
      · have : ¬i.toNat = k.toNat := by omega
        simp [this, e2]

  open LeanerIR Classical in
  /-- Swapping two found positions inside `[lo, hi)` preserves every count over it. -/
  theorem count_swap_found {α : Type} (f : α → RuntimeValue) (xs : Array α) (x : RuntimeValue)
      (i j : Int) (a b : α) (ha : xs[i.toNat]? = some a) (hb : xs[j.toNat]? = some b)
      (lo high : Int) (hlo : 0 ≤ lo) (hi1 : lo ≤ i) (hi2 : i < high) (hj1 : lo ≤ j) (hj2 : j < high) :
      count.spec
          (.vector (Array.map f ((xs.setIfInBounds i.toNat b).setIfInBounds j.toNat a)), x, lo,
            high, ()) =
        count.spec (.vector (Array.map f xs), x, lo, high, ()) := by
    obtain ⟨ib, rfl⟩ := Array.getElem?_eq_some_iff.mp ha
    obtain ⟨jb, rfl⟩ := Array.getElem?_eq_some_iff.mp hb
    exact count_swap_map f xs x i.toNat j.toNat ib jb lo high hlo (by omega) (by omega) (by omega)
      (by omega)

  verify partition by
    -- A lookup in a swapped vector is a lookup in the vector.
    all_goals (intros; try (rw [swap_lookup (ha := ?_) (hb := ?_) (hi := ?_) (hj := ?_) (hk := ?_)] <;>
      first | assumption | omega | skip))
    -- A lookup of an element found before is that element's lookup, and an
    -- invariant instance closes the rest.
    all_goals repeat' split
    all_goals try first
      | omega
      | assumption
      | leaner_denote_instance
      | (rw [‹∀ x : LeanerIR.RuntimeValue, count.spec
            (LeanerIR.RuntimeValue.vector (Array.map _ _), x, _, _, ()) = _›]
         symm
         apply count_swap_found <;> first | assumption | omega)
    -- The pivot is the original vector's element at the pivot position.
    all_goals (try rw [← lookup_eq (found := ‹original.values[_]? = some _›)])
    all_goals (try first | assumption | leaner_denote_instance)
    case leaf_5.isTrue =>
      rw [show lo.val = pivot_index.val by omega]
      assumption
    case leaf_6 =>
      by_cases before : i < scan.val
      · leaner_denote_instance
      · rw [show i = scan.val by omega, ← lookup_eq (found := ‹values.values[scan.val.toNat]? = some _›)]
        assumption
    case leaf_10.isTrue =>
      rw [show lo.val = scan.val by omega,
        ← lookup_eq (found := ‹values.values[scan.val.toNat]? = some _›)]
      assumption
    case leaf_10.isFalse.isTrue =>
      rw [← lookup_eq (found := ‹values.values[scan.val.toNat]? = some _›)]
      assumption

  -- ## Sorting

  fun quick_sort_range {T has Copy, Drop, Store}(values : Vector<T>, low : u64, high : u64) ->
      Vector<T> :=
    if low < high && 1 < high - low then do
      let partitioned := partition::<T>(values, high - 1, low)
      let left := quick_sort_range::<T>(partitioned.values, low, partitioned.pivot)
      quick_sort_range::<T>(left, partitioned.pivot + 1, high)
    else values
  spec quick_sort_range where
    requires low <= high && high <= values.length
    ensures result.length == values.length
    ensures sorted_range(result, low, high)
    ensures unchanged_outside(values, result, low, high)
    ensures same_counts(values, result, low, high)
    aborts_if false

  spec fun sorted_range {T}(values : Vector<T>, lo : Int, hi : Int) : Bool :=
    forall (i : Int), forall (j : Int),
      lo <= i && i < j && j < hi ==> core.prim.compare(values[i], values[j]) <= 0

  open LeanerIR Classical in
  /-- A count splits at any point of its range. -/
  theorem count_split (xs : Array RuntimeValue) (x : RuntimeValue) (lo mid : Int) :
      ∀ (k : Nat) (hi : Int), (hi - mid).toNat = k → lo ≤ mid → mid ≤ hi →
        count.spec (.vector xs, x, lo, hi, ()) =
          count.spec (.vector xs, x, lo, mid, ()) +
            count.spec (.vector xs, x, mid, hi, ()) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro hi hk hlm hmh
      by_cases same : hi = mid
      · subst same
        rw [count_unfold xs x hi hi, if_pos (Int.le_refl _)]
        omega
      · have outer : ¬hi ≤ lo := by omega
        have inner : ¬hi ≤ mid := by omega
        rw [count_unfold xs x lo hi, count_unfold xs x mid hi, if_neg outer, if_neg inner,
          ih (hi - 1 - mid).toNat (by omega) (hi - 1) rfl hlm (by omega)]
        omega

  open LeanerIR Classical in
  /-- Arrays agreeing on a range count alike over it. -/
  theorem count_congr (xs ys : Array RuntimeValue) (x : RuntimeValue) (lo : Int) :
      ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k →
        (∀ i : Int, lo ≤ i → i < hi → xs[i.toNat]?.getD .unit = ys[i.toNat]?.getD .unit) →
        count.spec (.vector xs, x, lo, hi, ()) =
          count.spec (.vector ys, x, lo, hi, ()) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro hi hk agree
      rw [count_unfold xs x lo hi, count_unfold ys x lo hi]
      by_cases hle : hi ≤ lo
      · rw [if_pos hle, if_pos hle]
      · rw [if_neg hle, if_neg hle, ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl
          (fun i h1 h2 => agree i h1 (by omega)), agree (hi - 1) (by omega) (by omega)]

  open LeanerIR Classical in
  theorem count_nonneg (xs : Array RuntimeValue) (x : RuntimeValue) (lo : Int) :
      ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k →
        0 ≤ count.spec (.vector xs, x, lo, hi, ()) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro hi hk
      rw [count_unfold xs x lo hi]
      by_cases hle : hi ≤ lo
      · rw [if_pos hle]; omega
      · rw [if_neg hle]
        have := ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl
        split <;> omega

  open LeanerIR Classical in
  /-- An element of a range is counted there. -/
  theorem count_pos (xs : Array RuntimeValue) (lo i : Int) :
      ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k → lo ≤ i → i < hi →
        1 ≤ count.spec (.vector xs, xs[i.toNat]?.getD .unit, lo, hi, ()) := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro hi hk h1 h2
      rw [count_unfold xs _ lo hi, if_neg (by omega)]
      have := count_nonneg xs (xs[i.toNat]?.getD .unit) lo _ (hi - 1) rfl
      by_cases last : hi - 1 = i
      · subst last
        rw [if_pos rfl]
        omega
      · have := ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl h1 (by omega)
        split <;> omega

  open LeanerIR Classical in
  /-- What is counted in a range is an element of it. -/
  theorem exists_of_count_pos (xs : Array RuntimeValue) (x : RuntimeValue) (lo : Int) :
      ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k →
        1 ≤ count.spec (.vector xs, x, lo, hi, ()) →
        ∃ i : Int, lo ≤ i ∧ i < hi ∧ xs[i.toNat]?.getD .unit = x := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro hi hk pos
      rw [count_unfold xs x lo hi] at pos
      by_cases hle : hi ≤ lo
      · rw [if_pos hle] at pos; omega
      · rw [if_neg hle] at pos
        by_cases hit : xs[(hi - 1).toNat]?.getD .unit = x
        · exact ⟨hi - 1, by omega, by omega, hit⟩
        · rw [if_neg hit] at pos
          obtain ⟨i, h1, h2, h3⟩ := ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl (by omega)
          exact ⟨i, h1, by omega, h3⟩

  open LeanerIR Classical in
  /-- The frame of a sorted range: partition, then the two recursive sorts. -/
  theorem sort_frame {α : Type} (f : α → RuntimeValue) (v P L R : Array α) (low high p : Int)
      (hvP : ∀ i : Int, 0 ≤ i → i < v.size → i < low ∨ high ≤ i →
        (Option.map f P[i.toNat]?).getD .unit = (Option.map f v[i.toNat]?).getD .unit)
      (hPL : ∀ i : Int, 0 ≤ i → i < P.size → i < low ∨ p ≤ i →
        (Option.map f L[i.toNat]?).getD .unit = (Option.map f P[i.toNat]?).getD .unit)
      (hLR : ∀ i : Int, 0 ≤ i → i < L.size → i < p + 1 ∨ high ≤ i →
        (Option.map f R[i.toNat]?).getD .unit = (Option.map f L[i.toNat]?).getD .unit)
      (sP : (P.size : Int) = v.size) (sL : (L.size : Int) = P.size) (hp : low ≤ p)
      (hp2 : p ≤ high - 1) :
      ∀ i : Int, 0 ≤ i → i < v.size → i < low ∨ high ≤ i →
        (Option.map f R[i.toNat]?).getD .unit = (Option.map f v[i.toNat]?).getD .unit := by
    intro i h0 h1 hout
    rw [hLR i h0 (by omega) (by omega), hPL i h0 (by omega) (by omega), hvP i h0 h1 hout]

  open LeanerIR Classical in
  /-- The counts of a sorted range: partition, then the two recursive sorts. -/
  theorem sort_counts {α : Type} (f : α → RuntimeValue) (v P L R : Array α) (low high p : Int)
      (cvP : ∀ x, count.spec (.vector (Array.map f v), x, low, high, ()) =
        count.spec (.vector (Array.map f P), x, low, high, ()))
      (cPL : ∀ x, count.spec (.vector (Array.map f P), x, low, p, ()) =
        count.spec (.vector (Array.map f L), x, low, p, ()))
      (hPL : ∀ i : Int, 0 ≤ i → i < P.size → i < low ∨ p ≤ i →
        (Option.map f L[i.toNat]?).getD .unit = (Option.map f P[i.toNat]?).getD .unit)
      (cLR : ∀ x, count.spec (.vector (Array.map f L), x, p + 1, high, ()) =
        count.spec (.vector (Array.map f R), x, p + 1, high, ()))
      (hLR : ∀ i : Int, 0 ≤ i → i < L.size → i < p + 1 ∨ high ≤ i →
        (Option.map f R[i.toNat]?).getD .unit = (Option.map f L[i.toNat]?).getD .unit)
      (sP : (P.size : Int) = v.size) (sL : (L.size : Int) = P.size) (hhigh : high ≤ v.size)
      (hlow : 0 ≤ low) (hp : low ≤ p) (hp2 : p ≤ high - 1) :
      ∀ x, count.spec (.vector (Array.map f v), x, low, high, ()) =
        count.spec (.vector (Array.map f R), x, low, high, ()) := by
    intro x
    have upper : count.spec (.vector (Array.map f P), x, p, high, ()) =
        count.spec (.vector (Array.map f L), x, p, high, ()) :=
      count_congr _ _ x p _ high rfl fun i h1 h2 => by
        simp only [Array.getElem?_map]
        exact (hPL i (by omega) (by omega) (Or.inr h1)).symm
    have lower : count.spec (.vector (Array.map f R), x, low, p + 1, ()) =
        count.spec (.vector (Array.map f L), x, low, p + 1, ()) :=
      count_congr _ _ x low _ (p + 1) rfl fun i h1 h2 => by
        simp only [Array.getElem?_map]
        exact hLR i (by omega) (by omega) (Or.inl h2)
    rw [cvP x, count_split _ x low p _ high rfl hp (by omega),
      count_split (Array.map f R) x low (p + 1) _ high rfl (by omega) (by omega), cPL x, upper,
      lower, ← cLR x, ← count_split _ x low p _ high rfl hp (by omega),
      ← count_split _ x low (p + 1) _ high rfl (by omega) (by omega)]

  open LeanerIR Classical in
  /-- An element of a range whose counts are those of another range is an
  element of that one. -/
  theorem element_of_counts {α : Type} (f : α → RuntimeValue) (xs ys : Array α) (lo hi i : Int)
      (counts : ∀ x, count.spec (.vector (Array.map f xs), x, lo, hi, ()) =
        count.spec (.vector (Array.map f ys), x, lo, hi, ()))
      (h1 : lo ≤ i) (h2 : i < hi) :
      ∃ k : Int, lo ≤ k ∧ k < hi ∧
        (Option.map f xs[k.toNat]?).getD .unit = (Option.map f ys[i.toNat]?).getD .unit := by
    have pos := count_pos (Array.map f ys) lo i _ hi rfl h1 h2
    rw [← counts] at pos
    obtain ⟨k, k1, k2, k3⟩ := exists_of_count_pos _ _ lo _ hi rfl pos
    simp only [Array.getElem?_map] at k3
    exact ⟨k, k1, k2, k3⟩

  open LeanerIR Classical in
  /-- The order of a sorted range: partition, then the two recursive sorts. -/
  theorem sort_order {α : Type} (f : α → RuntimeValue) (order : RuntimeValue → RuntimeValue → Ordering) [Std.TransCmp order]
      (P L R : Array α) (low high p : Int) (pivot : RuntimeValue)
      (hpivot : (Option.map f P[p.toNat]?).getD .unit = pivot)
      (below : ∀ i : Int, low ≤ i → i < p →
        order ((Option.map f P[i.toNat]?).getD .unit) pivot = .lt)
      (above : ∀ i : Int, p < i → i ≤ high - 1 →
        ¬order ((Option.map f P[i.toNat]?).getD .unit) pivot = .lt)
      (cPL : ∀ x, count.spec (.vector (Array.map f P), x, low, p, ()) =
        count.spec (.vector (Array.map f L), x, low, p, ()))
      (hPL : ∀ i : Int, 0 ≤ i → i < P.size → i < low ∨ p ≤ i →
        (Option.map f L[i.toNat]?).getD .unit = (Option.map f P[i.toNat]?).getD .unit)
      (cLR : ∀ x, count.spec (.vector (Array.map f L), x, p + 1, high, ()) =
        count.spec (.vector (Array.map f R), x, p + 1, high, ()))
      (hLR : ∀ i : Int, 0 ≤ i → i < L.size → i < p + 1 ∨ high ≤ i →
        (Option.map f R[i.toNat]?).getD .unit = (Option.map f L[i.toNat]?).getD .unit)
      (sortedL : ∀ i j : Int, low ≤ i → i < j → j < p →
        ¬order ((Option.map f L[i.toNat]?).getD .unit) ((Option.map f L[j.toNat]?).getD .unit) =
          .gt)
      (sortedR : ∀ i j : Int, p + 1 ≤ i → i < j → j < high →
        ¬order ((Option.map f R[i.toNat]?).getD .unit) ((Option.map f R[j.toNat]?).getD .unit) =
          .gt)
      (sL : (L.size : Int) = P.size) (sP : high ≤ P.size) (hlow : 0 ≤ low) (hp : low ≤ p)
      (hp2 : p ≤ high - 1) :
      ∀ i j : Int, low ≤ i → i < j → j < high →
        ¬order ((Option.map f R[i.toNat]?).getD .unit) ((Option.map f R[j.toNat]?).getD .unit) =
          .gt := by
    -- The lower part lies below the pivot, the pivot stays, the upper part
    -- lies above it.
    have lowerPart : ∀ k : Int, low ≤ k → k < p →
        order ((Option.map f R[k.toNat]?).getD .unit) pivot = .lt := by
      intro k h1 h2
      rw [hLR k (by omega) (by omega) (Or.inl (by omega))]
      obtain ⟨k', k1, k2, k3⟩ := element_of_counts f P L low p k cPL h1 h2
      rw [← k3]
      exact below k' k1 k2
    have atPivot : (Option.map f R[p.toNat]?).getD .unit = pivot := by
      rw [hLR p (by omega) (by omega) (Or.inl (by omega)),
        hPL p (by omega) (by omega) (Or.inr (Int.le_refl _)), hpivot]
    have upperPart : ∀ k : Int, p + 1 ≤ k → k < high →
        ¬order pivot ((Option.map f R[k.toNat]?).getD .unit) = .gt := by
      intro k h1 h2
      obtain ⟨k', k1, k2, k3⟩ := element_of_counts f L R (p + 1) high k cLR h1 h2
      rw [← k3, hPL k' (by omega) (by omega) (Or.inr (by omega)), Std.OrientedCmp.gt_iff_lt]
      exact above k' (by omega) (by omega)
    intro i j hi hij hj
    by_cases jl : j < p
    · rw [hLR i (by omega) (by omega) (Or.inl (by omega)),
        hLR j (by omega) (by omega) (Or.inl (by omega))]
      exact sortedL i j hi hij jl
    by_cases ih : p + 1 ≤ i
    · exact sortedR i j ih hij hj
    have leIsLE : ∀ {xs ys : RuntimeValue}, ¬order xs ys = .gt → (order xs ys).isLE = true := by
      intro xs ys h
      cases e : order xs ys <;> simp_all
    by_cases ip : i = p
    · subst ip
      rw [atPivot]
      exact upperPart j (by omega) hj
    · have below' := lowerPart i hi (by omega)
      by_cases jp : j = p
      · subst jp
        rw [atPivot, below']
        simp
      · rw [Std.TransCmp.lt_of_lt_of_isLE below' (leIsLE (upperPart j (by omega) hj))]
        simp

  verify quick_sort_range by
    case leaf_1 => apply sort_counts <;> assumption
    case leaf_2 => apply sort_frame <;> assumption
    case leaf_3 => apply sort_order <;> first | assumption | omega

  -- Generic in-place sort by the structural order.
  public fun quick_sort {T has Copy, Drop, Store}(values : Vector<T>) -> Vector<T> :=
    quick_sort_range::<T>(values, 0, values.length)
  spec quick_sort where
    ensures result.length == values.length
    ensures sorted_range(result, 0, result.length)
    ensures same_counts(values, result, 0, values.length)
    aborts_if false

  -- ## Execution

  -- Concrete entry points for execution.
  fun sort_u64(values : Vector<u64>) -> Vector<u64> := quick_sort::<u64>(values)

  fun sort_bool(values : Vector<Bool>) -> Vector<Bool> := quick_sort::<Bool>(values)

  fun sort_vectors(values : Vector<Vector<u64> >) -> Vector<Vector<u64> > :=
    quick_sort::<Vector<u64> >(values)

-- Run the functions on concrete inputs in the interpreter and compare the
-- outcomes.
open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let numbers (values : List Int) : RuntimeValue := .vector (values.map .integer).toArray
  let bools (values : List Bool) : RuntimeValue := .vector (values.map .bool).toArray
  assertRuns `«0x42».quicksort #[
    ⟨"sort_u64", #[numbers [5, 1, 4, 2, 3]], .returned #[numbers [1, 2, 3, 4, 5]], {}⟩,
    ⟨"sort_u64", #[numbers []], .returned #[numbers []], {}⟩,
    ⟨"sort_u64", #[numbers [2, 1, 2]], .returned #[numbers [1, 2, 2]], {}⟩,
    ⟨"sort_bool", #[bools [true, false, true]], .returned #[bools [false, true, true]], {}⟩,
    ⟨"sort_vectors", #[.vector #[numbers [1, 3], numbers [1, 2], numbers [0, 9]]],
      .returned #[.vector #[numbers [0, 9], numbers [1, 2], numbers [1, 3]]], {}⟩]

end LeanerLang.Tests.Check.Examples.Quicksort
