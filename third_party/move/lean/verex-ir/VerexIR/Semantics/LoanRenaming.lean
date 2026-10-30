-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Semantics.Focus

/-!
# Renaming loan identities

Two runs from starts that differ only in their loan bookkeeping and in the
loans their arguments are lent under mint the same loans in the same order,
offset by their frontiers' difference, and lend the same arguments under
loans of their own (`doc/historical/static-typing.md`, "Loan independence").
`rename` maps the loan identities of values, frames, and global memory by
an injective renaming; `Within` states that every identity a value holds
satisfies a predicate, such as having been minted at or beyond a frontier
or being one of the lent loans. Comparisons read values through
`RuntimeValue.observe`, which no renaming changes.
-/

namespace VerexIR

mutual
/-- A value with each loan identity it holds renamed by `ρ`. -/
def RuntimeValue.rename (ρ : Nat → Nat) : RuntimeValue → RuntimeValue
  | .vector elements => .vector (RuntimeValue.renameList ρ elements.toList).toArray
  | .tuple elements => .tuple (RuntimeValue.renameList ρ elements.toList).toArray
  | .nominal source variant fields =>
      .nominal source variant (RuntimeValue.renameList ρ fields.toList).toArray
  | .closure function mask typeInstantiation captures =>
      .closure function mask typeInstantiation
        (RuntimeValue.renameList ρ captures.toList).toArray
  | .borrow loan current => .borrow (ρ loan) (current.rename ρ)
  | .loanHole loan => .loanHole (ρ loan)
  | .ghost contents => .ghost (contents.rename ρ)
  | .unit => .unit
  | .bool value => .bool value
  | .character value => .character value
  | .integer value => .integer value
  | .address value => .address value
  | .signer value => .signer value
  | .string value => .string value
  | .bytes value => .bytes value
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

/-- Each value of a list with its loan identities renamed. -/
def RuntimeValue.renameList (ρ : Nat → Nat) : List RuntimeValue → List RuntimeValue
  | [] => []
  | value :: values => value.rename ρ :: RuntimeValue.renameList ρ values
termination_by values => sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
/-- Every loan identity a value holds satisfies `inside`. -/
def RuntimeValue.Within (inside : Nat → Prop) : RuntimeValue → Prop
  | .vector elements | .tuple elements | .nominal _ _ elements | .closure _ _ _ elements =>
      RuntimeValue.WithinList inside elements.toList
  | .borrow loan current => inside loan ∧ current.Within inside
  | .loanHole loan => inside loan
  | .ghost contents => contents.Within inside
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => True
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

/-- Every loan identity a list's values hold satisfies `inside`. -/
def RuntimeValue.WithinList (inside : Nat → Prop) : List RuntimeValue → Prop
  | [] => True
  | value :: values => value.Within inside ∧ RuntimeValue.WithinList inside values
termination_by values => sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem RuntimeValue.renameList_eq_map (ρ : Nat → Nat) :
    ∀ values : List RuntimeValue,
      RuntimeValue.renameList ρ values = values.map (·.rename ρ)
  | [] => by simp [RuntimeValue.renameList]
  | _ :: values => by
      simp [RuntimeValue.renameList, RuntimeValue.renameList_eq_map ρ values]

theorem RuntimeValue.withinList_iff (inside : Nat → Prop) :
    ∀ values : List RuntimeValue,
      RuntimeValue.WithinList inside values ↔ ∀ value ∈ values, value.Within inside
  | [] => by simp [RuntimeValue.WithinList]
  | _ :: values => by
      simp [RuntimeValue.WithinList, RuntimeValue.withinList_iff inside values]

section Equations

variable (ρ : Nat → Nat) (inside : Nat → Prop)

