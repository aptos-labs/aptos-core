-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Denote.Term

/-!
# Certified vector operation normalization

Swapping and removing elements cannot exceed an already certified vector's
length. Rewrite their denotations to one bounds check and a certified result,
without re-checking representability or splitting redundant optional reads.
The map laws preserve generic caller/callee transport of these results.
-/

namespace LeanerIR.Proofs

/-- Split a dependent computation while retaining its branch evidence. -/
theorem wp_dite_rule {σ ε α : Type} (condition : Prop) [Decidable condition]
    (yes : condition → Spec σ ε α) (no : ¬condition → Spec σ ε α)
    (ensures : α → σ → Prop) (aborts : ε → Prop) (state : σ) :
    wp (if h : condition then yes h else no h) ensures aborts state ↔
      ((∀ h : condition, wp (yes h) ensures aborts state) ∧
        (∀ h : ¬condition, wp (no h) ensures aborts state)) := by
  by_cases h : condition <;> simp [h]
end LeanerIR.Proofs

namespace LeanerIR.Proofs.Denote
variable {unit : Validation.ValidatedUnit} [Skolems unit]

/-- A logical integer read needs neither a runtime encoding nor a runtime
projection. Keep the certified value so its bounds remain available even
when the optional read is unknown. The missing-entry value is still zero. -/
@[lir_denote_norm] theorem asInt_getD_map_specInt (width : Nat) (signed : Bool)
    (nonzero : width ≠ 0) (value : Option (SpecInt (.bits width) signed)) :
    ((value.map (Codec.specInt (.bits width) signed).encode).getD .unit).asInt =
      (value.getD ⟨0, zero_fits width signed nonzero⟩).val := by
  cases value <;> rfl

/-- Executable unsigned indices cannot take a negative-index branch. -/
@[lir_denote_norm] theorem unsigned_val_not_negative {width : Nat}
    (value : SpecInt (.bits width) false) : ¬value.val < 0 := by
  have := (SpecInt.unsigned_bounds value).1
  omega

/-- Swapping preserves the representable length without a fresh runtime check. -/
def swapBounded {α : Type} (v : SpecVector α) (i j : Nat) : SpecVector α :=
  ⟨v.values.swapIfInBounds i j, by simpa only [Array.size_swapIfInBounds] using v.bounded⟩

@[lir_denote ↓ high] theorem denote_swap_direct {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    {width : Nat} {signed : Bool}
    (vector : Term unit ρ Γ (.vector τ)) (left right : Term unit ρ Γ (.int width signed))
    (executable : Validation.ExecutableUnit unit) (meanings : Meanings executable) (env : HEnv Γ) :
    (Term.swap vector left right).denote meanings env =
      Flow.bind (vector.denote meanings env) fun values env =>
        Flow.bind (left.denote meanings env) fun i env =>
          Flow.bind (right.denote meanings env) fun j env =>
            if i.val < 0 ∨ j.val < 0 ∨ values.values.size ≤ i.val.toNat ∨
                values.values.size ≤ j.val.toNat then
              Spec.abort (.abort, #[.integer i.val, .integer j.val])
            else Spec.pure (.value (swapBounded values i.val.toNat j.val.toNat) env) := by
  rw [Term.denote]
  congr 1
  funext values env
  congr 1
  funext i env
  congr 1
  funext j env
  by_cases h : i.val < 0 ∨ j.val < 0 ∨ values.values.size ≤ i.val.toNat ∨
      values.values.size ≤ j.val.toNat
  · simp only [if_pos h]
  · have hi : i.val.toNat < values.values.size := by omega
    have hj : j.val.toNat < values.values.size := by omega
    simp only [if_neg h, Array.getElem?_eq_getElem hi, Array.getElem?_eq_getElem hj]
    simp only [SpecVector.ofArray?, Array.size_set!, dif_pos values.bounded]
    simp only [swapBounded, Array.swapIfInBounds_def, Array.swap_def, dif_pos,
      Array.set!_eq_setIfInBounds, Array.setIfInBounds, hi, hj, Array.size_set]

@[lir_denote_norm] theorem swapBounded_values {α : Type} (v : SpecVector α) (i j : Nat) :
    (swapBounded v i j).values = v.values.swapIfInBounds i j := rfl

/-- Removing an element preserves representability, even for an empty vector. -/
def eraseBounded {α : Type} (v : SpecVector α) (i : Nat) : SpecVector α :=
  ⟨v.values.eraseIdxIfInBounds i, by
    have bounded := v.bounded
    rw [Array.eraseIdxIfInBounds_eq]
    split
    · simp only [Array.size_eraseIdx]; omega
    · exact bounded⟩

-- Keep the result carrier explicit: inferred partially expanded HLists
-- obstruct later normalization across generic caller/callee frames.
@[lir_denote ↓ high] theorem denote_remove_direct {ρ : ResultShape} {Γ : NRow} {τ : NTy}
    {width : Nat} {signed : Bool}
    (vector : Term unit ρ Γ (.vector τ)) (position : Term unit ρ Γ (.int width signed))
    (executable : Validation.ExecutableUnit unit) (meanings : Meanings executable) (env : HEnv Γ) :
    (Term.remove vector position).denote meanings env =
      Flow.bind (β := NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))
        (vector.denote meanings env) fun values env =>
        Flow.bind (β := NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))
          (position.denote meanings env) fun i env =>
          if h : i.val < 0 ∨ values.values.size ≤ i.val.toNat then
            (Spec.abort (.abort, #[.integer i.val]) :
              Comp unit (Flow ρ Γ (NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))))
          else Spec.pure (@Flow.value unit _ ρ Γ
            (NTy.carrier (.tuple (.cons τ (.cons (.vector τ) .nil))))
            ((values.values[i.val.toNat]'(Nat.lt_of_not_ge (fun outside => h (Or.inr outside))),
              (eraseBounded values i.val.toNat, ())) :
              HList (.cons τ (.cons (.vector τ) .nil))) env) := by
  rw [Term.denote]
  congr 1
  funext values env
  congr 1
  funext i env
  by_cases low : i.val < 0
  · simp only [if_pos low, dif_pos (Or.inl low)]
  · simp only [if_neg low]
    by_cases high : values.values.size ≤ i.val.toNat
    · simp only [Array.getElem?_eq_none high, dif_pos (Or.inr high)]
    · have within : i.val.toNat < values.values.size := by omega
      have bounded := (eraseBounded values i.val.toNat).bounded
      change (values.values.eraseIdxIfInBounds i.val.toNat).size < 2 ^ 64 at bounded
      have inside : ¬(i.val < 0 ∨ values.values.size ≤ i.val.toNat) := by omega
      simp only [Array.getElem?_eq_getElem within, dif_neg inside,
        SpecVector.ofArray?, dif_pos bounded]
      rfl

