-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Quicksort

A generic in-place quicksort: a Lomuto `partition` over a
type parameter, the recursive `quick_sort_range` calling itself at its own
type parameter, and `quick_sort`. Sortedness is stated by the structural
order and the permutation of each range by counts, a recursive
specification function, defined at the type parameter (`count.at_T0.spec`).
The solver leaves the effect of a swap on each partition invariant and the
composition of the two recursive sorts with the partition; authored proofs
discharge them with count lemmas over arrays (`Arrays.cnt`), which the
definition's unfolding theorem equates with the generated definition.
`quick_sort` and its concrete callers close automatically, through
`quick_sort_range`'s contract specialized at their type arguments.
-/

namespace VerexSL.Tests.Check.Examples.Quicksort

set_option verex.verifyHeartbeats 400000

namespace Arrays

/-! ## Counts over arrays

The generated `count` reads a vector's values only; its facts are proved
over arrays (`cnt`), a missing position reading as `d`. -/

/-- Occurrences of `x` among the positions `[lo, hi)` of an array. -/
def cnt {α : Type} [DecidableEq α] (d : α) (a : Array α) (x : α) (lo hi : Int) : Int :=
  if hi ≤ lo then 0 else cnt d a x lo (hi - 1) + if a[(hi - 1).toNat]?.getD d = x then 1 else 0
termination_by (hi - lo).toNat
decreasing_by omega

theorem cnt_unfold {α : Type} [DecidableEq α] (d : α) (a : Array α) (x : α) (lo hi : Int) :
    cnt d a x lo hi =
      if hi ≤ lo then 0 else cnt d a x lo (hi - 1) + if a[(hi - 1).toNat]?.getD d = x then 1 else 0 := by
  rw [cnt]

/-- An update outside `[lo, hi)` leaves the count over it unchanged. -/
theorem cnt_set_outside {α : Type} [DecidableEq α] (d : α) (a : Array α) (x y : α) (i : Nat) :
    ∀ (k : Nat) (lo hi : Int), (hi - lo).toNat = k → 0 ≤ lo → ((i : Int) < lo ∨ hi ≤ i) →
      cnt d (a.setIfInBounds i y) x lo hi = cnt d a x lo hi := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro lo hi hk hlo hout
    rw [cnt_unfold d (a.setIfInBounds i y), cnt_unfold d a]
    by_cases hle : hi ≤ lo
    · rw [if_pos hle, if_pos hle]
    · rw [if_neg hle, if_neg hle,
        ih (hi - 1 - lo).toNat (by omega) lo (hi - 1) rfl hlo (by omega),
        Array.getElem?_setIfInBounds_ne (by omega)]

/-- An update inside `[lo, hi)` trades the old element's count for the new one's. -/
theorem cnt_set_inside {α : Type} [DecidableEq α] (d : α) (a : Array α) (x y : α) (i : Nat)
    (bound : i < a.size) :
    ∀ (k : Nat) (lo hi : Int), (hi - lo).toNat = k → 0 ≤ lo → lo ≤ i → (i : Int) < hi →
      cnt d (a.setIfInBounds i y) x lo hi =
        cnt d a x lo hi - (if a[i] = x then 1 else 0) + (if y = x then 1 else 0) := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro lo hi hk hlo hlow hhigh
    rw [cnt_unfold d (a.setIfInBounds i y), cnt_unfold d a]
    have hlt : ¬hi ≤ lo := by omega
    rw [if_neg hlt, if_neg hlt]
    by_cases hlast : (hi - 1).toNat = i
    · rw [cnt_set_outside d a x y i (hi - 1 - lo).toNat lo (hi - 1) rfl hlo (by omega), hlast,
        Array.getElem?_setIfInBounds_self, if_pos bound, Option.getD_some,
        Array.getElem?_eq_getElem bound, Option.getD_some]
      omega
    · rw [ih (hi - 1 - lo).toNat (by omega) lo (hi - 1) rfl hlo hlow (by omega),
        Array.getElem?_setIfInBounds_ne (fun h => hlast h.symm)]
      omega

