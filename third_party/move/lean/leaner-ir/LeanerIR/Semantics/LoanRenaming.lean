-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Focus

/-!
# Renaming loan identities

Two runs from starts that differ only in their loan bookkeeping mint the
same loans in the same order, offset by their frontiers' difference
(`designs/static-typing.md`, "Loan independence"). `shift` raises the loan
identities of values, frames, and global memory by such an offset; `Above`
states that every identity a value holds was minted at or beyond a
frontier.
-/

namespace LeanerIR

mutual
/-- A value with each loan identity it holds raised by `offset`. -/
def RuntimeValue.shift (offset : Nat) : RuntimeValue → RuntimeValue
  | .vector elements => .vector (RuntimeValue.shiftList offset elements.toList).toArray
  | .tuple elements => .tuple (RuntimeValue.shiftList offset elements.toList).toArray
  | .nominal source variant fields =>
      .nominal source variant (RuntimeValue.shiftList offset fields.toList).toArray
  | .closure function mask typeInstantiation captures =>
      .closure function mask typeInstantiation
        (RuntimeValue.shiftList offset captures.toList).toArray
  | .borrow loan current => .borrow (loan + offset) (current.shift offset)
  | .loanHole loan => .loanHole (loan + offset)
  | .unit => .unit
  | .bool value => .bool value
  | .character value => .character value
  | .integer value => .integer value
  | .address value => .address value
  | .signer value => .signer value
  | .string value => .string value
  | .bytes value => .bytes value
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

/-- Each value of a list with its loan identities raised. -/
def RuntimeValue.shiftList (offset : Nat) : List RuntimeValue → List RuntimeValue
  | [] => []
  | value :: values => value.shift offset :: RuntimeValue.shiftList offset values
termination_by values => sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
/-- Every loan identity a value holds is at or beyond `frontier`. -/
def RuntimeValue.Above (frontier : Nat) : RuntimeValue → Prop
  | .vector elements | .tuple elements | .nominal _ _ elements | .closure _ _ _ elements =>
      RuntimeValue.AboveList frontier elements.toList
  | .borrow loan current => frontier ≤ loan ∧ current.Above frontier
  | .loanHole loan => frontier ≤ loan
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => True
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

/-- Every loan identity a list's values hold is at or beyond `frontier`. -/
def RuntimeValue.AboveList (frontier : Nat) : List RuntimeValue → Prop
  | [] => True
  | value :: values => value.Above frontier ∧ RuntimeValue.AboveList frontier values
termination_by values => sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem RuntimeValue.shiftList_eq_map (offset : Nat) :
    ∀ values : List RuntimeValue,
      RuntimeValue.shiftList offset values = values.map (·.shift offset)
  | [] => by simp [RuntimeValue.shiftList]
  | _ :: values => by
      simp [RuntimeValue.shiftList, RuntimeValue.shiftList_eq_map offset values]

theorem RuntimeValue.aboveList_iff (frontier : Nat) :
    ∀ values : List RuntimeValue,
      RuntimeValue.AboveList frontier values ↔ ∀ value ∈ values, value.Above frontier
  | [] => by simp [RuntimeValue.AboveList]
  | _ :: values => by
      simp [RuntimeValue.AboveList, RuntimeValue.aboveList_iff frontier values]

section Equations

variable (offset frontier : Nat)