@[lir_denote_norm] theorem eraseBounded_values {α : Type} (v : SpecVector α) (i : Nat) :
    (eraseBounded v i).values = v.values.eraseIdxIfInBounds i := rfl

-- Keep the certified constructors folded; their projections have normalization rules.
attribute [irreducible] swapBounded eraseBounded
attribute [lir_denote_norm] Array.size_swapIfInBounds
-- Removal reads carry an explicit bounds proof. Expose elementwise transport
-- through those reads, just as the normalizer already does for optional reads.
attribute [lir_denote_norm] Array.getElem_map

@[lir_denote_norm] theorem swapIfInBounds_map {α β : Type} (xs : Array α) (f : α → β) (i j : Nat) :
    (xs.map f).swapIfInBounds i j = (xs.swapIfInBounds i j).map f := by
  by_cases hi : i < xs.size <;> by_cases hj : j < xs.size
  all_goals simp [Array.swapIfInBounds_def, hi, hj, Array.swap_def]
private theorem list_map_erase {α β : Type} (xs : List α) (f : α → β) (i : Nat) :
    (xs.map f).eraseIdx i = (xs.eraseIdx i).map f := by
  induction xs generalizing i with
  | nil => simp
  | cons x xs ih => cases i <;> simp [ih]
@[lir_denote_norm] theorem eraseIdxIfInBounds_map {α β : Type} (xs : Array α) (f : α → β) (i : Nat) :
    (xs.map f).eraseIdxIfInBounds i = (xs.eraseIdxIfInBounds i).map f := by
  apply Array.toList_inj.mp
  simp only [Array.toList_eraseIdxIfInBounds, Array.toList_map]
  exact list_map_erase xs.toList f i

@[lir_denote_norm] theorem findIndex?_getD_lt_iff {α : Type} (eq : α → α → Bool)
    (elements : Array α) (needle : α) (count index : Nat) :
    (findIndex? eq elements needle count index).getD 0 < elements.size ↔ 0 < elements.size := by
  constructor
  · intro h; omega
  · intro h
    cases found : findIndex? eq elements needle count index with
    | none => simpa only [Option.getD_none]
    | some position =>
      simpa only [Option.getD_some] using findIndex?_lt eq elements needle count index position found
@[lir_denote_norm] theorem size_le_findIndex?_getD_iff {α : Type} (eq : α → α → Bool)
    (elements : Array α) (needle : α) (count index : Nat) :
    elements.size ≤ (findIndex? eq elements needle count index).getD 0 ↔ elements.size = 0 := by
  have := findIndex?_getD_lt_iff eq elements needle count index
  omega
end LeanerIR.Proofs.Denote
