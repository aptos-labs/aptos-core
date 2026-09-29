-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Runtime

/-!
# Laws of the structural order

`RuntimeValue.order` is a total order for every assignment of variant
positions: it is oriented (swapping the operands swaps the result), a tie
is equality, and it is transitive. Generic code comparing values of a type
it does not know relies on exactly these laws.
-/

namespace LeanerIR

/-- An array's size measure is one more than its list's. -/
private theorem sizeOf_array {α : Type} [SizeOf α] (array : Array α) :
    sizeOf array = 1 + sizeOf array.toList := by
  cases array
  simp

theorem compare_swap {α : Type} [Ord α] [Std.OrientedOrd α] (a b : α) :
    compare a b = (compare b a).swap := Std.OrientedCmp.eq_swap

theorem compareAddress_swap (a b : String) : compareAddress a b = (compareAddress b a).swap := by
  unfold compareAddress
  rw [Ordering.swap_then, ← compare_swap, ← compare_swap]

theorem compareVariant_swap (rank : StructHandle → String → Nat) (ls rs : StructHandle)
    (lv rv : Option String) :
    compareVariant rank ls rs lv rv = (compareVariant rank rs ls rv lv).swap := by
  cases lv <;> cases rv <;> simp only [compareVariant] <;>
    first
    | rfl
    | rw [Ordering.swap_then, ← compare_swap, ← compare_swap]

mutual
theorem RuntimeValue.order_swap (rank : StructHandle → String → Nat) (a b : RuntimeValue) :
    RuntimeValue.order rank a b = (RuntimeValue.order rank b a).swap := by
  rw [RuntimeValue.order, RuntimeValue.order, Ordering.swap_then,
    RuntimeValue.orderPayload_swap rank a b, ← compare_swap]
termination_by sizeOf a + sizeOf b + 1

theorem RuntimeValue.orderPayload_swap (rank : StructHandle → String → Nat) (a b : RuntimeValue) :
    RuntimeValue.orderPayload rank a b = (RuntimeValue.orderPayload rank b a).swap := by
  cases a <;> cases b <;> simp only [RuntimeValue.orderPayload] <;>
    first
    | rfl
    | exact compare_swap _ _
    | exact compareAddress_swap _ _
    | exact Std.OrientedCmp.eq_swap
    | (rw [Ordering.swap_then, Ordering.swap_then, Ordering.swap_then, ← compare_swap, ← compare_swap,
        ← compareVariant_swap, ← RuntimeValue.orderList_swap])
    | (rw [Ordering.swap_then, Ordering.swap_then, ← compare_swap, ← compare_swap,
        ← RuntimeValue.orderList_swap])
    | (rw [Ordering.swap_then, ← compare_swap, ← RuntimeValue.order_swap])
    | exact RuntimeValue.orderList_swap rank _ _
termination_by sizeOf a + sizeOf b
decreasing_by all_goals (simp_wf; (try simp only [sizeOf_array]); omega)

theorem RuntimeValue.orderList_swap (rank : StructHandle → String → Nat) (as bs : List RuntimeValue) :
    RuntimeValue.orderList rank as bs = (RuntimeValue.orderList rank bs as).swap := by
  cases as <;> cases bs <;> simp only [RuntimeValue.orderList] <;>
    first
    | rfl
    | rw [Ordering.swap_then, ← RuntimeValue.order_swap, ← RuntimeValue.orderList_swap]
termination_by sizeOf as + sizeOf bs
end



theorem eq_of_compare {α : Type} [Ord α] [Std.LawfulEqOrd α] {a b : α} (h : compare a b = .eq) :
    a = b := Std.LawfulEqCmp.eq_of_compare h

theorem eq_of_compareAddress {a b : String} (h : compareAddress a b = .eq) : a = b := by
  rw [compareAddress, Ordering.then_eq_eq] at h
  exact eq_of_compare h.2

theorem eq_of_compareVariant {rank : StructHandle → String → Nat} {ls rs : StructHandle}
    {lv rv : Option String} (h : compareVariant rank ls rs lv rv = .eq) : lv = rv := by
  cases lv <;> cases rv <;> simp only [compareVariant, reduceCtorEq] at h
  · rfl
  · rw [Ordering.then_eq_eq] at h
    rw [eq_of_compare h.2]


