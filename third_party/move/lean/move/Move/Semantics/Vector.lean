-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Semantics.Global

/-!
# Verification semantics for local vectors

The compiler-facing `Move.Vector` stays opaque so its operations survive Lean
elaboration. Source verification instead interprets authored vector literals
as logical lists and gives checked element borrows their Move abort behavior.
-/

namespace Move.Semantics.Vector

open Move

/-- Abort code used by Move's `std::vector` for an invalid index. -/
def indexOutOfBounds : Nat := 0x20000

/-- Checked immutable element borrow after immutable-reference erasure. -/
def borrowElemSpec (values : Move.Vector α) (index : U64) : Spec σ α where
  ok := fun initial result final =>
    values.toList[index.toNat]? = some result ∧ final = initial
  aborts := fun _ code =>
    values.toList[index.toNat]? = none ∧ code = indexOutOfBounds

/-- Checked functional update used by the compiler-facing `Vector.set`
primitive. -/
def setSpec (values : Move.Vector α) (index : U64) (value : α) :
    Spec σ (Move.Vector α) where
  ok := fun initial result final =>
    (∃ old, values.toList[index.toNat]? = some old) ∧
      result = Move.Vector.set values index value ∧ final = initial
  aborts := fun _ code =>
    values.toList[index.toNat]? = none ∧ code = indexOutOfBounds

@[simp] theorem borrowElemSpec_eq_pure {values : Move.Vector α} {index : U64}
    {value : α} (present : values.toList[index.toNat]? = some value) :
    (borrowElemSpec values index : Spec σ α) = Spec.pure value := by
  apply Spec.extensionality
  · funext initial result final
    simp [borrowElemSpec, Spec.pure, present, eq_comm]
  · funext initial code
    simp [borrowElemSpec, Spec.pure, present]

  · rfl
/-- Checked mutable element borrow. The element is represented by a prophecy
mutation while the vector owner is suspended. Successful reconciliation
returns both the body result and the updated logical vector. -/
def withBorrowElemMutSpec (values : Move.Vector α) (index : U64)
    (body : Mutation α → Spec σ (β × Mutation α)) :
    Spec σ (β × Move.Vector α) :=
  match values.toList[index.toNat]? with
  | none => Spec.abort indexOutOfBounds
  | some value => do
      let output ← withMutation value body
      pure (output.1, Move.Vector.set values index output.2)

