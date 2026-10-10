-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.Runtime

/-!
# Laws of the structural order

`RuntimeValue.order` is a total order for every assignment of variant
and function positions: it is oriented (swapping the operands swaps the result), a tie
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

theorem compareVariant_swap (rank : ValueRanks) (ls rs : StructHandle)
    (lv rv : Option String) :
    compareVariant rank ls rs lv rv = (compareVariant rank rs ls rv lv).swap := by
  cases lv <;> cases rv <;> simp only [compareVariant] <;>
    first
    | rfl
    | rw [Ordering.swap_then, ← compare_swap, ← compare_swap]

theorem compareInstantiation_swap (a b : Array (TypeId × TypeId)) :
    compareInstantiation a b = (compareInstantiation b a).swap := by
  unfold compareInstantiation
  exact compare_swap _ _

theorem instantiationKey_injective {a b : Array (TypeId × TypeId)}
    (h : instantiationKey a = instantiationKey b) : a = b := by
  obtain ⟨a⟩ := a
  obtain ⟨b⟩ := b
  simp only [instantiationKey] at h
  congr
  induction a generalizing b with
  | nil => cases b with
    | nil => rfl
    | cons _ _ => simp at h
  | cons head rest ih => cases b with
    | nil => simp at h
    | cons head' rest' =>
        obtain ⟨⟨source⟩, ⟨target⟩⟩ := head
        obtain ⟨⟨source'⟩, ⟨target'⟩⟩ := head'
        simp only [List.flatMap_cons, List.cons_append, List.nil_append, List.cons.injEq] at h
        obtain ⟨rfl, rfl, h⟩ := h
        rw [ih _ h]

theorem eq_of_compareInstantiation {a b : Array (TypeId × TypeId)}
    (h : compareInstantiation a b = .eq) : a = b :=
  instantiationKey_injective (Std.LawfulEqCmp.eq_of_compare h)

theorem compareInstantiation_isLE_trans {a b c : Array (TypeId × TypeId)}
    (left : (compareInstantiation a b).isLE) (right : (compareInstantiation b c).isLE) :
    (compareInstantiation a c).isLE :=
  Std.TransCmp.isLE_trans (cmp := (compare : List Nat → List Nat → Ordering)) left right

mutual
theorem RuntimeValue.order_swap (rank : ValueRanks) (a b : RuntimeValue) :
    RuntimeValue.order rank a b = (RuntimeValue.order rank b a).swap := by
  rw [RuntimeValue.order, RuntimeValue.order, Ordering.swap_then,
    RuntimeValue.orderPayload_swap rank a b, ← compare_swap]
termination_by sizeOf a + sizeOf b + 1

theorem RuntimeValue.orderPayload_swap (rank : ValueRanks) (a b : RuntimeValue) :
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
    | (simp only [Ordering.swap_then, ← compare_swap, ← compareInstantiation_swap]
       rw [← RuntimeValue.orderList_swap])
termination_by sizeOf a + sizeOf b
decreasing_by all_goals (simp_wf; (try simp only [sizeOf_array]); omega)

theorem RuntimeValue.orderList_swap (rank : ValueRanks) (as bs : List RuntimeValue) :
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

theorem eq_of_compareVariant {rank : ValueRanks} {ls rs : StructHandle}
    {lv rv : Option String} (h : compareVariant rank ls rs lv rv = .eq) : lv = rv := by
  cases lv <;> cases rv <;> simp only [compareVariant, reduceCtorEq] at h
  · rfl
  · rw [Ordering.then_eq_eq] at h
    rw [eq_of_compare h.2]


mutual
theorem RuntimeValue.eq_of_order {rank : ValueRanks} {a b : RuntimeValue}
    (h : RuntimeValue.order rank a b = .eq) : a = b := by
  rw [RuntimeValue.order, Ordering.then_eq_eq] at h
  exact RuntimeValue.eq_of_orderPayload a b (eq_of_compare h.1) h.2
termination_by sizeOf a + sizeOf b + 1

theorem RuntimeValue.eq_of_orderPayload {rank : ValueRanks} (a b : RuntimeValue) :
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
  case closure.closure leftFunction leftMask leftInstantiation leftCaptures rightFunction
      rightMask rightInstantiation rightCaptures =>
    obtain ⟨⟨_, namespaces, functions⟩, instantiations, masks, captures⟩ := h
    obtain ⟨⟨leftNamespace⟩, ⟨leftFunction⟩⟩ := leftFunction
    obtain ⟨⟨rightNamespace⟩, ⟨rightFunction⟩⟩ := rightFunction
    have namespaceEq := eq_of_compare namespaces
    have functionEq := eq_of_compare functions
    simp only at namespaceEq functionEq
    subst namespaceEq functionEq
    rw [eq_of_compareInstantiation instantiations, eq_of_compare masks,
      Array.toList_inj.mp (RuntimeValue.eq_of_orderList captures)]
  case borrow.borrow leftLoan leftCurrent rightLoan rightCurrent =>
    rw [eq_of_compare h.1, RuntimeValue.eq_of_order h.2]
  case loanHole.loanHole => rw [eq_of_compare h]
termination_by sizeOf a + sizeOf b
decreasing_by all_goals (simp_wf; (try simp only [sizeOf_array]); omega)

theorem RuntimeValue.eq_of_orderList {rank : ValueRanks} {as bs : List RuntimeValue}
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