mutual
theorem RuntimeValue.eq_of_order {rank : StructHandle → String → Nat} {a b : RuntimeValue}
    (h : RuntimeValue.order rank a b = .eq) : a = b := by
  rw [RuntimeValue.order, Ordering.then_eq_eq] at h
  exact RuntimeValue.eq_of_orderPayload a b (eq_of_compare h.1) h.2
termination_by sizeOf a + sizeOf b + 1

theorem RuntimeValue.eq_of_orderPayload {rank : StructHandle → String → Nat} (a b : RuntimeValue) :
    a.kindRank = b.kindRank → RuntimeValue.orderPayload rank a b = .eq → a = b := by
  cases a <;> cases b <;> intro kinds h <;> simp only [RuntimeValue.kindRank] at kinds <;>
    (try omega) <;> simp only [RuntimeValue.orderPayload, Ordering.then_eq_eq] at h
  case unit.unit => rfl
  case bool.bool => rw [eq_of_compare h]
  case character.character => rw [eq_of_compare h]
  case integer.integer => rw [eq_of_compare h]
  case address.address => rw [eq_of_compareAddress h]
  case signer.signer => rw [eq_of_compareAddress h]
  case string.string => rw [eq_of_compare h]
  case bytes.bytes left right =>
    have : left.toList = right.toList := eq_of_compare (α := List UInt8) h
    rw [Array.toList_inj.mp this]
  case vector.vector left right =>
    rw [Array.toList_inj.mp (RuntimeValue.eq_of_orderList h)]
  case tuple.tuple left right =>
    rw [Array.toList_inj.mp (RuntimeValue.eq_of_orderList h)]
  case nominal.nominal leftSource leftVariant leftFields rightSource rightVariant rightFields =>
    obtain ⟨⟨namespaces, structs⟩, variants, fields⟩ := h
    obtain ⟨⟨leftNamespace⟩, leftStruct⟩ := leftSource
    obtain ⟨⟨rightNamespace⟩, rightStruct⟩ := rightSource
    have namespaceEq := eq_of_compare namespaces
    have structEq := eq_of_compare structs
    simp only at namespaceEq structEq
    subst namespaceEq structEq
    rw [eq_of_compareVariant variants, Array.toList_inj.mp (RuntimeValue.eq_of_orderList fields)]
  case closure.closure leftFunction leftCaptures rightFunction rightCaptures =>
    obtain ⟨⟨namespaces, functions⟩, captures⟩ := h
    obtain ⟨⟨leftNamespace⟩, ⟨leftFunction⟩⟩ := leftFunction
    obtain ⟨⟨rightNamespace⟩, ⟨rightFunction⟩⟩ := rightFunction
    have namespaceEq := eq_of_compare namespaces
    have functionEq := eq_of_compare functions
    simp only at namespaceEq functionEq
    subst namespaceEq functionEq
    rw [Array.toList_inj.mp (RuntimeValue.eq_of_orderList captures)]
  case borrow.borrow leftLoan leftCurrent rightLoan rightCurrent =>
    rw [eq_of_compare h.1, RuntimeValue.eq_of_order h.2]
  case loanHole.loanHole => rw [eq_of_compare h]
termination_by sizeOf a + sizeOf b
decreasing_by all_goals (simp_wf; (try simp only [sizeOf_array]); omega)

theorem RuntimeValue.eq_of_orderList {rank : StructHandle → String → Nat} {as bs : List RuntimeValue}
    (h : RuntimeValue.orderList rank as bs = .eq) : as = bs := by
  cases as <;> cases bs <;> simp only [RuntimeValue.orderList, reduceCtorEq] at h
  · rfl
  · rw [Ordering.then_eq_eq] at h
    rw [RuntimeValue.eq_of_order h.1, RuntimeValue.eq_of_orderList h.2]
termination_by sizeOf as + sizeOf bs
end


/-! ## Transitivity -/