/-- Swapping two positions inside `[lo, hi)` preserves every count over it. -/
theorem cnt_swap {α : Type} [DecidableEq α] (d : α) (a : Array α) (x : α) (i j : Nat)
    (hi : i < a.size) (hj : j < a.size) (lo high : Int) (hlo : 0 ≤ lo) (hi1 : lo ≤ i)
    (hi2 : (i : Int) < high) (hj1 : lo ≤ j) (hj2 : (j : Int) < high) :
    cnt d ((a.setIfInBounds i a[j]).setIfInBounds j a[i]) x lo high = cnt d a x lo high := by
  rw [cnt_set_inside d (a.setIfInBounds i a[j]) x a[i] j (by simpa using hj) _ lo high rfl hlo hj1
      hj2,
    cnt_set_inside d a x a[j] i hi _ lo high rfl hlo hi1 hi2]
  rw [Array.getElem_setIfInBounds (by simpa using hj), ite_self]
  omega

/-- A certified swap inside the range preserves counts. -/
theorem cnt_swapIfInBounds {α : Type} [DecidableEq α] (d : α) (xs : Array α) (x : α)
    (i j : Int) (ib : i.toNat < xs.size) (jb : j.toNat < xs.size)
    (lo high : Int) (hlo : 0 ≤ lo) (hi1 : lo ≤ i) (hi2 : i < high) (hj1 : lo ≤ j) (hj2 : j < high) :
    cnt d (xs.swapIfInBounds i.toNat j.toNat) x lo high = cnt d xs x lo high := by
  rw [Array.swapIfInBounds_def, dif_pos ib, dif_pos jb, Array.swap_def]
  simpa only [Array.setIfInBounds, dif_pos ib, Array.size_set, dif_pos jb] using
    (cnt_swap d xs x i.toNat j.toNat ib jb lo high hlo (by omega) (by omega) (by omega)
      (by omega))

/-- An element found at a position is the lookup there. -/
theorem lookup_of_found {α : Type} (d : α) (xs : Array α) (k : Int) (a : α)
    (found : xs[k.toNat]? = some a) : xs[k.toNat]?.getD d = a := by
  rw [found]; rfl

/-- A lookup in a vector after a certified swap of two of its positions. -/
theorem swap_lookup {α : Type} (d : α) (xs : Array α) (i j k : Int)
    (ib : i.toNat < xs.size) (jb : j.toNat < xs.size) (hi : 0 ≤ i) (hj : 0 ≤ j) (hk : 0 ≤ k) :
    (xs.swapIfInBounds i.toNat j.toNat)[k.toNat]?.getD d =
      if j = k then xs[i.toNat]?.getD d
      else if i = k then xs[j.toNat]?.getD d
      else xs[k.toNat]?.getD d := by
  rw [Array.swapIfInBounds_def, dif_pos ib, dif_pos jb, Array.swap_def]
  simp only [Array.getElem?_set]
  by_cases e1 : j = k
  · simp [e1, Array.getElem?_eq_getElem ib]
  · have : ¬j.toNat = k.toNat := by omega
    simp only [this, e1, if_false]
    by_cases e2 : i = k
    · simp [e2, Array.getElem?_eq_getElem jb]
    · have : ¬i.toNat = k.toNat := by omega
      simp [this, e2]

/-- A count splits at any point of its range. -/
theorem cnt_split {α : Type} [DecidableEq α] (d : α) (xs : Array α) (x : α) (lo mid : Int) :
    ∀ (k : Nat) (hi : Int), (hi - mid).toNat = k → lo ≤ mid → mid ≤ hi →
      cnt d xs x lo hi = cnt d xs x lo mid + cnt d xs x mid hi := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro hi hk hlm hmh
    by_cases same : hi = mid
    · subst same
      rw [cnt_unfold d xs x hi hi, if_pos (Int.le_refl _)]
      omega
    · have outer : ¬hi ≤ lo := by omega
      have inner : ¬hi ≤ mid := by omega
      rw [cnt_unfold d xs x lo hi, cnt_unfold d xs x mid hi, if_neg outer, if_neg inner,
        ih (hi - 1 - mid).toNat (by omega) (hi - 1) rfl hlm (by omega)]
      omega

/-- Arrays agreeing on a range count alike over it. -/
theorem cnt_congr {α : Type} [DecidableEq α] (d : α) (xs ys : Array α) (x : α) (lo : Int) :
    ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k →
      (∀ i : Int, lo ≤ i → i < hi → xs[i.toNat]?.getD d = ys[i.toNat]?.getD d) →
      cnt d xs x lo hi = cnt d ys x lo hi := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro hi hk agree
    rw [cnt_unfold d xs x lo hi, cnt_unfold d ys x lo hi]
    by_cases hle : hi ≤ lo
    · rw [if_pos hle, if_pos hle]
    · rw [if_neg hle, if_neg hle, ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl
        (fun i h1 h2 => agree i h1 (by omega)), agree (hi - 1) (by omega) (by omega)]