@[simp] theorem RuntimeValue.rename_vector (elements : Array RuntimeValue) :
    (RuntimeValue.vector elements).rename ρ = .vector (elements.map (·.rename ρ)) := by
  rw [RuntimeValue.rename, RuntimeValue.renameList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.rename_tuple (elements : Array RuntimeValue) :
    (RuntimeValue.tuple elements).rename ρ = .tuple (elements.map (·.rename ρ)) := by
  rw [RuntimeValue.rename, RuntimeValue.renameList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.rename_nominal (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    (RuntimeValue.nominal source variant fields).rename ρ =
      .nominal source variant (fields.map (·.rename ρ)) := by
  rw [RuntimeValue.rename, RuntimeValue.renameList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.rename_closure (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) :
    (RuntimeValue.closure function mask typeInstantiation captures).rename ρ =
      .closure function mask typeInstantiation (captures.map (·.rename ρ)) := by
  rw [RuntimeValue.rename, RuntimeValue.renameList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.rename_borrow (loan : Nat) (current : RuntimeValue) :
    (RuntimeValue.borrow loan current).rename ρ = .borrow (ρ loan) (current.rename ρ) := by
  rw [RuntimeValue.rename]

@[simp] theorem RuntimeValue.rename_loanHole (loan : Nat) :
    (RuntimeValue.loanHole loan).rename ρ = .loanHole (ρ loan) := by
  rw [RuntimeValue.rename]

@[simp] theorem RuntimeValue.rename_ghost (contents : RuntimeValue) :
    (RuntimeValue.ghost contents).rename ρ = .ghost (contents.rename ρ) := by
  rw [RuntimeValue.rename]

@[simp] theorem RuntimeValue.rename_unit : RuntimeValue.unit.rename ρ = .unit := by
  rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_bool (value : Bool) :
    (RuntimeValue.bool value).rename ρ = .bool value := by rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_character (value : Nat) :
    (RuntimeValue.character value).rename ρ = .character value := by rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_integer (value : Int) :
    (RuntimeValue.integer value).rename ρ = .integer value := by rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_address (value : String) :
    (RuntimeValue.address value).rename ρ = .address value := by rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_signer (value : String) :
    (RuntimeValue.signer value).rename ρ = .signer value := by rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_string (value : String) :
    (RuntimeValue.string value).rename ρ = .string value := by rw [RuntimeValue.rename]
@[simp] theorem RuntimeValue.rename_bytes (value : Array UInt8) :
    (RuntimeValue.bytes value).rename ρ = .bytes value := by rw [RuntimeValue.rename]

@[simp] theorem RuntimeValue.within_vector (elements : Array RuntimeValue) :
    (RuntimeValue.vector elements).Within inside ↔ ∀ element ∈ elements, element.Within inside := by
  rw [RuntimeValue.Within, RuntimeValue.withinList_iff]; simp
@[simp] theorem RuntimeValue.within_tuple (elements : Array RuntimeValue) :
    (RuntimeValue.tuple elements).Within inside ↔ ∀ element ∈ elements, element.Within inside := by
  rw [RuntimeValue.Within, RuntimeValue.withinList_iff]; simp
@[simp] theorem RuntimeValue.within_nominal (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    (RuntimeValue.nominal source variant fields).Within inside ↔
      ∀ field ∈ fields, field.Within inside := by
  rw [RuntimeValue.Within, RuntimeValue.withinList_iff]; simp
@[simp] theorem RuntimeValue.within_closure (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) :
    (RuntimeValue.closure function mask typeInstantiation captures).Within inside ↔
      ∀ capture ∈ captures, capture.Within inside := by
  rw [RuntimeValue.Within, RuntimeValue.withinList_iff]; simp
@[simp] theorem RuntimeValue.within_borrow (loan : Nat) (current : RuntimeValue) :
    (RuntimeValue.borrow loan current).Within inside ↔ inside loan ∧ current.Within inside := by
  rw [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_loanHole (loan : Nat) :
    (RuntimeValue.loanHole loan).Within inside ↔ inside loan := by
  rw [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_ghost (contents : RuntimeValue) :
    (RuntimeValue.ghost contents).Within inside ↔ contents.Within inside := by
  rw [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_unit : RuntimeValue.unit.Within inside := by
  simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_bool (value : Bool) :
    (RuntimeValue.bool value).Within inside := by simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_character (value : Nat) :
    (RuntimeValue.character value).Within inside := by simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_integer (value : Int) :
    (RuntimeValue.integer value).Within inside := by simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_address (value : String) :
    (RuntimeValue.address value).Within inside := by simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_signer (value : String) :
    (RuntimeValue.signer value).Within inside := by simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_string (value : String) :
    (RuntimeValue.string value).Within inside := by simp [RuntimeValue.Within]
@[simp] theorem RuntimeValue.within_bytes (value : Array UInt8) :
    (RuntimeValue.bytes value).Within inside := by simp [RuntimeValue.Within]

end Equations

/-! ## Lemmas -/

mutual
/-- Renaming twice is renaming by the composite. -/
theorem RuntimeValue.rename_rename (ρ σ : Nat → Nat) (value : RuntimeValue) :
    (value.rename σ).rename ρ = value.rename (ρ ∘ σ) := by
  match value with
  | .vector elements | .tuple elements =>
      simp only [RuntimeValue.rename_vector, RuntimeValue.rename_tuple, Array.map_map]
      congr 1
      exact Array.map_congr_left fun element member => by
        simpa using RuntimeValue.rename_rename ρ σ element
  | .nominal source variant fields =>
      simp only [RuntimeValue.rename_nominal, Array.map_map]
      congr 1
      exact Array.map_congr_left fun element member => by
        simpa using RuntimeValue.rename_rename ρ σ element
  | .closure function mask typeInstantiation captures =>
      simp only [RuntimeValue.rename_closure, Array.map_map]
      congr 1
      exact Array.map_congr_left fun element member => by
        simpa using RuntimeValue.rename_rename ρ σ element
  | .borrow loan current =>
      simp [RuntimeValue.rename_rename ρ σ current]
  | .ghost contents => simp [RuntimeValue.rename_rename ρ σ contents]
  | .loanHole loan => simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; try first
  | (have := Array.sizeOf_lt_of_mem ‹_ ∈ _›; omega) | omega)
end

/-- Renamings that agree on the loans a value holds rename it alike. -/
theorem RuntimeValue.rename_congr {inside : Nat → Prop} {ρ σ : Nat → Nat}
    (same : ∀ loan, inside loan → ρ loan = σ loan) :
    ∀ (value : RuntimeValue), value.Within inside → value.rename ρ = value.rename σ
  | .vector elements, within | .tuple elements, within => by
      simp only [RuntimeValue.within_vector, RuntimeValue.within_tuple] at within
      simp only [RuntimeValue.rename_vector, RuntimeValue.rename_tuple]
      congr 1
      exact Array.map_congr_left fun element member =>
        RuntimeValue.rename_congr same element (within element member)
  | .nominal source variant fields, within => by
      simp only [RuntimeValue.within_nominal] at within
      simp only [RuntimeValue.rename_nominal]
      congr 1
      exact Array.map_congr_left fun element member =>
        RuntimeValue.rename_congr same element (within element member)
  | .closure function mask typeInstantiation captures, within => by
      simp only [RuntimeValue.within_closure] at within
      simp only [RuntimeValue.rename_closure]
      congr 1
      exact Array.map_congr_left fun element member =>
        RuntimeValue.rename_congr same element (within element member)
  | .borrow loan current, within => by
      simp only [RuntimeValue.within_borrow] at within
      simp [same loan within.1, RuntimeValue.rename_congr same current within.2]
  | .ghost contents, within => by
      simp only [RuntimeValue.within_ghost] at within
      simp only [RuntimeValue.rename_ghost, RuntimeValue.rename_congr same contents within]
  | .loanHole loan, within => by
      simp only [RuntimeValue.within_loanHole] at within
      simp [same loan within]
  | .unit, _ | .bool _, _ | .character _, _ | .integer _, _ | .address _, _ | .signer _, _
  | .string _, _ | .bytes _, _ => by simp
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; try first
  | (have := Array.sizeOf_lt_of_mem ‹_ ∈ _›; omega) | omega)

/-- The identity renaming changes nothing. -/
theorem RuntimeValue.rename_id : ∀ (value : RuntimeValue), value.rename id = value
  | .vector elements | .tuple elements => by
      simp only [RuntimeValue.rename_vector, RuntimeValue.rename_tuple]
      congr 1
      conv => rhs; rw [← Array.map_id elements]
      exact Array.map_congr_left fun element member => RuntimeValue.rename_id element
  | .nominal source variant fields => by
      simp only [RuntimeValue.rename_nominal]
      congr 1
      conv => rhs; rw [← Array.map_id fields]
      exact Array.map_congr_left fun element member => RuntimeValue.rename_id element
  | .closure function mask typeInstantiation captures => by
      simp only [RuntimeValue.rename_closure]
      congr 1
      conv => rhs; rw [← Array.map_id captures]
      exact Array.map_congr_left fun element member => RuntimeValue.rename_id element
  | .borrow loan current => by simp [RuntimeValue.rename_id current]
  | .ghost contents => by simp [RuntimeValue.rename_id contents]
  | .loanHole loan => by simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => by simp
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; try first
  | (have := Array.sizeOf_lt_of_mem ‹_ ∈ _›; omega) | omega)

/-- A value is the renaming back of its renaming by a renaming with a left
inverse on the loans it holds. -/
theorem RuntimeValue.rename_of_inverse {inside : Nat → Prop} {ρ σ : Nat → Nat}
    (inverse : ∀ loan, inside loan → ρ (σ loan) = loan) (value : RuntimeValue)
    (within : value.Within inside) : (value.rename σ).rename ρ = value := by
  rw [RuntimeValue.rename_rename,
    RuntimeValue.rename_congr (ρ := ρ ∘ σ) (σ := id) (fun loan member => inverse loan member)
      value within,
    RuntimeValue.rename_id]

/-- A renaming's values hold loans in its image. -/
theorem RuntimeValue.within_rename {inside : Nat → Prop} (ρ : Nat → Nat) :
    ∀ (value : RuntimeValue), value.Within inside →
      (value.rename ρ).Within fun loan => ∃ base, inside base ∧ ρ base = loan
  | .vector elements, within | .tuple elements, within => by
      simp only [RuntimeValue.within_vector, RuntimeValue.within_tuple] at within
      simp only [RuntimeValue.rename_vector, RuntimeValue.rename_tuple,
        RuntimeValue.within_vector, RuntimeValue.within_tuple, Array.mem_map]
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.within_rename ρ element (within element member)
  | .nominal source variant fields, within => by
      simp only [RuntimeValue.within_nominal] at within
      simp only [RuntimeValue.rename_nominal, RuntimeValue.within_nominal, Array.mem_map]
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.within_rename ρ element (within element member)
  | .closure function mask typeInstantiation captures, within => by
      simp only [RuntimeValue.within_closure] at within
      simp only [RuntimeValue.rename_closure, RuntimeValue.within_closure, Array.mem_map]
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.within_rename ρ element (within element member)
  | .borrow loan current, within => by
      simp only [RuntimeValue.within_borrow] at within
      simp only [RuntimeValue.rename_borrow, RuntimeValue.within_borrow]
      exact ⟨⟨loan, within.1, rfl⟩, RuntimeValue.within_rename ρ current within.2⟩
  | .ghost contents, within => by
      simp only [RuntimeValue.within_ghost] at within
      simp only [RuntimeValue.rename_ghost, RuntimeValue.within_ghost]
      exact RuntimeValue.within_rename ρ contents within
  | .loanHole loan, within => by
      simp only [RuntimeValue.within_loanHole] at within
      simp only [RuntimeValue.rename_loanHole, RuntimeValue.within_loanHole]
      exact ⟨loan, within, rfl⟩
  | .unit, _ | .bool _, _ | .character _, _ | .integer _, _ | .address _, _ | .signer _, _
  | .string _, _ | .bytes _, _ => by simp
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; try first
  | (have := Array.sizeOf_lt_of_mem ‹_ ∈ _›; omega) | omega)

/-- A weaker predicate holds of every loan a value holds. -/
theorem RuntimeValue.Within.mono {inside outside : Nat → Prop}
    (weaker : ∀ loan, inside loan → outside loan) :
    ∀ {value : RuntimeValue}, value.Within inside → value.Within outside
  | .vector elements, within | .tuple elements, within => by
      simp only [RuntimeValue.within_vector, RuntimeValue.within_tuple] at within ⊢
      exact fun element member => RuntimeValue.Within.mono weaker (within element member)
  | .nominal source variant fields, within => by
      simp only [RuntimeValue.within_nominal] at within ⊢
      exact fun element member => RuntimeValue.Within.mono weaker (within element member)
  | .closure function mask typeInstantiation captures, within => by
      simp only [RuntimeValue.within_closure] at within ⊢
      exact fun element member => RuntimeValue.Within.mono weaker (within element member)
  | .borrow loan current, within => by
      simp only [RuntimeValue.within_borrow] at within ⊢
      exact ⟨weaker loan within.1, RuntimeValue.Within.mono weaker within.2⟩
  | .ghost contents, within => by
      simp only [RuntimeValue.within_ghost] at within ⊢
      exact RuntimeValue.Within.mono weaker within
  | .loanHole loan, within => by
      simp only [RuntimeValue.within_loanHole] at within ⊢
      exact weaker loan within
  | .unit, _ | .bool _, _ | .character _, _ | .integer _, _ | .address _, _ | .signer _, _
  | .string _, _ | .bytes _, _ => by simp
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; try first
  | (have := Array.sizeOf_lt_of_mem ‹_ ∈ _›; omega) | omega)

/-- Every value holds only loans everything satisfies. -/
theorem RuntimeValue.within_true : ∀ (value : RuntimeValue), value.Within fun _ => True
  | .vector elements | .tuple elements => by
      simp only [RuntimeValue.within_vector, RuntimeValue.within_tuple]
      exact fun element member => RuntimeValue.within_true element
  | .nominal source variant fields => by
      simp only [RuntimeValue.within_nominal]
      exact fun element member => RuntimeValue.within_true element
  | .closure function mask typeInstantiation captures => by
      simp only [RuntimeValue.within_closure]
      exact fun element member => RuntimeValue.within_true element
  | .borrow loan current => by simp [RuntimeValue.within_true current]
  | .ghost contents => by simp [RuntimeValue.within_true contents]
  | .loanHole loan => by simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => by simp
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; try first
  | (have := Array.sizeOf_lt_of_mem ‹_ ∈ _›; omega) | omega)

open Classical in
/-- A left inverse of an injective renaming: the loan it renames to a loan,
where there is one. -/
noncomputable def invertLoans (ρ : Nat → Nat) (loan : Nat) : Nat :=
  if found : ∃ base, ρ base = loan then Classical.choose found else 0

theorem invertLoans_rename {ρ : Nat → Nat} (injective : Function.Injective ρ) (loan : Nat) :
    invertLoans ρ (ρ loan) = loan := by
  have found : ∃ base, ρ base = ρ loan := ⟨loan, rfl⟩
  unfold invertLoans
  rw [dif_pos found]
  exact injective (Classical.choose_spec found)

/-- An injective renaming renames distinct values apart. -/
theorem RuntimeValue.rename_injective {ρ : Nat → Nat} (injective : Function.Injective ρ)
    {left right : RuntimeValue} (same : left.rename ρ = right.rename ρ) : left = right := by
  have inverse : ∀ loan, (fun _ => True) loan → invertLoans ρ (ρ loan) = loan :=
    fun loan _ => invertLoans_rename injective loan
  rw [← RuntimeValue.rename_of_inverse inverse left (RuntimeValue.within_true left),
    same, RuntimeValue.rename_of_inverse inverse right (RuntimeValue.within_true right)]

theorem RuntimeValue.rename_beq {ρ : Nat → Nat} (injective : Function.Injective ρ)
    (left right : RuntimeValue) : (left.rename ρ == right.rename ρ) = (left == right) := by
  rw [Bool.eq_iff_iff, beq_iff_eq, beq_iff_eq]
  exact ⟨RuntimeValue.rename_injective injective, fun same => same ▸ rfl⟩

/-- An injective renaming keeps loans apart. -/
theorem _root_.Function.Injective.beq_eq {ρ : Nat → Nat} (injective : Function.Injective ρ)
    (left right : Nat) : (ρ left == ρ right) = (left == right) := by
  rw [Bool.eq_iff_iff, beq_iff_eq, beq_iff_eq]
  exact injective.eq_iff

/-! ## Loan-free values -/

theorem RuntimeValue.rename_of_plain (ρ : Nat → Nat) :
    ∀ {value : RuntimeValue}, SemanticOperations.Plain value → value.rename ρ = value
  | _, .unit | _, .bool _ | _, .character _ | _, .integer _ | _, .address _ | _, .signer _
  | _, .string _ | _, .bytes _ => by simp
  | _, .ghost contents plain => by
      simp only [RuntimeValue.rename_ghost, RuntimeValue.rename_of_plain ρ plain]
  | _, .vector elements plain => by
      simp only [RuntimeValue.rename_vector, RuntimeValue.vector.injEq]
      conv => rhs; rw [← Array.map_id elements]
      exact Array.map_congr_left fun element member =>
        RuntimeValue.rename_of_plain ρ (plain element member)
  | _, .tuple elements plain => by
      simp only [RuntimeValue.rename_tuple, RuntimeValue.tuple.injEq]
      conv => rhs; rw [← Array.map_id elements]
      exact Array.map_congr_left fun element member =>
        RuntimeValue.rename_of_plain ρ (plain element member)
  | _, .nominal source variant fields plain => by
      simp only [RuntimeValue.rename_nominal, RuntimeValue.nominal.injEq, true_and]
      conv => rhs; rw [← Array.map_id fields]
      exact Array.map_congr_left fun field member =>
        RuntimeValue.rename_of_plain ρ (plain field member)
  | _, .closure function mask typeInstantiation captures plain => by
      simp only [RuntimeValue.rename_closure, RuntimeValue.closure.injEq, true_and]
      conv => rhs; rw [← Array.map_id captures]
      exact Array.map_congr_left fun capture member =>
        RuntimeValue.rename_of_plain ρ (plain capture member)

theorem RuntimeValue.within_of_plain (inside : Nat → Prop) :
    ∀ {value : RuntimeValue}, SemanticOperations.Plain value → value.Within inside
  | _, .unit | _, .bool _ | _, .character _ | _, .integer _ | _, .address _ | _, .signer _
  | _, .string _ | _, .bytes _ => by simp
  | _, .ghost _ plain => by simpa using RuntimeValue.within_of_plain inside plain
  | _, .vector _ plain | _, .tuple _ plain => by
      simpa using fun element member => RuntimeValue.within_of_plain inside (plain element member)
  | _, .nominal _ _ _ plain => by
      simpa using fun element member => RuntimeValue.within_of_plain inside (plain element member)
  | _, .closure _ _ _ _ plain => by
      simpa using fun element member => RuntimeValue.within_of_plain inside (plain element member)

/-! ## Observation

A comparison reads no loan identity, so it reads a value and its renaming
alike. -/

mutual
theorem RuntimeValue.observe_rename (ρ : Nat → Nat) (value : RuntimeValue) :
    (value.rename ρ).observe = value.observe := by
  match value with
  | .vector elements =>
      rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe,
        RuntimeValue.observeList_renameList ρ elements.toList]
  | .tuple elements =>
      rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe,
        RuntimeValue.observeList_renameList ρ elements.toList]
  | .nominal source variant fields =>
      rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe,
        RuntimeValue.observeList_renameList ρ fields.toList]
  | .closure function mask typeInstantiation captures =>
      rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe,
        RuntimeValue.observeList_renameList ρ captures.toList]
  | .borrow loan current =>
      rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe,
        RuntimeValue.observe_rename ρ current]
  | .ghost contents =>
      rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe,
        RuntimeValue.observe_rename ρ contents]
  | .loanHole loan => rw [RuntimeValue.rename, RuntimeValue.observe, RuntimeValue.observe]
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => rw [RuntimeValue.rename]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