/-- Lexicographic transitivity: the head comparisons are transitive and
strict, and the tails are transitive where the heads tie. -/
theorem lex_isLE_trans {x y z p q r : Ordering}
    (trans : x.isLE → y.isLE → z.isLE)
    (strictLeft : x = .lt → y.isLE → z ≠ .eq)
    (strictRight : x.isLE → y = .lt → z ≠ .eq)
    (tie : x = .eq → y = .eq → z = .eq)
    (inner : x = .eq → y = .eq → p.isLE → q.isLE → r.isLE)
    (left : (x.then p).isLE) (right : (y.then q).isLE) : (z.then r).isLE := by
  cases x <;> cases y <;> simp only [Ordering.then, Ordering.isLE, Bool.false_eq_true] at left right
  · have := trans rfl rfl
    have := strictLeft rfl rfl
    cases z <;> simp_all [Ordering.then, Ordering.isLE]
  · have := trans rfl rfl
    have := strictLeft rfl rfl
    cases z <;> simp_all [Ordering.then, Ordering.isLE]
  · have := trans rfl rfl
    have := strictRight rfl rfl
    cases z <;> simp_all [Ordering.then, Ordering.isLE]
  · rw [tie rfl rfl]
    exact inner rfl rfl left right

/-- Lexicographic transitivity with a lawful head comparison. -/
theorem lex_isLE_trans_of {α : Type} {cmp : α → α → Ordering} [Std.TransCmp cmp] {a b c : α}
    {p q r : Ordering}
    (inner : cmp a b = .eq → cmp b c = .eq → p.isLE → q.isLE → r.isLE)
    (left : ((cmp a b).then p).isLE) (right : ((cmp b c).then q).isLE) :
    ((cmp a c).then r).isLE :=
  lex_isLE_trans Std.TransCmp.isLE_trans
    (fun lt le => by rw [Std.TransCmp.lt_of_lt_of_isLE lt le]; decide)
    (fun le lt => by rw [Std.TransCmp.lt_of_isLE_of_lt le lt]; decide)
    Std.TransCmp.eq_trans inner left right

theorem compareAddress_isLE_trans {a b c : String} (left : (compareAddress a b).isLE)
    (right : (compareAddress b c).isLE) : (compareAddress a c).isLE :=
  lex_isLE_trans_of (fun _ _ => Std.TransCmp.isLE_trans) left right

theorem compareVariant_isLE_trans {rank : StructHandle → String → Nat} {ls ms rs : StructHandle}
    {lv mv rv : Option String} (left : (compareVariant rank ls ms lv mv).isLE)
    (right : (compareVariant rank ms rs mv rv).isLE) : (compareVariant rank ls rs lv rv).isLE := by
  cases lv <;> cases mv <;> cases rv <;> simp only [compareVariant] at left right ⊢ <;>
    first
    | rfl
    | exact absurd left (by decide)
    | exact absurd right (by decide)
    | exact lex_isLE_trans_of (fun _ _ => Std.TransCmp.isLE_trans) left right

/-- A comparison equal to its own swap ties. -/
theorem eq_of_self_swap {x : Ordering} (h : x = x.swap) : x = .eq := by
  cases x <;> simp_all [Ordering.swap]

/-- Lexicographic transitivity for a head comparison with the swap and
equality laws: strictness and ties follow from them. -/
theorem lex_isLE_trans_laws {α : Type} (cmp : α → α → Ordering)
    (swap : ∀ a b, cmp a b = (cmp b a).swap) (eq : ∀ {a b}, cmp a b = .eq → a = b)
    {a b c : α} (trans : (cmp a b).isLE → (cmp b c).isLE → (cmp a c).isLE) {p q r : Ordering}
    (inner : cmp a b = .eq → cmp b c = .eq → p.isLE → q.isLE → r.isLE)
    (left : ((cmp a b).then p).isLE) (right : ((cmp b c).then q).isLE) :
    ((cmp a c).then r).isLE := by
  refine lex_isLE_trans trans ?_ ?_ ?_ inner left right
  · intro lt le tie
    obtain rfl := eq tie
    rw [swap b a, lt] at le
    exact absurd le (by decide)
  · intro le lt tie
    obtain rfl := eq tie
    rw [swap a b, lt] at le
    exact absurd le (by decide)
  · intro tieLeft tieRight
    obtain rfl := eq tieLeft
    obtain rfl := eq tieRight
    exact eq_of_self_swap (swap a a)

