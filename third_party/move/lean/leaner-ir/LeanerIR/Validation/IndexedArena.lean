-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Syntax

namespace LeanerIR.Validation

/-- A proof-facing index of an immutable arena. Runtime arenas remain arrays;
the balanced view makes closed certificate lookups logarithmic. -/
inductive IndexedArena (α : Type) where
  | empty
  | leaf (value : α)
  | branch (leftSize : Nat) (left right : IndexedArena α)
  deriving Repr, BEq, Inhabited

namespace IndexedArena

def get? : IndexedArena α → Nat → Option α
  | .empty, _ => none
  | .leaf value, index => if index == 0 then some value else none
  | .branch leftSize left right, index =>
      if index < leftSize then left.get? index else right.get? (index - leftSize)

/-- Structural fuel keeps construction reducible in the kernel. -/
def ofListFuel : Nat → List α → IndexedArena α
  | 0, _ => .empty
  | _ + 1, [] => .empty
  | _ + 1, [value] => .leaf value
  | fuel + 1, values@(_ :: _ :: _) =>
      let middle := values.length / 2
      .branch middle (ofListFuel fuel (values.take middle))
        (ofListFuel fuel (values.drop middle))

theorem get?_ofListFuel (fuel : Nat) (values : List α) (index : Nat)
    (enough : values.length ≤ fuel) :
    (ofListFuel fuel values).get? index = values[index]? := by
  induction fuel generalizing values index with
  | zero =>
      have : values = [] := List.length_eq_zero_iff.mp (by omega)
      subst values
      rfl
  | succ fuel ih =>
      cases values with
      | nil => rfl
      | cons first rest =>
          cases rest with
          | nil =>
              cases index <;> simp [ofListFuel, get?]
          | cons second rest =>
              simp only [ofListFuel, get?]
              split
              · rw [ih _ _ (by simp; simp at enough; omega)]
                exact List.getElem?_take_of_lt (by assumption)
              · rw [ih _ _ (by simp; simp at enough; omega)]
                simp only [List.getElem?_drop]
                congr 1
                omega

def ofArray (values : Array α) : IndexedArena α :=
  ofListFuel values.size values.toList

theorem get?_ofArray (values : Array α) (index : Nat) :
    (ofArray values).get? index = values[index]? := by
  rw [ofArray, get?_ofListFuel _ _ _ (by simp)]
  simp

theorem get?_of_index_eq {values : Array α} {tree : IndexedArena α}
    (certificate : ofArray values = tree) (index : Nat) :
    values[index]? = tree.get? index := by
  rw [← get?_ofArray, certificate]

end IndexedArena
end LeanerIR.Validation