@[simp] theorem RuntimeValue.shift_vector (elements : Array RuntimeValue) :
    (RuntimeValue.vector elements).shift offset = .vector (elements.map (·.shift offset)) := by
  rw [RuntimeValue.shift, RuntimeValue.shiftList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.shift_tuple (elements : Array RuntimeValue) :
    (RuntimeValue.tuple elements).shift offset = .tuple (elements.map (·.shift offset)) := by
  rw [RuntimeValue.shift, RuntimeValue.shiftList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.shift_nominal (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    (RuntimeValue.nominal source variant fields).shift offset =
      .nominal source variant (fields.map (·.shift offset)) := by
  rw [RuntimeValue.shift, RuntimeValue.shiftList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.shift_closure (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) :
    (RuntimeValue.closure function mask typeInstantiation captures).shift offset =
      .closure function mask typeInstantiation (captures.map (·.shift offset)) := by
  rw [RuntimeValue.shift, RuntimeValue.shiftList_eq_map]; congr 1; apply Array.ext'; simp

@[simp] theorem RuntimeValue.shift_borrow (loan : Nat) (current : RuntimeValue) :
    (RuntimeValue.borrow loan current).shift offset = .borrow (loan + offset) (current.shift offset) := by
  rw [RuntimeValue.shift]

@[simp] theorem RuntimeValue.shift_loanHole (loan : Nat) :
    (RuntimeValue.loanHole loan).shift offset = .loanHole (loan + offset) := by
  rw [RuntimeValue.shift]

@[simp] theorem RuntimeValue.shift_unit : RuntimeValue.unit.shift offset = .unit := by
  rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_bool (value : Bool) :
    (RuntimeValue.bool value).shift offset = .bool value := by rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_character (value : Nat) :
    (RuntimeValue.character value).shift offset = .character value := by rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_integer (value : Int) :
    (RuntimeValue.integer value).shift offset = .integer value := by rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_address (value : String) :
    (RuntimeValue.address value).shift offset = .address value := by rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_signer (value : String) :
    (RuntimeValue.signer value).shift offset = .signer value := by rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_string (value : String) :
    (RuntimeValue.string value).shift offset = .string value := by rw [RuntimeValue.shift]
@[simp] theorem RuntimeValue.shift_bytes (value : Array UInt8) :
    (RuntimeValue.bytes value).shift offset = .bytes value := by rw [RuntimeValue.shift]

@[simp] theorem RuntimeValue.above_vector (elements : Array RuntimeValue) :
    (RuntimeValue.vector elements).Above frontier ↔ ∀ element ∈ elements, element.Above frontier := by
  rw [RuntimeValue.Above, RuntimeValue.aboveList_iff]; simp
@[simp] theorem RuntimeValue.above_tuple (elements : Array RuntimeValue) :
    (RuntimeValue.tuple elements).Above frontier ↔ ∀ element ∈ elements, element.Above frontier := by
  rw [RuntimeValue.Above, RuntimeValue.aboveList_iff]; simp
@[simp] theorem RuntimeValue.above_nominal (source : StructHandle) (variant : Option String)
    (fields : Array RuntimeValue) :
    (RuntimeValue.nominal source variant fields).Above frontier ↔
      ∀ field ∈ fields, field.Above frontier := by
  rw [RuntimeValue.Above, RuntimeValue.aboveList_iff]; simp
@[simp] theorem RuntimeValue.above_closure (function : FunctionHandle) (mask : Nat)
    (typeInstantiation : Array (TypeId × TypeId)) (captures : Array RuntimeValue) :
    (RuntimeValue.closure function mask typeInstantiation captures).Above frontier ↔
      ∀ capture ∈ captures, capture.Above frontier := by
  rw [RuntimeValue.Above, RuntimeValue.aboveList_iff]; simp
@[simp] theorem RuntimeValue.above_borrow (loan : Nat) (current : RuntimeValue) :
    (RuntimeValue.borrow loan current).Above frontier ↔ frontier ≤ loan ∧ current.Above frontier := by
  rw [RuntimeValue.Above]
@[simp] theorem RuntimeValue.above_loanHole (loan : Nat) :
    (RuntimeValue.loanHole loan).Above frontier ↔ frontier ≤ loan := by
  rw [RuntimeValue.Above]

end Equations

/-! ## Loan-free values -/

theorem RuntimeValue.shift_of_plain (offset : Nat) :
    ∀ {value : RuntimeValue}, SemanticOperations.Plain value → value.shift offset = value
  | _, .unit | _, .bool _ | _, .character _ | _, .integer _ | _, .address _ | _, .signer _
  | _, .string _ | _, .bytes _ => by simp
  | _, .vector elements plain => by
      simp only [RuntimeValue.shift_vector, RuntimeValue.vector.injEq]
      conv => rhs; rw [← Array.map_id elements]
      exact Array.map_congr_left fun element member =>
        RuntimeValue.shift_of_plain offset (plain element member)
  | _, .tuple elements plain => by
      simp only [RuntimeValue.shift_tuple, RuntimeValue.tuple.injEq]
      conv => rhs; rw [← Array.map_id elements]
      exact Array.map_congr_left fun element member =>
        RuntimeValue.shift_of_plain offset (plain element member)
  | _, .nominal source variant fields plain => by
      simp only [RuntimeValue.shift_nominal, RuntimeValue.nominal.injEq, true_and]
      conv => rhs; rw [← Array.map_id fields]
      exact Array.map_congr_left fun field member =>
        RuntimeValue.shift_of_plain offset (plain field member)
  | _, .closure function mask typeInstantiation captures plain => by
      simp only [RuntimeValue.shift_closure, RuntimeValue.closure.injEq, true_and]
      conv => rhs; rw [← Array.map_id captures]
      exact Array.map_congr_left fun capture member =>
        RuntimeValue.shift_of_plain offset (plain capture member)

theorem RuntimeValue.above_of_plain (frontier : Nat) :
    ∀ {value : RuntimeValue}, SemanticOperations.Plain value → value.Above frontier
  | _, .unit | _, .bool _ | _, .character _ | _, .integer _ | _, .address _ | _, .signer _
  | _, .string _ | _, .bytes _ => by simp [RuntimeValue.Above]
  | _, .vector _ plain | _, .tuple _ plain => by
      simpa using fun element member => RuntimeValue.above_of_plain frontier (plain element member)
  | _, .nominal _ _ _ plain => by
      simpa using fun element member => RuntimeValue.above_of_plain frontier (plain element member)
  | _, .closure _ _ _ _ plain => by
      simpa using fun element member => RuntimeValue.above_of_plain frontier (plain element member)

/-! ## Lowering

A value whose loans are at or beyond a frontier is the shift of one by the
frontier, so whatever commutes with shifts keeps values above it. -/

mutual
/-- A value with each loan identity it holds lowered by `offset`. -/
def RuntimeValue.unshift (offset : Nat) : RuntimeValue → RuntimeValue
  | .vector elements => .vector (RuntimeValue.unshiftList offset elements.toList).toArray
  | .tuple elements => .tuple (RuntimeValue.unshiftList offset elements.toList).toArray
  | .nominal source variant fields =>
      .nominal source variant (RuntimeValue.unshiftList offset fields.toList).toArray
  | .closure function mask typeInstantiation captures =>
      .closure function mask typeInstantiation
        (RuntimeValue.unshiftList offset captures.toList).toArray
  | .borrow loan current => .borrow (loan - offset) (current.unshift offset)
  | .loanHole loan => .loanHole (loan - offset)
  | .unit => .unit
  | .bool value => .bool value
  | .character value => .character value
  | .integer value => .integer value
  | .address value => .address value
  | .signer value => .signer value
  | .string value => .string value
  | .bytes value => .bytes value
termination_by value => sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

def RuntimeValue.unshiftList (offset : Nat) : List RuntimeValue → List RuntimeValue
  | [] => []
  | value :: values => value.unshift offset :: RuntimeValue.unshiftList offset values
termination_by values => sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem RuntimeValue.shift_unshift (offset : Nat) (value : RuntimeValue)
    (above : value.Above offset) : (value.unshift offset).shift offset = value := by
  match value, above with
  | .vector elements, above =>
      rw [RuntimeValue.unshift, RuntimeValue.shift_vector]
      simp only [RuntimeValue.above_vector] at above
      congr 1
      apply Array.ext'
      simp only [List.map_toArray]
      exact RuntimeValue.shiftList_unshiftList offset elements.toList
        (fun element member => above element (by simpa using member))
  | .tuple elements, above =>
      rw [RuntimeValue.unshift, RuntimeValue.shift_tuple]
      simp only [RuntimeValue.above_tuple] at above
      congr 1
      apply Array.ext'
      simp only [List.map_toArray]
      exact RuntimeValue.shiftList_unshiftList offset elements.toList
        (fun element member => above element (by simpa using member))
  | .nominal source variant fields, above =>
      rw [RuntimeValue.unshift, RuntimeValue.shift_nominal]
      simp only [RuntimeValue.above_nominal] at above
      congr 1
      apply Array.ext'
      simp only [List.map_toArray]
      exact RuntimeValue.shiftList_unshiftList offset fields.toList
        (fun element member => above element (by simpa using member))
  | .closure function mask typeInstantiation captures, above =>
      rw [RuntimeValue.unshift, RuntimeValue.shift_closure]
      simp only [RuntimeValue.above_closure] at above
      congr 1
      apply Array.ext'
      simp only [List.map_toArray]
      exact RuntimeValue.shiftList_unshiftList offset captures.toList
        (fun element member => above element (by simpa using member))
  | .borrow loan current, above =>
      rw [RuntimeValue.unshift, RuntimeValue.shift_borrow]
      simp only [RuntimeValue.above_borrow] at above
      rw [RuntimeValue.shift_unshift offset current above.2, Nat.sub_add_cancel above.1]
  | .loanHole loan, above =>
      rw [RuntimeValue.unshift, RuntimeValue.shift_loanHole]
      simp only [RuntimeValue.above_loanHole] at above
      rw [Nat.sub_add_cancel above]
  | .unit, _ | .bool _, _ | .character _, _ | .integer _, _ | .address _, _ | .signer _, _
  | .string _, _ | .bytes _, _ => rw [RuntimeValue.unshift]; simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.shiftList_unshiftList (offset : Nat) (values : List RuntimeValue)
    (above : ∀ value ∈ values, value.Above offset) :
    (RuntimeValue.unshiftList offset values).map (·.shift offset) = values := by
  match values, above with
  | [], _ => simp [RuntimeValue.unshiftList]
  | head :: tail, above =>
      rw [RuntimeValue.unshiftList, List.map_cons,
        RuntimeValue.shift_unshift offset head (above head (by simp)),
        RuntimeValue.shiftList_unshiftList offset tail
          (fun value member => above value (by simp [member]))]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem RuntimeValue.above_shift {frontier : Nat} (offset : Nat) (value : RuntimeValue)
    (above : value.Above frontier) : (value.shift offset).Above (frontier + offset) := by
  match value, above with
  | .vector elements, above =>
      simp only [RuntimeValue.above_vector, RuntimeValue.shift_vector, Array.mem_map] at above ⊢
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.above_shiftList offset elements.toList
        (fun element member => above element (by simpa using member)) _
        (List.mem_map.mpr ⟨element, by simpa using member, rfl⟩)
  | .tuple elements, above =>
      simp only [RuntimeValue.above_tuple, RuntimeValue.shift_tuple, Array.mem_map] at above ⊢
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.above_shiftList offset elements.toList
        (fun element member => above element (by simpa using member)) _
        (List.mem_map.mpr ⟨element, by simpa using member, rfl⟩)
  | .nominal source variant fields, above =>
      simp only [RuntimeValue.above_nominal, RuntimeValue.shift_nominal, Array.mem_map] at above ⊢
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.above_shiftList offset fields.toList
        (fun element member => above element (by simpa using member)) _
        (List.mem_map.mpr ⟨element, by simpa using member, rfl⟩)
  | .closure function mask typeInstantiation captures, above =>
      simp only [RuntimeValue.above_closure, RuntimeValue.shift_closure, Array.mem_map] at above ⊢
      rintro _ ⟨element, member, rfl⟩
      exact RuntimeValue.above_shiftList offset captures.toList
        (fun element member => above element (by simpa using member)) _
        (List.mem_map.mpr ⟨element, by simpa using member, rfl⟩)
  | .borrow loan current, above =>
      simp only [RuntimeValue.above_borrow, RuntimeValue.shift_borrow] at above ⊢
      exact ⟨by omega, RuntimeValue.above_shift offset current above.2⟩
  | .loanHole loan, above =>
      simp only [RuntimeValue.above_loanHole, RuntimeValue.shift_loanHole] at above ⊢
      omega
  | .unit, _ | .bool _, _ | .character _, _ | .integer _, _ | .address _, _ | .signer _, _
  | .string _, _ | .bytes _, _ => simp [RuntimeValue.Above]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.above_shiftList {frontier : Nat} (offset : Nat) (values : List RuntimeValue)
    (above : ∀ value ∈ values, value.Above frontier) :
    ∀ value ∈ values.map (·.shift offset), value.Above (frontier + offset) := by
  match values, above with
  | [], _ => simp
  | head :: tail, above =>
      intro value member
      rcases List.mem_cons.mp member with same | member
      · exact same ▸ RuntimeValue.above_shift offset head (above head (by simp))
      · exact RuntimeValue.above_shiftList offset tail
          (fun value member => above value (by simp [member])) value member
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem RuntimeValue.above_zero (value : RuntimeValue) : value.Above 0 := by
  match value with
  | .vector elements =>
      simp only [RuntimeValue.above_vector]
      exact fun element member => RuntimeValue.aboveList_zero elements.toList element
        (by simpa using member)
  | .tuple elements =>
      simp only [RuntimeValue.above_tuple]
      exact fun element member => RuntimeValue.aboveList_zero elements.toList element
        (by simpa using member)
  | .nominal _ _ fields =>
      simp only [RuntimeValue.above_nominal]
      exact fun element member => RuntimeValue.aboveList_zero fields.toList element
        (by simpa using member)
  | .closure _ _ _ captures =>
      simp only [RuntimeValue.above_closure]
      exact fun element member => RuntimeValue.aboveList_zero captures.toList element
        (by simpa using member)
  | .borrow _ current =>
      simp only [RuntimeValue.above_borrow]
      exact ⟨Nat.zero_le _, RuntimeValue.above_zero current⟩
  | .loanHole _ => simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => simp [RuntimeValue.Above]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.aboveList_zero (values : List RuntimeValue) :
    ∀ value ∈ values, value.Above 0 := by
  match values with
  | [] => simp
  | head :: tail =>
      intro value member
      rcases List.mem_cons.mp member with same | member
      · exact same ▸ RuntimeValue.above_zero head
      · exact RuntimeValue.aboveList_zero tail value member
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem RuntimeValue.unshift_shift (offset : Nat) (value : RuntimeValue) :
    (value.shift offset).unshift offset = value := by
  match value with
  | .vector elements =>
      rw [RuntimeValue.shift, RuntimeValue.unshift, RuntimeValue.vector.injEq]
      apply Array.ext'
      exact RuntimeValue.unshiftList_shiftList offset elements.toList
  | .tuple elements =>
      rw [RuntimeValue.shift, RuntimeValue.unshift, RuntimeValue.tuple.injEq]
      apply Array.ext'
      exact RuntimeValue.unshiftList_shiftList offset elements.toList
  | .nominal source variant fields =>
      rw [RuntimeValue.shift, RuntimeValue.unshift, RuntimeValue.nominal.injEq]
      refine ⟨rfl, rfl, ?_⟩
      apply Array.ext'
      exact RuntimeValue.unshiftList_shiftList offset fields.toList
  | .closure function mask typeInstantiation captures =>
      rw [RuntimeValue.shift, RuntimeValue.unshift, RuntimeValue.closure.injEq]
      refine ⟨rfl, rfl, rfl, ?_⟩
      apply Array.ext'
      exact RuntimeValue.unshiftList_shiftList offset captures.toList
  | .borrow loan current =>
      rw [RuntimeValue.shift, RuntimeValue.unshift, RuntimeValue.unshift_shift offset current]
      simp
  | .loanHole loan => rw [RuntimeValue.shift, RuntimeValue.unshift]; simp
  | .unit | .bool _ | .character _ | .integer _ | .address _ | .signer _ | .string _
  | .bytes _ => rw [RuntimeValue.shift, RuntimeValue.unshift]
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem RuntimeValue.unshiftList_shiftList (offset : Nat) (values : List RuntimeValue) :
    RuntimeValue.unshiftList offset (RuntimeValue.shiftList offset values) = values := by
  match values with
  | [] => rw [RuntimeValue.shiftList, RuntimeValue.unshiftList]
  | head :: tail =>
      rw [RuntimeValue.shiftList, RuntimeValue.unshiftList, RuntimeValue.unshift_shift offset head,
        RuntimeValue.unshiftList_shiftList offset tail]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

theorem RuntimeValue.shift_injective (offset : Nat) {left right : RuntimeValue}
    (same : left.shift offset = right.shift offset) : left = right := by
  rw [← RuntimeValue.unshift_shift offset left, same, RuntimeValue.unshift_shift]

theorem RuntimeValue.shift_beq (offset : Nat) (left right : RuntimeValue) :
    (left.shift offset == right.shift offset) = (left == right) := by
  rw [Bool.eq_iff_iff, beq_iff_eq, beq_iff_eq]
  exact ⟨RuntimeValue.shift_injective offset, fun same => same ▸ rfl⟩

/-- A shift by a frontier is above it. -/
theorem RuntimeValue.above_shift_self (frontier : Nat) (value : RuntimeValue) :
    (value.shift frontier).Above frontier := by
  simpa using RuntimeValue.above_shift frontier value (RuntimeValue.above_zero value)

/-! ## Walks

A walk over a value with a node matcher that commutes with a shift commutes
with it as a whole. -/

namespace SemanticOperations

section Walks

variable {offset : Nat}

mutual
theorem rewriteFirst_shift {f g : RuntimeValue → Option RuntimeValue}
    (node : ∀ value, f (value.shift offset) = (g value).map (·.shift offset))
    (value : RuntimeValue) :
    rewriteFirst f (value.shift offset) = (rewriteFirst g value).map (·.shift offset) := by
  unfold rewriteFirst
  rw [node value]
  cases g value with
  | some rewritten => rfl
  | none =>
      cases value with
      | vector elements =>
          simp only [Option.map_none, RuntimeValue.shift_vector, Array.toList_map,
            rewriteFirstList_shift node elements.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | tuple elements =>
          simp only [Option.map_none, RuntimeValue.shift_tuple, Array.toList_map,
            rewriteFirstList_shift node elements.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | nominal source variant fields =>
          simp only [Option.map_none, RuntimeValue.shift_nominal, Array.toList_map,
            rewriteFirstList_shift node fields.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | closure function mask typeInstantiation captures =>
          simp only [Option.map_none, RuntimeValue.shift_closure, Array.toList_map,
            rewriteFirstList_shift node captures.toList, Option.map_map]
          congr 1; funext rewritten; simp
      | borrow loan current =>
          simp only [Option.map_none, RuntimeValue.shift_borrow,
            rewriteFirst_shift node current, Option.map_map]
          congr 1; funext x; simp
      | loanHole loan => simp
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem rewriteFirstList_shift {f g : RuntimeValue → Option RuntimeValue}
    (node : ∀ value, f (value.shift offset) = (g value).map (·.shift offset))
    (values : List RuntimeValue) :
    rewriteFirstList f (values.map (·.shift offset)) =
      (rewriteFirstList g values).map (·.map (·.shift offset)) := by
  cases values with
  | nil => simp [rewriteFirstList]
  | cons value values =>
      rw [List.map_cons, rewriteFirstList, rewriteFirstList, rewriteFirst_shift node value]
      cases rewriteFirst g value with
      | some rewritten => simp
      | none =>
          simp only [Option.map_none, rewriteFirstList_shift node values, Option.map_map]
          congr 1
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem findFirst_shift {α β : Type} {f : RuntimeValue → Option β} {g : RuntimeValue → Option α}
    {φ : α → β}
    (node : ∀ value, f (value.shift offset) = (g value).map φ)
    (value : RuntimeValue) :
    findFirst f (value.shift offset) = (findFirst g value).map φ := by
  unfold findFirst
  rw [node value]
  cases g value with
  | some found => rfl
  | none =>
      cases value with
      | vector elements =>
          simp only [Option.map_none, RuntimeValue.shift_vector, Array.toList_map,
            findFirstList_shift node elements.toList]
      | tuple elements =>
          simp only [Option.map_none, RuntimeValue.shift_tuple, Array.toList_map,
            findFirstList_shift node elements.toList]
      | nominal source variant fields =>
          simp only [Option.map_none, RuntimeValue.shift_nominal, Array.toList_map,
            findFirstList_shift node fields.toList]
      | closure function mask typeInstantiation captures =>
          simp only [Option.map_none, RuntimeValue.shift_closure, Array.toList_map,
            findFirstList_shift node captures.toList]
      | borrow loan current =>
          simp only [Option.map_none, RuntimeValue.shift_borrow, findFirst_shift node current]
      | loanHole loan => simp
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem findFirstList_shift {α β : Type} {f : RuntimeValue → Option β}
    {g : RuntimeValue → Option α} {φ : α → β}
    (node : ∀ value, f (value.shift offset) = (g value).map φ)
    (values : List RuntimeValue) :
    findFirstList f (values.map (·.shift offset)) = (findFirstList g values).map φ := by
  cases values with
  | nil => simp [findFirstList]
  | cons value values =>
      rw [List.map_cons, findFirstList, findFirstList, findFirst_shift node value]
      cases findFirst g value with
      | some found => simp
      | none => simp only [Option.map_none, findFirstList_shift node values]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

mutual
theorem collectPruned_shift {α β : Type} {f : RuntimeValue → Option β}
    {g : RuntimeValue → Option α} {φ : α → β}
    (node : ∀ value, f (value.shift offset) = (g value).map φ)
    (value : RuntimeValue) :
    collectPruned f (value.shift offset) = (collectPruned g value).map φ := by
  unfold collectPruned
  rw [node value]
  cases g value with
  | some found => simp
  | none =>
      cases value with
      | vector elements =>
          simp only [Option.map_none, RuntimeValue.shift_vector, Array.toList_map,
            collectPrunedList_shift node elements.toList]
      | tuple elements =>
          simp only [Option.map_none, RuntimeValue.shift_tuple, Array.toList_map,
            collectPrunedList_shift node elements.toList]
      | nominal source variant fields =>
          simp only [Option.map_none, RuntimeValue.shift_nominal, Array.toList_map,
            collectPrunedList_shift node fields.toList]
      | closure function mask typeInstantiation captures =>
          simp only [Option.map_none, RuntimeValue.shift_closure, Array.toList_map,
            collectPrunedList_shift node captures.toList]
      | borrow loan current =>
          simp only [Option.map_none, RuntimeValue.shift_borrow, collectPruned_shift node current]
      | loanHole loan => simp
      | unit | bool _ | character _ | integer _ | address _ | signer _ | string _ | bytes _ =>
          simp
termination_by sizeOf value
decreasing_by all_goals (simp_wf; (try simp only [Array.sizeOf_eq_toList]); omega)

theorem collectPrunedList_shift {α β : Type} {f : RuntimeValue → Option β}
    {g : RuntimeValue → Option α} {φ : α → β}
    (node : ∀ value, f (value.shift offset) = (g value).map φ)
    (values : List RuntimeValue) :
    collectPrunedList f (values.map (·.shift offset)) = (collectPrunedList g values).map φ := by
  cases values with
  | nil => simp [collectPrunedList]
  | cons value values =>
      rw [List.map_cons, collectPrunedList, collectPrunedList, collectPruned_shift node value,
        collectPrunedList_shift node values, Array.map_append]
termination_by sizeOf values
decreasing_by all_goals (simp_wf; omega)
end

/-! Node matchers commute with a shift of their loan and replacement. -/

theorem holeFill?_shift (loan : Nat) (replacement value : RuntimeValue) :
    holeFill? (loan + offset) (replacement.shift offset) (value.shift offset) =
      (holeFill? loan replacement value).map (·.shift offset) := by
  cases value <;> simp [Nat.add_right_cancel_iff]

theorem holeMark?_shift (loan : Nat) (value : RuntimeValue) :
    holeMark? (loan + offset) (value.shift offset) = (holeMark? loan value).map id := by
  cases value <;> simp [Nat.add_right_cancel_iff]

theorem anyHole?_shift (value : RuntimeValue) :
    anyHole? (value.shift offset) = (anyHole? value).map (· + offset) := by
  cases value <;> simp

theorem borrowCurrent?_shift (loan : Nat) (value : RuntimeValue) :
    borrowCurrent? (loan + offset) (value.shift offset) =
      (borrowCurrent? loan value).map (·.shift offset) := by
  cases value <;> simp [Nat.add_right_cancel_iff]

theorem borrowRewrite?_shift (loan : Nat) (replacement value : RuntimeValue) :
    borrowRewrite? (loan + offset) (replacement.shift offset) (value.shift offset) =
      (borrowRewrite? loan replacement value).map (·.shift offset) := by
  cases value <;> simp [Nat.add_right_cancel_iff]

theorem borrowClear?_shift (loan : Nat) (value : RuntimeValue) :
    borrowClear? (loan + offset) (value.shift offset) =
      (borrowClear? loan value).map (·.shift offset) := by
  cases value <;> simp [Nat.add_right_cancel_iff]



end Walks
end SemanticOperations

/-- A pending write-back or registration with its loan raised. -/
abbrev shiftEntry {α : Type} (offset : Nat) (entry : Nat × α) : Nat × α :=
  (entry.1 + offset, entry.2)

/-- Global memory with the loans its slots hold raised. -/
def GlobalMap.shift (offset : Nat) (globals : GlobalMap) : GlobalMap :=
  ⟨globals.entries.map fun slot => { slot with value := slot.value.shift offset }⟩

/-- A frame with its loans raised: its locals' values, the instance each
site's loan has, and the loans whose places it caches. -/
def RuntimeFrame.shift (offset : Nat) (frame : RuntimeFrame) : RuntimeFrame :=
  { frame with
    locals := frame.locals.map (Option.map (·.shift offset))
    activeLoans := frame.activeLoans.map fun entry => (entry.1, entry.2 + offset)
    loanLocations := frame.loanLocations.map (shiftEntry offset) }

/-- Every loan a frame holds or tracks is at or beyond `frontier`. -/
structure RuntimeFrame.Above (frontier : Nat) (frame : RuntimeFrame) : Prop where
  locals : ∀ slot ∈ frame.locals, ∀ value, slot = some value → value.Above frontier
  activeLoans : ∀ entry ∈ frame.activeLoans, frontier ≤ entry.2
  loanLocations : ∀ entry ∈ frame.loanLocations, frontier ≤ entry.1

/-- Global memory with the loans its slots hold lowered. -/
def GlobalMap.unshift (offset : Nat) (globals : GlobalMap) : GlobalMap :=
  ⟨globals.entries.map fun slot => { slot with value := slot.value.unshift offset }⟩

theorem GlobalMap.shift_unshift {frontier : Nat} {globals : GlobalMap}
    (above : ∀ slot ∈ globals.entries, slot.value.Above frontier) :
    (globals.unshift frontier).shift frontier = globals := by
  obtain ⟨entries⟩ := globals
  simp only [GlobalMap.shift, GlobalMap.unshift, Array.map_map, GlobalMap.mk.injEq]
  conv => rhs; rw [← Array.map_id entries]
  apply Array.map_congr_left
  intro slot member
  simp only [Function.comp_apply, id]
  rw [RuntimeValue.shift_unshift frontier slot.value (above slot member)]

theorem GlobalMap.above_shift_self (frontier : Nat) (globals : GlobalMap) :
    ∀ slot ∈ (globals.shift frontier).entries, slot.value.Above frontier := by
  intro slot member
  simp only [GlobalMap.shift, Array.mem_map] at member
  obtain ⟨base, -, rfl⟩ := member
  exact RuntimeValue.above_shift_self frontier base.value

/-- A frame with its loans lowered. -/
def RuntimeFrame.unshift (offset : Nat) (frame : RuntimeFrame) : RuntimeFrame :=
  { frame with
    locals := frame.locals.map (Option.map (·.unshift offset))
    activeLoans := frame.activeLoans.map fun entry => (entry.1, entry.2 - offset)
    loanLocations := frame.loanLocations.map fun entry => (entry.1 - offset, entry.2) }

theorem RuntimeFrame.shift_unshift {frontier : Nat} {frame : RuntimeFrame}
    (above : frame.Above frontier) : (frame.unshift frontier).shift frontier = frame := by
  obtain ⟨locals, activeLoans, loanLocations, typeInstantiation⟩ := frame
  simp only [RuntimeFrame.shift, RuntimeFrame.unshift, Array.map_map, RuntimeFrame.mk.injEq,
    and_true]
  refine ⟨?_, ?_, ?_⟩
  · conv => rhs; rw [← Array.map_id locals]
    apply Array.map_congr_left
    intro slot member
    cases slot with
    | none => rfl
    | some value =>
        simp only [Function.comp_apply, Option.map_some, id, Option.some.injEq]
        exact RuntimeValue.shift_unshift frontier value (above.locals _ member value rfl)
  · conv => rhs; rw [← Array.map_id activeLoans]
    apply Array.map_congr_left
    intro entry member
    have := above.activeLoans entry member
    simp only [Function.comp_apply, id]
    ext <;> simp; omega
  · conv => rhs; rw [← Array.map_id loanLocations]
    apply Array.map_congr_left
    intro entry member
    have := above.loanLocations entry member
    simp only [Function.comp_apply, id, shiftEntry]
    ext <;> simp; omega

theorem RuntimeFrame.above_shift_self (frontier : Nat) (frame : RuntimeFrame) :
    (frame.shift frontier).Above frontier where
  locals slot member value value_eq := by
    simp only [RuntimeFrame.shift, Array.mem_map] at member
    obtain ⟨base, -, rfl⟩ := member
    cases base with
    | none => cases value_eq
    | some base =>
        simp only [Option.map_some, Option.some.injEq] at value_eq
        exact value_eq ▸ RuntimeValue.above_shift_self frontier base
  activeLoans entry member := by
    simp only [RuntimeFrame.shift, Array.mem_map] at member
    obtain ⟨base, -, rfl⟩ := member
    simp
  loanLocations entry member := by
    simp only [RuntimeFrame.shift, Array.mem_map] at member
    obtain ⟨base, -, rfl⟩ := member
    simp

/-- A control with the loans of its values raised. -/
def Control.shift (offset : Nat) : Control → Control
  | .value result => .value (result.shift offset)
  | .break_ nest result => .break_ nest (result.map (·.shift offset))
  | .continue_ nest => .continue_ nest
  | .return_ values => .return_ (values.map (·.shift offset))
  | .throw_ kind arguments => .throw_ kind (arguments.map (·.shift offset))

/-- Every loan the values of a control hold is at or beyond `frontier`. -/
def Control.Above (frontier : Nat) : Control → Prop
  | .value result => result.Above frontier
  | .break_ _ result => ∀ value ∈ result, value.Above frontier
  | .continue_ _ => True
  | .return_ values | .throw_ _ values => ∀ value ∈ values, value.Above frontier

/-- An outcome with the loans of its values raised. -/
def Outcome.shift (offset : Nat) : Outcome → Outcome
  | .returned values => .returned (values.map (·.shift offset))
  | .threw kind arguments => .threw kind (arguments.map (·.shift offset))

/-- Every loan the values of an outcome hold is at or beyond `frontier`. -/
def Outcome.Above (frontier : Nat) : Outcome → Prop
  | .returned values | .threw _ values => ∀ value ∈ values, value.Above frontier

/-- A loan registry of a second run mirroring one of a first: the first's
registrations of the loans minted at or beyond `frontier`, raised, before
entries below each run's start frontier, which no loan of the run reaches. -/
def RegistryShifted (offset frontier : Nat) (registry registry' : List (Nat × GlobalKey)) :
    Prop :=
  ∃ minted junk junk', registry = minted ++ junk ∧
    registry' = minted.map (shiftEntry offset) ++ junk' ∧
    (∀ entry ∈ minted, frontier ≤ entry.1) ∧ (∀ entry ∈ junk, entry.1 < frontier) ∧
    ∀ entry ∈ junk', entry.1 < frontier + offset

/-- A state of a second run mirroring one of a first: its global memory, and
the write-backs pending past each run's first `inert`, with their loans
raised by `offset`; its frontier raised by it; and its loan registry the
first's registrations of the loans minted at or beyond `frontier`, raised,
before entries below each run's start frontier. Every loan the first run's
state holds outside its inert write-backs is at or beyond `frontier`. -/
structure StateShifted (offset frontier inert inert' : Nat) (state state' : RuntimeState) :
    Prop where
  globals : state'.globals = state.globals.shift offset
  nextLoan : state'.nextLoan = state.nextLoan + offset
  frontier_le : frontier ≤ state.nextLoan
  inert_le : inert ≤ state.pending.size
  inert_le' : inert' ≤ state'.pending.size
  pending : state'.pending.extract inert' state'.pending.size =
    (state.pending.extract inert state.pending.size).map fun entry =>
      (entry.1 + offset, entry.2.shift offset)
  registry : RegistryShifted offset frontier state.globalLoans state'.globalLoans
  globalsAbove : ∀ slot ∈ state.globals.entries, slot.value.Above frontier
  pendingAbove : ∀ entry ∈ state.pending.extract inert state.pending.size,
    frontier ≤ entry.1 ∧ entry.2.Above frontier

end LeanerIR