theorem cnt_nonneg {α : Type} [DecidableEq α] (d : α) (xs : Array α) (x : α) (lo : Int) :
    ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k → 0 ≤ cnt d xs x lo hi := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro hi hk
    rw [cnt_unfold d xs x lo hi]
    by_cases hle : hi ≤ lo
    · rw [if_pos hle]; omega
    · rw [if_neg hle]
      have := ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl
      split <;> omega

/-- An element of a range is counted there. -/
theorem cnt_pos {α : Type} [DecidableEq α] (d : α) (xs : Array α) (lo i : Int) :
    ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k → lo ≤ i → i < hi →
      1 ≤ cnt d xs (xs[i.toNat]?.getD d) lo hi := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro hi hk h1 h2
    rw [cnt_unfold d xs _ lo hi, if_neg (by omega)]
    have := cnt_nonneg d xs (xs[i.toNat]?.getD d) lo _ (hi - 1) rfl
    by_cases last : hi - 1 = i
    · subst last
      rw [if_pos rfl]
      omega
    · have := ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl h1 (by omega)
      split <;> omega

/-- What is counted in a range is an element of it. -/
theorem exists_of_cnt_pos {α : Type} [DecidableEq α] (d : α) (xs : Array α) (x : α) (lo : Int) :
    ∀ (k : Nat) (hi : Int), (hi - lo).toNat = k → 1 ≤ cnt d xs x lo hi →
      ∃ i : Int, lo ≤ i ∧ i < hi ∧ xs[i.toNat]?.getD d = x := by
  intro k
  induction k using Nat.strongRecOn with
  | _ k ih =>
    intro hi hk pos
    rw [cnt_unfold d xs x lo hi] at pos
    by_cases hle : hi ≤ lo
    · rw [if_pos hle] at pos; omega
    · rw [if_neg hle] at pos
      by_cases hit : xs[(hi - 1).toNat]?.getD d = x
      · exact ⟨hi - 1, by omega, by omega, hit⟩
      · rw [if_neg hit] at pos
        obtain ⟨i, h1, h2, h3⟩ := ih (hi - 1 - lo).toNat (by omega) (hi - 1) rfl (by omega)
        exact ⟨i, h1, by omega, h3⟩

/-- The frame of a sorted range: partition, then the two recursive sorts. -/
theorem sort_frame {α : Type} (d : α) (v P L R : Array α) (low high p : Int)
    (hvP : ∀ i : Int, 0 ≤ i → i < v.size → i < low ∨ high ≤ i →
      P[i.toNat]?.getD d = v[i.toNat]?.getD d)
    (hPL : ∀ i : Int, 0 ≤ i → i < P.size → i < low ∨ p ≤ i →
      L[i.toNat]?.getD d = P[i.toNat]?.getD d)
    (hLR : ∀ i : Int, 0 ≤ i → i < L.size → i < p + 1 ∨ high ≤ i →
      R[i.toNat]?.getD d = L[i.toNat]?.getD d)
    (sP : (P.size : Int) = v.size) (sL : (L.size : Int) = P.size) (hp : low ≤ p)
    (hp2 : p ≤ high - 1) :
    ∀ i : Int, 0 ≤ i → i < v.size → i < low ∨ high ≤ i →
      R[i.toNat]?.getD d = v[i.toNat]?.getD d := by
  intro i h0 h1 hout
  rw [hLR i h0 (by omega) (by omega), hPL i h0 (by omega) (by omega), hvP i h0 h1 hout]