theorem compareVariant_isLE_trans {rank : ValueRanks} {ls ms rs : StructHandle}
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
theorem RuntimeValue.order_isLE_trans (rank : ValueRanks) (a b c : RuntimeValue) :
    (RuntimeValue.order rank a b).isLE → (RuntimeValue.order rank b c).isLE →
      (RuntimeValue.order rank a c).isLE := by
  intro left right
  rw [RuntimeValue.order] at left right ⊢
  exact lex_isLE_trans_of (fun kab kbc =>
    RuntimeValue.orderPayload_isLE_trans rank a b c (eq_of_compare kab) (eq_of_compare kbc)) left right
termination_by 2 * sizeOf a + 1
decreasing_by all_goals (simp_wf <;> omega)

theorem RuntimeValue.orderPayload_isLE_trans (rank : ValueRanks)
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
    simp only [Ordering.then_assoc] at left right ⊢
    refine lex_isLE_trans_of (fun _ _ left right => ?_) left right
    refine lex_isLE_trans_of (fun _ _ left right => ?_) left right
    refine lex_isLE_trans_of (fun _ _ left right => ?_) left right
    refine lex_isLE_trans_laws compareInstantiation compareInstantiation_swap
      eq_of_compareInstantiation compareInstantiation_isLE_trans
      (fun _ _ left right => ?_) left right
    exact lex_isLE_trans_of
      (fun _ _ left right => RuntimeValue.orderList_isLE_trans rank _ _ _ left right) left right
  case borrow.borrow.borrow =>
    exact lex_isLE_trans_of
      (fun _ _ left right => RuntimeValue.order_isLE_trans rank _ _ _ left right) left right
  case loanHole.loanHole.loanHole => exact Std.TransCmp.isLE_trans left right
termination_by 2 * sizeOf a
decreasing_by all_goals (simp_wf; (try simp only [sizeOf_array]); omega)

theorem RuntimeValue.orderList_isLE_trans (rank : ValueRanks)
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

@[simp] theorem RuntimeValue.order_integer (rank : ValueRanks) (a b : Int) :
    RuntimeValue.order rank (.integer a) (.integer b) = compare a b := by
  simp [RuntimeValue.order, RuntimeValue.orderPayload, RuntimeValue.kindRank]

@[simp] theorem RuntimeValue.order_bool (rank : ValueRanks) (a b : Bool) :
    RuntimeValue.order rank (.bool a) (.bool b) = compare a b := by
  simp [RuntimeValue.order, RuntimeValue.orderPayload, RuntimeValue.kindRank]

@[simp] theorem RuntimeValue.order_vector (rank : ValueRanks) (a b : Array RuntimeValue) :
    RuntimeValue.order rank (.vector a) (.vector b) =
      RuntimeValue.orderList rank a.toList b.toList := by
  simp [RuntimeValue.order, RuntimeValue.orderPayload, RuntimeValue.kindRank]

@[simp] theorem RuntimeValue.order_nominal (rank : ValueRanks) (left right : StructHandle)
    (leftVariant rightVariant : Option String) (leftFields rightFields : Array RuntimeValue) :
    RuntimeValue.order rank (.nominal left leftVariant leftFields)
        (.nominal right rightVariant rightFields) =
      ((compare left.namespaceId.index right.namespaceId.index).then
        (compare left.structId right.structId)).then
      ((compareVariant rank left right leftVariant rightVariant).then
        (RuntimeValue.orderList rank leftFields.toList rightFields.toList)) := by
  simp [RuntimeValue.order, RuntimeValue.orderPayload, RuntimeValue.kindRank]

@[simp] theorem RuntimeValue.orderList_nil_nil (rank : ValueRanks) :
    RuntimeValue.orderList rank [] [] = .eq := by
  simp [RuntimeValue.orderList]

@[simp] theorem RuntimeValue.orderList_nil_cons (rank : ValueRanks) (right : RuntimeValue)
    (rights : List RuntimeValue) : RuntimeValue.orderList rank [] (right :: rights) = .lt := by
  simp [RuntimeValue.orderList]

@[simp] theorem RuntimeValue.orderList_cons_nil (rank : ValueRanks) (left : RuntimeValue)
    (lefts : List RuntimeValue) : RuntimeValue.orderList rank (left :: lefts) [] = .gt := by
  simp [RuntimeValue.orderList]

@[simp] theorem RuntimeValue.orderList_cons_cons (rank : ValueRanks) (left right : RuntimeValue)
    (lefts rights : List RuntimeValue) :
    RuntimeValue.orderList rank (left :: lefts) (right :: rights) =
      (RuntimeValue.order rank left right).then (RuntimeValue.orderList rank lefts rights) := by
  simp [RuntimeValue.orderList]

/-! ## Instances -/

instance RuntimeValue.order_oriented (rank : ValueRanks) :
    Std.OrientedCmp (RuntimeValue.order rank) :=
  ⟨fun {a b} => RuntimeValue.order_swap rank a b⟩

instance RuntimeValue.order_trans (rank : ValueRanks) :
    Std.TransCmp (RuntimeValue.order rank) :=
  { isLE_trans := fun {a b c} => RuntimeValue.order_isLE_trans rank a b c }

instance RuntimeValue.order_lawfulEq (rank : ValueRanks) :
    Std.LawfulEqCmp (RuntimeValue.order rank) where
  compare_self := fun {a} => eq_of_self_swap (RuntimeValue.order_swap rank a a)
  eq_of_compare := RuntimeValue.eq_of_order


/-- The structural order is reflexive. -/
@[simp] theorem RuntimeValue.order_self (rank : ValueRanks) (a : RuntimeValue) :
    RuntimeValue.order rank a a = .eq :=
  eq_of_self_swap (RuntimeValue.order_swap rank a a)

end LeanerIR