/-- Prophecy semantics of `vector::insert` through a mutable reference. Like
the runtime operation, it aborts both out of bounds and when the grown
vector would leave Move's `u64` length domain. -/
def insertSpec (reference : Mutation (Move.Vector α)) (index : U64)
    (value : α) : Spec σ (Unit × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  if room : index.toNat ≤ values.length ∧ values.length + 1 < U64.size then
    let updated := Move.Vector.ofList
      (values.take index.toNat ++ value :: values.drop index.toNat)
      (by
        have inBounds := room.1
        have hasRoom : values.length + 1 <
            MoveModel.IR.IntWidth.size .w64 := room.2
        simp only [List.length_append, List.length_cons, List.length_take,
          List.length_drop]
        omega)
    Spec.pure ((), reference.write updated)
  else
    Spec.abort indexOutOfBounds

/-- Prophecy semantics of `vector::remove` through a mutable reference. -/
def removeSpec (reference : Mutation (Move.Vector α)) (index : U64) :
    Spec σ (α × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  match values[index.toNat]? with
  | none => Spec.abort indexOutOfBounds
  | some removed =>
      let updated := Move.Vector.ofList
        (values.take index.toNat ++ values.drop (index.toNat + 1))
        (by
          have bounded : values.length < MoveModel.IR.IntWidth.size .w64 :=
            reference.read.toList_length_lt
          simp only [List.length_append, List.length_take, List.length_drop]
          omega)
      Spec.pure (removed, reference.write updated)

/-- Prophecy semantics of `vector::pop_back` through a mutable reference. -/
def popBackSpec (reference : Mutation (Move.Vector α)) :
    Spec σ (α × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  match values.getLast? with
  | none => Spec.abort indexOutOfBounds
  | some removed =>
      let updated := Move.Vector.ofList values.dropLast (by
        have bounded : values.length < U64.size := reference.read.toList_length_lt
        simpa only [List.length_dropLast] using
          Nat.lt_of_le_of_lt (Nat.sub_le values.length 1) bounded)
      Spec.pure (removed, reference.write updated)

def swapSpec (reference : Mutation (Move.Vector α)) (i j : U64) :
    Spec σ (Unit × Mutation (Move.Vector α)) :=
  let values := reference.read
  match values.toList[i.toNat]?, values.toList[j.toNat]? with
  | some vi, some vj =>
      Spec.pure ((), reference.write (Move.Vector.set
        (Move.Vector.set values i vj) j vi))
  | _, _ => Spec.abort indexOutOfBounds

def swapRemoveSpec (reference : Mutation (Move.Vector α)) (i : U64) :
    Spec σ (α × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  match values[i.toNat]?, values.getLast? with
  | some removed, some last =>
      let updatedValues := (values.set i.toNat last).dropLast
      let updated := Move.Vector.ofList updatedValues (by
        change (values.set i.toNat last).dropLast.length < U64.size
        simp only [List.length_dropLast, List.length_set]
        exact Nat.lt_of_le_of_lt (Nat.sub_le _ _) reference.read.toList_length_lt)
      Spec.pure (removed, reference.write updated)
  | _, _ => Spec.abort indexOutOfBounds

def appendSpec (reference : Mutation (Move.Vector α)) (other : Move.Vector α) :
    Spec σ (Unit × Mutation (Move.Vector α)) :=
  let lhs := reference.read.toList
  let rhs := other.toList
  if room : lhs.length + rhs.length < U64.size then
    let updated := Move.Vector.ofList (lhs ++ rhs) (by simpa using room)
    Spec.pure ((), reference.write updated)
  else Spec.abort indexOutOfBounds

def reverseSpec (reference : Mutation (Move.Vector α)) :
    Spec σ (Unit × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  let updated := Move.Vector.ofList values.reverse (by
    simpa using reference.read.toList_length_lt)
  Spec.pure ((), reference.write updated)

def reverseSliceSpec (reference : Mutation (Move.Vector α))
    (left right : U64) : Spec σ (Unit × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  if range : left.toNat ≤ right.toNat ∧ right.toNat ≤ values.length then
    let result := values.take left.toNat ++
      ((values.drop left.toNat).take (right.toNat - left.toNat)).reverse ++
      values.drop right.toNat
    let updated := Move.Vector.ofList result (by
      change (values.take left.toNat ++
        ((values.drop left.toNat).take (right.toNat - left.toNat)).reverse ++
        values.drop right.toNat).length < U64.size
      simp only [List.length_append, List.length_take, List.length_reverse,
        List.length_drop]
      rw [Nat.min_eq_left (Nat.le_trans range.1 range.2)]
      rw [Nat.min_eq_left (by omega : right.toNat - left.toNat ≤
        values.length - left.toNat)]
      have bounded := reference.read.toList_length_lt
      change values.length < U64.size at bounded
      omega)
    Spec.pure ((), reference.write updated)
  else Spec.abort 0x20001

def trimSpec (reference : Mutation (Move.Vector α)) (newLen : U64) :
    Spec σ (Move.Vector α × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  if bound : newLen.toNat ≤ values.length then
    let retained := Move.Vector.ofList (values.take newLen.toNat)
      (Nat.lt_of_le_of_lt (List.length_take_le ..) newLen.toNat_lt)
    let evicted := Move.Vector.ofList (values.drop newLen.toNat) (by
      simp only [List.length_drop]
      exact Nat.lt_of_le_of_lt (Nat.sub_le _ _) reference.read.toList_length_lt)
    Spec.pure (evicted, reference.write retained)
  else Spec.abort indexOutOfBounds

def trimReverseSpec (reference : Mutation (Move.Vector α)) (newLen : U64) :
    Spec σ (Move.Vector α × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  if bound : newLen.toNat ≤ values.length then
    let retained := Move.Vector.ofList (values.take newLen.toNat)
      (Nat.lt_of_le_of_lt (List.length_take_le ..) newLen.toNat_lt)
    let evicted := Move.Vector.ofList (values.drop newLen.toNat).reverse (by
      simp only [List.length_reverse, List.length_drop]
      exact Nat.lt_of_le_of_lt (Nat.sub_le _ _) reference.read.toList_length_lt)
    Spec.pure (evicted, reference.write retained)
  else Spec.abort indexOutOfBounds

def rotateSliceValues (values : List α) (left rot right : Nat) : List α :=
  values.take left ++ (values.drop rot).take (right - rot) ++
    (values.drop left).take (rot - left) ++ values.drop right

def rotateSliceSpec (reference : Mutation (Move.Vector α))
    (left rot right : U64) : Spec σ (U64 × Mutation (Move.Vector α)) :=
  let values := reference.read.toList
  if range : (left.toNat ≤ rot.toNat ∧ rot.toNat ≤ right.toNat) ∧
      right.toNat ≤ values.length then
    let result := rotateSliceValues values left.toNat rot.toNat right.toNat
    let updated := Move.Vector.ofList result (by
      change (rotateSliceValues values left.toNat rot.toNat right.toNat).length < U64.size
      simp only [rotateSliceValues, List.length_append, List.length_take,
        List.length_drop]
      rw [Nat.min_eq_left (by omega : left.toNat ≤ values.length)]
      rw [Nat.min_eq_left (by omega : right.toNat - rot.toNat ≤
        values.length - rot.toNat)]
      rw [Nat.min_eq_left (by omega : rot.toNat - left.toNat ≤
        values.length - left.toNat)]
      have bounded : values.length < U64.size := reference.read.toList_length_lt
      change values.length < U64.size at bounded
      omega)
    Spec.pure (U64.ofNat (left.toNat + (right.toNat - rot.toNat)),
      reference.write updated)
  else Spec.abort 0x20001

def rotateSpec (reference : Mutation (Move.Vector α)) (rot : U64) :
    Spec σ (U64 × Mutation (Move.Vector α)) :=
  rotateSliceSpec reference 0 rot (Move.Vector.length reference.read)

def destroyEmptySpec (values : Move.Vector α) : Spec σ Unit :=
  if values.toList.isEmpty then Spec.pure () else Spec.abort indexOutOfBounds

@[simp] theorem swapSpec_undefined (reference : Mutation (Move.Vector α))
    (i j : U64) (state : σ) : ¬(swapSpec reference i j).undefined state := by
  simp [swapSpec]
  split <;> simp

@[simp] theorem swapRemoveSpec_undefined (reference : Mutation (Move.Vector α))
    (i : U64) (state : σ) : ¬(swapRemoveSpec reference i).undefined state := by
  simp [swapRemoveSpec]
  split <;> simp

@[simp] theorem appendSpec_undefined (reference : Mutation (Move.Vector α))
    (other : Move.Vector α) (state : σ) :
    ¬(appendSpec reference other).undefined state := by simp [appendSpec]

@[simp] theorem reverseSpec_undefined (reference : Mutation (Move.Vector α))
    (state : σ) : ¬(reverseSpec reference).undefined state := by simp [reverseSpec]

@[simp] theorem reverseSliceSpec_undefined (reference : Mutation (Move.Vector α))
    (left right : U64) (state : σ) :
    ¬(reverseSliceSpec reference left right).undefined state := by simp [reverseSliceSpec]

@[simp] theorem trimSpec_undefined (reference : Mutation (Move.Vector α))
    (newLen : U64) (state : σ) :
    ¬(trimSpec reference newLen).undefined state := by simp [trimSpec]

@[simp] theorem trimReverseSpec_undefined (reference : Mutation (Move.Vector α))
    (newLen : U64) (state : σ) :
    ¬(trimReverseSpec reference newLen).undefined state := by simp [trimReverseSpec]

@[simp] theorem rotateSliceSpec_undefined (reference : Mutation (Move.Vector α))
    (left rot right : U64) (state : σ) :
    ¬(rotateSliceSpec reference left rot right).undefined state := by simp [rotateSliceSpec]

@[simp] theorem rotateSpec_undefined (reference : Mutation (Move.Vector α))
    (rot : U64) (state : σ) :
    ¬(rotateSpec reference rot).undefined state := by
  exact rotateSliceSpec_undefined _ _ _ _ _

@[simp] theorem destroyEmptySpec_undefined (values : Move.Vector α) (state : σ) :
    ¬(destroyEmptySpec values).undefined state := by simp [destroyEmptySpec]

/-- Both vector mutations are total: they abort out of bounds rather than
leaving an obligation behind. -/
@[simp] theorem insertSpec_undefined (reference : Mutation (Move.Vector α))
    (index : U64) (value : α) (state : σ) :
    ¬(insertSpec reference index value : Spec σ _).undefined state := by
  simp only [insertSpec]
  split <;> simp

@[simp] theorem removeSpec_undefined (reference : Mutation (Move.Vector α))
    (index : U64) (state : σ) :
    ¬(removeSpec reference index : Spec σ _).undefined state := by
  simp only [removeSpec]
  split <;> simp

@[simp] theorem popBackSpec_undefined (reference : Mutation (Move.Vector α))
    (state : σ) :
    ¬(popBackSpec reference : Spec σ _).undefined state := by
  simp only [popBackSpec]
  split <;> simp

@[simp] theorem total_borrowElemSpec (values : Move.Vector α) (index : U64) :
    Spec.Total (borrowElemSpec values index : Spec σ α) := fun _ h => h.elim

@[simp] theorem total_setSpec (values : Move.Vector α) (index : U64) (value : α) :
    Spec.Total (setSpec values index value : Spec σ _) := fun _ h => h.elim

/-- Pointwise well-definedness of a mutable element borrow. -/
theorem withBorrowElemMutSpec_defined {values : Move.Vector α} {index : U64}
    {state : σ} {body : Mutation α → Spec σ (β × Mutation α)}
    (scope : ∀ reference, ¬(body reference).undefined state) :
    ¬(withBorrowElemMutSpec values index body).undefined state := by
  simp only [withBorrowElemMutSpec]
  split
  · exact fun h => h.elim
  · refine Spec.bind_defined (Move.Semantics.withMutation_defined scope) ?_
    intro output middle _
    exact fun h => h.elim

theorem withBorrowElemMutSpec_total {values : Move.Vector α} {index : U64}
    {body : Mutation α → Spec σ (β × Mutation α)}
    (total : ∀ reference, Spec.Total (body reference)) :
    Spec.Total (withBorrowElemMutSpec values index body) := by
  intro state
  simp only [withBorrowElemMutSpec]
  split
  · exact fun h => h.elim
  · refine Spec.Total.bind (Move.Semantics.withMutation_total total) ?_ state
    intro output
    exact fun _ h => h.elim

theorem insertSpec_total (reference : Mutation (Move.Vector α))
    (index : U64) (value : α) :
    Spec.Total (insertSpec reference index value : Spec σ _) :=
  insertSpec_undefined reference index value

theorem removeSpec_total (reference : Mutation (Move.Vector α))
    (index : U64) :
    Spec.Total (removeSpec reference index : Spec σ _) :=
  removeSpec_undefined reference index

end Move.Semantics.Vector