theorem RuntimeValue.observeList_renameList (ρ : Nat → Nat) (values : List RuntimeValue) :
    RuntimeValue.observeList (RuntimeValue.renameList ρ values) =
      RuntimeValue.observeList values := by
  match values with
  | [] => rw [RuntimeValue.renameList]
  | head :: tail =>
      rw [RuntimeValue.renameList, RuntimeValue.observeList, RuntimeValue.observeList,
        RuntimeValue.observe_rename ρ head, RuntimeValue.observeList_renameList ρ tail]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

/-- Elements each read as themselves read as the array's list. -/
theorem RuntimeValue.observeList_plain_aux (elements : Array RuntimeValue)
    (same : ∀ element ∈ elements, element.observe = element) :
    elements.toList.map RuntimeValue.observe = elements.toList := by
  conv => rhs; rw [← List.map_id elements.toList]
  exact List.map_congr_left fun element member => same element (Array.mem_toList_iff.mp member)

theorem RuntimeValue.observeList_eq_map (values : List RuntimeValue) :
    RuntimeValue.observeList values = values.map RuntimeValue.observe := by
  induction values with
  | nil => rw [RuntimeValue.observeList]; rfl
  | cons head tail ih => rw [RuntimeValue.observeList, ih]; rfl

/-- A value holding no loan reads as itself. -/
theorem RuntimeValue.observe_of_plain :
    ∀ {value : RuntimeValue}, SemanticOperations.Plain value → value.observe = value
  | _, .unit | _, .bool _ | _, .character _ | _, .integer _ | _, .address _ | _, .signer _
  | _, .string _ | _, .bytes _ => by rw [RuntimeValue.observe]
  | _, .ghost contents plain => by
      rw [RuntimeValue.observe, RuntimeValue.observe_of_plain plain]
  | _, .vector elements plain => by
      rw [RuntimeValue.observe, RuntimeValue.observeList_eq_map,
        RuntimeValue.observeList_plain_aux elements (fun element member =>
          RuntimeValue.observe_of_plain (plain element member)), Array.toArray_toList]
  | _, .tuple elements plain => by
      rw [RuntimeValue.observe, RuntimeValue.observeList_eq_map,
        RuntimeValue.observeList_plain_aux elements (fun element member =>
          RuntimeValue.observe_of_plain (plain element member)), Array.toArray_toList]
  | _, .nominal source variant fields plain => by
      rw [RuntimeValue.observe, RuntimeValue.observeList_eq_map,
        RuntimeValue.observeList_plain_aux fields (fun field member =>
          RuntimeValue.observe_of_plain (plain field member)), Array.toArray_toList]
  | _, .closure function mask typeInstantiation captures plain => by
      rw [RuntimeValue.observe, RuntimeValue.observeList_eq_map,
        RuntimeValue.observeList_plain_aux captures (fun capture member =>
          RuntimeValue.observe_of_plain (plain capture member)), Array.toArray_toList]