mutual
theorem RuntimeValue.order_isLE_trans (rank : StructHandle → String → Nat) (a b c : RuntimeValue) :
    (RuntimeValue.order rank a b).isLE → (RuntimeValue.order rank b c).isLE →
      (RuntimeValue.order rank a c).isLE := by
  intro left right
  rw [RuntimeValue.order] at left right ⊢
  exact lex_isLE_trans_of (fun kab kbc =>
    RuntimeValue.orderPayload_isLE_trans rank a b c (eq_of_compare kab) (eq_of_compare kbc)) left right
termination_by 2 * sizeOf a + 1
decreasing_by all_goals (simp_wf <;> omega)

theorem RuntimeValue.orderPayload_isLE_trans (rank : StructHandle → String → Nat)
    (a b c : RuntimeValue) :
    a.kindRank = b.kindRank → b.kindRank = c.kindRank →
      (RuntimeValue.orderPayload rank a b).isLE → (RuntimeValue.orderPayload rank b c).isLE →
        (RuntimeValue.orderPayload rank a c).isLE := by
  cases a <;> cases b <;> cases c <;> intro k1 k2 <;> simp only [RuntimeValue.kindRank] at k1 k2 <;>
    (try omega) <;> intro left right <;> simp only [RuntimeValue.orderPayload] at left right ⊢
  case unit.unit.unit => rfl
  case bool.bool.bool => exact Std.TransCmp.isLE_trans left right
  case character.character.character => exact Std.TransCmp.isLE_trans left right
  case integer.integer.integer => exact Std.TransCmp.isLE_trans left right
  case address.address.address => exact compareAddress_isLE_trans left right
  case signer.signer.signer => exact compareAddress_isLE_trans left right
  case string.string.string => exact Std.TransCmp.isLE_trans left right
  case bytes.bytes.bytes =>
    exact Std.TransCmp.isLE_trans (cmp := (compare : List UInt8 → List UInt8 → Ordering)) left right
  case vector.vector.vector => exact RuntimeValue.orderList_isLE_trans rank _ _ _ left right
  case tuple.tuple.tuple => exact RuntimeValue.orderList_isLE_trans rank _ _ _ left right
  case nominal.nominal.nominal leftSource leftVariant leftFields middleSource middleVariant
      middleFields rightSource rightVariant rightFields =>
    rw [Ordering.then_assoc] at left right ⊢
    refine lex_isLE_trans_of (fun namespacesLeft namespacesRight left right => ?_) left right
    refine lex_isLE_trans_of (fun structsLeft structsRight left right => ?_) left right
    obtain ⟨⟨leftNamespace⟩, leftStruct⟩ := leftSource
    obtain ⟨⟨middleNamespace⟩, middleStruct⟩ := middleSource
    obtain ⟨⟨rightNamespace⟩, rightStruct⟩ := rightSource
    have e1 := eq_of_compare namespacesLeft
    have e2 := eq_of_compare namespacesRight
    have e3 := eq_of_compare structsLeft
    have e4 := eq_of_compare structsRight
    simp only at e1 e2 e3 e4
    subst e1 e2 e3 e4
    exact lex_isLE_trans_laws (compareVariant rank _ _) (compareVariant_swap rank _ _)
      eq_of_compareVariant compareVariant_isLE_trans
      (fun _ _ left right => RuntimeValue.orderList_isLE_trans rank _ _ _ left right) left right
  case closure.closure.closure =>
    rw [Ordering.then_assoc] at left right ⊢
    refine lex_isLE_trans_of (fun _ _ left right => ?_) left right
    exact lex_isLE_trans_of
      (fun _ _ left right => RuntimeValue.orderList_isLE_trans rank _ _ _ left right) left right
  case borrow.borrow.borrow =>
    exact lex_isLE_trans_of
      (fun _ _ left right => RuntimeValue.order_isLE_trans rank _ _ _ left right) left right
  case loanHole.loanHole.loanHole => exact Std.TransCmp.isLE_trans left right