/-- The counts of a sorted range: partition, then the two recursive sorts. -/
theorem sort_counts {α : Type} [DecidableEq α] (d : α) (v P L R : Array α) (low high p : Int)
    (cvP : ∀ x, cnt d v x low high = cnt d P x low high)
    (cPL : ∀ x, cnt d P x low p = cnt d L x low p)
    (hPL : ∀ i : Int, 0 ≤ i → i < P.size → i < low ∨ p ≤ i →
      L[i.toNat]?.getD d = P[i.toNat]?.getD d)
    (cLR : ∀ x, cnt d L x (p + 1) high = cnt d R x (p + 1) high)
    (hLR : ∀ i : Int, 0 ≤ i → i < L.size → i < p + 1 ∨ high ≤ i →
      R[i.toNat]?.getD d = L[i.toNat]?.getD d)
    (sP : (P.size : Int) = v.size) (sL : (L.size : Int) = P.size) (hhigh : high ≤ v.size)
    (hlow : 0 ≤ low) (hp : low ≤ p) (hp2 : p ≤ high - 1) :
    ∀ x, cnt d v x low high = cnt d R x low high := by
  intro x
  have upper : cnt d P x p high = cnt d L x p high :=
    cnt_congr d _ _ x p _ high rfl fun i h1 h2 =>
      (hPL i (by omega) (by omega) (Or.inr h1)).symm
  have lower : cnt d R x low (p + 1) = cnt d L x low (p + 1) :=
    cnt_congr d _ _ x low _ (p + 1) rfl fun i h1 h2 =>
      hLR i (by omega) (by omega) (Or.inl h2)
  rw [cvP x, cnt_split d _ x low p _ high rfl hp (by omega),
    cnt_split d R x low (p + 1) _ high rfl (by omega) (by omega), cPL x, upper,
    lower, ← cLR x, ← cnt_split d _ x low p _ high rfl hp (by omega),
    ← cnt_split d _ x low (p + 1) _ high rfl (by omega) (by omega)]

/-- An element of a range whose counts are those of another range is an
element of that one. -/
theorem element_of_counts {α : Type} [DecidableEq α] (d : α) (xs ys : Array α) (lo hi i : Int)
    (counts : ∀ x, cnt d xs x lo hi = cnt d ys x lo hi)
    (h1 : lo ≤ i) (h2 : i < hi) :
    ∃ k : Int, lo ≤ k ∧ k < hi ∧ xs[k.toNat]?.getD d = ys[i.toNat]?.getD d := by
  have pos := cnt_pos d ys lo i _ hi rfl h1 h2
  rw [← counts] at pos
  exact exists_of_cnt_pos d _ _ lo _ hi rfl pos

