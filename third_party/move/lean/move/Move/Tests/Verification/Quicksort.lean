-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move
import MoveModel.Tests.Common

/-!
# Generic in-place quicksort

A self-contained end-to-end example: generic Move source, declarative
contracts, proofs against the relational source semantics derived from the
authored function bodies, and compiled execution tests.
-/

namespace Tests.MovePrograms

open Move
open scoped Move Move.Compiler Move.Spec

module Quicksort where

  struct PartitionResult (T) has Copy, Drop, Store where
    values : Vector T
    pivot : U64

  /-! ## Mathematical model

  Contract vocabulary: segment extraction, the Lomuto partition invariant,
  and the shape of a completed range sort. The relational semantics of the
  functions themselves is derived from their source bodies. -/

  namespace Model

  /-- The elements of the half-open index range `[lo, hi)`. -/
  def slice (values : List α) (lo hi : Nat) : List α :=
    (values.take hi).drop lo

  /-- The element at `index`, or `default` out of bounds. Move type
  parameters are always inhabited, so contracts may use this total
  accessor. -/
  def entry [Inhabited α] (values : List α) (index : Nat) : α :=
    values.getD index default

  /-- Lomuto invariant on entry to `partition_loop`: the write cursor does not
  pass the scan cursor, the pivot sits in bounds past the scanned range, and
  everything already scanned but not stored compares not-less than the
  pivot. -/
  structure PartitionPre {T : Type} [Inhabited T]
      (inp : List T) (pivotIndex scan store : Nat) : Prop where
    store_le_scan : store ≤ scan
    scan_le_pivot : scan ≤ pivotIndex
    pivot_lt_length : pivotIndex < inp.length
    scanned : ∀ i, store ≤ i → i < scan →
      ¬Compare.Less (entry inp i) (entry inp pivotIndex)

  /-- Completed Lomuto partition of `[store, pivotIndex]`: the pivot value
  lands at `p`, strictly smaller elements fill `[store, p)`, not-smaller
  elements fill `(p, pivotIndex]`, everything outside the working range is
  untouched, and the working range is only rearranged. -/
  structure Partitioned {T : Type} [Inhabited T]
      (inp : List T) (pivotIndex store : Nat) (out : List T) (p : Nat) :
      Prop where
    length_eq : out.length = inp.length
    store_le : store ≤ p
    le_pivot : p ≤ pivotIndex
    entry_p : entry out p = entry inp pivotIndex
    lower : ∀ i, store ≤ i → i < p →
      Compare.Less (entry out i) (entry inp pivotIndex)
    upper : ∀ i, p < i → i ≤ pivotIndex →
      ¬Compare.Less (entry out i) (entry inp pivotIndex)
    frame : ∀ i, i < store ∨ pivotIndex < i → entry out i = entry inp i
    perm : (slice out store (pivotIndex + 1)).Perm
      (slice inp store (pivotIndex + 1))

  /-- A completed sort of the half-open range `[lo, hi)`: the range is
  rearranged into pairwise order and everything outside it is untouched. -/
  structure Sorts {T : Type} [Inhabited T]
      (inp : List T) (lo hi : Nat) (out : List T) : Prop where
    length_eq : out.length = inp.length
    frame : ∀ i, i < lo ∨ hi ≤ i → entry out i = entry inp i
    perm : (slice out lo hi).Perm (slice inp lo hi)
    sorted : (slice out lo hi).Pairwise
      fun left right => ¬Compare.Less right left

  def Sorted [Compare.Lawful α] (values : Vector α) : Prop :=
    values.toList.Pairwise fun left right => ¬Compare.Less right left

  def Permutation (before after : Vector α) : Prop :=
    after.toList.Perm before.toList

  end Model

  /-! ## Functions -/

  /-- Lomuto partition of the half-open range ending at `pivotIndex`. -/
  partial fun partition_loop {T}
      (values : Vector T) (pivotIndex scan store : U64) :
      Action (PartitionResult T) := do
    let pivotRef ← &values[pivotIndex]
    let pivot ← *pivotRef
    if scan < pivotIndex then
      let candidateRef ← &values[scan]
      let candidate ← *candidateRef
      if candidate < pivot then
        if store < scan then
          let storedRef ← &values[store]
          let stored ← *storedRef
          let destination ← &mut values[store]
          destination := candidate
          let source ← &mut values[scan]
          source := stored
        partition_loop values pivotIndex (scan + 1) (store + 1)
      else
        partition_loop values pivotIndex (scan + 1) store
    else
      if store < pivotIndex then
        let storedRef ← &values[store]
        let stored ← *storedRef
        let destination ← &mut values[store]
        destination := pivot
        let source ← &mut values[pivotIndex]
        source := stored
      pure { values, pivot := store }

  spec partition_loop {T} [Compare.Total T]
      (values : Vector T) (pivotIndex : U64) (scan : U64)
      (store : U64) where
    requires Model.PartitionPre values.toList
      pivotIndex.toNat scan.toNat store.toNat;
    ensures Model.Partitioned values.toList pivotIndex.toNat store.toNat
      result.values.toList result.pivot.toNat;
    aborts_if False

  partial fun quick_sort_range {T}
      (values : Vector T) (low high : U64) :
      Action (Vector T) := do
    if low < high then
      let span := high - low
      if 1 < span then
        let partitioned ← partition_loop values (high - 1) low low
        let left ← quick_sort_range partitioned.values low partitioned.pivot
        quick_sort_range left (partitioned.pivot + 1) high
      else
        pure values
    else
      pure values

  spec quick_sort_range {T} [Compare.Total T]
      (values : Vector T) (low : U64) (high : U64) where
    requires high.toNat ≤ values.toList.length;
    ensures Model.Sorts values.toList low.toNat high.toNat result.toList;
    aborts_if False

  /-- Generic in-place sort using Move's built-in lexicographic comparison. -/
  public fun quick_sort {T}
      (values : Vector T) : Action (Vector T) :=
    quick_sort_range values 0 values.length

  /- `Vector` certifies Move's physical length bound by construction,
  so the `u64` cursor arithmetic provably never overflows: sorting neither
  requires anything nor aborts. -/
  spec quick_sort {T} [Compare.Total T]
      (values : Vector T) where
    ensures Model.Sorted result ∧ Model.Permutation values result;
    aborts_if False

  /-! ## Proofs -/

  namespace Model

  section Slice

  variable {α : Type}

  theorem slice_getElem? (l : List α) (lo hi k : Nat) :
      (slice l lo hi)[k]? = if lo + k < hi then l[lo + k]? else none := by
    simp [slice, List.getElem?_drop, List.getElem?_take]

  theorem slice_length {l : List α} {lo hi : Nat}
      (bound : hi ≤ l.length) (order : lo ≤ hi) :
      (slice l lo hi).length = hi - lo := by
    simp [slice, List.length_drop, List.length_take]
    omega

  theorem slice_length_le (l : List α) (lo hi : Nat) :
      (slice l lo hi).length ≤ hi - lo := by
    simp [slice, List.length_drop, List.length_take]
    omega

  theorem slice_empty {l : List α} {lo hi : Nat} (h : hi ≤ lo) :
      slice l lo hi = [] := by
    apply List.ext_getElem?
    intro k
    rw [slice_getElem?, if_neg (by omega)]
    simp

  theorem slice_congr {l₁ l₂ : List α} {lo hi : Nat}
      (agree : ∀ i, lo ≤ i → i < hi → l₁[i]? = l₂[i]?) :
      slice l₁ lo hi = slice l₂ lo hi := by
    apply List.ext_getElem?
    intro k
    rw [slice_getElem?, slice_getElem?]
    split
    · exact agree (lo + k) (by omega) (by omega)
    · rfl

  theorem slice_append {l : List α} {lo mid hi : Nat}
      (left : lo ≤ mid) (right : mid ≤ hi) (bound : mid ≤ l.length) :
      slice l lo mid ++ slice l mid hi = slice l lo hi := by
    apply List.ext_getElem?
    intro k
    rw [List.getElem?_append, slice_length bound left]
    by_cases inLeft : k < mid - lo
    · rw [if_pos inLeft, slice_getElem?, slice_getElem?,
        if_pos (by omega), if_pos (by omega)]
    · rw [if_neg inLeft, slice_getElem?, slice_getElem?,
        show mid + (k - (mid - lo)) = lo + k by omega]

  theorem slice_cons {l : List α} {lo hi : Nat} {a : α}
      (head : l[lo]? = some a) (lt : lo < hi) :
      slice l lo hi = a :: slice l (lo + 1) hi := by
    apply List.ext_getElem?
    intro k
    cases k with
    | zero =>
        rw [slice_getElem?, if_pos (by omega)]
        simpa using head
    | succ k =>
        rw [slice_getElem?, List.getElem?_cons_succ, slice_getElem?,
          show lo + (k + 1) = lo + 1 + k by omega]

  theorem mem_slice {l : List α} {lo hi : Nat} {x : α} :
      x ∈ slice l lo hi ↔ ∃ i, lo ≤ i ∧ i < hi ∧ l[i]? = some x := by
    rw [List.mem_iff_getElem?]
    constructor
    · rintro ⟨k, hk⟩
      rw [slice_getElem?] at hk
      split at hk
      · exact ⟨lo + k, by omega, by omega, hk⟩
      · cases hk
    · rintro ⟨i, hlo, hhi, hi⟩
      refine ⟨i - lo, ?_⟩
      rw [slice_getElem?, if_pos (by omega),
        show lo + (i - lo) = i by omega]
      exact hi

  theorem set_entry_self {l : List α} {i : Nat} {a : α}
      (h : l[i]? = some a) : l.set i a = l := by
    apply List.ext_getElem?
    intro k
    rw [List.getElem?_set]
    split
    · next eq =>
        subst eq
        rw [if_pos (List.getElem?_eq_some_iff.mp h).1]
        exact h.symm
    · rfl

  end Slice

  section Entry

  variable {α : Type} [Inhabited α]

  theorem entry_getElem? {l : List α} {i : Nat} (h : i < l.length) :
      l[i]? = some (entry l i) := by
    rw [List.getElem?_eq_getElem h]
    simp [entry, List.getD_eq_getElem?_getD, List.getElem?_eq_getElem h]

  theorem entry_of_getElem? {l : List α} {i : Nat} {a : α}
      (h : l[i]? = some a) : entry l i = a := by
    simp [entry, List.getD_eq_getElem?_getD, h]

  theorem getElem?_eq_of_entry {l₁ l₂ : List α} (lengths : l₁.length = l₂.length)
      {i : Nat} (h : entry l₁ i = entry l₂ i) : l₁[i]? = l₂[i]? := by
    by_cases bound : i < l₁.length
    · rw [entry_getElem? bound, entry_getElem? (lengths ▸ bound), h]
    · rw [List.getElem?_eq_none (by omega), List.getElem?_eq_none (by omega)]

  theorem entry_set_self {l : List α} {i : Nat} {a : α} (h : i < l.length) :
      entry (l.set i a) i = a :=
    entry_of_getElem? (List.getElem?_set_self h)

  theorem entry_set_ne {l : List α} {i j : Nat} {a : α} (h : i ≠ j) :
      entry (l.set i a) j = entry l j := by
    simp [entry, List.getD_eq_getElem?_getD, List.getElem?_set_ne h]

  /-- Transposing two in-range positions permutes any covering segment. -/
  theorem slice_swap_perm {l : List α} {i j lo hi : Nat} {a b : α}
      (ha : l[i]? = some a) (hb : l[j]? = some b)
      (hlo : lo ≤ i) (hij : i ≤ j) (hhi : j < hi) :
      (slice ((l.set i b).set j a) lo hi).Perm (slice l lo hi) := by
    have iBound : i < l.length := (List.getElem?_eq_some_iff.mp ha).1
    have jBound : j < l.length := (List.getElem?_eq_some_iff.mp hb).1
    by_cases eq : i = j
    · subst eq
      have : a = b := by rw [ha] at hb; cases hb; rfl
      subst this
      rw [List.set_set, set_entry_self ha]
    · have lt : i < j := by omega
      have setLength : ((l.set i b).set j a).length = l.length := by simp
      have atI : ((l.set i b).set j a)[i]? = some b := by
        rw [List.getElem?_set_ne (by omega),
          List.getElem?_set_self iBound]
      have atJ : ((l.set i b).set j a)[j]? = some a := by
        rw [List.getElem?_set_self (by simpa using jBound)]
      have elsewhere : ∀ k, k ≠ i → k ≠ j →
          ((l.set i b).set j a)[k]? = l[k]? := by
        intro k ki kj
        rw [List.getElem?_set_ne (by omega),
          List.getElem?_set_ne (by omega)]
      have decompose : ∀ (m : List α), m[i]? = some (entry m i) →
          m[j]? = some (entry m j) → m.length = l.length →
          slice m lo hi =
            slice m lo i ++ entry m i ::
              (slice m (i + 1) j ++ entry m j :: slice m (j + 1) hi) := by
        intro m mi mj mLength
        rw [← slice_append (l := m) (mid := i) hlo (by omega) (by omega),
          slice_cons mi (by omega),
          ← slice_append (l := m) (lo := i + 1) (mid := j) (by omega)
            (by omega) (by omega),
          slice_cons mj (by omega)]
      have entryI : entry ((l.set i b).set j a) i = b := entry_of_getElem? atI
      have entryJ : entry ((l.set i b).set j a) j = a := entry_of_getElem? atJ
      rw [decompose ((l.set i b).set j a)
          (by rw [atI, entryI]) (by rw [atJ, entryJ]) setLength,
        decompose l (entry_getElem? iBound) (entry_getElem? jBound) rfl,
        entryI, entryJ,
        entry_of_getElem? ha, entry_of_getElem? hb,
        slice_congr (lo := lo) (hi := i) (l₂ := l)
          (fun k klo khi => elsewhere k (by omega) (by omega)),
        slice_congr (lo := i + 1) (hi := j) (l₂ := l)
          (fun k klo khi => elsewhere k (by omega) (by omega)),
        slice_congr (lo := j + 1) (hi := hi) (l₂ := l)
          (fun k klo khi => elsewhere k (by omega) (by omega))]
      apply List.Perm.append_left
      exact ((List.perm_middle.cons b).trans
        (List.Perm.swap a b _)).trans (List.perm_middle.symm.cons a)

  end Entry

  /-! ### Lomuto partition steps

  One lemma per source branch of `partition_loop`, phrased over lists. The
  `advance` lemmas transport the recursive call's pre- and postcondition
  across one scan step; the `final` lemmas discharge the terminating swap. -/

  section Partition

  variable {T : Type} [Inhabited T]

  theorem PartitionPre.advance_ge {inp : List T} {pivotIndex scan store : Nat}
      (pre : PartitionPre inp pivotIndex scan store)
      (scanLt : scan < pivotIndex)
      (notLess : ¬Compare.Less (entry inp scan) (entry inp pivotIndex)) :
      PartitionPre inp pivotIndex (scan + 1) store where
    store_le_scan := by have := pre.store_le_scan; omega
    scan_le_pivot := by omega
    pivot_lt_length := pre.pivot_lt_length
    scanned := by
      intro i lower upper
      by_cases atScan : i = scan
      · rw [atScan]
        exact notLess
      · exact pre.scanned i lower (by omega)

  theorem PartitionPre.advance_self {inp : List T} {pivotIndex scan : Nat}
      (pre : PartitionPre inp pivotIndex scan scan)
      (scanLt : scan < pivotIndex) :
      PartitionPre inp pivotIndex (scan + 1) (scan + 1) where
    store_le_scan := Nat.le_refl _
    scan_le_pivot := by omega
    pivot_lt_length := pre.pivot_lt_length
    scanned := by intro i lower upper; omega

  theorem PartitionPre.advance_swap {inp : List T} {pivotIndex scan store : Nat}
      (pre : PartitionPre inp pivotIndex scan store)
      (scanLt : scan < pivotIndex) (storeLt : store < scan) :
      PartitionPre ((inp.set store (entry inp scan)).set scan (entry inp store))
        pivotIndex (scan + 1) (store + 1) := by
    have scanBound : scan < inp.length := by
      have := pre.pivot_lt_length; omega
    have storeBound : store < inp.length := by omega
    have pivotEntry :
        entry ((inp.set store (entry inp scan)).set scan (entry inp store))
          pivotIndex = entry inp pivotIndex := by
      rw [entry_set_ne (by omega), entry_set_ne (by omega)]
    refine ⟨by omega, by omega, by simpa using pre.pivot_lt_length, ?_⟩
    intro i lower upper
    rw [pivotEntry]
    by_cases atScan : i = scan
    · rw [atScan, entry_set_self (by simpa using scanBound)]
      exact pre.scanned store (Nat.le_refl _) storeLt
    · rw [entry_set_ne (by omega), entry_set_ne (by omega)]
      exact pre.scanned i (by omega) (by omega)

  theorem Partitioned.advance_self {inp out : List T} {pivotIndex scan p : Nat}
      (pre : PartitionPre inp pivotIndex scan scan)
      (scanLt : scan < pivotIndex)
      (less : Compare.Less (entry inp scan) (entry inp pivotIndex))
      (next : Partitioned inp pivotIndex (scan + 1) out p) :
      Partitioned inp pivotIndex scan out p where
    length_eq := next.length_eq
    store_le := by have := next.store_le; omega
    le_pivot := next.le_pivot
    entry_p := next.entry_p
    lower := by
      intro i lower upper
      by_cases atScan : i = scan
      · rw [atScan, next.frame scan (Or.inl (by omega))]
        exact less
      · exact next.lower i (by omega) upper
    upper := next.upper
    frame := by
      intro i outside
      exact next.frame i (by omega)
    perm := by
      have scanBoundOut : scan < out.length := by
        have := next.length_eq; have := pre.pivot_lt_length; omega
      have outAtScan : out[scan]? = some (entry inp scan) := by
        rw [entry_getElem? scanBoundOut,
          next.frame scan (Or.inl (by omega))]
      have inpAtScan : inp[scan]? = some (entry inp scan) :=
        entry_getElem? (by have := pre.pivot_lt_length; omega)
      rw [slice_cons outAtScan (by omega), slice_cons inpAtScan (by omega)]
      exact next.perm.cons _

  theorem Partitioned.advance_swap {inp out : List T}
      {pivotIndex scan store p : Nat}
      (pre : PartitionPre inp pivotIndex scan store)
      (scanLt : scan < pivotIndex) (storeLt : store < scan)
      (less : Compare.Less (entry inp scan) (entry inp pivotIndex))
      (next : Partitioned
        ((inp.set store (entry inp scan)).set scan (entry inp store))
        pivotIndex (store + 1) out p) :
      Partitioned inp pivotIndex store out p := by
    have pivotBound : pivotIndex < inp.length := pre.pivot_lt_length
    have scanBound : scan < inp.length := by omega
    have storeBound : store < inp.length := by omega
    have swappedLength :
        ((inp.set store (entry inp scan)).set scan (entry inp store)).length =
          inp.length := by simp
    have pivotEntry :
        entry ((inp.set store (entry inp scan)).set scan (entry inp store))
          pivotIndex = entry inp pivotIndex := by
      rw [entry_set_ne (by omega), entry_set_ne (by omega)]
    have storeEntry :
        entry ((inp.set store (entry inp scan)).set scan (entry inp store))
          store = entry inp scan := by
      rw [entry_set_ne (by omega), entry_set_self storeBound]
    refine ⟨next.length_eq.trans swappedLength, by have := next.store_le; omega,
      next.le_pivot, next.entry_p.trans pivotEntry, ?_, ?_, ?_, ?_⟩
    · intro i lower upper
      by_cases atStore : i = store
      · rw [atStore, next.frame store (Or.inl (by omega)), storeEntry]
        exact less
      · rw [← pivotEntry]
        exact next.lower i (by omega) upper
    · intro i lower upper
      rw [← pivotEntry]
      exact next.upper i lower upper
    · intro i outside
      have swappedEntry :
          entry ((inp.set store (entry inp scan)).set scan (entry inp store))
            i = entry inp i := by
        rw [entry_set_ne (by omega), entry_set_ne (by omega)]
      rw [next.frame i (by omega), swappedEntry]
    · have storeBoundOut : store < out.length := by
        have := next.length_eq; rw [swappedLength] at this; omega
      have outAtStore : out[store]? = some (entry inp scan) := by
        rw [entry_getElem? storeBoundOut,
          next.frame store (Or.inl (by omega)), storeEntry]
      have swappedAtStore :
          ((inp.set store (entry inp scan)).set scan (entry inp store))[store]? =
            some (entry inp scan) := by
        rw [entry_getElem? (by simpa [swappedLength] using storeBound),
          storeEntry]
      have swappedPerm :
          (slice ((inp.set store (entry inp scan)).set scan (entry inp store))
            store (pivotIndex + 1)).Perm
            (slice inp store (pivotIndex + 1)) :=
        slice_swap_perm (entry_getElem? storeBound)
          (entry_getElem? scanBound) (Nat.le_refl _) (by omega) (by omega)
      rw [slice_cons outAtStore (by omega)]
      refine List.Perm.trans ?_ swappedPerm
      rw [slice_cons swappedAtStore (by omega)]
      exact next.perm.cons _

  theorem Partitioned.final_swap {inp : List T} {pivotIndex store : Nat}
      (pre : PartitionPre inp pivotIndex pivotIndex store)
      (storeLt : store < pivotIndex) :
      Partitioned inp pivotIndex store
        ((inp.set store (entry inp pivotIndex)).set pivotIndex
          (entry inp store))
        store := by
    have pivotBound : pivotIndex < inp.length := pre.pivot_lt_length
    have storeBound : store < inp.length := by omega
    refine ⟨by simp, Nat.le_refl _, by omega, ?_, ?_, ?_, ?_, ?_⟩
    · rw [entry_set_ne (by omega), entry_set_self storeBound]
    · intro i lower upper
      omega
    · intro i lower upper
      by_cases atPivot : i = pivotIndex
      · rw [atPivot, entry_set_self (by simpa using pivotBound)]
        exact pre.scanned store (Nat.le_refl _) storeLt
      · rw [entry_set_ne (by omega), entry_set_ne (by omega)]
        exact pre.scanned i (by omega) (by omega)
    · intro i outside
      rw [entry_set_ne (by omega), entry_set_ne (by omega)]
    · exact slice_swap_perm (entry_getElem? storeBound)
        (entry_getElem? pivotBound) (Nat.le_refl _) (by omega) (by omega)

  theorem Partitioned.final_self {inp : List T} {pivotIndex : Nat} :
      Partitioned inp pivotIndex pivotIndex inp pivotIndex where
    length_eq := rfl
    store_le := Nat.le_refl _
    le_pivot := Nat.le_refl _
    entry_p := rfl
    lower := by intro i lower upper; omega
    upper := by intro i lower upper; omega
    frame := fun i _ => rfl
    perm := List.Perm.refl _

  end Partition

  /-! ### Range sort composition -/

  section Sorting

  variable {T : Type} [Inhabited T]

  theorem Sorts.small {inp : List T} {lo hi : Nat} (small : hi ≤ lo + 1) :
      Sorts inp lo hi inp where
    length_eq := rfl
    frame := fun i _ => rfl
    perm := List.Perm.refl _
    sorted := by
      have short := slice_length_le inp lo hi
      cases shape : slice inp lo hi with
      | nil => exact List.Pairwise.nil
      | cons x rest =>
          cases rest with
          | nil => simp
          | cons y rest' =>
              rw [shape] at short
              simp at short
              omega

  theorem Sorts.compose [Compare.Total T]
      {inp mid1 mid2 out : List T} {low high p : Nat}
      (bound : high ≤ inp.length)
      (span : low + 1 < high)
      (part : Partitioned inp (high - 1) low mid1 p)
      (sortLeft : Sorts mid1 low p mid2)
      (sortRight : Sorts mid2 (p + 1) high out) :
      Sorts inp low high out := by
    have len1 : mid1.length = inp.length := part.length_eq
    have len2 : mid2.length = inp.length := sortLeft.length_eq.trans len1
    have lenOut : out.length = inp.length := sortRight.length_eq.trans len2
    have pLow : low ≤ p := part.store_le
    have pHigh : p < high := by have := part.le_pivot; omega
    have highSucc : high - 1 + 1 = high := by omega
    have entryOutP : entry out p = entry inp (high - 1) := by
      rw [sortRight.frame p (Or.inl (by omega)),
        sortLeft.frame p (Or.inr (Nat.le_refl _)), part.entry_p]
    have eqLeft : slice out low p = slice mid2 low p :=
      slice_congr fun i lower upper =>
        getElem?_eq_of_entry (by omega)
          (sortRight.frame i (Or.inl (by omega)))
    have eqRightMid : slice mid2 (p + 1) high = slice mid1 (p + 1) high :=
      slice_congr fun i lower upper =>
        getElem?_eq_of_entry (by omega)
          (sortLeft.frame i (Or.inr (by omega)))
    have outAtP : out[p]? = some (entry inp (high - 1)) := by
      rw [entry_getElem? (by omega), entryOutP]
    have mid1AtP : mid1[p]? = some (entry inp (high - 1)) := by
      rw [entry_getElem? (by omega), part.entry_p]
    have decOut : slice out low high =
        slice out low p ++ entry inp (high - 1) :: slice out (p + 1) high := by
      rw [← slice_append pLow (by omega) (by omega : p ≤ out.length),
        slice_cons outAtP pHigh]
    have decMid1 : slice mid1 low high =
        slice mid1 low p ++ entry inp (high - 1) :: slice mid1 (p + 1) high := by
      rw [← slice_append pLow (by omega) (by omega : p ≤ mid1.length),
        slice_cons mid1AtP pHigh]
    have membersLeft : ∀ x ∈ slice out low p,
        Compare.Less x (entry inp (high - 1)) := by
      intro x member
      rw [eqLeft] at member
      have := sortLeft.perm.mem_iff.mp member
      obtain ⟨i, lower, upper, present⟩ := mem_slice.mp this
      rw [← entry_of_getElem? present]
      exact part.lower i lower upper
    have membersRight : ∀ y ∈ slice out (p + 1) high,
        ¬Compare.Less y (entry inp (high - 1)) := by
      intro y member
      have := sortRight.perm.mem_iff.mp member
      rw [eqRightMid] at this
      obtain ⟨i, lower, upper, present⟩ := mem_slice.mp this
      rw [← entry_of_getElem? present]
      exact part.upper i (by omega) (by omega)
    refine ⟨lenOut, ?_, ?_, ?_⟩
    · intro i outside
      rw [sortRight.frame i (by omega), sortLeft.frame i (by omega),
        part.frame i (by omega)]
    · have permLeft : (slice out low p).Perm (slice mid1 low p) := by
        rw [eqLeft]; exact sortLeft.perm
      have permRight :
          (slice out (p + 1) high).Perm (slice mid1 (p + 1) high) := by
        rw [← eqRightMid]; exact sortRight.perm
      have permPart := part.perm
      rw [highSucc] at permPart
      rw [decOut]
      exact (permLeft.append (permRight.cons _)).trans (decMid1 ▸ permPart)
    · rw [decOut, List.pairwise_append]
      refine ⟨?_, ?_, ?_⟩
      · rw [eqLeft]; exact sortLeft.sorted
      · rw [List.pairwise_cons]
        exact ⟨membersRight, sortRight.sorted⟩
      · intro x xMember y yMember
        have lessX := membersLeft x xMember
        rcases List.mem_cons.mp yMember with rfl | yMember
        · exact Compare.Lawful.asymm lessX
        · intro contra
          exact membersRight y yMember (Compare.Lawful.trans contra lessX)

  /-- A sort of the whole index range sorts and permutes the whole list. -/
  theorem Sorts.whole [Compare.Lawful T] {inp out : List T}
      (h : Sorts inp 0 inp.length out) :
      out.Pairwise (fun left right => ¬Compare.Less right left) ∧
        out.Perm inp := by
    have whole : ∀ (l : List T), l.length = inp.length →
        slice l 0 inp.length = l := by
      intro l lengths
      rw [slice, List.drop_zero, ← lengths, List.take_length]
    have outWhole := whole out h.length_eq
    have inpWhole := whole inp rfl
    constructor
    · have := h.sorted
      rwa [outWhole] at this
    · have := h.perm
      rwa [outWhole, inpWhole] at this

  end Sorting

  end Model

  verify partition_loop by
    contract_intro
    obtain ⟨values, pivotIndex, scan, store⟩ := args
    replace permitted : Model.PartitionPre values.toList
      pivotIndex.toNat scan.toNat store.toNat := permitted
    have pivotBound : pivotIndex.toNat < values.toList.length :=
      permitted.pivot_lt_length
    have oneNat : (1 : U64).toNat = 1 := rfl
    simp only [Move.Semantics.Spec.pure_bind]
    rw [Move.Semantics.Vector.borrowElemSpec_eq_pure
        (Model.entry_getElem? pivotBound),
      Move.Semantics.Spec.pure_bind]
    by_cases hscan : Move.Verify.Source.logicalLT scan pivotIndex
    · rw [if_pos hscan]
      rw [Move.Verify.Source.logicalLT_uint] at hscan
      rw [Move.Semantics.Vector.borrowElemSpec_eq_pure
          (Model.entry_getElem? (by omega)),
        Move.Semantics.Spec.pure_bind]
      by_cases hcand : Move.Verify.Source.logicalLT
        (Model.entry values.toList scan.toNat)
        (Model.entry values.toList pivotIndex.toNat)
      · rw [if_pos hcand]
        rw [Move.Verify.Source.logicalLT_move] at hcand
        by_cases hstore : Move.Verify.Source.logicalLT store scan
        · -- Strictly smaller candidate, swapped into the store slot.
          rw [if_pos hstore]
          rw [Move.Verify.Source.logicalLT_uint] at hstore
          rw [Move.Semantics.Vector.borrowElemSpec_eq_pure
              (Model.entry_getElem?
                (by omega : store.toNat < values.toList.length)),
            Move.Semantics.Spec.pure_bind]
          rw [Move.Verify.withBorrowElemMutSpec_write_eq_pure
              (Model.entry_getElem?
                (by omega : store.toNat < values.toList.length)),
            Move.Semantics.Spec.pure_bind]
          have present2 : (Move.Vector.set values store
              (Model.entry values.toList scan.toNat)).toList[scan.toNat]? =
              some (Model.entry values.toList scan.toNat) := by
            rw [Move.Vector.toList_set, List.getElem?_set_ne (by omega)]
            exact Model.entry_getElem? (by omega)
          rw [Move.Verify.withBorrowElemMutSpec_write_eq_pure present2,
            Move.Semantics.Spec.pure_bind]
          checked_cases hno1
          checked_cases hno2
          refine Move.Verify.wp_mono
            (Move.Verify.wp_of_satisfies recursiveVerified ?_) ?_ ?_
          · spec_norm
            exact Model.PartitionPre.advance_swap permitted hscan hstore
          · rintro result final ⟨part, rfl⟩
            spec_norm at part ⊢
            exact ⟨Model.Partitioned.advance_swap permitted hscan hstore
              hcand part, trivial⟩
          · intro code h
            exact h
        · -- Strictly smaller candidate already in place.
          rw [if_neg hstore]
          rw [Move.Verify.Source.logicalLT_uint] at hstore
          have storeEq : store.toNat = scan.toNat := by
            have := permitted.store_le_scan; omega
          checked_cases hno1
          checked_cases hno2
          refine Move.Verify.wp_mono
            (Move.Verify.wp_of_satisfies recursiveVerified ?_) ?_ ?_
          · spec_norm
            show Model.PartitionPre values.toList pivotIndex.toNat
              (scan.toNat + 1) (store.toNat + 1)
            rw [storeEq] at permitted ⊢
            exact Model.PartitionPre.advance_self permitted hscan
          · rintro result final ⟨part, rfl⟩
            spec_norm at part ⊢
            refine ⟨?_, trivial⟩
            rw [storeEq] at permitted part ⊢
            exact Model.Partitioned.advance_self permitted hscan
              (by rw [← storeEq] at hcand ⊢; exact hcand) part
          · intro code h
            exact h
      · -- Candidate not smaller: scanned range extends.
        rw [if_neg hcand]
        rw [Move.Verify.Source.logicalLT_move] at hcand
        checked_cases overflow
        refine Move.Verify.wp_of_satisfies recursiveVerified ?_
        spec_norm
        exact Model.PartitionPre.advance_ge permitted hscan hcand
    · rw [if_neg hscan]
      rw [Move.Verify.Source.logicalLT_uint] at hscan
      have scanEq : scan.toNat = pivotIndex.toNat := by
        have := permitted.scan_le_pivot; omega
      by_cases hstore : Move.Verify.Source.logicalLT store pivotIndex
      · -- Terminal swap of the pivot into the store slot.
        rw [if_pos hstore]
        rw [Move.Verify.Source.logicalLT_uint] at hstore
        rw [Move.Semantics.Vector.borrowElemSpec_eq_pure
            (Model.entry_getElem?
              (by omega : store.toNat < values.toList.length)),
          Move.Semantics.Spec.pure_bind]
        rw [Move.Verify.withBorrowElemMutSpec_write_eq_pure
            (Model.entry_getElem?
              (by omega : store.toNat < values.toList.length)),
          Move.Semantics.Spec.pure_bind]
        have present2 : (Move.Vector.set values store
            (Model.entry values.toList pivotIndex.toNat)).toList[pivotIndex.toNat]? =
            some (Model.entry values.toList pivotIndex.toNat) := by
          rw [Move.Vector.toList_set, List.getElem?_set_ne (by omega)]
          exact Model.entry_getElem? (by omega)
        rw [Move.Verify.withBorrowElemMutSpec_write_eq_pure present2,
          Move.Semantics.Spec.pure_bind, Move.Verify.wp_pure]
        rw [scanEq] at permitted
        exact ⟨Model.Partitioned.final_swap permitted hstore, rfl⟩
      · -- Empty working range: the pivot already sits at the store slot.
        rw [if_neg hstore]
        rw [Move.Verify.Source.logicalLT_uint] at hstore
        rw [Move.Verify.wp_pure]
        have storeEq : store.toNat = pivotIndex.toNat := by
          have := permitted.store_le_scan; omega
        refine ⟨?_, rfl⟩
        rw [storeEq]
        exact Model.Partitioned.final_self

  verify quick_sort_range by
    contract_intro
    obtain ⟨values, low, high⟩ := args
    replace permitted : high.toNat ≤ values.toList.length := permitted
    have oneNat : (1 : U64).toNat = 1 := rfl
    simp only [Move.Semantics.Spec.pure_bind]
    by_cases hrange : Move.Verify.Source.logicalLT low high
    · rw [if_pos hrange]
      rw [Move.Verify.Source.logicalLT_uint] at hrange
      rw [Move.Semantics.Checked.subSpec_eq_pure_unsigned
          (by omega : low.toNat ≤ high.toNat),
        Move.Semantics.Spec.pure_bind]
      by_cases hspan : Move.Verify.Source.logicalLT 1
        (U64.ofNat (high.toNat - low.toNat))
      · rw [if_pos hspan]
        rw [Move.Verify.Source.logicalLT_uint] at hspan
        spec_norm at hspan
        rw [Move.Semantics.Checked.subSpec_eq_pure_unsigned
            (by omega : (1 : U64).toNat ≤ high.toNat),
          Move.Semantics.Spec.pure_bind, Move.Verify.wp_bind]
        refine Move.Verify.wp_mono
          (Move.Verify.wp_of_satisfies
            (partition_loop.verified _moveSpecState) ?_) ?_ ?_
        · exact ⟨Nat.le_refl _, by u64_omega, by u64_omega,
            fun i lower upper => absurd (Nat.lt_of_lt_of_le upper lower)
              (Nat.lt_irrefl i)⟩
        · rintro partitioned final ⟨part, rfl⟩
          spec_norm at part
          rw [Move.Verify.wp_bind]
          have pivotHigh : partitioned.pivot.toNat < high.toNat := by
            have := part.le_pivot; omega
          have partLength :
              partitioned.values.toList.length = values.toList.length :=
            part.length_eq
          refine Move.Verify.wp_mono
            (Move.Verify.wp_of_satisfies recursiveVerified ?_) ?_ ?_
          · show partitioned.pivot.toNat ≤ partitioned.values.toList.length
            omega
          · rintro left final ⟨sortsLeft, rfl⟩
            have leftLength : left.toList.length = values.toList.length :=
              sortsLeft.length_eq.trans partLength
            have pivotBound := part.le_pivot
            checked_cases overflow
            refine Move.Verify.wp_mono
              (Move.Verify.wp_of_satisfies recursiveVerified ?_) ?_ ?_
            · show high.toNat ≤ left.toList.length
              omega
            · rintro result final ⟨sortsRight, rfl⟩
              spec_norm at sortsRight
              exact ⟨Model.Sorts.compose permitted (by omega) part
                sortsLeft sortsRight, rfl⟩
            · intro code h
              exact h
          · intro code h
            exact h
        · intro code h
          exact h
      · rw [if_neg hspan]
        rw [Move.Verify.Source.logicalLT_uint] at hspan
        spec_norm at hspan
        rw [Move.Verify.wp_pure]
        exact ⟨Model.Sorts.small (by omega), rfl⟩
    · rw [if_neg hrange]
      rw [Move.Verify.Source.logicalLT_uint] at hrange
      rw [Move.Verify.wp_pure]
      exact ⟨Model.Sorts.small (by omega), rfl⟩

  verify quick_sort by
    contract_intro
    refine Move.Verify.wp_mono
      (Move.Verify.wp_of_satisfies
        (quick_sort_range.verified _moveSpecState) ?_) ?_ ?_
    · show (Move.Vector.length args).toNat ≤ args.toList.length
      rw [Move.Vector.length_toNat]
      exact Nat.le_refl _
    · rintro result final ⟨sorts, rfl⟩
      dsimp only at sorts
      rw [Move.Vector.length_toNat] at sorts
      obtain ⟨sorted, perm⟩ := Model.Sorts.whole sorts
      exact ⟨⟨sorted, perm⟩, rfl⟩
    · intro code h
      exact h

  /-! ## Tests -/

  def compiled : MoveModel.IR.Module := lowerToIR ``Tests.MovePrograms.Quicksort

  private def run := Tests.run compiled

  #test run "quick_sort" [] [.vector [.u64 5, .u64 1, .u64 4, .u64 2, .u64 3]] =
    Tests.okRet [] [.vector [.u64 1, .u64 2, .u64 3, .u64 4, .u64 5]]
  #test run "quick_sort" [] [.vector []] = Tests.okRet [] [.vector []]
  #test run "quick_sort" [] [.vector [.bool true, .bool false, .bool true]] =
    Tests.okRet [] [.vector [.bool false, .bool true, .bool true]]
  #test run "quick_sort" [] [.vector
      [.vector [.u64 1, .u64 3], .vector [.u64 1, .u64 2],
       .vector [.u64 0, .u64 9]]] =
    Tests.okRet [] [.vector
      [.vector [.u64 0, .u64 9], .vector [.u64 1, .u64 2],
       .vector [.u64 1, .u64 3]]]
  #test run "quick_sort" [] [.vector [.u64 2, .u64 1, .u64 2]] =
    Tests.okRet [] [.vector [.u64 1, .u64 2, .u64 2]]

end Tests.MovePrograms