/-! ## Walks

A walk over a value with a node matcher that commutes with a renaming commutes
with it as a whole. -/

namespace SemanticOperations

section Walks

variable {ρ : Nat → Nat}

mutual
theorem rewriteFirst_rename {f g : RuntimeValue → Option RuntimeValue}
    (node : ∀ value, f (value.rename ρ) = (g value).map (·.rename ρ))
    (value : RuntimeValue) :
    rewriteFirst f (value.rename ρ) = (rewriteFirst g value).map (·.rename ρ) := by
  unfold rewriteFirst
  rw [node value]
  cases g value with
  | some rewritten => rfl
  | none =>
      cases value with
      | vector elements =>
          simp only [Option.map_none, RuntimeValue.rename_vector, Array.toList_map,
            rewriteFirstList_rename node elements.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | tuple elements =>
          simp only [Option.map_none, RuntimeValue.rename_tuple, Array.toList_map,
            rewriteFirstList_rename node elements.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | nominal source variant fields =>
          simp only [Option.map_none, RuntimeValue.rename_nominal, Array.toList_map,
            rewriteFirstList_rename node fields.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | closure function mask typeInstantiation captures =>
          simp only [Option.map_none, RuntimeValue.rename_closure, Array.toList_map,
            rewriteFirstList_rename node captures.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | borrow loan current =>
          simp only [Option.map_none, RuntimeValue.rename_borrow,
            rewriteFirst_rename node current, Option.map_map]
          congr 1; funext x; simp
      | loanHole loan => simp
      | ghost contents => simp
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

theorem rewriteFirstList_rename {f g : RuntimeValue → Option RuntimeValue}
    (node : ∀ value, f (value.rename ρ) = (g value).map (·.rename ρ))
    (values : List RuntimeValue) :
    rewriteFirstList f (values.map (·.rename ρ)) =
      (rewriteFirstList g values).map (·.map (·.rename ρ)) := by
  cases values with
  | nil => simp [rewriteFirstList]
  | cons value values =>
      rw [List.map_cons, rewriteFirstList, rewriteFirstList, rewriteFirst_rename node value]
      cases rewriteFirst g value with
      | some rewritten => simp
      | none =>
          simp only [Option.map_none, rewriteFirstList_rename node values, Option.map_map]
          congr 1
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem findFirst_rename {α β : Type} {f : RuntimeValue → Option β} {g : RuntimeValue → Option α}
    {φ : α → β}
    (node : ∀ value, f (value.rename ρ) = (g value).map φ)
    (value : RuntimeValue) :
    findFirst f (value.rename ρ) = (findFirst g value).map φ := by
  unfold findFirst
  rw [node value]
  cases g value with
  | some found => rfl
  | none =>
      cases value with
      | vector elements =>
          simp only [Option.map_none, RuntimeValue.rename_vector, Array.toList_map,
            findFirstList_rename node elements.toList]
      | tuple elements =>
          simp only [Option.map_none, RuntimeValue.rename_tuple, Array.toList_map,
            findFirstList_rename node elements.toList]
      | nominal source variant fields =>
          simp only [Option.map_none, RuntimeValue.rename_nominal, Array.toList_map,
            findFirstList_rename node fields.toList]
      | closure function mask typeInstantiation captures =>
          simp only [Option.map_none, RuntimeValue.rename_closure, Array.toList_map,
            findFirstList_rename node captures.toList]
      | borrow loan current =>
          simp only [Option.map_none, RuntimeValue.rename_borrow, findFirst_rename node current]
      | loanHole loan => simp
      | ghost contents => simp
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

theorem findFirstList_rename {α β : Type} {f : RuntimeValue → Option β}
    {g : RuntimeValue → Option α} {φ : α → β}
    (node : ∀ value, f (value.rename ρ) = (g value).map φ)
    (values : List RuntimeValue) :
    findFirstList f (values.map (·.rename ρ)) = (findFirstList g values).map φ := by
  cases values with
  | nil => simp [findFirstList]
  | cons value values =>
      rw [List.map_cons, findFirstList, findFirstList, findFirst_rename node value]
      cases findFirst g value with
      | some found => simp
      | none => simp only [Option.map_none, findFirstList_rename node values]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem collectPruned_rename {α β : Type} {f : RuntimeValue → Option β}
    {g : RuntimeValue → Option α} {φ : α → β}
    (node : ∀ value, f (value.rename ρ) = (g value).map φ)
    (value : RuntimeValue) :
    collectPruned f (value.rename ρ) = (collectPruned g value).map φ := by
  unfold collectPruned
  rw [node value]
  cases g value with
  | some found => simp
  | none =>
      cases value with
      | vector elements =>
          simp only [Option.map_none, RuntimeValue.rename_vector, Array.toList_map,
            collectPrunedList_rename node elements.toList]
      | tuple elements =>
          simp only [Option.map_none, RuntimeValue.rename_tuple, Array.toList_map,
            collectPrunedList_rename node elements.toList]
      | nominal source variant fields =>
          simp only [Option.map_none, RuntimeValue.rename_nominal, Array.toList_map,
            collectPrunedList_rename node fields.toList]
      | closure function mask typeInstantiation captures =>
          simp only [Option.map_none, RuntimeValue.rename_closure, Array.toList_map,
            collectPrunedList_rename node captures.toList]
      | borrow loan current =>
          simp only [Option.map_none, RuntimeValue.rename_borrow, collectPruned_rename node current]
      | loanHole loan => simp
      | ghost contents => simp
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); try omega)

theorem collectPrunedList_rename {α β : Type} {f : RuntimeValue → Option β}
    {g : RuntimeValue → Option α} {φ : α → β}
    (node : ∀ value, f (value.rename ρ) = (g value).map φ)
    (values : List RuntimeValue) :
    collectPrunedList f (values.map (·.rename ρ)) = (collectPrunedList g values).map φ := by
  cases values with
  | nil => simp [collectPrunedList]
  | cons value values =>
      rw [List.map_cons, collectPrunedList, collectPrunedList, collectPruned_rename node value,
        collectPrunedList_rename node values, Array.map_append]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

/-! Node matchers commute with a renaming of their loan and replacement. -/

theorem holeFill?_rename (injective : Function.Injective ρ) (loan : Nat)
    (replacement value : RuntimeValue) :
    holeFill? (ρ loan) (replacement.rename ρ) (value.rename ρ) =
      (holeFill? loan replacement value).map (·.rename ρ) := by
  cases value <;> simp [injective.eq_iff]

theorem holeMark?_rename (injective : Function.Injective ρ) (loan : Nat) (value : RuntimeValue) :
    holeMark? (ρ loan) (value.rename ρ) = (holeMark? loan value).map id := by
  cases value <;> simp [injective.eq_iff]

theorem anyHole?_rename (value : RuntimeValue) :
    anyHole? (value.rename ρ) = (anyHole? value).map ρ := by
  cases value <;> simp

theorem borrowCurrent?_rename (injective : Function.Injective ρ) (loan : Nat)
    (value : RuntimeValue) :
    borrowCurrent? (ρ loan) (value.rename ρ) =
      (borrowCurrent? loan value).map (·.rename ρ) := by
  cases value <;> simp [injective.eq_iff]

theorem borrowRewrite?_rename (injective : Function.Injective ρ) (loan : Nat)
    (replacement value : RuntimeValue) :
    borrowRewrite? (ρ loan) (replacement.rename ρ) (value.rename ρ) =
      (borrowRewrite? loan replacement value).map (·.rename ρ) := by
  cases value <;> simp [injective.eq_iff]

theorem borrowClear?_rename (injective : Function.Injective ρ) (loan : Nat) (value : RuntimeValue) :
    borrowClear? (ρ loan) (value.rename ρ) =
      (borrowClear? loan value).map (·.rename ρ) := by
  cases value <;> simp [injective.eq_iff]



end Walks
end SemanticOperations



/-! ## Frames, states, and outcomes -/

/-- A pending write-back or registration with its loan renamed. -/
abbrev renameEntry {α : Type} (ρ : Nat → Nat) (entry : Nat × α) : Nat × α :=
  (ρ entry.1, entry.2)

/-- Global memory with the loans its slots hold renamed. -/
def GlobalMap.rename (ρ : Nat → Nat) (globals : GlobalMap) : GlobalMap :=
  ⟨globals.entries.map fun slot => { slot with value := slot.value.rename ρ }⟩

/-- A frame with its loans renamed: its locals' values, the instance each
site's loan has, and the loans whose places it caches. -/
def RuntimeFrame.rename (ρ : Nat → Nat) (frame : RuntimeFrame) : RuntimeFrame :=
  { frame with
    locals := frame.locals.map (Option.map (·.rename ρ))
    activeLoans := frame.activeLoans.map fun entry => (entry.1, ρ entry.2)
    loanLocations := frame.loanLocations.map (renameEntry ρ) }

/-- Every loan a frame holds or tracks satisfies `inside`. -/
structure RuntimeFrame.Within (inside : Nat → Prop) (frame : RuntimeFrame) : Prop where
  locals : ∀ slot ∈ frame.locals, ∀ value, slot = some value → value.Within inside
  activeLoans : ∀ entry ∈ frame.activeLoans, inside entry.2
  loanLocations : ∀ entry ∈ frame.loanLocations, inside entry.1

theorem GlobalMap.rename_of_inverse {inside : Nat → Prop} {ρ σ : Nat → Nat}
    (inverse : ∀ loan, inside loan → ρ (σ loan) = loan) {globals : GlobalMap}
    (within : ∀ slot ∈ globals.entries, slot.value.Within inside) :
    (globals.rename σ).rename ρ = globals := by
  obtain ⟨entries⟩ := globals
  simp only [GlobalMap.rename, Array.map_map, GlobalMap.mk.injEq]
  conv => rhs; rw [← Array.map_id entries]
  apply Array.map_congr_left
  intro slot member
  simp only [Function.comp_apply, id]
  rw [RuntimeValue.rename_of_inverse inverse slot.value (within slot member)]

theorem GlobalMap.within_rename {inside : Nat → Prop} (ρ : Nat → Nat) {globals : GlobalMap}
    (within : ∀ slot ∈ globals.entries, slot.value.Within inside) :
    ∀ slot ∈ (globals.rename ρ).entries,
      slot.value.Within fun loan => ∃ base, inside base ∧ ρ base = loan := by
  intro slot member
  simp only [GlobalMap.rename, Array.mem_map] at member
  obtain ⟨base, member, rfl⟩ := member
  exact RuntimeValue.within_rename ρ base.value (within base member)

theorem RuntimeFrame.rename_of_inverse {inside : Nat → Prop} {ρ σ : Nat → Nat}
    (inverse : ∀ loan, inside loan → ρ (σ loan) = loan) {frame : RuntimeFrame}
    (within : frame.Within inside) : (frame.rename σ).rename ρ = frame := by
  obtain ⟨locals, activeLoans, loanLocations, typeInstantiation⟩ := frame
  simp only [RuntimeFrame.rename, Array.map_map, RuntimeFrame.mk.injEq, and_true]
  refine ⟨?_, ?_, ?_⟩
  · conv => rhs; rw [← Array.map_id locals]
    apply Array.map_congr_left
    intro slot member
    cases slot with
    | none => rfl
    | some value =>
        simp only [Function.comp_apply, Option.map_some, id, Option.some.injEq]
        exact RuntimeValue.rename_of_inverse inverse value (within.locals _ member value rfl)
  · conv => rhs; rw [← Array.map_id activeLoans]
    apply Array.map_congr_left
    intro entry member
    simp [inverse entry.2 (within.activeLoans entry member)]
  · conv => rhs; rw [← Array.map_id loanLocations]
    apply Array.map_congr_left
    intro entry member
    simp [renameEntry, inverse entry.1 (within.loanLocations entry member)]

theorem RuntimeFrame.within_rename {inside : Nat → Prop} (ρ : Nat → Nat) {frame : RuntimeFrame}
    (within : frame.Within inside) :
    (frame.rename ρ).Within fun loan => ∃ base, inside base ∧ ρ base = loan where
  locals slot member value value_eq := by
    simp only [RuntimeFrame.rename, Array.mem_map] at member
    obtain ⟨base, member, rfl⟩ := member
    cases base with
    | none => cases value_eq
    | some base =>
        simp only [Option.map_some, Option.some.injEq] at value_eq
        exact value_eq ▸ RuntimeValue.within_rename ρ base (within.locals _ member base rfl)
  activeLoans entry member := by
    simp only [RuntimeFrame.rename, Array.mem_map] at member
    obtain ⟨base, member, rfl⟩ := member
    exact ⟨base.2, within.activeLoans base member, rfl⟩
  loanLocations entry member := by
    simp only [RuntimeFrame.rename, Array.mem_map] at member
    obtain ⟨base, member, rfl⟩ := member
    exact ⟨base.1, within.loanLocations base member, rfl⟩

theorem RuntimeFrame.Within.mono {inside outside : Nat → Prop}
    (weaker : ∀ loan, inside loan → outside loan) {frame : RuntimeFrame}
    (within : frame.Within inside) : frame.Within outside where
  locals slot member value value_eq :=
    RuntimeValue.Within.mono weaker (within.locals slot member value value_eq)
  activeLoans entry member := weaker _ (within.activeLoans entry member)
  loanLocations entry member := weaker _ (within.loanLocations entry member)

/-- A control with the loans of its values renamed. -/
def Control.rename (ρ : Nat → Nat) : Control → Control
  | .value result => .value (result.rename ρ)
  | .break_ nest result => .break_ nest (result.map (·.rename ρ))
  | .continue_ nest => .continue_ nest
  | .return_ values => .return_ (values.map (·.rename ρ))
  | .throw_ kind arguments => .throw_ kind (arguments.map (·.rename ρ))

/-- Every loan the values of a control hold satisfies `inside`. -/
def Control.Within (inside : Nat → Prop) : Control → Prop
  | .value result => result.Within inside
  | .break_ _ result => ∀ value ∈ result, value.Within inside
  | .continue_ _ => True
  | .return_ values | .throw_ _ values => ∀ value ∈ values, value.Within inside

/-- An outcome with the loans of its values renamed. -/
def Outcome.rename (ρ : Nat → Nat) : Outcome → Outcome
  | .returned values => .returned (values.map (·.rename ρ))
  | .threw kind arguments => .threw kind (arguments.map (·.rename ρ))

/-- Every loan the values of an outcome hold satisfies `inside`. -/
def Outcome.Within (inside : Nat → Prop) : Outcome → Prop
  | .returned values | .threw _ values => ∀ value ∈ values, value.Within inside

/-- An enumeration of the loans a predicate admits: an injective renaming
into them, with a left inverse on them. A value holding only such loans is
the enumeration's renaming of one, so what commutes with renamings keeps
values among them. -/
structure LoanBase (β γ : Nat → Nat) (inside : Nat → Prop) : Prop where
  injective : Function.Injective β
  inverse : ∀ loan, inside loan → β (γ loan) = loan
  image : ∀ loan, inside (β loan)

/-- The loans at or beyond a frontier, enumerated by raising. -/
theorem LoanBase.above (frontier : Nat) :
    LoanBase (· + frontier) (· - frontier) (frontier ≤ ·) where
  injective _ _ same := by simpa using same
  inverse loan above := by omega
  image loan := by omega

namespace LoanBase

variable {β γ : Nat → Nat} {inside : Nat → Prop}

/-- A value holding only loans an enumeration admits is the enumeration's
renaming of one. -/
theorem rename_inverse (base : LoanBase β γ inside) (value : RuntimeValue)
    (within : value.Within inside) : (value.rename γ).rename β = value :=
  RuntimeValue.rename_of_inverse base.inverse value within

/-- An enumeration's renaming holds only loans it admits. -/
theorem within (base : LoanBase β γ inside) (value : RuntimeValue) :
    (value.rename β).Within inside :=
  (RuntimeValue.within_rename β value (RuntimeValue.within_true value)).mono
    fun _ ⟨original, _, renamed⟩ => renamed ▸ base.image original

theorem rename_inverse_frame (base : LoanBase β γ inside) {frame : RuntimeFrame}
    (within : frame.Within inside) : (frame.rename γ).rename β = frame :=
  RuntimeFrame.rename_of_inverse base.inverse within

theorem within_frame (base : LoanBase β γ inside) (frame : RuntimeFrame) :
    (frame.rename β).Within inside :=
  (RuntimeFrame.within_rename β (inside := fun _ => True)
    { locals := fun _ _ value _ => RuntimeValue.within_true value
      activeLoans := fun _ _ => trivial
      loanLocations := fun _ _ => trivial }).mono
    fun _ ⟨original, _, renamed⟩ => renamed ▸ base.image original

theorem rename_inverse_globals (base : LoanBase β γ inside) {globals : GlobalMap}
    (within : ∀ slot ∈ globals.entries, slot.value.Within inside) :
    (globals.rename γ).rename β = globals :=
  GlobalMap.rename_of_inverse base.inverse within

theorem within_globals (base : LoanBase β γ inside) (globals : GlobalMap) :
    ∀ slot ∈ (globals.rename β).entries, slot.value.Within inside := by
  intro slot member
  simp only [GlobalMap.rename, Array.mem_map] at member
  obtain ⟨original, -, rfl⟩ := member
  exact base.within original.value

end LoanBase

/-- A renaming of the loans of a first run to those of a second: injective,
and the shift by `offset` from `frontier` on, where both mint, with the
loans minted from the frontier on among those the runs hold (`inside`),
which an enumeration admits. -/
structure LoanRenaming (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop) : Prop where
  injective : Function.Injective ρ
  minted : ∀ loan, frontier ≤ loan → ρ loan = loan + offset
  covers : ∀ loan, frontier ≤ loan → inside loan
  base : ∃ β γ, LoanBase β γ inside

/-- A loan registry of a second run mirroring one of a first: the first's
registrations of the loans it holds (`inside`), renamed, before entries of
loans neither run holds, which no loan of the run reaches. -/
def RegistryRenamed (ρ : Nat → Nat) (inside : Nat → Prop)
    (registry registry' : List (Nat × GlobalKey)) : Prop :=
  ∃ held junk junk', registry = held ++ junk ∧
    registry' = held.map (renameEntry ρ) ++ junk' ∧
    (∀ entry ∈ held, inside entry.1) ∧ (∀ entry ∈ junk, ¬inside entry.1) ∧
    ∀ entry ∈ junk', ∀ loan, inside loan → ρ loan ≠ entry.1

/-- A state of a second run mirroring one of a first: its global memory and
the write-backs pending past each run's first `inert`, with their loans
renamed; the same table handles; its frontier raised by `offset`; and its loan
registry the first's registrations of the loans minted at or beyond
registrations of the loans it holds, renamed, before entries of loans neither
run holds. Every loan the first run's state holds outside its inert
write-backs is `inside`. -/
structure StateRenamed (ρ : Nat → Nat) (offset frontier : Nat) (inside : Nat → Prop)
    (inert inert' : Nat) (state state' : RuntimeState) : Prop where
  renames : LoanRenaming ρ offset frontier inside
  globals : state'.globals = state.globals.rename ρ
  tableHandles : state'.tableHandles = state.tableHandles
  nextLoan : state'.nextLoan = state.nextLoan + offset
  frontier_le : frontier ≤ state.nextLoan
  inert_le : inert ≤ state.pending.size
  inert_le' : inert' ≤ state'.pending.size
  pending : state'.pending.extract inert' state'.pending.size =
    (state.pending.extract inert state.pending.size).map fun entry =>
      (ρ entry.1, entry.2.rename ρ)
  registry : RegistryRenamed ρ inside state.globalLoans state'.globalLoans
  globalsWithin : ∀ slot ∈ state.globals.entries, slot.value.Within inside
  pendingWithin : ∀ entry ∈ state.pending.extract inert state.pending.size,
    inside entry.1 ∧ entry.2.Within inside

section States

variable {ρ : Nat → Nat} {offset frontier : Nat} {inside : Nat → Prop} {inert inert' : Nat}
  {state state' : RuntimeState}

/-- The first run's next loan is renamed to the second's. -/
theorem StateRenamed.next (shifted : StateRenamed ρ offset frontier inside inert inert' state state') :
    ρ state.nextLoan = state'.nextLoan := by
  rw [shifted.renames.minted _ shifted.frontier_le, shifted.nextLoan]

/-- The first run's next loan is one it may hold. -/
theorem StateRenamed.next_inside
    (shifted : StateRenamed ρ offset frontier inside inert inert' state state') :
    inside state.nextLoan :=
  shifted.renames.covers _ shifted.frontier_le

end States

end VerexIR