termination_by 2 * sizeOf a
decreasing_by all_goals (simp_wf; (try simp only [sizeOf_array]); omega)

theorem RuntimeValue.orderList_isLE_trans (rank : StructHandle → String → Nat)
    (as bs cs : List RuntimeValue) :
    (RuntimeValue.orderList rank as bs).isLE → (RuntimeValue.orderList rank bs cs).isLE →
      (RuntimeValue.orderList rank as cs).isLE := by
  cases as <;> cases bs <;> cases cs <;> intro left right <;>
    simp only [RuntimeValue.orderList] at left right ⊢ <;>
    first
    | rfl
    | exact absurd left (by decide)
    | exact absurd right (by decide)
    | exact lex_isLE_trans_laws (RuntimeValue.order rank) (RuntimeValue.order_swap rank)
        RuntimeValue.eq_of_order (RuntimeValue.order_isLE_trans rank _ _ _)
        (fun _ _ left right => RuntimeValue.orderList_isLE_trans rank _ _ _ left right) left right
termination_by 2 * sizeOf as + 2
decreasing_by all_goals (simp_wf <;> omega)
end

/-! ## Normal forms

The integer `compare` returns is tested against `0`; the tests read as the
ordering itself, and on primitive values the order is the natural one. -/

@[simp] theorem orderValue_lt_zero (o : Ordering) : orderValue o < 0 ↔ o = .lt := by
  cases o <;> decide
@[simp] theorem orderValue_eq_zero (o : Ordering) : orderValue o = 0 ↔ o = .eq := by
  cases o <;> decide
@[simp] theorem zero_lt_orderValue (o : Ordering) : 0 < orderValue o ↔ o = .gt := by
  cases o <;> decide
@[simp] theorem orderValue_le_zero (o : Ordering) : orderValue o ≤ 0 ↔ o ≠ .gt := by
  cases o <;> decide
@[simp] theorem zero_le_orderValue (o : Ordering) : 0 ≤ orderValue o ↔ o ≠ .lt := by
  cases o <;> decide
@[simp] theorem orderValue_eq_neg_one (o : Ordering) : orderValue o = -1 ↔ o = .lt := by
  cases o <;> decide
@[simp] theorem orderValue_eq_one (o : Ordering) : orderValue o = 1 ↔ o = .gt := by
  cases o <;> decide

@[simp] theorem RuntimeValue.order_integer (rank : StructHandle → String → Nat) (a b : Int) :
    RuntimeValue.order rank (.integer a) (.integer b) = compare a b := by
  simp [RuntimeValue.order, RuntimeValue.orderPayload, RuntimeValue.kindRank]

@[simp] theorem RuntimeValue.order_bool (rank : StructHandle → String → Nat) (a b : Bool) :
    RuntimeValue.order rank (.bool a) (.bool b) = compare a b := by
  simp [RuntimeValue.order, RuntimeValue.orderPayload, RuntimeValue.kindRank]

/-! ## Instances -/

instance RuntimeValue.order_oriented (rank : StructHandle → String → Nat) :
    Std.OrientedCmp (RuntimeValue.order rank) :=
  ⟨fun {a b} => RuntimeValue.order_swap rank a b⟩

instance RuntimeValue.order_trans (rank : StructHandle → String → Nat) :
    Std.TransCmp (RuntimeValue.order rank) :=
  { isLE_trans := fun {a b c} => RuntimeValue.order_isLE_trans rank a b c }

instance RuntimeValue.order_lawfulEq (rank : StructHandle → String → Nat) :
    Std.LawfulEqCmp (RuntimeValue.order rank) where
  compare_self := fun {a} => eq_of_self_swap (RuntimeValue.order_swap rank a a)
  eq_of_compare := RuntimeValue.eq_of_order


/-- The structural order is reflexive. -/
@[simp] theorem RuntimeValue.order_self (rank : StructHandle → String → Nat) (a : RuntimeValue) :
    RuntimeValue.order rank a a = .eq :=
  eq_of_self_swap (RuntimeValue.order_swap rank a a)

end LeanerIR
