-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.MapCoverage
import VerexIR.Proofs.Denote.VectorOperations

/-!
# The built-in lemmas of the proof language

The theorems the built-in lemmas of `apply` are (`doc/plan/proof-language.md`,
"Lemmas"): the arithmetic lemmas over the specification's `*`, `/`, `%`, `<<`
and `>>`, which elaborate to `Int.tdiv`, `Int.tmod`, `Int.shiftLeft` and
`Int.shiftRight`; the vector lemmas over the vector operations as the
denotation states them; and the coverage of a map's keys by a vector. Each
binder a lemma's parameter fills carries the parameter's name; the others are
types, instances, an element type's fallback, or premises the application
owes.
-/

namespace VerexIR.Proofs.Denote.StepLemmas

/-! ## Arithmetic -/

theorem mul_nonneg (a b : Int) (_ : 0 ≤ a) (_ : 0 ≤ b) : 0 ≤ a * b :=
  Int.mul_nonneg ‹_› ‹_›

theorem mul_mono (a b c d : Int) (_ : 0 ≤ a) (_ : a ≤ b) (_ : 0 ≤ c) (_ : c ≤ d) :
    a * c ≤ b * d :=
  Int.mul_le_mul ‹_› ‹_› ‹_› (Int.le_trans ‹0 ≤ a› ‹a ≤ b›)

theorem mul_strict_mono (a b c : Int) (_ : a < b) (_ : 0 < c) : a * c < b * c :=
  Int.mul_lt_mul_of_pos_right ‹_› ‹_›

theorem div_nonneg (x c : Int) (_ : 0 ≤ x) (_ : 0 < c) : 0 ≤ x.tdiv c := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.ediv_nonneg ‹_› (Int.le_of_lt ‹_›)

theorem div_le (x q c : Int) (_ : 0 ≤ x) (_ : 0 < c) (_ : x ≤ q * c) : x.tdiv c ≤ q := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.ediv_le_of_le_mul ‹_› ‹_›

theorem div_lt (x q c : Int) (_ : 0 ≤ x) (_ : 0 < c) (_ : x < q * c) : x.tdiv c < q := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.ediv_lt_of_lt_mul ‹_› ‹_›

theorem le_div (x q c : Int) (_ : 0 ≤ x) (_ : 0 < c) (_ : q * c ≤ x) : q ≤ x.tdiv c := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.le_ediv_of_mul_le ‹_› ‹_›

theorem div_mono (x y c : Int) (_ : 0 ≤ x) (_ : x ≤ y) (_ : 0 < c) : x.tdiv c ≤ y.tdiv c := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›, Int.tdiv_eq_ediv_of_nonneg (Int.le_trans ‹0 ≤ x› ‹_›)]
  exact Int.ediv_le_ediv ‹_› ‹_›

theorem div_mul_le (x c : Int) (_ : 0 ≤ x) (_ : 0 < c) : x.tdiv c * c ≤ x := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.ediv_mul_le x (Int.ne_of_gt ‹_›)

