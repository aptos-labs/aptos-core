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

/-- The index of the first `count` values, and the values after them. The
tree is `ofListFuel`'s, built in one traversal: splitting the list at every
level walks each level's values again, which the kernel pays for whenever a
certificate reduces an index. -/
def build : Nat → Nat → List α → IndexedArena α × List α
  | 0, _, values => (.empty, values)
  | _ + 1, 0, values => (.empty, values)
  | _ + 1, 1, [] => (.empty, [])
  | _ + 1, 1, value :: rest => (.leaf value, rest)
  | fuel + 1, count + 2, values =>
      let middle := (count + 2) / 2
      let (left, values) := build fuel middle values
      let (right, values) := build fuel (count + 2 - middle) values
      (.branch middle left right, values)

private theorem ofListFuel_long (fuel : Nat) (values : List α) (long : 2 ≤ values.length) :
    ofListFuel (fuel + 1) values =
      .branch (values.length / 2) (ofListFuel fuel (values.take (values.length / 2)))
        (ofListFuel fuel (values.drop (values.length / 2))) := by
  match values, long with
  | _ :: _ :: _, _ => rfl

theorem build_eq (fuel count : Nat) (values : List α) (small : count ≤ fuel)
    (enough : count ≤ values.length) :
    build fuel count values = (ofListFuel fuel (values.take count), values.drop count) := by
  induction fuel generalizing count values with
  | zero =>
      have : count = 0 := by omega
      subst this
      rfl
  | succ fuel ih =>
      match count, values, small, enough with
      | 0, values, _, _ => simp [build, ofListFuel]
      | 1, [], _, _ => simp [build, ofListFuel]
      | 1, value :: rest, _, _ => simp [build, ofListFuel]
      | count + 2, values, small, enough =>
          have taken : (values.take (count + 2)).length = count + 2 := by
            rw [List.length_take]; omega
          have middle_le : (count + 2) / 2 ≤ count + 2 := Nat.div_le_self _ _
          simp only [build]
          rw [ih _ _ (by omega) (by omega),
            ih _ _ (by omega) (by rw [List.length_drop]; omega),
            ofListFuel_long _ _ (by omega), taken, List.take_take, List.drop_take,
            List.drop_drop, Nat.min_eq_left middle_le,
            show (count + 2) / 2 + (count + 2 - (count + 2) / 2) = count + 2 by omega]

def ofArray (values : Array α) : IndexedArena α :=
  (build values.size values.size values.toList).1

theorem ofArray_eq (values : Array α) :
    ofArray values = ofListFuel values.size values.toList := by
  rw [ofArray, build_eq _ _ _ (Nat.le_refl _) (by simp), List.take_of_length_le (by simp)]

theorem get?_ofArray (values : Array α) (index : Nat) :
    (ofArray values).get? index = values[index]? := by
  rw [ofArray_eq, get?_ofListFuel _ _ _ (by simp)]
  simp

theorem get?_of_index_eq {values : Array α} {tree : IndexedArena α}
    (certificate : ofArray values = tree) (index : Nat) :
    values[index]? = tree.get? index := by
  rw [← get?_ofArray, certificate]

end IndexedArena

/-- A search tree over sparse natural keys: an index of a few entries of a
large key space, such as the anchors of a function's loan deaths. -/
inductive KeyTree (α : Type) where
  | leaf
  | node (left : KeyTree α) (key : Nat) (value : α) (right : KeyTree α)
  deriving Repr, Inhabited

namespace KeyTree

def find? : KeyTree α → Nat → Option α
  | .leaf, _ => none
  | .node left key value right, probe =>
      if Nat.blt probe key then left.find? probe
      else if Nat.blt key probe then right.find? probe
      else some value

/-- The balanced tree of the first `count` entries of a list sorted by key,
and the entries after them, built in one traversal. -/
def build : Nat → Nat → List (Nat × α) → KeyTree α × List (Nat × α)
  | 0, _, entries => (.leaf, entries)
  | _ + 1, 0, entries => (.leaf, entries)
  | _ + 1, _ + 1, [] => (.leaf, [])
  | fuel + 1, count + 1, entries@(_ :: _) =>
      let (left, entries) := build fuel (count / 2) entries
      match entries with
      | [] => (left, [])
      | (key, value) :: entries =>
          let (right, entries) := build fuel (count - count / 2) entries
          (.node left key value right, entries)

/-- The tree of entries sorted by key, without duplicate keys. -/
def ofSorted (entries : List (Nat × α)) : KeyTree α :=
  (build entries.length entries.length entries).1

end KeyTree
end LeanerIR.Validation
