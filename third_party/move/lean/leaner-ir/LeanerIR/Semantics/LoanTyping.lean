-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.StateTyping

/-!
# Loan operations preserve typing

The loan operations rewrite one node of a value: a hole is filled, a borrow
is retired to the unit, or a borrow's current is replaced. Each keeps every
type the value has (`Subsumes`), whatever position the node holds, so a
write-back through a cached location needs no place typing.
-/

namespace LeanerIR

open Validation
open SemanticOperations

variable {inert : Nat}

/-! ## Shared references -/

theorem HasType.toUnshared {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {type : SemTy} (typed : HasType unit loans value type) :
    HasType unit loans value type.unshared := by
  induction type using SemTy.unshared.induct with
  | case1 referent ih =>
      simp only [SemTy.unshared]
      exact ih typed.shared_referent
  | case2 type not_shared =>
      rw [SemTy.unshared.eq_2 _ not_shared]
      exact typed

theorem HasType.ofUnshared {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {type : SemTy} (typed : HasType unit loans value type.unshared) :
    HasType unit loans value type := by
  induction type using SemTy.unshared.induct with
  | case1 referent ih =>
      simp only [SemTy.unshared] at typed
      exact .shared _ _ (ih typed)
  | case2 type not_shared =>
      rw [SemTy.unshared.eq_2 _ not_shared] at typed
      exact typed

/-- A value has every type that differs from one of its types only in outer
shared references. -/
theorem HasType.transport {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {type type' : SemTy} (typed : HasType unit loans value type)
    (same : type.unshared = type'.unshared) : HasType unit loans value type' :=
  HasType.ofUnshared (same ▸ typed.toUnshared)

/-- A hole has the types of its loan's referent. -/
theorem HasType.hole_referent {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat} :
    ∀ {type : SemTy}, HasType unit loans (.loanHole loan) type →
      ∃ referent, loans loan = some referent ∧ referent.unshared = type.unshared
  | _, .hole _ referent _ loan_eq same _ => ⟨referent, loan_eq, same⟩
  | _, .shared _ _ typed => by
      obtain ⟨referent, loan_eq, same⟩ := HasType.hole_referent typed
      exact ⟨referent, loan_eq, by simpa [SemTy.unshared] using same⟩

/-- A borrow's current has its loan's referent type. -/
theorem HasType.borrow_current {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {current : RuntimeValue} :
    ∀ {type : SemTy}, HasType unit loans (.borrow loan current) type →
      ∃ referent, loans loan = some referent ∧ HasType unit loans current referent
  | _, .borrow _ _ referent loan_eq current_typed => ⟨referent, loan_eq, current_typed⟩
  | _, .shared _ _ typed => HasType.borrow_current typed

/-! ## Subsumption -/

/-- `replacement` has every type `value` has. -/
def Subsumes (unit : ValidatedUnit) (loans : LoanTypes) (replacement value : RuntimeValue) :
    Prop :=
  ∀ type, HasType unit loans value type → HasType unit loans replacement type

theorem Subsumes.refl {unit : ValidatedUnit} {loans : LoanTypes} (value : RuntimeValue) :
    Subsumes unit loans value value := fun _ typed => typed

/-- A filled hole: the replacement has its loan's referent type. -/
theorem Subsumes.fill {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {referent : SemTy} {replacement : RuntimeValue}
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans replacement referent) :
    Subsumes unit loans replacement (.loanHole loan) := fun _ hole => by
  obtain ⟨referent', loan_eq', same⟩ := hole.hole_referent
  rw [loan_eq, Option.some.injEq] at loan_eq'
  subst loan_eq'
  exact typed.transport same

/-- A retired borrow: the unit a settled borrow leaves is a dead reference. -/
theorem Subsumes.clear {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {current : RuntimeValue} : Subsumes unit loans .unit (.borrow loan current) := by
  intro type typed
  exact go typed
where
  go : ∀ {type : SemTy}, HasType unit loans (.borrow loan current) type →
      HasType unit loans .unit type
    | _, .borrow _ _ referent _ _ => .dead referent
    | _, .shared _ _ typed => .shared _ _ (go typed)

/-- A borrow whose current is replaced by a value of its loan's referent. -/
theorem Subsumes.rewrite {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {current replacement : RuntimeValue} {referent : SemTy}
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans replacement referent) :
    Subsumes unit loans (.borrow loan replacement) (.borrow loan current) := by
  intro type borrowed
  exact go borrowed
where
  go : ∀ {type : SemTy}, HasType unit loans (.borrow loan current) type →
      HasType unit loans (.borrow loan replacement) type
    | _, .borrow _ _ referent' loan_eq' _ => by
        rw [loan_eq, Option.some.injEq] at loan_eq'
        subst loan_eq'
        exact .borrow _ _ _ loan_eq typed
    | _, .shared _ _ typed => .shared _ _ (go typed)

theorem rewriteFirstList_length {f : RuntimeValue → Option RuntimeValue} :
    ∀ {values rewritten : List RuntimeValue}, rewriteFirstList f values = some rewritten →
      rewritten.length = values.length
  | [], _, rewrite => by simp [rewriteFirstList] at rewrite
  | value :: rest, rewritten, rewrite => by
      rw [rewriteFirstList] at rewrite
      split at rewrite
      · simp only [Option.some.injEq] at rewrite
        subst rewrite
        rfl
      · simp only [Option.map_eq_some_iff] at rewrite
        obtain ⟨rest', rest'_eq, rfl⟩ := rewrite
        simp [rewriteFirstList_length rest'_eq]

/-- The rewrite inside a value's children, which `rewriteFirst` tries when
the value's own node does not match. -/
def rewriteChildren (f : RuntimeValue → Option RuntimeValue) : RuntimeValue → Option RuntimeValue
  | .vector elements => (rewriteFirstList f elements.toList).map fun rewritten =>
      .vector rewritten.toArray
  | .tuple elements => (rewriteFirstList f elements.toList).map fun rewritten =>
      .tuple rewritten.toArray
  | .nominal name variant fields => (rewriteFirstList f fields.toList).map fun rewritten =>
      .nominal name variant rewritten.toArray
  | .closure function mask instantiation captures =>
      (rewriteFirstList f captures.toList).map fun rewritten =>
        .closure function mask instantiation rewritten.toArray
  | .borrow loanInstance current => (rewriteFirst f current).map (.borrow loanInstance)
  | _ => none

theorem rewriteFirst_eq (f : RuntimeValue → Option RuntimeValue) (value : RuntimeValue) :
    rewriteFirst f value = (f value).or (rewriteChildren f value) := by
  unfold rewriteFirst
  cases f value <;> cases value <;> rfl

section Rewrite

variable {unit : ValidatedUnit} {loans : LoanTypes} {f : RuntimeValue → Option RuntimeValue}
  (node : ∀ value replacement, f value = some replacement → Subsumes unit loans replacement value)

include node in
/-- A rewrite at a value's own node or inside its children. -/
private theorem rewriteFirst_via {value replacement : RuntimeValue} {type : SemTy}
    (typed : HasType unit loans value type)
    (rewrite : rewriteFirst f value = some replacement)
    (inside : rewriteChildren f value = some replacement → HasType unit loans replacement type) :
    HasType unit loans replacement type := by
  rw [rewriteFirst_eq] at rewrite
  cases matched : f value with
  | some rewritten =>
      rw [matched, Option.some_or, Option.some.injEq] at rewrite
      subst rewrite
      exact node value rewritten matched type typed
  | none =>
      rw [matched, Option.none_or] at rewrite
      exact inside rewrite

include node in
mutual
/-- Rewriting inside the children of a value with a node-subsuming `f`
keeps every type of the value. -/
theorem rewriteChildren_typed :
    ∀ {value replacement : RuntimeValue} {type : SemTy}, HasType unit loans value type →
      rewriteChildren f value = some replacement → HasType unit loans replacement type
  | _, _, _, .tuple _ _ elements, inside => by
      simp only [rewriteChildren, Option.map_eq_some_iff] at inside
      obtain ⟨rewritten, rewritten_eq, rfl⟩ := inside
      exact .tuple _ _ (by simpa using rewriteFirstList_types elements rewritten_eq)
  | _, _, _, .vector _ _ _ length_matches elements, inside => by
      simp only [rewriteChildren, Option.map_eq_some_iff] at inside
      obtain ⟨rewritten, rewritten_eq, rfl⟩ := inside
      refine .vector _ _ _ ?_ (by simpa using rewriteFirstList_each elements rewritten_eq)
      simpa [rewriteFirstList_length rewritten_eq] using length_matches
  | _, _, _, .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq
      resolved fields_typed, inside => by
      simp only [rewriteChildren, Option.map_eq_some_iff] at inside
      obtain ⟨rewritten, rewritten_eq, rfl⟩ := inside
      exact .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq
        resolved (by simpa using rewriteFirstList_types fields_typed rewritten_eq)
  | _, _, _, .closure _ _ _ _ _ _ targetNs declaration arguments allParameters results
      namespace_eq declaration_eq signature_eq faithful mask_bound captures_typed
      parameters_eq packs, inside => by
      simp only [rewriteChildren, Option.map_eq_some_iff] at inside
      obtain ⟨rewritten, rewritten_eq, rfl⟩ := inside
      exact .closure _ _ _ _ _ _ targetNs declaration arguments allParameters results
        namespace_eq declaration_eq signature_eq faithful mask_bound
        (by simpa using rewriteFirstList_types captures_typed rewritten_eq) parameters_eq packs
  | _, _, _, .borrow _ _ _ loan_eq current_typed, inside => by
      simp only [rewriteChildren, Option.map_eq_some_iff] at inside
      obtain ⟨rewritten, rewritten_eq, rfl⟩ := inside
      exact .borrow _ _ _ loan_eq (rewriteFirst_via node current_typed rewritten_eq
        (rewriteChildren_typed current_typed))
  | _, _, _, .shared _ _ inner, inside => .shared _ _ (rewriteChildren_typed inner inside)
  | _, _, _, .unit, inside | _, _, _, .bool _, inside | _, _, _, .character _ _, inside
  | _, _, _, .string _, inside | _, _, _, .bytes _, inside | _, _, _, .integer _ _ _ _, inside
  | _, _, _, .address _, inside | _, _, _, .signer _, inside | _, _, _, .dead _, inside
  | _, _, _, .hole _ _ _ _ _ _, inside | _, _, _, .unitEmptyTuple, inside => by
      simp [rewriteChildren] at inside
  | _, _, _, .emptyTupleUnit, inside => by
      simp [rewriteChildren, rewriteFirstList] at inside

theorem rewriteFirstList_types :
    ∀ {values rewritten : List RuntimeValue} {types : List SemTy},
      HasTypes unit loans values types → rewriteFirstList f values = some rewritten →
      HasTypes unit loans rewritten types
  | [], _, _, .nil, rewrite => by simp [rewriteFirstList] at rewrite
  | _ :: _, _, _ :: _, .cons head tail, rewrite => by
      rw [rewriteFirstList] at rewrite
      split at rewrite
      · rename_i first first_eq
        simp only [Option.some.injEq] at rewrite
        subst rewrite
        exact .cons (rewriteFirst_via node head first_eq (rewriteChildren_typed head)) tail
      · simp only [Option.map_eq_some_iff] at rewrite
        obtain ⟨rest', rest'_eq, rfl⟩ := rewrite
        exact .cons head (rewriteFirstList_types tail rest'_eq)

theorem rewriteFirstList_each :
    ∀ {values rewritten : List RuntimeValue} {type : SemTy},
      HasTypeEach unit loans values type → rewriteFirstList f values = some rewritten →
      HasTypeEach unit loans rewritten type
  | [], _, _, .nil, rewrite => by simp [rewriteFirstList] at rewrite
  | _ :: _, _, _, .cons head tail, rewrite => by
      rw [rewriteFirstList] at rewrite
      split at rewrite
      · rename_i first first_eq
        simp only [Option.some.injEq] at rewrite
        subst rewrite
        exact .cons (rewriteFirst_via node head first_eq (rewriteChildren_typed head)) tail
      · simp only [Option.map_eq_some_iff] at rewrite
        obtain ⟨rest', rest'_eq, rfl⟩ := rewrite
        exact .cons head (rewriteFirstList_each tail rest'_eq)
end

include node in
/-- Rewriting the first node a node-subsuming `f` matches keeps every type
of the value. -/
theorem rewriteFirst_subsumes {value replacement : RuntimeValue}
    (rewrite : rewriteFirst f value = some replacement) :
    Subsumes unit loans replacement value := fun _ typed =>
  rewriteFirst_via node typed rewrite (rewriteChildren_typed node typed)

end Rewrite

/-! ## Queries -/

/-- The query inside a value's children, which `findFirst` tries when the
value's own node does not answer. -/
def findChildren {α : Type} (f : RuntimeValue → Option α) : RuntimeValue → Option α
  | .vector elements | .tuple elements => findFirstList f elements.toList
  | .nominal _ _ fields => findFirstList f fields.toList
  | .closure _ _ _ captures => findFirstList f captures.toList
  | .borrow _ current => findFirst f current
  | _ => none

theorem findFirst_eq {α : Type} (f : RuntimeValue → Option α) (value : RuntimeValue) :
    findFirst f value = (f value).or (findChildren f value) := by
  unfold findFirst
  cases f value <;> cases value <;> rfl

/-- The collection inside a value's children, which `collectPruned` takes
when the value's own node does not answer. -/
def collectChildren {α : Type} (f : RuntimeValue → Option α) : RuntimeValue → Array α
  | .vector elements | .tuple elements => collectPrunedList f elements.toList
  | .nominal _ _ fields => collectPrunedList f fields.toList
  | .closure _ _ _ captures => collectPrunedList f captures.toList
  | .borrow _ current => collectPruned f current
  | _ => #[]

theorem collectPruned_eq {α : Type} (f : RuntimeValue → Option α) (value : RuntimeValue) :
    collectPruned f value = match f value with
      | some found => #[found]
      | none => collectChildren f value := by
  unfold collectPruned
  cases f value <;> cases value <;> rfl

section Query

variable {unit : ValidatedUnit} {loans : LoanTypes} {α : Type} {f : RuntimeValue → Option α}
  {P : α → Prop}
  (node : ∀ value type found, HasType unit loans value type → f value = some found → P found)

include node in
private theorem findFirst_via {value : RuntimeValue} {type : SemTy} {found : α}
    (typed : HasType unit loans value type) (query : findFirst f value = some found)
    (inside : findChildren f value = some found → P found) : P found := by
  rw [findFirst_eq] at query
  cases matched : f value with
  | some answer =>
      rw [matched, Option.some_or, Option.some.injEq] at query
      subst query
      exact node value type answer typed matched
  | none =>
      rw [matched, Option.none_or] at query
      exact inside query

include node in
private theorem collectPruned_via {value : RuntimeValue} {type : SemTy} {found : α}
    (typed : HasType unit loans value type) (member : found ∈ collectPruned f value)
    (inside : found ∈ collectChildren f value → P found) : P found := by
  rw [collectPruned_eq] at member
  cases matched : f value with
  | some answer =>
      rw [matched] at member
      simp only [Array.mem_singleton] at member
      subst member
      exact node value type _ typed matched
  | none =>
      rw [matched] at member
      exact inside member

include node in
mutual
/-- What a query finds inside a typed value satisfies what it finds at any
typed node. -/
theorem findChildren_typed :
    ∀ {value : RuntimeValue} {type : SemTy} {found : α}, HasType unit loans value type →
      findChildren f value = some found → P found
  | _, _, _, .tuple _ _ elements, query => findFirstList_types elements query
  | _, _, _, .vector _ _ _ _ elements, query => findFirstList_each elements query
  | _, _, _, .nominal _ _ _ _ _ _ _ _ _ _ _ fields_typed, query =>
      findFirstList_types fields_typed query
  | _, _, _, .closure _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ captures_typed _ _, query =>
      findFirstList_types captures_typed query
  | _, _, _, .borrow _ _ _ _ current_typed, query =>
      findFirst_via node current_typed query (findChildren_typed current_typed)
  | _, _, _, .shared _ _ inner, query => findChildren_typed inner query
  | _, _, _, .unit, query | _, _, _, .bool _, query | _, _, _, .character _ _, query
  | _, _, _, .string _, query | _, _, _, .bytes _, query | _, _, _, .integer _ _ _ _, query
  | _, _, _, .address _, query | _, _, _, .signer _, query | _, _, _, .dead _, query
  | _, _, _, .hole _ _ _ _ _ _, query | _, _, _, .unitEmptyTuple, query => by
      simp [findChildren] at query
  | _, _, _, .emptyTupleUnit, query => by simp [findChildren, findFirstList] at query

theorem findFirstList_types :
    ∀ {values : List RuntimeValue} {types : List SemTy} {found : α},
      HasTypes unit loans values types → findFirstList f values = some found → P found
  | [], _, _, .nil, query => by simp [findFirstList] at query
  | _ :: _, _ :: _, _, .cons head tail, query => by
      rw [findFirstList] at query
      split at query
      · rename_i answer answer_eq
        simp only [Option.some.injEq] at query
        subst query
        exact findFirst_via node head answer_eq (findChildren_typed head)
      · exact findFirstList_types tail query

theorem findFirstList_each :
    ∀ {values : List RuntimeValue} {type : SemTy} {found : α},
      HasTypeEach unit loans values type → findFirstList f values = some found → P found
  | [], _, _, .nil, query => by simp [findFirstList] at query
  | _ :: _, _, _, .cons head tail, query => by
      rw [findFirstList] at query
      split at query
      · rename_i answer answer_eq
        simp only [Option.some.injEq] at query
        subst query
        exact findFirst_via node head answer_eq (findChildren_typed head)
      · exact findFirstList_each tail query
end

include node in
theorem findFirst_typed {value : RuntimeValue} {type : SemTy} {found : α}
    (typed : HasType unit loans value type) (query : findFirst f value = some found) : P found :=
  findFirst_via node typed query (findChildren_typed node typed)

include node in
mutual
/-- What a collection gathers inside a typed value satisfies what it
gathers at any typed node. -/
theorem collectChildren_typed :
    ∀ {value : RuntimeValue} {type : SemTy} {found : α}, HasType unit loans value type →
      found ∈ collectChildren f value → P found
  | _, _, _, .tuple _ _ elements, member => collectPrunedList_types elements member
  | _, _, _, .vector _ _ _ _ elements, member => collectPrunedList_each elements member
  | _, _, _, .nominal _ _ _ _ _ _ _ _ _ _ _ fields_typed, member =>
      collectPrunedList_types fields_typed member
  | _, _, _, .closure _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ _ captures_typed _ _, member =>
      collectPrunedList_types captures_typed member
  | _, _, _, .borrow _ _ _ _ current_typed, member =>
      collectPruned_via node current_typed member (collectChildren_typed current_typed)
  | _, _, _, .shared _ _ inner, member => collectChildren_typed inner member
  | _, _, _, .unit, member | _, _, _, .bool _, member | _, _, _, .character _ _, member
  | _, _, _, .string _, member | _, _, _, .bytes _, member | _, _, _, .integer _ _ _ _, member
  | _, _, _, .address _, member | _, _, _, .signer _, member | _, _, _, .dead _, member
  | _, _, _, .hole _ _ _ _ _ _, member | _, _, _, .unitEmptyTuple, member => by
      simp [collectChildren] at member
  | _, _, _, .emptyTupleUnit, member => by simp [collectChildren, collectPrunedList] at member

theorem collectPrunedList_types :
    ∀ {values : List RuntimeValue} {types : List SemTy} {found : α},
      HasTypes unit loans values types → found ∈ collectPrunedList f values → P found
  | [], _, _, .nil, member => by simp [collectPrunedList] at member
  | _ :: _, _ :: _, _, .cons head tail, member => by
      rw [collectPrunedList, Array.mem_append] at member
      rcases member with member | member
      · exact collectPruned_via node head member (collectChildren_typed head)
      · exact collectPrunedList_types tail member

theorem collectPrunedList_each :
    ∀ {values : List RuntimeValue} {type : SemTy} {found : α},
      HasTypeEach unit loans values type → found ∈ collectPrunedList f values → P found
  | [], _, _, .nil, member => by simp [collectPrunedList] at member
  | _ :: _, _, _, .cons head tail, member => by
      rw [collectPrunedList, Array.mem_append] at member
      rcases member with member | member
      · exact collectPruned_via node head member (collectChildren_typed head)
      · exact collectPrunedList_each tail member
end

include node in
theorem collectPruned_typed {value : RuntimeValue} {type : SemTy} {found : α}
    (typed : HasType unit loans value type) (member : found ∈ collectPruned f value) : P found :=
  collectPruned_via node typed member (collectChildren_typed node typed)

end Query

/-! ## The loan nodes -/

/-- Filling a loan's hole with a value of its referent keeps every type. -/
theorem fillHole?_subsumes {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {referent : SemTy} {replacement value filled : RuntimeValue}
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans replacement referent)
    (fill : fillHole? loan replacement value = some filled) :
    Subsumes unit loans filled value := by
  refine rewriteFirst_subsumes (fun node node' matched => ?_) fill
  cases node <;> simp only [holeFill?, reduceCtorEq] at matched
  split at matched
  · rename_i same
    simp only [Option.some.injEq] at matched
    subst matched
    rw [beq_iff_eq.mp same]
    exact Subsumes.fill loan_eq typed
  · cases matched

/-- Retiring a loan's borrow to the unit keeps every type. -/
theorem borrowClear?_subsumes {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {value cleared : RuntimeValue}
    (clear : rewriteFirst (borrowClear? loan) value = some cleared) :
    Subsumes unit loans cleared value := by
  refine rewriteFirst_subsumes (fun node node' matched => ?_) clear
  cases node <;> simp only [borrowClear?, reduceCtorEq] at matched
  split at matched
  · simp only [Option.some.injEq] at matched
    subst matched
    exact Subsumes.clear
  · cases matched

/-- Replacing a loan's borrow's current by a value of its referent keeps
every type. -/
theorem borrowRewrite?_subsumes {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {referent : SemTy} {replacement value rewritten : RuntimeValue}
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans replacement referent)
    (rewrite : rewriteFirst (borrowRewrite? loan replacement) value = some rewritten) :
    Subsumes unit loans rewritten value := by
  refine rewriteFirst_subsumes (fun node node' matched => ?_) rewrite
  cases node <;> simp only [borrowRewrite?, reduceCtorEq] at matched
  split at matched
  · rename_i same
    simp only [Option.some.injEq] at matched
    subst matched
    rw [beq_iff_eq.mp same]
    exact Subsumes.rewrite loan_eq typed
  · cases matched

/-! ## Frames and states -/

/-- A frame's typing reads only its locals. -/
theorem TypedFrame.of_locals {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame frame' : RuntimeFrame} (typed : TypedFrame unit loans ns locals env frame)
    (locals_eq : frame'.locals = frame.locals) :
    TypedFrame unit loans ns locals env frame' := by
  unfold TypedFrame
  rw [locals_eq]
  exact typed

/-- Replacing a local's value by one with every type it has keeps the
frame typed. -/
theorem TypedFrame.subsume {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {index : Nat} {value value' : RuntimeValue}
    (typed : TypedFrame unit loans ns locals env frame)
    (slot_eq : frame.locals[index]? = some (some value))
    (subsumes : Subsumes unit loans value' value) :
    TypedFrame unit loans ns locals env
      { frame with locals := frame.locals.set! index (some value') } := by
  obtain ⟨size_eq, locals_typed⟩ := typed
  refine ⟨by simp [size_eq], fun index' declaration stored declaration_eq stored_eq => ?_⟩
  simp only [Array.set!_eq_setIfInBounds, Array.getElem?_setIfInBounds] at stored_eq
  split at stored_eq
  · rename_i same
    subst same
    split at stored_eq
    · simp only [Option.some.injEq] at stored_eq
      subst stored_eq
      obtain ⟨type, resolves, value_typed⟩ :=
        locals_typed index declaration value declaration_eq slot_eq
      exact ⟨type, resolves, subsumes type value_typed⟩
    · cases stored_eq
  · exact locals_typed index' declaration stored declaration_eq stored_eq

theorem Array.mem_extract_of_le {α : Type} {xs : Array α} {i j n : Nat} {x : α}
    (member : x ∈ xs.extract i n) (le : j ≤ i) : x ∈ xs.extract j n := by
  rw [Array.mem_iff_getElem] at member ⊢
  obtain ⟨k, bound, rfl⟩ := member
  simp only [Array.size_extract] at bound
  refine ⟨k + (i - j), by simp only [Array.size_extract]; omega, ?_⟩
  simp only [Array.getElem_extract]
  congr 1
  omega

theorem Array.mem_extract_push {α : Type} {xs : Array α} {i : Nat} {x y : α}
    (member : y ∈ (xs.push x).extract i (xs.push x).size) (le : i ≤ xs.size) :
    y ∈ xs.extract i xs.size ∨ y = x := by
  rw [Array.mem_iff_getElem] at member
  obtain ⟨k, bound, rfl⟩ := member
  simp only [Array.size_extract, Array.size_push] at bound
  simp only [Array.getElem_extract, Array.getElem_push]
  split
  · left
    rw [Array.mem_iff_getElem]
    exact ⟨k, by simp only [Array.size_extract]; omega, by simp [Array.getElem_extract]⟩
  · right
    rfl

/-- A state's typing reads only its globals, pending write-backs, and
frontier. -/
theorem TypedState.of_eq {unit : ValidatedUnit} {loans : LoanTypes} {state state' : RuntimeState}
    (typed : TypedState unit loans inert state) (globals_eq : state'.globals = state.globals)
    (pending_eq : state'.pending = state.pending) (next_eq : state'.nextLoan = state.nextLoan) :
    TypedState unit loans inert state' where
  globals slot member := typed.globals slot (globals_eq ▸ member)
  inert_le := pending_eq ▸ typed.inert_le
  pending entry member := typed.pending entry (pending_eq ▸ member)
  bounded loan type loan_eq := next_eq ▸ typed.bounded loan type loan_eq

/-- Publishing a value with every type of the one a key holds keeps the
state typed. -/
theorem TypedState.subsume {unit : ValidatedUnit} {loans : LoanTypes} {state : RuntimeState}
    {key : GlobalKey} {value value' : RuntimeValue}
    (typed : TypedState unit loans inert state)
    (lookup_eq : state.globals.lookup key = some value)
    (subsumes : Subsumes unit loans value' value) :
    TypedState unit loans inert { state with globals := state.globals.insert key value' } where
  globals slot member := by
    rcases GlobalMap.mem_insert.mp member with ⟨member, -⟩ | rfl
    · exact typed.globals slot member
    · obtain ⟨ns, type, ns_eq, resolves, value_typed⟩ := typed.lookup lookup_eq
      exact ⟨ns, type, ns_eq, resolves, subsumes type value_typed⟩
  inert_le := typed.inert_le
  pending := typed.pending
  bounded := typed.bounded

/-- Replacing a slot's value by one with every type it has keeps the state
typed. -/
theorem TypedState.subsume_entry {unit : ValidatedUnit} {loans : LoanTypes}
    {state : RuntimeState} {index : Nat} {slot : GlobalSlot} {value' : RuntimeValue}
    (typed : TypedState unit loans inert state)
    (slot_eq : state.globals.entries[index]? = some slot)
    (subsumes : Subsumes unit loans value' slot.value) :
    TypedState unit loans inert
      { state with globals := ⟨state.globals.entries.set! index { slot with value := value' }⟩ } where
  globals slot' member := by
    simp only [Array.set!_eq_setIfInBounds] at member
    rcases Array.mem_or_eq_of_mem_setIfInBounds member with member | rfl
    · exact typed.globals slot' member
    · obtain ⟨ns, type, ns_eq, resolves, value_typed⟩ :=
        typed.globals slot (Array.mem_of_getElem? slot_eq)
      exact ⟨ns, type, ns_eq, resolves, subsumes type value_typed⟩
  inert_le := typed.inert_le
  pending := typed.pending
  bounded := typed.bounded

/-- Exporting a write-back of a loan's referent keeps the state typed. -/
theorem TypedState.push_pending {unit : ValidatedUnit} {loans : LoanTypes}
    {state : RuntimeState} {loan : Nat} {referent : SemTy} {value : RuntimeValue}
    (typed : TypedState unit loans inert state) (loan_eq : loans loan = some referent)
    (value_typed : HasType unit loans value referent) :
    TypedState unit loans inert { state with pending := state.pending.push (loan, value) } where
  globals := typed.globals
  inert_le := by simpa using Nat.le_succ_of_le typed.inert_le
  pending entry member := by
    rcases Array.mem_extract_push member typed.inert_le with member | rfl
    · exact typed.pending entry member
    · exact ⟨referent, loan_eq, value_typed⟩
  bounded := typed.bounded

/-! ## Writing a subsuming value back -/

private theorem HasTypes.set_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {values : List RuntimeValue} {types : List SemTy} {index : Nat} {part part' : RuntimeValue}
    (typed : HasTypes unit loans values types) (part_eq : values[index]? = some part)
    (subsumes : Subsumes unit loans part' part) :
    HasTypes unit loans (values.set index part') types := by
  obtain ⟨type, type_eq, part_typed⟩ := typed.get part_eq
  exact typed.set type_eq (subsumes type part_typed)

private theorem HasTypeEach.set_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {values : List RuntimeValue} {type : SemTy} {index : Nat} {part part' : RuntimeValue}
    (typed : HasTypeEach unit loans values type) (part_eq : values[index]? = some part)
    (subsumes : Subsumes unit loans part' part) :
    HasTypeEach unit loans (values.set index part') type := by
  refine HasTypeEach.of_forall fun value member => ?_
  rcases List.mem_or_eq_of_mem_set member with member | rfl
  · exact typed.mem value member
  · exact subsumes type (typed.mem part (List.mem_of_getElem? part_eq))

private theorem field_subsumes {unit : ValidatedUnit} {loans : LoanTypes} {source : StructHandle}
    {variant : Option String} {fields : Array RuntimeValue} {index : Nat}
    {part part' : RuntimeValue} (part_eq : fields[index]? = some part)
    (subsumes : Subsumes unit loans part' part) :
    Subsumes unit loans (.nominal source variant (fields.set! index part'))
      (.nominal source variant fields) := fun _ typed => go typed
where
  go : ∀ {type : SemTy}, HasType unit loans (.nominal source variant fields) type →
      HasType unit loans (.nominal source variant (fields.set! index part')) type
    | _, .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq resolved
        typed =>
        .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq resolved
          (by simpa [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds] using
            typed.set_subsumes (by simpa using part_eq) subsumes)
    | _, .shared _ _ inner => .shared _ _ (go inner)

private theorem vectorIndex_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {elements : Array RuntimeValue} {index : Nat} {part part' : RuntimeValue}
    (part_eq : elements[index]? = some part) (subsumes : Subsumes unit loans part' part) :
    Subsumes unit loans (.vector (elements.set! index part')) (.vector elements) :=
  fun _ typed => go typed
where
  go : ∀ {type : SemTy}, HasType unit loans (.vector elements) type →
      HasType unit loans (.vector (elements.set! index part')) type
    | _, .vector _ _ _ length_matches typed =>
        .vector _ _ _ (by simpa using length_matches)
          (by simpa [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds] using
            typed.set_subsumes (by simpa using part_eq) subsumes)
    | _, .shared _ _ inner => .shared _ _ (go inner)

private theorem tupleIndex_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {elements : Array RuntimeValue} {index : Nat} {part part' : RuntimeValue}
    (part_eq : elements[index]? = some part) (subsumes : Subsumes unit loans part' part) :
    Subsumes unit loans (.tuple (elements.set! index part')) (.tuple elements) :=
  fun _ typed => go rfl typed
where
  go : ∀ {values : Array RuntimeValue} {type : SemTy}, values = elements →
      HasType unit loans (.tuple values) type →
      HasType unit loans (.tuple (elements.set! index part')) type
    | _, _, rfl, .tuple _ _ typed =>
        .tuple _ _ (by simpa [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds] using
          typed.set_subsumes (by simpa using part_eq) subsumes)
    | _, _, same, .shared _ _ inner => .shared _ _ (go same inner)
    | _, _, same, .emptyTupleUnit => by subst same; simp at part_eq

private theorem deref_subsumes {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {current current' : RuntimeValue} (subsumes : Subsumes unit loans current' current) :
    Subsumes unit loans (.borrow loan current') (.borrow loan current) := fun _ typed => go typed
where
  go : ∀ {type : SemTy}, HasType unit loans (.borrow loan current) type →
      HasType unit loans (.borrow loan current') type
    | _, .borrow _ _ referent loan_eq typed => .borrow _ _ _ loan_eq (subsumes referent typed)
    | _, .shared _ _ inner => .shared _ _ (go inner)

private theorem subslice_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {elements inner : Array RuntimeValue} {first last : Nat}
    (ordered : first ≤ last) (bounded : last ≤ elements.size)
    (size_eq : inner.size = last - first)
    (subsumes : Subsumes unit loans (.vector inner) (.vector (elements.extract first last))) :
    Subsumes unit loans
      (.vector (elements.extract 0 first ++ inner ++ elements.extract last elements.size))
      (.vector elements) := fun _ typed => go typed
where
  go : ∀ {type : SemTy}, HasType unit loans (.vector elements) type →
      HasType unit loans
        (.vector (elements.extract 0 first ++ inner ++ elements.extract last elements.size)) type
    | _, .vector _ element _ length_matches typed => by
        have extract_typed : HasType unit loans (.vector (elements.extract first last))
            (.vector element none) :=
          .vector _ _ _ trivial (HasTypeEach.of_forall fun value member =>
            typed.mem value (by
              obtain ⟨_, _, rfl⟩ := Array.mem_extract_iff_getElem.mp
                (Array.mem_toList_iff.mp member)
              simp))
        have inner_typed := (subsumes _ extract_typed).vector_elements
        refine .vector _ _ _ ?_ (HasTypeEach.of_forall fun value member => ?_)
        · have total : (elements.extract 0 first ++ inner ++
              elements.extract last elements.size).size = elements.size := by
            simp only [Array.size_append, Array.size_extract, size_eq]
            omega
          rw [total]
          exact length_matches
        · simp only [Array.toList_append, List.mem_append] at member
          rcases member with (member | member) | member
          · obtain ⟨_, _, rfl⟩ := Array.mem_extract_iff_getElem.mp (Array.mem_toList_iff.mp member)
            exact typed.mem _ (by simp)
          · exact inner_typed.mem value member
          · obtain ⟨_, _, rfl⟩ := Array.mem_extract_iff_getElem.mp (Array.mem_toList_iff.mp member)
            exact typed.mem _ (by simp)
    | _, .shared _ _ inner => .shared _ _ (go inner)

/-- Writing a value with every type of the one a path reaches keeps every
type of the whole. -/
theorem writeProjections?_subsumes {unit : ValidatedUnit} {loans : LoanTypes} :
    ∀ (projections : List RuntimeProjection) {value part replacement updated : RuntimeValue},
      readProjections? value projections = some part →
      writeProjections? value projections replacement = some updated →
      Subsumes unit loans replacement part → Subsumes unit loans updated value
  | [], _, _, _, _, read, write, subsumes => by
      simp only [readProjections?, Option.some.injEq] at read
      simp only [writeProjections?, Option.some.injEq] at write
      subst read write
      exact subsumes
  | projection :: rest, value, part, replacement, updated, read, write, subsumes => by
      cases projection with
      | field index =>
          cases value with
          | nominal source variant fields =>
              simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
              obtain ⟨child, child_eq, read⟩ := read
              simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff,
                pure, Option.some.injEq] at write
              obtain ⟨child', child'_eq, written, written_eq, rfl⟩ := write
              rw [child_eq, Option.some.injEq] at child'_eq
              subst child'_eq
              exact field_subsumes child_eq
                (writeProjections?_subsumes rest read written_eq subsumes)
          | _ => simp [readProjections?] at read
      | index index =>
          cases value with
          | vector elements =>
              simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
              obtain ⟨child, child_eq, read⟩ := read
              simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff,
                pure, Option.some.injEq] at write
              obtain ⟨child', child'_eq, written, written_eq, rfl⟩ := write
              rw [child_eq, Option.some.injEq] at child'_eq
              subst child'_eq
              exact vectorIndex_subsumes child_eq
                (writeProjections?_subsumes rest read written_eq subsumes)
          | tuple elements =>
              simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
              obtain ⟨child, child_eq, read⟩ := read
              simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff,
                pure, Option.some.injEq] at write
              obtain ⟨child', child'_eq, written, written_eq, rfl⟩ := write
              rw [child_eq, Option.some.injEq] at child'_eq
              subst child'_eq
              exact tupleIndex_subsumes child_eq
                (writeProjections?_subsumes rest read written_eq subsumes)
          | _ => simp [readProjections?] at read
      | subslice start stop fromEnd =>
          cases value with
          | vector elements =>
              simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
              obtain ⟨⟨first, last⟩, bounds, read⟩ := read
              simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff,
                pure] at write
              obtain ⟨⟨first', last'⟩, bounds', written, written_eq, write⟩ := write
              rw [bounds, Option.some.injEq, Prod.mk.injEq] at bounds'
              obtain ⟨rfl, rfl⟩ := bounds'
              have inner_subsumes := writeProjections?_subsumes rest read written_eq subsumes
              split at write
              · rename_i inner
                split at write
                · cases write
                · rename_i same
                  simp only [Option.some.injEq] at write
                  subst write
                  simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
                  unfold subsliceBounds? at bounds
                  have ordered : first ≤ last ∧ last ≤ elements.size := by
                    split at bounds
                    · split at bounds
                      · simp only [Option.some.injEq, Prod.mk.injEq] at bounds
                        omega
                      · cases bounds
                    · split at bounds
                      · rename_i within
                        simp only [Bool.and_eq_true, decide_eq_true_eq] at within
                        simp only [Option.some.injEq, Prod.mk.injEq] at bounds
                        omega
                      · cases bounds
                  exact subslice_subsumes ordered.1 ordered.2 same inner_subsumes
              · cases write
          | _ => simp [readProjections?] at read
      | downcast variant =>
          cases value with
          | nominal source actual fields =>
              cases actual with
              | none => simp [readProjections?] at read
              | some actual =>
                  simp only [readProjections?] at read
                  simp only [writeProjections?] at write
                  split at read
                  · rename_i same
                    simp only [same, if_true] at write
                    exact writeProjections?_subsumes rest read write subsumes
                  · cases read
          | _ => simp [readProjections?] at read
      | deref =>
          cases value with
          | borrow loan current =>
              simp only [readProjections?] at read
              simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff, pure,
                Option.some.injEq] at write
              obtain ⟨written, written_eq, rfl⟩ := write
              exact deref_subsumes (writeProjections?_subsumes rest read written_eq subsumes)
          | _ => simp [readProjections?] at read

/-- Writing back, at a place, a value with every type of the one it reads
keeps frame and state typed. -/
theorem writeRuntimePlace?_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame frame' : RuntimeFrame} {state state' : RuntimeState} {place : RuntimePlace}
    {value value' : RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (read : readRuntimePlace? frame state place = some value)
    (write : writeRuntimePlace? frame state place value' = some (frame', state'))
    (subsumes : Subsumes unit loans value' value) :
    TypedFrame unit loans ns locals env frame' ∧ TypedState unit loans inert state' := by
  simp only [readRuntimePlace?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
  obtain ⟨rootValue, rootRead, read⟩ := read
  have store : ∀ stored, Subsumes unit loans stored rootValue →
      writeRoot? frame state place.root stored = some (frame', state') →
      TypedFrame unit loans ns locals env frame' ∧ TypedState unit loans inert state' := by
    intro stored stored_subsumes stored_eq
    cases root_eq : place.root with
    | «local» localId =>
        rw [root_eq] at stored_eq rootRead
        simp only [writeRoot?] at stored_eq
        split at stored_eq
        · cases stored_eq
        · simp only [Option.some.injEq, Prod.mk.injEq] at stored_eq
          obtain ⟨rfl, rfl⟩ := stored_eq
          simp only [readRoot?, readLocal?, Option.join_eq_some_iff] at rootRead
          exact ⟨frameTyped.subsume rootRead stored_subsumes, stateTyped⟩
    | global key =>
        rw [root_eq] at stored_eq rootRead
        simp only [writeRoot?, Option.bind_eq_bind, Option.bind_eq_some_iff,
          Option.some.injEq, Prod.mk.injEq] at stored_eq
        obtain ⟨_, -, rfl, rfl⟩ := stored_eq
        simp only [readRoot?] at rootRead
        exact ⟨frameTyped, stateTyped.subsume rootRead stored_subsumes⟩
  simp only [writeRuntimePlace?] at write
  split at write
  · cases write
  split at write
  · rename_i empty
    rw [Array.isEmpty_iff.mp empty] at read
    simp only [readProjections?, Option.some.injEq] at read
    subst read
    exact store value' subsumes write
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at write
    obtain ⟨rootValue', rootRead', updated, updated_eq, write⟩ := write
    rw [rootRead, Option.some.injEq] at rootRead'
    subst rootRead'
    exact store updated (writeProjections?_subsumes _ read updated_eq subsumes) write

/-! ## Write-backs -/

/-- Filling a loan's hole where it is visible keeps frame and state typed. -/
theorem fillVisibleHole_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame frame' : RuntimeFrame} {state state' : RuntimeState} {loan : Nat}
    {referent : SemTy} {replacement : RuntimeValue} {found : Bool}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans replacement referent)
    (fill : fillVisibleHole frame state loan replacement = (frame', state', found)) :
    TypedFrame unit loans ns locals env frame' ∧ TypedState unit loans inert state' := by
  unfold fillVisibleHole at fill
  split at fill
  · split at fill
    · split at fill
      · rename_i filled filled_eq
        simp only [Prod.mk.injEq] at fill
        obtain ⟨rfl, rfl, -⟩ := fill
        simp only [Option.bind_eq_some_iff] at filled_eq
        obtain ⟨_, slot_eq, value, rfl, filled_eq⟩ := filled_eq
        exact ⟨frameTyped.subsume slot_eq (fillHole?_subsumes loan_eq typed filled_eq),
          stateTyped⟩
      · simp only [Prod.mk.injEq] at fill
        obtain ⟨rfl, rfl, -⟩ := fill
        exact ⟨frameTyped, stateTyped⟩
    · simp only [Prod.mk.injEq] at fill
      obtain ⟨rfl, rfl, -⟩ := fill
      exact ⟨frameTyped, stateTyped⟩
  · split at fill
    · split at fill
      · rename_i key _ _ filled filled_eq
        simp only [Prod.mk.injEq] at fill
        obtain ⟨rfl, rfl, -⟩ := fill
        simp only [Option.bind_eq_some_iff] at filled_eq
        obtain ⟨value, lookup_eq, filled_eq⟩ := filled_eq
        refine ⟨frameTyped, ?_⟩
        cases key with
        | global key =>
            exact (stateTyped.subsume lookup_eq (fillHole?_subsumes loan_eq typed filled_eq)).of_eq
              rfl rfl rfl
        | table key => exact stateTyped.of_eq rfl rfl rfl
      · simp only [Prod.mk.injEq] at fill
        obtain ⟨rfl, rfl, -⟩ := fill
        exact ⟨frameTyped, stateTyped⟩
    · simp only [Prod.mk.injEq] at fill
      obtain ⟨rfl, rfl, -⟩ := fill
      exact ⟨frameTyped, stateTyped⟩

/-- A write-back of a loan's referent keeps frame and state typed. -/
theorem applyWriteBack_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {loan : Nat}
    {referent : SemTy} {current : RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans current referent) :
    TypedFrame unit loans ns locals env (applyWriteBack frame state loan current).1 ∧
      TypedState unit loans inert (applyWriteBack frame state loan current).2 := by
  unfold applyWriteBack
  rcases fill_eq : fillVisibleHole frame state loan current with ⟨frame', state', found⟩
  obtain ⟨frame'Typed, state'Typed⟩ :=
    fillVisibleHole_typed frameTyped stateTyped loan_eq typed fill_eq
  cases found
  · exact ⟨frame'Typed, state'Typed.push_pending loan_eq typed⟩
  · exact ⟨frame'Typed, state'Typed⟩

/-! ## Borrows at rest -/

/-- A loan's borrow has a current of the loan's referent. -/
def CurrentTyped (unit : ValidatedUnit) (loans : LoanTypes) (loan : Nat)
    (current : RuntimeValue) : Prop :=
  ∃ referent, loans loan = some referent ∧ HasType unit loans current referent

/-- Every local value of a typed frame has a type. -/
theorem TypedFrame.slot {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {index : Nat} {value : RuntimeValue}
    (typed : TypedFrame unit loans ns locals env frame)
    (slot_eq : frame.locals[index]? = some (some value)) :
    ∃ type, HasType unit loans value type := by
  obtain ⟨size_eq, locals_typed⟩ := typed
  have bound : index < locals.size := by
    rw [← size_eq]
    exact (Array.getElem?_eq_some_iff.mp slot_eq).1
  obtain ⟨type, -, value_typed⟩ :=
    locals_typed index locals[index] value (Array.getElem?_eq_getElem bound) slot_eq
  exact ⟨type, value_typed⟩

theorem TypedFrame.mem {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {value : RuntimeValue}
    (typed : TypedFrame unit loans ns locals env frame)
    (member : some value ∈ frame.locals) :
    ∃ type, HasType unit loans value type := by
  obtain ⟨index, bound, slot_eq⟩ := Array.mem_iff_getElem.mp member
  exact typed.slot (by rw [Array.getElem?_eq_getElem bound, slot_eq])

theorem TypedState.slot {unit : ValidatedUnit} {loans : LoanTypes} {state : RuntimeState}
    {slot : GlobalSlot} (typed : TypedState unit loans inert state)
    (member : slot ∈ state.globals.entries) :
    ∃ type, HasType unit loans slot.value type := by
  obtain ⟨_, type, -, -, value_typed⟩ := typed.globals slot member
  exact ⟨type, value_typed⟩

/-- The current of a loan's borrow found in a typed value. -/
theorem findFirst_borrowCurrent {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {value current : RuntimeValue} {type : SemTy} (typed : HasType unit loans value type)
    (found : findFirst (borrowCurrent? loan) value = some current) :
    CurrentTyped unit loans loan current := by
  refine findFirst_typed (P := CurrentTyped unit loans loan) (fun node _ answer typed matched => ?_)
    typed found
  cases node <;> simp only [borrowCurrent?, reduceCtorEq] at matched
  split at matched
  · rename_i same
    simp only [Option.some.injEq] at matched
    subst matched
    rw [← beq_iff_eq.mp same]
    exact typed.borrow_current
  · cases matched

/-- The current `findBorrowValue?` finds has its loan's referent. -/
theorem findBorrowValue?_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {loan : Nat} {current : RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (found : findBorrowValue? frame state loan = some current) :
    CurrentTyped unit loans loan current := by
  unfold findBorrowValue? at found
  dsimp only at found
  split at found
  · rename_i answer answer_eq
    simp only [Option.some.injEq] at found
    subst found
    obtain ⟨slot, member, slot_eq⟩ := Array.exists_of_findSome?_eq_some answer_eq
    cases slot with
    | none => cases slot_eq
    | some value =>
        obtain ⟨_, typed⟩ := frameTyped.mem member
        exact findFirst_borrowCurrent typed slot_eq
  · obtain ⟨slot, member, slot_eq⟩ := Array.exists_of_findSome?_eq_some found
    obtain ⟨_, typed⟩ := stateTyped.slot member
    exact findFirst_borrowCurrent typed slot_eq

/-- An outermost borrow of a typed value has a current of its referent. -/
theorem outermostBorrows_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {type : SemTy} {entry : Nat × RuntimeValue}
    (typed : HasType unit loans value type) (member : entry ∈ outermostBorrows value) :
    CurrentTyped unit loans entry.1 entry.2 := by
  unfold outermostBorrows at member
  obtain ⟨found, found_mem, found_eq⟩ := Array.mem_filterMap.mp member
  refine collectPruned_typed
    (P := fun found => ∀ entry, found = some entry → CurrentTyped unit loans entry.1 entry.2)
    (fun node _ answer typed matched => ?_) typed found_mem entry found_eq
  cases node <;> simp only [borrowEntry?, reduceCtorEq, Option.some.injEq] at matched
  subst matched
  intro entry entry_eq
  simp only [Option.some.injEq] at entry_eq
  subst entry_eq
  exact typed.borrow_current

/-- Every borrow a typed frame holds has a current of its referent. -/
theorem frameBorrows_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {entry : Nat × RuntimeValue}
    (typed : TypedFrame unit loans ns locals env frame)
    (member : entry ∈ frameBorrows frame) :
    CurrentTyped unit loans entry.1 entry.2 := by
  unfold frameBorrows at member
  have folded : ∀ (slots : List (Option RuntimeValue)) (init : Array (Nat × RuntimeValue)),
      (∀ slot ∈ slots, slot ∈ frame.locals.toList) →
      entry ∈ slots.foldl (fun borrows slot => match slot with
        | some value => borrows ++ outermostBorrows value
        | none => borrows) init →
      entry ∈ init ∨ CurrentTyped unit loans entry.1 entry.2 := by
    intro slots
    induction slots with
    | nil => intro init _ member; exact .inl member
    | cons slot rest ih =>
        intro init within member
        rcases ih _ (fun slot member => within slot (List.mem_cons_of_mem _ member)) member with
          member | done
        · cases slot with
          | none => exact .inl member
          | some value =>
              rcases Array.mem_append.mp member with member | member
              · exact .inl member
              · obtain ⟨_, value_typed⟩ :=
                  typed.mem (Array.mem_toList_iff.mp (within (some value) (by simp)))
                exact .inr (outermostBorrows_typed value_typed member)
        · exact .inr done
  rw [← Array.foldl_toList] at member
  rcases folded frame.locals.toList #[] (fun _ member => member) member with member | done
  · simp at member
  · exact done

/-- Retiring a loan's borrow keeps frame and state typed. -/
theorem clearBorrowValue_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {loan : Nat}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state) :
    TypedFrame unit loans ns locals env (clearBorrowValue frame state loan).1 ∧
      TypedState unit loans inert (clearBorrowValue frame state loan).2 := by
  unfold clearBorrowValue
  dsimp only
  split
  · split
    · rename_i updated updated_eq
      simp only [Option.bind_eq_some_iff] at updated_eq
      obtain ⟨_, slot_eq, value, rfl, cleared⟩ := updated_eq
      exact ⟨frameTyped.subsume slot_eq (borrowClear?_subsumes cleared), stateTyped⟩
    · exact ⟨frameTyped, stateTyped⟩
  · split
    · split
      · rename_i updated updated_eq
        simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at updated_eq
        obtain ⟨slot, slot_eq, value, cleared, rfl⟩ := updated_eq
        exact ⟨frameTyped, stateTyped.subsume_entry slot_eq (borrowClear?_subsumes cleared)⟩
      · exact ⟨frameTyped, stateTyped⟩
    · exact ⟨frameTyped, stateTyped⟩

/-- Replacing a loan's borrow's current by a value of its referent keeps
frame and state typed. -/
theorem updateBorrowValue?_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame frame' : RuntimeFrame} {state state' : RuntimeState} {loan : Nat}
    {referent : SemTy} {replacement : RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans replacement referent)
    (update : updateBorrowValue? frame state loan replacement = some (frame', state')) :
    TypedFrame unit loans ns locals env frame' ∧ TypedState unit loans inert state' := by
  unfold updateBorrowValue? at update
  split at update
  · rename_i updated local_eq
    simp only [Option.some.injEq] at update
    subst update
    simp only [updateLocalBorrowValue?, Option.bind_eq_bind, Option.bind_eq_some_iff] at local_eq
    obtain ⟨place, -, value, read, rewritten, rewritten_eq, write⟩ := local_eq
    exact writeRuntimePlace?_subsumes frameTyped stateTyped read write
      (borrowRewrite?_subsumes loan_eq typed rewritten_eq)
  · dsimp only at update
    split at update
    · simp only [Option.map_eq_some_iff, Option.bind_eq_some_iff] at update
      obtain ⟨updated, ⟨_, slot_eq, value, rfl, rewritten_eq⟩, update⟩ := update
      simp only [Prod.mk.injEq] at update
      obtain ⟨rfl, rfl⟩ := update
      exact ⟨frameTyped.subsume slot_eq (borrowRewrite?_subsumes loan_eq typed rewritten_eq),
        stateTyped⟩
    · split at update
      · simp only [Option.bind_eq_some_iff, Option.map_eq_some_iff] at update
        obtain ⟨slot, slot_eq, value, rewritten_eq, update⟩ := update
        simp only [Prod.mk.injEq] at update
        obtain ⟨rfl, rfl⟩ := update
        exact ⟨frameTyped,
          stateTyped.subsume_entry slot_eq (borrowRewrite?_subsumes loan_eq typed rewritten_eq)⟩
      · cases update

theorem Array.foldr_preserves {α β : Type} (P : β → Prop) (f : α → β → β)
    (xs : Array α) (init : β) (initial : P init) (step : ∀ a b, P b → P (f a b)) :
    P (xs.foldr f init) := by
  rw [← Array.foldr_toList]
  induction xs.toList with
  | nil => exact initial
  | cons a rest ih => exact step a _ ih

theorem Array.foldl_preserves {α β : Type} (P : β → Prop) (f : β → α → β)
    (xs : Array α) (init : β) (initial : P init) (step : ∀ b a, a ∈ xs → P b → P (f b a)) :
    P (xs.foldl f init) := by
  rw [← Array.foldl_toList]
  have within : ∀ a ∈ xs.toList, a ∈ xs := fun a member => Array.mem_toList_iff.mp member
  revert within
  generalize xs.toList = list
  induction list generalizing init with
  | nil => intro _; exact initial
  | cons a rest ih =>
      intro within
      exact ih _ (step init a (within a (by simp)) initial)
        (fun a member => within a (List.mem_cons_of_mem _ member))

/-- Ending a node's loans keeps frame and state typed. -/
theorem settleLoans_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {ended : Array LoanId}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state) :
    TypedFrame unit loans ns locals env (settleLoans ended frame state).1 ∧
      TypedState unit loans inert (settleLoans ended frame state).2 := by
  unfold settleLoans
  refine Array.foldr_preserves
    (fun (pair : RuntimeFrame × RuntimeState) =>
      TypedFrame unit loans ns locals env pair.1 ∧ TypedState unit loans inert pair.2) _ _
    _ ⟨frameTyped, stateTyped⟩ fun lexical ⟨frame, state⟩ ⟨frameTyped, stateTyped⟩ => ?_
  dsimp only
  split
  · exact ⟨frameTyped, stateTyped⟩
  · have retired : TypedFrame unit loans ns locals env
        { frame with activeLoans := frame.activeLoans.filter (·.1 != ⟨lexical.index⟩) } :=
      frameTyped.of_locals rfl
    split
    · rename_i current current_eq
      obtain ⟨referent, loan_eq, current_typed⟩ :=
        findBorrowValue?_typed retired stateTyped current_eq
      obtain ⟨cleared, clearedState⟩ := clearBorrowValue_typed (loan := _) retired stateTyped
      exact applyWriteBack_typed cleared clearedState loan_eq current_typed
    · exact ⟨frameTyped, stateTyped⟩

/-- Ending a node's loans after a value, or skipping them, keeps frame and
state typed. -/
theorem settleAfter_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {ended : Array LoanId} {control : Control}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state) :
    TypedFrame unit loans ns locals env (settleAfter ended control frame state).1 ∧
      TypedState unit loans inert (settleAfter ended control frame state).2 := by
  unfold settleAfter
  split
  · exact settleLoans_typed frameTyped stateTyped
  · exact ⟨frameTyped, stateTyped⟩

/-- Settling the loans whose holes die with a frame keeps it typed. -/
theorem settleFrameLoans_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg} :
    ∀ (fuel : Nat) {frame : RuntimeFrame} {state : RuntimeState},
      TypedFrame unit loans ns locals env frame → TypedState unit loans inert state →
      TypedFrame unit loans ns locals env (settleFrameLoans frame state fuel).1 ∧
        TypedState unit loans inert (settleFrameLoans frame state fuel).2
  | 0, _, _, frameTyped, stateTyped => ⟨frameTyped, stateTyped⟩
  | fuel + 1, frame, state, frameTyped, stateTyped => by
      unfold settleFrameLoans
      split
      · exact ⟨frameTyped, stateTyped⟩
      · rename_i loan current found
        obtain ⟨referent, loan_eq, current_typed⟩ :=
          frameBorrows_typed frameTyped (Array.mem_of_find?_eq_some found)
        obtain ⟨cleared, clearedState⟩ := clearBorrowValue_typed (loan := loan) frameTyped stateTyped
        rcases fill_eq : fillVisibleHole (clearBorrowValue frame state loan).1
          (clearBorrowValue frame state loan).2 loan current with ⟨frame', state', found'⟩
        obtain ⟨filled, filledState⟩ :=
          fillVisibleHole_typed cleared clearedState loan_eq current_typed fill_eq
        simp only [fill_eq]
        exact settleFrameLoans_typed fuel filled filledState

/-- The write-backs of a settled frame keep the state typed. -/
theorem exportSettledLoans_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state) :
    TypedState unit loans inert (exportSettledLoans frame state) := by
  unfold exportSettledLoans
  refine Array.foldl_preserves (TypedState unit loans inert) _ _ _ stateTyped
    fun state ⟨loan, current⟩ member stateTyped => ?_
  dsimp only
  split
  · exact stateTyped
  · obtain ⟨referent, loan_eq, current_typed⟩ := frameBorrows_typed frameTyped member
    have empty : TypedFrame unit loans ns #[] env { locals := #[] } :=
      ⟨rfl, fun _ _ _ declaration_eq _ => by simp at declaration_eq⟩
    exact (applyWriteBack_typed empty stateTyped loan_eq current_typed).2

/-- The loans a dying frame exports keep the state typed. -/
theorem exportFrameLoans_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state) :
    TypedState unit loans inert (exportFrameLoans frame state) := by
  unfold exportFrameLoans
  split
  · exact exportSettledLoans_typed frameTyped stateTyped
  · obtain ⟨settled, settledState⟩ :=
      settleFrameLoans_typed (frameBorrows frame).size frameTyped stateTyped
    exact exportSettledLoans_typed settled settledState

theorem maskReturnedBorrowList_length (masked : Array Nat) :
    ∀ values : List RuntimeValue,
      (maskReturnedBorrowList masked values).length = values.length
  | [] => by simp [maskReturnedBorrowList]
  | _ :: values => by simp [maskReturnedBorrowList, maskReturnedBorrowList_length masked values]

mutual
/-- Masking returned borrows keeps every type of a value: a masked borrow
leaves a dead reference. -/
theorem maskReturnedBorrows_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {masked : Array Nat} :
    ∀ {value : RuntimeValue} {type : SemTy}, HasType unit loans value type →
      HasType unit loans (maskReturnedBorrows masked value) type
  | _, _, .tuple _ _ elements => by
      unfold maskReturnedBorrows
      exact .tuple _ _ (by simpa using maskReturnedBorrowList_types elements)
  | _, _, .vector _ _ _ length_matches elements => by
      unfold maskReturnedBorrows
      refine .vector _ _ _ ?_ (by simpa using maskReturnedBorrowList_each elements)
      simpa [maskReturnedBorrowList_length] using length_matches
  | _, _, .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq
      resolved fields_typed => by
      unfold maskReturnedBorrows
      exact .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq
        resolved (by simpa using maskReturnedBorrowList_types fields_typed)
  | _, _, .closure _ _ _ _ _ _ targetNs declaration arguments allParameters results
      namespace_eq declaration_eq signature_eq faithful mask_bound captures_typed
      parameters_eq packs => by
      unfold maskReturnedBorrows
      exact .closure _ _ _ _ _ _ targetNs declaration arguments allParameters results
        namespace_eq declaration_eq signature_eq faithful mask_bound
        (by simpa using maskReturnedBorrowList_types captures_typed) parameters_eq packs
  | _, _, .borrow _ _ referent loan_eq current_typed => by
      unfold maskReturnedBorrows
      split
      · exact .dead referent
      · exact .borrow _ _ _ loan_eq current_typed
  | _, _, .shared _ _ inner => .shared _ _ (maskReturnedBorrows_typed inner)
  | _, _, .unit | _, _, .bool _ | _, _, .character _ _ | _, _, .string _ | _, _, .bytes _
  | _, _, .integer _ _ _ _ | _, _, .address _ | _, _, .signer _ | _, _, .dead _
  | _, _, .hole _ _ _ _ _ _ | _, _, .unitEmptyTuple => by
      unfold maskReturnedBorrows
      constructor <;> assumption
  | _, _, .emptyTupleUnit => by
      unfold maskReturnedBorrows
      simpa [maskReturnedBorrowList] using HasType.emptyTupleUnit

theorem maskReturnedBorrowList_types {unit : ValidatedUnit} {loans : LoanTypes}
    {masked : Array Nat} :
    ∀ {values : List RuntimeValue} {types : List SemTy}, HasTypes unit loans values types →
      HasTypes unit loans (maskReturnedBorrowList masked values) types
  | [], _, .nil => by simpa [maskReturnedBorrowList] using HasTypes.nil
  | _ :: _, _ :: _, .cons head tail => by
      simp only [maskReturnedBorrowList]
      exact .cons (maskReturnedBorrows_typed head) (maskReturnedBorrowList_types tail)

theorem maskReturnedBorrowList_each {unit : ValidatedUnit} {loans : LoanTypes}
    {masked : Array Nat} :
    ∀ {values : List RuntimeValue} {type : SemTy}, HasTypeEach unit loans values type →
      HasTypeEach unit loans (maskReturnedBorrowList masked values) type
  | [], _, .nil => by simpa [maskReturnedBorrowList] using HasTypeEach.nil
  | _ :: _, _, .cons head tail => by
      simp only [maskReturnedBorrowList]
      exact .cons (maskReturnedBorrows_typed head) (maskReturnedBorrowList_each tail)
end

/-- The loans a function's dying frame exports beside its results keep the
state typed. -/
theorem exportReturnedFrameLoans_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {results : Array RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state) :
    TypedState unit loans inert (exportReturnedFrameLoans results frame state) := by
  unfold exportReturnedFrameLoans
  split
  · exact exportFrameLoans_typed frameTyped stateTyped
  split
  · exact exportFrameLoans_typed frameTyped stateTyped
  split
  · exact stateTyped
  dsimp only
  split
  · exact exportFrameLoans_typed frameTyped stateTyped
  refine exportFrameLoans_typed (ns := ns) (env := env) ⟨by simpa using frameTyped.1,
    fun index declaration value declaration_eq value_eq => ?_⟩ stateTyped
  simp only [Array.getElem?_map, Option.map_eq_some_iff] at value_eq
  obtain ⟨stored, stored_eq, value_eq⟩ := value_eq
  cases stored with
  | none => simp at value_eq
  | some original =>
      simp only [Option.some.injEq, exists_eq_left'] at value_eq
      subst value_eq
      obtain ⟨type, resolves, typed⟩ := frameTyped.2 index declaration original declaration_eq
        stored_eq
      exact ⟨type, resolves, maskReturnedBorrows_typed typed⟩

/-- A write-back crossing a call boundary keeps frame and state typed. -/
theorem applyPendingWriteBack_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {loan : Nat}
    {referent : SemTy} {current : RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (loan_eq : loans loan = some referent)
    (typed : HasType unit loans current referent) :
    TypedFrame unit loans ns locals env (applyPendingWriteBack frame state loan current).1 ∧
      TypedState unit loans inert (applyPendingWriteBack frame state loan current).2 := by
  unfold applyPendingWriteBack
  split
  · rename_i resolved resolved_eq
    simp only [fillLocalLoanHole?, Option.bind_eq_bind, Option.bind_eq_some_iff] at resolved_eq
    obtain ⟨place, -, value, read, filled, filled_eq, ⟨frame', state'⟩, write, resolved_eq⟩ :=
      resolved_eq
    simp only [Option.some.injEq] at resolved_eq
    subst resolved_eq
    obtain ⟨frame'Typed, -⟩ := writeRuntimePlace?_subsumes frameTyped stateTyped read write
      (fillHole?_subsumes loan_eq typed filled_eq)
    exact ⟨frame'Typed.of_locals rfl, stateTyped⟩
  · split
    · split
      · split
        · rename_i filled filled_eq
          simp only [Option.bind_eq_some_iff] at filled_eq
          obtain ⟨_, slot_eq, value, rfl, filled_eq⟩ := filled_eq
          exact ⟨(frameTyped.subsume slot_eq (fillHole?_subsumes loan_eq typed filled_eq)).of_locals
            rfl, stateTyped⟩
        · exact ⟨frameTyped, stateTyped.push_pending loan_eq typed⟩
      · exact ⟨frameTyped, stateTyped.push_pending loan_eq typed⟩
    · exact ⟨frameTyped, stateTyped.push_pending loan_eq typed⟩

/-- Applying a callee's write-backs keeps frame and state typed. -/
theorem applyPendingFrom_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} {state : RuntimeState} {inherited : Array (Nat × RuntimeValue)}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (inherited_le : inert ≤ inherited.size)
    (inheritedTyped : ∀ entry ∈ inherited.extract inert inherited.size,
      CurrentTyped unit loans entry.1 entry.2) :
    TypedFrame unit loans ns locals env (applyPendingFrom inherited frame state).1 ∧
      TypedState unit loans inert (applyPendingFrom inherited frame state).2 := by
  unfold applyPendingFrom
  have restarted : TypedState unit loans inert { state with pending := inherited } :=
    TypedState.mk stateTyped.globals inherited_le inheritedTyped stateTyped.bounded
  refine Array.foldl_preserves
    (fun (pair : RuntimeFrame × RuntimeState) =>
      TypedFrame unit loans ns locals env pair.1 ∧ TypedState unit loans inert pair.2) _ _ _
    ⟨frameTyped, restarted⟩
    fun ⟨frame, state⟩ ⟨loan, current⟩ member ⟨frameTyped, stateTyped'⟩ => ?_
  obtain ⟨referent, loan_eq, current_typed⟩ :=
    stateTyped.pending (loan, current) (Array.mem_extract_of_le member inherited_le)
  exact applyPendingWriteBack_typed frameTyped stateTyped' loan_eq current_typed

/-- Assigning through a mutable reference keeps frame and state typed. -/
theorem mutateBorrow?_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame frame' : RuntimeFrame} {state state' : RuntimeState} {referent type : SemTy}
    {reference value result : RuntimeValue}
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (reference_typed : HasType unit loans reference (.reference .mutable referent))
    (value_typed : HasType unit loans value referent)
    (packed : StaticTyping.packs [] type = true)
    (mutate : mutateBorrow? #[reference, value] frame state = some (frame', state', result)) :
    TypedFrame unit loans ns locals env frame' ∧ TypedState unit loans inert state' ∧
      HasType unit loans result type := by
  unfold mutateBorrow? at mutate
  split at mutate
  · rename_i loan _ _ arguments_eq
    simp only [List.cons.injEq, and_true] at arguments_eq
    obtain ⟨rfl, rfl⟩ := arguments_eq
    obtain ⟨referent', loan_eq, -⟩ := reference_typed.borrow_current
    have same : referent' = referent := by
      cases reference_typed with
      | borrow _ _ _ loan_eq' _ =>
          rw [loan_eq, Option.some.injEq] at loan_eq'
          exact loan_eq'
    subst same
    split at mutate
    · rename_i updated update_eq
      simp only [Option.some.injEq, Prod.mk.injEq] at mutate
      obtain ⟨rfl, rfl, rfl⟩ := mutate
      obtain ⟨updatedFrame, updatedState⟩ :=
        updateBorrowValue?_typed frameTyped stateTyped loan_eq value_typed update_eq
      exact ⟨updatedFrame, updatedState, packResults_typed (values := #[]) .nil packed⟩
    · simp only [Option.some.injEq, Prod.mk.injEq] at mutate
      obtain ⟨rfl, rfl, rfl⟩ := mutate
      obtain ⟨writtenFrame, writtenState⟩ :=
        applyWriteBack_typed frameTyped stateTyped loan_eq value_typed
      exact ⟨writtenFrame, writtenState, packResults_typed (values := #[]) .nil packed⟩
  · cases mutate

/-- Filling, in a write-back, the holes of the borrows a callee returns keeps
every type of the write-back. -/
theorem resolveReturnedBorrows_subsumes {unit : ValidatedUnit} {loans : LoanTypes}
    {results : Array RuntimeValue} {replacement : RuntimeValue}
    (resultsTyped : ∀ result ∈ results, ∃ type, HasType unit loans result type) :
    Subsumes unit loans (resolveReturnedBorrows results replacement) replacement := by
  unfold resolveReturnedBorrows
  refine Array.foldl_preserves (fun resolved => Subsumes unit loans resolved replacement) _ _ _
    (Subsumes.refl _) fun resolved result member subsumes => ?_
  obtain ⟨_, result_typed⟩ := resultsTyped result member
  refine Array.foldl_preserves (fun resolved' => Subsumes unit loans resolved' replacement) _ _ _
    subsumes fun resolved' returned returned_mem subsumes' => ?_
  obtain ⟨referent, loan_eq, current_typed⟩ := outermostBorrows_typed result_typed returned_mem
  cases filled_eq : fillHole? returned.1 returned.2 resolved' with
  | none => simpa using subsumes'
  | some filled =>
      simp only [Option.getD_some]
      exact fun type typed =>
        fillHole?_subsumes loan_eq current_typed filled_eq type (subsumes' type typed)

/-! ## Borrowing -/

/-- The loans with one more, at a referent type. -/
def LoanTypes.extend (loans : LoanTypes) (loan : Nat) (referent : SemTy) : LoanTypes :=
  fun loan' => if loan' = loan then some referent else loans loan'

theorem LoanTypes.extend_self (loans : LoanTypes) (loan : Nat) (referent : SemTy) :
    loans.extend loan referent loan = some referent := by
  simp [LoanTypes.extend]

theorem LoanTypes.extend_extends {loans : LoanTypes} {loan : Nat} {referent : SemTy}
    (fresh : loans loan = none) : (loans.extend loan referent).Extends loans := by
  intro loan' type loan'_eq
  unfold LoanTypes.extend
  split
  · rename_i same
    subst same
    rw [fresh] at loan'_eq
    cases loan'_eq
  · exact loan'_eq

theorem TypedFrame.weaken {unit : ValidatedUnit} {smaller larger : LoanTypes}
    {ns : ValidatedNamespace} {locals : Array LocalDecl} {env : Array SemArg}
    {frame : RuntimeFrame} (extends_ : larger.Extends smaller)
    (typed : TypedFrame unit smaller ns locals env frame) :
    TypedFrame unit larger ns locals env frame := by
  obtain ⟨size_eq, locals_typed⟩ := typed
  refine ⟨size_eq, fun index declaration value declaration_eq value_eq => ?_⟩
  obtain ⟨type, resolves, value_typed⟩ := locals_typed index declaration value declaration_eq value_eq
  exact ⟨type, resolves, value_typed.weaken extends_⟩

/-- A typed state under loans its frontier bounds. -/
theorem TypedState.weaken {unit : ValidatedUnit} {smaller larger : LoanTypes}
    {state : RuntimeState} (extends_ : larger.Extends smaller)
    (typed : TypedState unit smaller inert state)
    (bounded : ∀ loan type, larger loan = some type → loan < state.nextLoan) :
    TypedState unit larger inert state where
  globals slot member := by
    obtain ⟨ns, type, ns_eq, resolves, value_typed⟩ := typed.globals slot member
    exact ⟨ns, type, ns_eq, resolves, value_typed.weaken extends_⟩
  inert_le := typed.inert_le
  pending entry member := by
    obtain ⟨type, loan_eq, value_typed⟩ := typed.pending entry member
    exact ⟨type, extends_ _ _ loan_eq, value_typed.weaken extends_⟩
  bounded := bounded

/-- Minting the frontier loan at a referent keeps the state typed. -/
theorem TypedState.mint {unit : ValidatedUnit} {loans : LoanTypes} {state : RuntimeState}
    {referent : SemTy} (typed : TypedState unit loans inert state) :
    (loans.extend state.nextLoan referent).Extends loans ∧
      TypedState unit (loans.extend state.nextLoan referent) inert
        { state with nextLoan := state.nextLoan + 1 } := by
  have fresh : loans state.nextLoan = none := by
    cases loan_eq : loans state.nextLoan with
    | none => rfl
    | some type' => exact absurd (typed.bounded _ _ loan_eq) (Nat.lt_irrefl _)
  have extends_ := LoanTypes.extend_extends (referent := referent) fresh
  refine ⟨extends_, TypedState.mk (fun slot member => ?_) typed.inert_le
    (fun entry member => ?_) (fun loan type loan_eq => ?_)⟩
  · obtain ⟨ns, type, ns_eq, resolves, value_typed⟩ := typed.globals slot member
    exact ⟨ns, type, ns_eq, resolves, value_typed.weaken extends_⟩
  · obtain ⟨type, loan_eq, value_typed⟩ := typed.pending entry member
    exact ⟨type, extends_ _ _ loan_eq, value_typed.weaken extends_⟩
  · unfold LoanTypes.extend at loan_eq
    split at loan_eq
    · rename_i same
      subst same
      exact Nat.lt_succ_self _
    · exact Nat.lt_succ_of_lt (typed.bounded _ _ loan_eq)

/-- A shared borrow is the value it reads. -/
theorem borrowRuntimePlace?_shared_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {ns : ValidatedNamespace} {site : ExprId}
    {referenceType : ReferenceType} {frame frame' : RuntimeFrame}
    {state state' : RuntimeState} {place : RuntimePlace} {type : SemTy}
    {variant : Option String} {result : RuntimeValue}
    (placeTyped : PlaceTyped unit loans context frame place type variant)
    (borrow : borrowRuntimePlace? unit ns site referenceType .immutable frame state place =
      some (frame', state', result)) :
    frame' = frame ∧ state' = state ∧ HasType unit loans result (.reference .shared type) := by
  simp only [borrowRuntimePlace?, Option.bind_eq_bind, Option.bind_some] at borrow
  split at borrow
  · cases borrow
  obtain ⟨value, read, borrow⟩ := Option.bind_eq_some_iff.mp borrow
  simp only [Option.some.injEq, Prod.mk.injEq] at borrow
  obtain ⟨rfl, rfl, rfl⟩ := borrow
  exact ⟨rfl, rfl, .shared _ _ (placeTyped.read read)⟩

/-- A mutable borrow mints a loan at the place's type: its hole stays in
the place, and the borrow carries the value. -/
theorem borrowRuntimePlace?_mutable_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {ns : ValidatedNamespace} {site : ExprId}
    {referenceType : ReferenceType} {frame frame' : RuntimeFrame}
    {state state' : RuntimeState} {place : RuntimePlace} {type : SemTy}
    {variant : Option String} {result : RuntimeValue}
    (frameTyped : TypedFrame unit loans context.ns context.locals context.env frame)
    (stateTyped : TypedState unit loans inert state)
    (placed : ∀ loans', TypedFrame unit loans' context.ns context.locals context.env frame →
      PlaceTyped unit loans' context frame place type variant)
    (borrow : borrowRuntimePlace? unit ns site referenceType .mutable frame state place =
      some (frame', state', result)) :
    let loans' := loans.extend state.nextLoan type
    TypedFrame unit loans' context.ns context.locals context.env frame' ∧
      TypedState unit loans' inert state' ∧ HasType unit loans' result (.reference .mutable type) ∧
      state'.nextLoan = state.nextLoan + 1 := by
  intro loans'
  obtain ⟨extends_, advanced⟩ := stateTyped.mint (referent := type)
  have placeTyped := placed loans' (frameTyped.weaken extends_)
  simp only [borrowRuntimePlace?, Option.bind_eq_bind, Option.bind_some] at borrow
  split at borrow
  · cases borrow
  obtain ⟨value, read, borrow⟩ := Option.bind_eq_some_iff.mp borrow
  obtain ⟨lexical, -, borrow⟩ := Option.bind_eq_some_iff.mp borrow
  obtain ⟨⟨written, writtenState⟩, write, borrow⟩ := Option.bind_eq_some_iff.mp borrow
  simp only [Option.some.injEq, Prod.mk.injEq] at borrow
  obtain ⟨rfl, rfl, rfl⟩ := borrow
  have hole : HasType unit loans' (.loanHole state.nextLoan) type :=
    .hole _ type type (LoanTypes.extend_self _ _ _) rfl (placeTyped.read read).inhabited
  obtain ⟨writtenTyped, rfl⟩ := placeTyped.write (frameTyped.weaken extends_) hole write
  refine ⟨writtenTyped.of_locals rfl, ?_, .borrow _ _ _ (LoanTypes.extend_self _ _ _)
    (placeTyped.read read), ?_⟩
  · split
    · exact advanced.of_eq rfl rfl rfl
    · exact advanced
  · split <;> rfl

/-! ## Global operations -/

/-- Removing a resource keeps the state typed. -/
theorem TypedState.erase {unit : ValidatedUnit} {loans : LoanTypes} {state : RuntimeState}
    {key : GlobalKey} (typed : TypedState unit loans inert state) :
    TypedState unit loans inert { state with globals := state.globals.erase key } where
  globals slot member := by
    unfold GlobalMap.erase at member
    exact typed.globals slot (Array.mem_filter.mp member).1
  inert_le := typed.inert_le
  pending := typed.pending
  bounded := typed.bounded

/-- Publishing a value of its key's type keeps the state typed. -/
theorem TypedState.insert {unit : ValidatedUnit} {loans : LoanTypes} {state : RuntimeState}
    {key : GlobalKey} {value : RuntimeValue} {ns : ValidatedNamespace} {type : SemTy}
    (typed : TypedState unit loans inert state)
    (ns_eq : unit.namespaces[key.namespaceId.index]? = some ns)
    (resolves : Resolves ns.tables #[] key.typeId type)
    (value_typed : HasType unit loans value type) :
    TypedState unit loans inert { state with globals := state.globals.insert key value } where
  globals slot member := by
    rcases GlobalMap.mem_insert.mp member with ⟨member, -⟩ | rfl
    · exact typed.globals slot member
    · exact ⟨ns, type, ns_eq, resolves, value_typed⟩
  inert_le := typed.inert_le
  pending := typed.pending
  bounded := typed.bounded

/-- A resource read at a key of the frame's namespace has the resource's
type. -/
theorem TypedState.resource {unit : ValidatedUnit} {loans : LoanTypes} {state : RuntimeState}
    {ns : ValidatedNamespace} {resourceType : TypeId} {type : SemTy}
    {key value : RuntimeValue} (typed : TypedState unit loans inert state)
    (ns_eq : unit.namespaces[ns.identity.index]? = some ns)
    (resolves : Resolves ns.tables #[] resourceType type)
    (lookup_eq : state.globals.lookup (globalKey ns.identity resourceType key) = some value) :
    HasType unit loans value type := by
  obtain ⟨ns', type', ns'_eq, resolves', value_typed⟩ := typed.lookup lookup_eq
  simp only [globalKey] at ns'_eq resolves'
  rw [ns_eq, Option.some.injEq] at ns'_eq
  subst ns'_eq
  rw [Resolves.unique resolves resolves'] 
  exact value_typed

/-- What a global operation leaves: a value of the operation's type in a
typed frame and state under loans extending the frame's, or an abort that
leaves both as they were. -/
def GlobalResultTyped (unit : ValidatedUnit) (loans : LoanTypes) (inert : Nat)
    (ns : ValidatedNamespace)
    (locals : Array LocalDecl) (env : Array SemArg) (frame : RuntimeFrame) (state : RuntimeState)
    (type : SemTy) : GlobalOperationResult → Prop
  | .value frame' state' value => ∃ loans', loans'.Extends loans ∧
      TypedFrame unit loans' ns locals env frame' ∧ TypedState unit loans' inert state' ∧
      HasType unit loans' value type
  | .throw_ frame' state' _ _ => frame' = frame ∧ state' = state

section Global

variable {unit : ValidatedUnit} {loans : LoanTypes} {ns : ValidatedNamespace}
  {locals : Array LocalDecl} {env : Array SemArg} {frame : RuntimeFrame} {state : RuntimeState}
  {resultType : TypeId} {site : ExprId} {resource : TypeUse} {resourceType : SemTy}
  {arguments : Array RuntimeValue} {result : GlobalOperationResult}

theorem evaluateGlobal_contains_typed
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (eval : evaluateGlobalOperation? unit ns resultType site .contains #[.typeArg resource]
      arguments frame state = some result) :
    GlobalResultTyped unit loans inert ns locals env frame state .bool result := by
  rw [evaluateGlobalOperation?_typeArg] at eval
  dsimp only at eval
  split at eval
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
    obtain ⟨_, -, eval⟩ := eval
    simp only [Option.some.injEq] at eval
    subst eval
    exact ⟨loans, .refl _, frameTyped, stateTyped, .bool _⟩
  · cases eval

theorem evaluateGlobal_take_typed
    (ns_eq : unit.namespaces[ns.identity.index]? = some ns)
    (resolves : Resolves ns.tables #[]
      (instantiatedTypeId frame.typeInstantiation resource.typeId) resourceType)
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (eval : evaluateGlobalOperation? unit ns resultType site .take #[.typeArg resource]
      arguments frame state = some result) :
    GlobalResultTyped unit loans inert ns locals env frame state resourceType result := by
  rw [evaluateGlobalOperation?_typeArg] at eval
  dsimp only at eval
  split at eval
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
    obtain ⟨_, -, eval⟩ := eval
    split at eval
    · simp only [Option.some.injEq] at eval
      subst eval
      exact ⟨rfl, rfl⟩
    · rename_i value lookup_eq
      simp only [Option.some.injEq] at eval
      subst eval
      exact ⟨loans, .refl _, frameTyped, stateTyped.erase,
        stateTyped.resource ns_eq resolves lookup_eq⟩
  · cases eval

theorem evaluateGlobal_publish_typed
    (ns_eq : unit.namespaces[ns.identity.index]? = some ns)
    (resolves : Resolves ns.tables #[]
      (instantiatedTypeId frame.typeInstantiation resource.typeId) resourceType)
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (value_typed : ∀ key value, arguments.toList = [key, value] →
      HasType unit loans value resourceType)
    {type : SemTy} (packed : StaticTyping.packs [] type = true)
    (eval : evaluateGlobalOperation? unit ns resultType site .publish #[.typeArg resource]
      arguments frame state = some result) :
    GlobalResultTyped unit loans inert ns locals env frame state type result := by
  rw [evaluateGlobalOperation?_typeArg] at eval
  dsimp only at eval
  split at eval
  · rename_i key value arguments_eq
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
    obtain ⟨_, -, eval⟩ := eval
    split at eval
    · simp only [Option.some.injEq] at eval
      subst eval
      exact ⟨rfl, rfl⟩
    · simp only [Option.some.injEq] at eval
      subst eval
      exact ⟨loans, .refl _, frameTyped,
        stateTyped.insert ns_eq resolves (value_typed key value arguments_eq),
        packResults_typed (values := #[]) .nil packed⟩
  · cases eval

/-- Borrowing a published resource: a shared borrow is the resource, a
mutable one mints a loan at its type and leaves the hole in the slot. -/
theorem borrowRuntimePlace?_global_typed {referenceType : ReferenceType} {kind : BorrowKind}
    {referenceKind : ReferenceKind} {resourceTypeId : TypeId} {key : RuntimeValue}
    {frame' : RuntimeFrame} {state' : RuntimeState} {value : RuntimeValue}
    (ns_eq : unit.namespaces[ns.identity.index]? = some ns)
    (resolves : Resolves ns.tables #[] resourceTypeId resourceType)
    (kind_eq : StaticTyping.borrowedKind kind = some referenceKind)
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (borrow : borrowRuntimePlace? unit ns site referenceType kind frame state
      { root := .global (globalKey ns.identity resourceTypeId key) } =
        some (frame', state', value)) :
    ∃ loans', loans'.Extends loans ∧ TypedFrame unit loans' ns locals env frame' ∧
      TypedState unit loans' inert state' ∧
      HasType unit loans' value (.reference referenceKind resourceType) := by
  have reads : ∀ value, readRuntimePlace? frame state
      { root := .global (globalKey ns.identity resourceTypeId key) } = some value →
      HasType unit loans value resourceType := by
    intro value read
    simp only [readRuntimePlace?, readRoot?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
    obtain ⟨root, lookup_eq, read⟩ := read
    simp only [readProjections?, Option.some.injEq] at read
    subst read
    exact stateTyped.resource ns_eq resolves lookup_eq
  cases kind with
  | immutable =>
      simp only [StaticTyping.borrowedKind, Option.some.injEq] at kind_eq
      subst kind_eq
      simp only [borrowRuntimePlace?, Option.bind_eq_bind, Option.bind_some] at borrow
      split at borrow
      · cases borrow
      obtain ⟨read, read_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      simp only [Option.some.injEq, Prod.mk.injEq] at borrow
      obtain ⟨rfl, rfl, rfl⟩ := borrow
      exact ⟨loans, .refl _, frameTyped, stateTyped, .shared _ _ (reads _ read_eq)⟩
  | mutable =>
      simp only [StaticTyping.borrowedKind, Option.some.injEq] at kind_eq
      subst kind_eq
      obtain ⟨extends_, advanced⟩ := stateTyped.mint (referent := resourceType)
      simp only [borrowRuntimePlace?, Option.bind_eq_bind, Option.bind_some] at borrow
      split at borrow
      · cases borrow
      obtain ⟨read, read_eq, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      obtain ⟨lexical, -, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      obtain ⟨⟨written, writtenState⟩, write, borrow⟩ := Option.bind_eq_some_iff.mp borrow
      simp only [Option.some.injEq, Prod.mk.injEq] at borrow
      obtain ⟨rfl, rfl, rfl⟩ := borrow
      simp only [writeRuntimePlace?, Bool.not_true, Bool.false_eq_true, if_false,
        Array.isEmpty_empty, if_true, writeRoot?, Option.bind_eq_bind,
        Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at write
      obtain ⟨_, -, rfl, rfl⟩ := write
      have hole : HasType unit (loans.extend state.nextLoan resourceType)
          (.loanHole state.nextLoan) resourceType :=
        .hole _ resourceType resourceType (LoanTypes.extend_self _ _ _) rfl
          (reads _ read_eq).inhabited
      refine ⟨_, extends_, (frameTyped.weaken extends_).of_locals rfl, ?_,
        .borrow _ _ _ (LoanTypes.extend_self _ _ _) ((reads _ read_eq).weaken extends_)⟩
      exact (advanced.insert (by simpa [globalKey] using ns_eq) (by simpa [globalKey] using resolves)
        hole).of_eq rfl rfl rfl
  | profile _ => simp [StaticTyping.borrowedKind] at kind_eq

theorem evaluateGlobal_borrow_typed {kind : BorrowKind} {referenceKind : ReferenceKind}
    (ns_eq : unit.namespaces[ns.identity.index]? = some ns)
    (resolves : Resolves ns.tables #[]
      (instantiatedTypeId frame.typeInstantiation resource.typeId) resourceType)
    (kind_eq : StaticTyping.borrowedKind kind = some referenceKind)
    (frameTyped : TypedFrame unit loans ns locals env frame)
    (stateTyped : TypedState unit loans inert state)
    (eval : evaluateGlobalOperation? unit ns resultType site (.borrow kind) #[.typeArg resource]
      arguments frame state = some result) :
    GlobalResultTyped unit loans inert ns locals env frame state
      (.reference referenceKind resourceType) result := by
  rw [evaluateGlobalOperation?_typeArg] at eval
  dsimp only at eval
  split at eval
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
    obtain ⟨_, -, eval⟩ := eval
    split at eval
    · simp only [Option.some.injEq] at eval
      subst eval
      exact ⟨rfl, rfl⟩
    · obtain ⟨node, -, eval⟩ := Option.bind_eq_some_iff.mp eval
      cases node with
      | reference referenceType =>
          obtain ⟨⟨frame', state', value⟩, borrow, eval⟩ := Option.bind_eq_some_iff.mp eval
          simp only [Option.some.injEq] at eval
          subst eval
          exact borrowRuntimePlace?_global_typed ns_eq resolves kind_eq frameTyped stateTyped
            borrow
      | _ => cases eval
  · cases eval

end Global

/-! ## Frames -/

/-- A fresh frame of arguments of the parameters' types, which the checker
makes the leading locals' types, is typed. -/
theorem initialFrame?_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {declaration : FunctionDecl FunctionBody}
    {arguments : Array RuntimeValue} {parameters : List SemTy}
    {instantiation : Array (TypeId × TypeId)} {frame : RuntimeFrame}
    (locals_eq : context.locals = declaration.locals)
    (parameters_eq : ∀ index parameter, parameters[index]? = some parameter →
      context.localType ⟨index⟩ = some parameter)
    (typed : HasTypes unit loans arguments.toList parameters)
    (init : initialFrame? declaration arguments instantiation = some frame) :
    TypedFrame unit loans context.ns context.locals context.env frame := by
  unfold initialFrame? at init
  split at init
  · cases init
  split at init
  · cases init
  simp only [Option.some.injEq] at init
  subst init
  refine ⟨by simp [initialLocals, locals_eq], fun index local_ value local_eq value_eq => ?_⟩
  simp only [initialLocals, Array.getElem?_ofFn] at value_eq
  split at value_eq
  · rename_i within
    simp only [Option.some.injEq] at value_eq
    split at value_eq
    · rename_i argument_bound
      simp only [Option.some.injEq] at value_eq
      subst value_eq
      obtain ⟨parameter, parameter_eq, argument_typed⟩ :=
        typed.get (index := index) (value := arguments[index]) (by simp [argument_bound])
      have local_type := parameters_eq index parameter parameter_eq
      simp only [StaticTyping.Context.localType, StaticTyping.Context.typeOf,
        Option.bind_eq_bind, Option.bind_eq_some_iff] at local_type
      obtain ⟨local', local'_eq, resolved⟩ := local_type
      rw [local_eq, Option.some.injEq] at local'_eq
      subst local'_eq
      exact ⟨parameter, ⟨_, resolved⟩, argument_typed⟩
    · cases value_eq
  · cases value_eq

/-- A nominal type a pattern names is the declaration of the handle the
runtime resolves its name to. -/
theorem nominalTarget_resolveNominal {unit : ValidatedUnit} {ns targetNs : ValidatedNamespace}
    {name : NameId} {declaration : StructDecl} {spelled : QualifiedName} {handle : StructHandle}
    (target : StaticTyping.nominalTarget? unit ns name = some (targetNs, declaration, spelled))
    (resolve : resolveNominal? unit ns name = some handle) :
    unit.namespaces[handle.namespaceId.index]? = some targetNs ∧
      targetNs.structs[handle.structId]? = some declaration ∧
      structName? unit handle = some spelled := by
  simp only [StaticTyping.nominalTarget?, Option.bind_eq_bind, Option.bind_eq_some_iff] at target
  obtain ⟨qualified, qualified_eq, typeId, typeId_eq, targetNs', targetNs_eq, declaration',
    declaration_eq, spelled', spelled_eq, namespaceRef, namespaceRef_eq, target⟩ := target
  simp only [Option.some.injEq, Prod.mk.injEq] at target
  obtain ⟨rfl, rfl, rfl⟩ := target
  simp only [resolveNominal?, qualified_eq, typeId_eq, targetNs_eq, declaration_eq,
    Option.bind_eq_bind, Option.bind_some, pure, Option.some.injEq] at resolve
  subst resolve
  refine ⟨targetNs_eq, declaration_eq, ?_⟩
  simp [structName?, targetNs_eq, declaration_eq, declaredName?, spelled_eq, namespaceRef_eq]

/-- Binding a value of a pattern's type keeps the frame typed: each
variable takes a part of its local's type. -/
theorem bindPatternFuel_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} (unit_eq : context.unit = unit) :
    ∀ (fuel fuel' : Nat) {pattern : PatternId} {type : SemTy} {value : RuntimeValue}
      {frame frame' : RuntimeFrame},
      TypedFrame unit loans context.ns context.locals context.env frame →
      context.patternTypedFuel fuel pattern type = true →
      HasType unit loans value type →
      bindPatternFuel unit context.ns frame fuel' pattern value = some frame' →
      TypedFrame unit loans context.ns context.locals context.env frame' := by
  intro fuel
  induction fuel with
  | zero => intro _ _ _ _ _ _ _ static; simp [StaticTyping.Context.patternTypedFuel] at static
  | succ fuel ih =>
  intro fuel' pattern type value frame frame' frameTyped static typed bind
  cases fuel' with
  | zero => simp [bindPatternFuel] at bind
  | succ fuel' =>
  have row : ∀ (patterns : List PatternId) (types : List SemTy) (values : List RuntimeValue)
      (frame frame' : RuntimeFrame),
      TypedFrame unit loans context.ns context.locals context.env frame →
      (patterns.zip types).all (fun (pattern, type) =>
        context.patternTypedFuel fuel pattern type) = true →
      patterns.length = types.length → HasTypes unit loans values types →
      bindPatternRow (fun pattern value frame =>
        bindPatternFuel unit context.ns frame fuel' pattern value) patterns values frame =
          some frame' →
      TypedFrame unit loans context.ns context.locals context.env frame' := by
    intro patterns
    induction patterns with
    | nil =>
        intro types values frame frame' frameTyped _ length typed bind
        cases types with
        | nil =>
            cases typed
            simp only [bindPatternRow, Option.some.injEq] at bind
            exact bind ▸ frameTyped
        | cons _ _ => cases length
    | cons pattern patterns ih' =>
        intro types values frame frame' frameTyped all length typed bind
        cases types with
        | nil => cases length
        | cons type types =>
            cases typed with
            | cons head tail =>
                simp only [List.zip_cons_cons, List.all_cons, Bool.and_eq_true] at all
                simp only [bindPatternRow, Option.bind_eq_bind, Option.bind_eq_some_iff] at bind
                obtain ⟨bound, bound_eq, bind⟩ := bind
                exact ih' types _ bound frame' (ih fuel' frameTyped all.1 head bound_eq) all.2
                  (by simpa using length) tail bind
  unfold StaticTyping.Context.patternTypedFuel at static
  unfold bindPatternFuel at bind
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at bind
  obtain ⟨⟨loc, typeId, kind⟩, node_eq, bind⟩ := bind
  simp only [node_eq] at static
  cases kind with
  | wildcard =>
      simp only [pure, Option.some.injEq] at bind
      exact bind ▸ frameTyped
  | «variable» localId =>
      simp only [beq_iff_eq] at static
      simp only at bind
      split at bind
      · simp only [pure, Option.some.injEq] at bind
        subst bind
        exact frameTyped.set static typed
      · cases bind
  | tuple elements =>
      cases type with
      | tuple types =>
          simp only [Bool.and_eq_true, beq_iff_eq] at static
          obtain ⟨length, all⟩ := static
          cases value with
          | tuple values =>
              simp only at bind
              split at bind
              · cases bind
              cases typed with
              | tuple _ _ elements_typed =>
                  exact row elements.toList types values.toList frame frame' frameTyped all
                    (by simpa using length) elements_typed bind
          | _ => simp at bind
      | _ => simp at static
  | constructor name instantiations variant fields =>
      simp only at static bind
      split at static
      · rename_i targetNs declaration spelled arguments target _
        simp only [Bool.and_eq_true, beq_iff_eq] at static
        obtain ⟨rfl, static⟩ := static
        split at static
        · rename_i types types_eq
          simp only [Bool.and_eq_true, beq_iff_eq] at static
          obtain ⟨length, all⟩ := static
          cases value with
          | nominal source actualVariant values =>
              simp only [Option.bind_eq_some_iff] at bind
              obtain ⟨expected, expected_eq, bind⟩ := bind
              split at bind
              · cases bind
              rename_i matched
              simp only [Bool.or_eq_true, bne_iff_ne, ne_eq, not_or, Decidable.not_not] at matched
              obtain ⟨⟨rfl, rfl⟩, -⟩ := matched
              rw [unit_eq] at target
              obtain ⟨namespace_eq, declaration_eq, -⟩ :=
                nominalTarget_resolveNominal target expected_eq
              obtain ⟨declared, declared_eq, resolved⟩ := fieldTypes_resolve types_eq
              obtain ⟨_, declared', fieldTypes, -, handle_eq, resolved', fieldsTyped⟩ :=
                typed.nominal_fields
              rw [fieldsOf_handleFields namespace_eq declaration_eq declared_eq] at handle_eq
              simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq
              obtain ⟨rfl, rfl⟩ := handle_eq
              rw [← ResolvesAll.unique resolved resolved'] at fieldsTyped
              exact row fields.toList types values.toList frame frame' frameTyped all
                (by simpa using length) fieldsTyped bind
          | _ => simp at bind
        · cases static
      · cases static
  | literal literal =>
      simp only [Option.bind_eq_some_iff] at bind
      obtain ⟨_, -, bind⟩ := bind
      split at bind
      · simp only [pure, Option.some.injEq] at bind
        exact bind ▸ frameTyped
      · cases bind
  | range lower upper inclusive =>
      have unchanged : frame' = frame := by
        revert bind
        repeat' split
        all_goals simp_all [Option.bind, pure]
      exact unchanged ▸ frameTyped

end LeanerIR