/-- The order of a sorted range: partition, then the two recursive sorts. -/
theorem sort_order {α : Type} [DecidableEq α] (d : α) (order : α → α → Ordering) [Std.TransCmp order]
    (P L R : Array α) (low high p : Int) (pivot : α)
    (hpivot : P[p.toNat]?.getD d = pivot)
    (below : ∀ i : Int, low ≤ i → i < p → order (P[i.toNat]?.getD d) pivot = .lt)
    (above : ∀ i : Int, p < i → i ≤ high - 1 → ¬order (P[i.toNat]?.getD d) pivot = .lt)
    (cPL : ∀ x, cnt d P x low p = cnt d L x low p)
    (hPL : ∀ i : Int, 0 ≤ i → i < P.size → i < low ∨ p ≤ i →
      L[i.toNat]?.getD d = P[i.toNat]?.getD d)
    (cLR : ∀ x, cnt d L x (p + 1) high = cnt d R x (p + 1) high)
    (hLR : ∀ i : Int, 0 ≤ i → i < L.size → i < p + 1 ∨ high ≤ i →
      R[i.toNat]?.getD d = L[i.toNat]?.getD d)
    (sortedL : ∀ i j : Int, low ≤ i → i < j → j < p →
      ¬order (L[i.toNat]?.getD d) (L[j.toNat]?.getD d) = .gt)
    (sortedR : ∀ i j : Int, p + 1 ≤ i → i < j → j < high →
      ¬order (R[i.toNat]?.getD d) (R[j.toNat]?.getD d) = .gt)
    (sL : (L.size : Int) = P.size) (sR : (R.size : Int) = L.size) (sP : high ≤ P.size)
    (hlow : 0 ≤ low) (hp : low ≤ p) (hp2 : p ≤ high - 1) :
    ∀ i j : Int, low ≤ i → i < j → j < high →
      ¬order (R[i.toNat]?.getD d) (R[j.toNat]?.getD d) = .gt := by
  -- The lower part lies below the pivot, the pivot stays, the upper part
  -- lies above it.
  have lowerPart : ∀ k : Int, low ≤ k → k < p → order (R[k.toNat]?.getD d) pivot = .lt := by
    intro k h1 h2
    rw [hLR k (by omega) (by omega) (Or.inl (by omega))]
    obtain ⟨k', k1, k2, k3⟩ := element_of_counts d P L low p k cPL h1 h2
    rw [← k3]
    exact below k' k1 k2
  have atPivot : R[p.toNat]?.getD d = pivot := by
    rw [hLR p (by omega) (by omega) (Or.inl (by omega)),
      hPL p (by omega) (by omega) (Or.inr (Int.le_refl _)), hpivot]
  have upperPart : ∀ k : Int, p + 1 ≤ k → k < high → ¬order pivot (R[k.toNat]?.getD d) = .gt := by
    intro k h1 h2
    obtain ⟨k', k1, k2, k3⟩ := element_of_counts d L R (p + 1) high k cLR h1 h2
    rw [← k3, hPL k' (by omega) (by omega) (Or.inr (by omega)), Std.OrientedCmp.gt_iff_lt]
    exact above k' (by omega) (by omega)
  intro i j hi hij hj
  by_cases jl : j < p
  · rw [hLR i (by omega) (by omega) (Or.inl (by omega)),
      hLR j (by omega) (by omega) (Or.inl (by omega))]
    exact sortedL i j hi hij jl
  by_cases ih : p + 1 ≤ i
  · exact sortedR i j ih hij hj
  have leIsLE : ∀ {a b : α}, ¬order a b = .gt → (order a b).isLE = true := by
    intro a b h
    cases e : order a b <;> simp_all
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

end Arrays

verex module 0x42::quicksort where
  open Classical Arrays
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
  -- Opaque: callers reason through the contract, not the loop.
  spec partition where
    pragma opaque
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

  /-- The generated definition of `count` counts over its vector's values. -/
  theorem count_cnt {u : VerexIR.Validation.ValidatedUnit} [Θ : VerexIR.Proofs.Denote.Skolems u]
      (v : (VerexIR.Proofs.Denote.NTy.param 0).vector.carrier)
      (x : (VerexIR.Proofs.Denote.NTy.param 0).carrier) :
      ∀ (k : Nat) (lo hi : Int), (hi - lo).toNat = k →
        count.at_T0.spec u Θ (v, x, lo, hi, ()) = cnt default v.values x lo hi := by
    intro k
    induction k using Nat.strongRecOn with
    | _ k ih =>
      intro lo hi hk
      rw [count.at_T0.spec.unfold, cnt_unfold]
      by_cases hle : hi ≤ lo
      · simp [hle]
      · simp only [hle, if_false, dite_eq_ite]
        rw [ih (hi - 1 - lo).toNat (by omega) lo (hi - 1) rfl]
        rfl

  /-- The same, at any bundle: the form `simp` rewrites by. -/
  theorem count_eq {u : VerexIR.Validation.ValidatedUnit} [Θ : VerexIR.Proofs.Denote.Skolems u]
      (bundle : (VerexIR.Proofs.Denote.NTy.param 0).vector.carrier ×
        (VerexIR.Proofs.Denote.NTy.param 0).carrier × Int × Int × Unit) :
      count.at_T0.spec u Θ bundle =
        cnt default bundle.1.values bundle.2.1 bundle.2.2.1 bundle.2.2.2.1 := by
    obtain ⟨v, x, lo, hi, ⟨⟩⟩ := bundle
    exact count_cnt v x _ lo hi rfl

  verify partition by lean =>
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    all_goals (try simp only [count_eq, VerexIR.Proofs.Denote.swapBounded] at *)
    all_goals (try intros)
    -- A count across a swap inside its range is the count before it.
    all_goals (try (rw [cnt_swapIfInBounds] <;> first | omega | skip))
    -- A lookup in a swapped vector is a lookup in the vector.
    all_goals (try (rw [swap_lookup] <;> first | omega | skip))
    all_goals (repeat' split)
    -- The pivot is the original vector's element at its position, and the
    -- element compared at the scan is the one found there.
    all_goals (try rw [lookup_of_found (found := ‹original.values[_]? = some _›)])
    all_goals (try first | rfl | omega | assumption | verex_instance)
    case ensures_3.isTrue =>
      rw [show lo.val = pivot_index.val by omega]
    case invariant_3_step_2.isTrue =>
      rw [show lo.val = scan.val by omega,
        lookup_of_found (found := ‹values.values[scan.val.toNat]? = some _›)]
      assumption
    case invariant_3_step_2.isFalse.isTrue =>
      rw [lookup_of_found (found := ‹values.values[scan.val.toNat]? = some _›)]
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

  verify quick_sort_range by lean =>
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    all_goals (try simp only [count_eq] at *)
    case ensures_4_3 => apply sort_counts <;> assumption
    case ensures_3_3 => apply sort_frame <;> assumption
    case ensures_2_3 => apply sort_order <;> first | assumption | omega

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
open VerexIR VerexE2ETests.CheckSupport in
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

end VerexSL.Tests.Check.Examples.Quicksort