theorem div_add_mul (x b c : Int) (_ : 0 ≤ x) (_ : 0 ≤ b) (_ : 0 < c) :
    (x + b * c).tdiv c = x.tdiv c + b := by
  have : 0 ≤ x + b * c := Int.add_nonneg ‹_› (Int.mul_nonneg ‹_› (Int.le_of_lt ‹_›))
  rw [Int.tdiv_eq_ediv_of_nonneg this, Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.add_mul_ediv_right x b (Int.ne_of_gt ‹_›)

theorem mod_small (x c : Int) (_ : 0 ≤ x) (_ : x < c) : x.tmod c = x := by
  rw [Int.tmod_eq_emod_of_nonneg ‹_›]
  exact Int.emod_eq_of_lt ‹_› ‹_›

theorem shl_mul (x k : Int) : x.shiftLeft k.toNat = x * 2 ^ k.toNat :=
  Int.shiftLeft_eq x k.toNat

theorem shr_div (x k : Int) (_ : 0 ≤ x) : x.shiftRight k.toNat = x.tdiv (2 ^ k.toNat) := by
  rw [Int.tdiv_eq_ediv_of_nonneg ‹_›]
  exact Int.shiftRight_eq_div_pow x k.toNat

/-! ## Vectors

A vector's length is an integer, its element at an integer position read
as a clause reads it, the element type's fallback outside the vector. -/

variable {α : Type}

theorem vector_len_remove (v : SpecVector α) (i : Int) (_ : 0 ≤ i) (_ : i < v.values.size) :
    ((v.values.eraseIdxIfInBounds i.toNat).size : Int) = v.values.size - 1 := by
  rw [Array.eraseIdxIfInBounds_eq]
  split
  · simp only [Array.size_eraseIdx]; omega
  · omega

private theorem vector_index_remove_values (v : SpecVector α) (fallback : α) (i k : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) (_ : 0 ≤ k) :
    (k < i → (eraseBounded v i.toNat).values[k.toNat]?.getD fallback =
        v.values[k.toNat]?.getD fallback) ∧
      (i ≤ k → (eraseBounded v i.toNat).values[k.toNat]?.getD fallback =
        v.values[(k + 1).toNat]?.getD fallback) := by
  have within : i.toNat < v.values.size := by omega
  rw [eraseBounded_values, eraseIdxIfInBounds_of_lt _ _ within]
  refine ⟨fun before => ?_, fun after => ?_⟩
  · rw [getElem?_eraseIdx_toNat_of_lt _ _ within k ‹_› (by omega)]
  · rw [getElem?_eraseIdx_toNat_of_ge _ _ within k (by omega)]

theorem vector_len_pop_back (v : SpecVector α) (_ : 0 < (v.values.size : Int)) :
    ((v.values.eraseIdxIfInBounds (v.values.size - 1)).size : Int) = v.values.size - 1 := by
  have := vector_len_remove v (v.values.size - 1 : Nat) (by omega) (by omega)
  simpa only [Int.toNat_natCast] using this

private theorem vector_index_pop_back_values (v : SpecVector α) (fallback : α) (k : Int) (_ : 0 ≤ k)
    (_ : k < (v.values.size : Int) - 1) :
    (eraseBounded v (v.values.size - 1)).values[k.toNat]?.getD fallback =
      v.values[k.toNat]?.getD fallback := by
  have := (vector_index_remove_values v fallback (v.values.size - 1 : Nat) k (by omega) (by omega)
    ‹_›).1 (by omega)
  simpa only [Int.toNat_natCast] using this

theorem vector_len_swap (v : SpecVector α) (i j : Int) :
    ((v.values.swapIfInBounds i.toNat j.toNat).size : Int) = v.values.size := by
  rw [Array.size_swapIfInBounds]

private theorem swap_read (xs : Array α) (i j k : Nat) (hi : i < xs.size) (hj : j < xs.size)
    (fallback : α) :
    (xs.swap i j)[k]?.getD fallback = if k = i then xs[j]?.getD fallback
      else if k = j then xs[i]?.getD fallback else xs[k]?.getD fallback := by
  rw [Array.getElem?_swap, Array.getElem?_eq_getElem hi, Array.getElem?_eq_getElem hj]
  by_cases h1 : k = i <;> by_cases h2 : k = j
  · subst h1; subst h2; simp
  · subst h1; simp [Ne.symm h2]
  · subst h2; simp [h1]
  · simp [Ne.symm h1, Ne.symm h2, h1, h2]

private theorem vector_index_swap_values (v : SpecVector α) (fallback : α) (i j k : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) (_ : 0 ≤ j) (_ : j < v.values.size) (_ : 0 ≤ k) :
    (k = i → (swapBounded v i.toNat j.toNat).values[k.toNat]?.getD fallback =
        v.values[j.toNat]?.getD fallback) ∧
      (k = j → k ≠ i → (swapBounded v i.toNat j.toNat).values[k.toNat]?.getD fallback =
        v.values[i.toNat]?.getD fallback) ∧
      (k ≠ i → k ≠ j → (swapBounded v i.toNat j.toNat).values[k.toNat]?.getD fallback =
        v.values[k.toNat]?.getD fallback) := by
  have hi : i.toNat < v.values.size := by omega
  have hj : j.toNat < v.values.size := by omega
  rw [swapBounded_values, Array.swapIfInBounds_def]
  simp only [hi, hj, dite_true, swap_read _ _ _ _ hi hj]
  refine ⟨fun same => ?_, fun same other => ?_, fun other other' => ?_⟩
  · simp [show k.toNat = i.toNat by omega]
  · have : ¬k.toNat = i.toNat := by omega
    rw [if_neg this, if_pos (by omega)]
  · simp [show ¬k.toNat = i.toNat by omega, show ¬k.toNat = j.toNat by omega]

theorem vector_len_swap_remove (v : SpecVector α) (i : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) :
    (((v.values.swapIfInBounds i.toNat (v.values.size - 1)).eraseIdxIfInBounds
      (v.values.size - 1)).size : Int) = v.values.size - 1 := by
  rw [Array.eraseIdxIfInBounds_eq]
  split
  · simp only [Array.size_eraseIdx, Array.size_swapIfInBounds]; omega
  · simp only [Array.size_swapIfInBounds] at *; omega

private theorem vector_index_swap_remove_values (v : SpecVector α) (fallback : α) (i k : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) (_ : 0 ≤ k) (_ : k < (v.values.size : Int) - 1) :
    (k = i → (eraseBounded (swapBounded v i.toNat (v.values.size - 1))
        (v.values.size - 1)).values[k.toNat]?.getD fallback =
        v.values[(v.values.size - 1 : Nat)]?.getD fallback) ∧
      (k ≠ i → (eraseBounded (swapBounded v i.toNat (v.values.size - 1))
        (v.values.size - 1)).values[k.toNat]?.getD fallback =
        v.values[k.toNat]?.getD fallback) := by
  have swapped := vector_index_swap_values v fallback i (v.values.size - 1 : Nat) k ‹_› ‹_› (by omega)
    (by omega) ‹_›
  have size : (swapBounded v i.toNat (v.values.size - 1)).values.size = v.values.size := by
    rw [swapBounded_values, Array.size_swapIfInBounds]
  have popped := vector_index_pop_back_values (swapBounded v i.toNat (v.values.size - 1)) fallback k ‹_›
    (by rw [size]; omega)
  simp only [Int.toNat_natCast, size] at swapped popped
  rw [popped]
  refine ⟨fun same => (swapped.1 same), fun other => swapped.2.2 other (by omega)⟩

theorem vector_len_push_back (v : SpecVector α) (e : α) (_ : (v.values.size : Int) + 1 < 2 ^ 64) :
    ((v.push e).values.size : Int) = v.values.size + 1 := by
  rw [SpecVector.values_push e (by omega), Array.size_push]
  omega

private theorem vector_index_push_back_values (v : SpecVector α) (fallback e : α) (k : Int) (_ : 0 ≤ k)
    (_ : (v.values.size : Int) + 1 < 2 ^ 64) :
    (k < v.values.size → (v.push e).values[k.toNat]?.getD fallback =
        v.values[k.toNat]?.getD fallback) ∧
      (k = v.values.size → (v.push e).values[k.toNat]?.getD fallback = e) := by
  rw [SpecVector.values_push e (by omega)]
  refine ⟨fun before => ?_, fun last => ?_⟩
  · rw [getElem?_push_of_lt _ _ _ (by omega)]
  · rw [Array.getElem?_push]
    simp only [show k.toNat = v.values.size by omega, if_true, Option.getD_some]

theorem vector_len_update (v : SpecVector α) (i : Int) (e : α) :
    ((v.set i.toNat e).values.size : Int) = v.values.size := by
  rw [SpecVector.values_set, Array.size_set!]

private theorem vector_index_update_values (v : SpecVector α) (fallback : α) (i : Int) (e : α) (k : Int)
    (_ : 0 ≤ i) (_ : i < v.values.size) (_ : 0 ≤ k) :
    (k = i → (v.set i.toNat e).values[k.toNat]?.getD fallback = e) ∧
      (k ≠ i → (v.set i.toNat e).values[k.toNat]?.getD fallback =
        v.values[k.toNat]?.getD fallback) := by
  rw [SpecVector.values_set, Array.set!_eq_setIfInBounds]
  refine ⟨fun same => ?_, fun other => ?_⟩
  · subst same
    rw [Array.getElem?_setIfInBounds_self_of_lt (by omega)]
    rfl
  · rw [Array.getElem?_setIfInBounds_ne (by omega)]

theorem vector_len_concat (v w : SpecVector α)
    (_ : (v.values.size : Int) + w.values.size < 2 ^ 64) :
    ((v.append w).values.size : Int) = v.values.size + w.values.size := by
  rw [SpecVector.values_append (by omega), Array.size_append]
  omega

private theorem vector_index_concat_values (v w : SpecVector α) (fallback : α) (k : Int) (_ : 0 ≤ k)
    (_ : (v.values.size : Int) + w.values.size < 2 ^ 64) :
    (k < v.values.size → (v.append w).values[k.toNat]?.getD fallback =
        v.values[k.toNat]?.getD fallback) ∧
      (v.values.size ≤ k → (v.append w).values[k.toNat]?.getD fallback =
        w.values[(k - v.values.size).toNat]?.getD fallback) := by
  rw [SpecVector.values_append (by omega)]
  refine ⟨fun before => ?_, fun after => ?_⟩
  · rw [Array.getElem?_append_left (by omega)]
  · rw [Array.getElem?_append_right (by omega)]
    congr 2
    omega

theorem vector_len_slice (v : SpecVector α) (a b : Int) (_ : 0 ≤ a) (_ : a ≤ b)
    (_ : b ≤ v.values.size) :
    ((v.extract a.toNat b.toNat).values.size : Int) = b - a := by
  rw [SpecVector.values_extract, Array.size_extract]
  omega

private theorem vector_index_slice_values (v : SpecVector α) (fallback : α) (a b k : Int) (_ : 0 ≤ a)
    (_ : 0 ≤ k) (_ : k < b - a) (_ : b ≤ v.values.size) :
    (v.extract a.toNat b.toNat).values[k.toNat]?.getD fallback =
      v.values[(a + k).toNat]?.getD fallback := by
  rw [SpecVector.values_extract, Array.getElem?_extract]
  have : k.toNat < min b.toNat v.values.size - a.toNat := by omega
  simp only [this, if_true]
  congr 2
  omega

/-! The element lemmas read the elements as a clause reads them (`read`):
an integer by its value, any other value itself. -/

section
variable {β : Sort _} (read : α → β)

theorem vector_index_remove (v : SpecVector α) (fallback : α) (i k : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) (_ : 0 ≤ k) :
    (k < i → read ((v.values.eraseIdxIfInBounds i.toNat)[k.toNat]?.getD fallback) =
        read (v.values[k.toNat]?.getD fallback)) ∧
      (i ≤ k → read ((v.values.eraseIdxIfInBounds i.toNat)[k.toNat]?.getD fallback) =
        read (v.values[(k + 1).toNat]?.getD fallback)) := by
  have h := vector_index_remove_values v fallback i k ‹_› ‹_› ‹_›
  simp only [eraseBounded_values] at h
  exact ⟨fun c => congrArg read (h.1 c), fun c => congrArg read (h.2 c)⟩

theorem vector_index_pop_back (v : SpecVector α) (fallback : α) (k : Int) (_ : 0 ≤ k)
    (_ : k < (v.values.size : Int) - 1) :
    read ((v.values.eraseIdxIfInBounds (v.values.size - 1))[k.toNat]?.getD fallback) =
      read (v.values[k.toNat]?.getD fallback) := by
  have h := vector_index_pop_back_values v fallback k ‹_› ‹_›
  simp only [eraseBounded_values] at h
  exact congrArg read h

theorem vector_index_swap (v : SpecVector α) (fallback : α) (i j k : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) (_ : 0 ≤ j) (_ : j < v.values.size) (_ : 0 ≤ k) :
    (k = i → read ((v.values.swapIfInBounds i.toNat j.toNat)[k.toNat]?.getD fallback) =
        read (v.values[j.toNat]?.getD fallback)) ∧
      (k = j → k ≠ i → read ((v.values.swapIfInBounds i.toNat j.toNat)[k.toNat]?.getD fallback) =
        read (v.values[i.toNat]?.getD fallback)) ∧
      (k ≠ i → k ≠ j → read ((v.values.swapIfInBounds i.toNat j.toNat)[k.toNat]?.getD fallback) =
        read (v.values[k.toNat]?.getD fallback)) := by
  have h := vector_index_swap_values v fallback i j k ‹_› ‹_› ‹_› ‹_› ‹_›
  simp only [swapBounded_values] at h
  exact ⟨fun c => congrArg read (h.1 c), fun c d => congrArg read (h.2.1 c d),
    fun c d => congrArg read (h.2.2 c d)⟩

theorem vector_index_swap_remove (v : SpecVector α) (fallback : α) (i k : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) (_ : 0 ≤ k) (_ : k < (v.values.size : Int) - 1) :
    (k = i → read (((v.values.swapIfInBounds i.toNat (v.values.size - 1)).eraseIdxIfInBounds (v.values.size - 1))[k.toNat]?.getD fallback) =
        read (v.values[(v.values.size - 1 : Nat)]?.getD fallback)) ∧
      (k ≠ i → read (((v.values.swapIfInBounds i.toNat (v.values.size - 1)).eraseIdxIfInBounds (v.values.size - 1))[k.toNat]?.getD fallback) =
        read (v.values[k.toNat]?.getD fallback)) := by
  have h := vector_index_swap_remove_values v fallback i k ‹_› ‹_› ‹_› ‹_›
  simp only [eraseBounded_values, swapBounded_values] at h
  exact ⟨fun c => congrArg read (h.1 c), fun c => congrArg read (h.2 c)⟩

theorem vector_index_push_back (v : SpecVector α) (fallback e : α) (k : Int) (low : 0 ≤ k)
    (bounded : (v.values.size : Int) + 1 < 2 ^ 64) :
    (k < v.values.size → read ((v.push e).values[k.toNat]?.getD fallback) =
        read (v.values[k.toNat]?.getD fallback)) ∧
      (k = v.values.size → read ((v.push e).values[k.toNat]?.getD fallback) = read e) := by
  have h := vector_index_push_back_values v fallback e k low bounded
  exact ⟨fun c => congrArg read (h.1 c), fun c => congrArg read (h.2 c)⟩

theorem vector_index_update (v : SpecVector α) (fallback : α) (i : Int) (e : α) (k : Int)
    (_ : 0 ≤ i) (_ : i < v.values.size) (_ : 0 ≤ k) :
    (k = i → read ((v.set i.toNat e).values[k.toNat]?.getD fallback) = read e) ∧
      (k ≠ i → read ((v.set i.toNat e).values[k.toNat]?.getD fallback) =
        read (v.values[k.toNat]?.getD fallback)) := by
  have h := vector_index_update_values v fallback i e k ‹_› ‹_› ‹_›
  exact ⟨fun c => congrArg read (h.1 c), fun c => congrArg read (h.2 c)⟩

theorem vector_index_concat (v w : SpecVector α) (fallback : α) (k : Int) (low : 0 ≤ k)
    (bounded : (v.values.size : Int) + w.values.size < 2 ^ 64) :
    (k < v.values.size → read ((v.append w).values[k.toNat]?.getD fallback) =
        read (v.values[k.toNat]?.getD fallback)) ∧
      (v.values.size ≤ k → read ((v.append w).values[k.toNat]?.getD fallback) =
        read (w.values[(k - v.values.size).toNat]?.getD fallback)) := by
  have h := vector_index_concat_values v w fallback k low bounded
  exact ⟨fun c => congrArg read (h.1 c), fun c => congrArg read (h.2 c)⟩

theorem vector_index_slice (v : SpecVector α) (fallback : α) (a b k : Int) (_ : 0 ≤ a)
    (_ : 0 ≤ k) (_ : k < b - a) (_ : b ≤ v.values.size) :
    read ((v.extract a.toNat b.toNat).values[k.toNat]?.getD fallback) =
      read (v.values[(a + k).toNat]?.getD fallback) := by
  have h := vector_index_slice_values v fallback a b k ‹_› ‹_› ‹_› ‹_›
  exact congrArg read h

/-- The search `index_of` makes, as the denotation states its position:
where the vector holds the needle, the position is within the vector and
holds it. -/
theorem vector_index_of (v : SpecVector α) (fallback e : α) (eq : α → α → Bool)
    (sound : ∀ a b, eq a b = true ↔ a = b) (_ : e ∈ v.values) :
    (((findIndex? eq v.values e v.values.size 0).getD 0 : Nat) : Int) < v.values.size ∧
      read (v.values[(findIndex? eq v.values e v.values.size 0).getD 0]?.getD fallback) =
        read e := by
  obtain ⟨i, hi, he⟩ := Array.getElem_of_mem ‹e ∈ v.values›
  cases found : findIndex? eq v.values e v.values.size 0 with
  | none =>
      exfalso
      have := (findIndex?_eq_none_mem_iff eq v.values e).mp found
      exact absurd (this _ (Array.getElem_mem hi)) (by simp [(sound _ _).mpr he])
  | some position =>
      obtain ⟨within, ⟨x, hx, hit⟩, -⟩ := (findIndex?_eq_some_iff eq v.values e position).mp found
      have same : x = e := (sound _ _).mp hit
      subst same
      refine ⟨by simp only [Option.getD_some]; omega, ?_⟩
      simp only [Option.getD_some, hx]

end

/-- A vector holds the element at each of its positions. -/
theorem vector_contains_index (v : SpecVector α) (fallback : α) (i : Int) (_ : 0 ≤ i)
    (_ : i < v.values.size) : v.values[i.toNat]?.getD fallback ∈ v.values := by
  have within : i.toNat < v.values.size := by omega
  rw [Array.getElem?_eq_getElem within, Option.getD_some]
  exact Array.getElem_mem within

/-- A distinct vector without the element at a position holds that element
no more. -/
theorem vector_contains_remove {β : Sort _} (read : α → β) (v : SpecVector α) (fallback : α)
    (i : Int) (e : α) (_ : 0 ≤ i) (_ : i < v.values.size)
    (distinct : ∀ a : Int, 0 ≤ a → a < v.values.size → ∀ b : Int, 0 ≤ b → b < v.values.size →
      read (v.values[a.toNat]?.getD fallback) = read (v.values[b.toNat]?.getD fallback) → a = b)
    (held : read (v.values[i.toNat]?.getD fallback) = read e) :
    ¬e ∈ v.values.eraseIdxIfInBounds i.toNat := by
  have within : i.toNat < v.values.size := by omega
  rw [eraseIdxIfInBounds_of_lt _ _ within, Array.mem_eraseIdx_iff_getElem]
  rintro ⟨j, hj, other, found⟩
  have same := distinct j (by omega) (by omega) i ‹_› ‹_› (by
    simp only [Int.toNat_natCast, Array.getElem?_eq_getElem hj, Option.getD_some, found]
    exact held.symm)
  omega

/-- Vectors whose elements read alike, by a read that tells elements apart,
are equal. -/
theorem vector_ext {β : Sort _} (read : α → β) (injective : ∀ a b, read a = read b → a = b)
    (v w : SpecVector α) (fallback : α) (_ : (v.values.size : Int) = w.values.size)
    (_ : ∀ k : Int, 0 ≤ k → k < v.values.size →
      read (v.values[k.toNat]?.getD fallback) = read (w.values[k.toNat]?.getD fallback)) :
    v = w := by
  apply SpecVector.ext
  apply Array.ext (by omega)
  intro k hv hw
  rename_i same
  have := injective _ _ (same k (by omega) (by omega))
  simp only [Int.toNat_natCast, Array.getElem?_eq_getElem hv, Array.getElem?_eq_getElem hw,
    Option.getD_some] at this
  exact this

/-- The reads a clause reads elements by tell them apart. -/
theorem int_read_injective {width : IntWidth} {signed : Bool} :
    ∀ a b : SpecInt width signed, a.val = b.val → a = b :=
  fun _ _ => SpecInt.ext

theorem bool_read_injective : ∀ a b : Bool, (a = true) = (b = true) → a = b := by
  intro a b h
  cases a <;> cases b <;> simp_all

theorem self_read_injective {α : Type} : ∀ a b : α, a = b → a = b :=
  fun _ _ h => h

/-! ## Map coverage -/

/-- A key of a map is in a distinct vector of the map's size whose elements
are keys of it: its absence is absurd. -/
theorem contains_key_len_search_absurd {K V : Type} [DecidableEq K] (map : SpecMap K V)
    (keys : SpecVector K) (fallback : K) (key : K)
    (_ : ∀ i : Int, 0 ≤ i → i < keys.values.size →
      SpecMap.hasKey map (keys.values[i.toNat]?.getD fallback) = true)
    (_ : (SpecMap.size map : Int) = keys.values.size)
    (_ : ∀ i : Int, 0 ≤ i → i < keys.values.size → ∀ j : Int, 0 ≤ j → j < keys.values.size →
      keys.values[i.toNat]?.getD fallback = keys.values[j.toNat]?.getD fallback → i = j)
    (_ : SpecMap.hasKey map key = true) (missing : ¬key ∈ keys.values) : False :=
  map_key_search_absurd map keys.values fallback ‹_› (by omega) ‹_› key ‹_›
    fun i low high found => missing ((mem_iff_exists_int_index _ _).mpr ⟨i, low, high, found⟩)

end VerexIR.Proofs.Denote.StepLemmas
