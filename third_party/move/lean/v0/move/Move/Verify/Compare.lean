-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Basic

/-!
# Generic comparison laws

Proof interface for Move's built-in lexicographic comparison.  The compiler
lowers generic comparison through `std::cmp::compare<T>`.
-/

namespace Move.Compare

/-- Strict-order laws available for Move's native lexicographic comparison. -/
class Lawful (T : Type) : Prop where
  trans : ∀ {left middle right : T},
    Less left middle → Less middle right → Less left right
  asymm : ∀ {left right : T}, Less left right → ¬Less right left

/-- Move's generic equality operator agrees with Lean equality in source-level
proofs. This is a proof interface only; deployable functions carry no runtime
dictionary. -/
class LawfulEq (T : Type) : Prop where
  eq_iff : ∀ left right : T, equal left right = true ↔ left = right

@[simp] theorem equal_eq_true_iff [LawfulEq T] (left right : T) :
    equal left right = true ↔ left = right :=
  LawfulEq.eq_iff left right

/-- Move's structural comparison is a strict total order whose equality case
is the language's built-in structural equality. `Lawful` is enough for
sorting; ordered containers additionally use totality and equality. -/
class Total (T : Type) : Prop extends Lawful T, LawfulEq T where
  total : ∀ left right : T,
    Less left right ∨ Less right left ∨
      left = right

end Move.Compare
