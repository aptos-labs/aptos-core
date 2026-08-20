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
    values.toList[index.toNat]? = none ∧ code = Resource.executionFailure

/-- Checked functional update used by the compiler-facing `Vector.set`
primitive. -/
def setSpec (values : Move.Vector α) (index : U64) (value : α) :
    Spec σ (Move.Vector α) where
  ok := fun initial result final =>
    (∃ old, values.toList[index.toNat]? = some old) ∧
      result = Move.Vector.set values index value ∧ final = initial
  aborts := fun _ code =>
    values.toList[index.toNat]? = none ∧ code = Resource.executionFailure

@[simp] theorem borrowElemSpec_eq_pure {values : Move.Vector α} {index : U64}
    {value : α} (present : values.toList[index.toNat]? = some value) :
    (borrowElemSpec values index : Spec σ α) = Spec.pure value := by
  apply Spec.extensionality
  · funext initial result final
    simp [borrowElemSpec, Spec.pure, present, eq_comm]
  · funext initial code
    simp [borrowElemSpec, Spec.pure, present]

/-- Checked mutable element borrow. The element is represented by a prophecy
mutation while the vector owner is suspended. Successful reconciliation
returns both the body result and the updated logical vector. -/
def withBorrowElemMutSpec (values : Move.Vector α) (index : U64)
    (body : Mutation α → Spec σ (β × Mutation α)) :
    Spec σ (β × Move.Vector α) :=
  match values.toList[index.toNat]? with
  | none => Spec.abort Resource.executionFailure
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

end Move.Semantics.Vector
