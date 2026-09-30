-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Semantics.ValueTyping

/-!
# Typed frames and states

The invariants preservation threads through evaluation
(`designs/static-typing.md`, Phase 3), and the operations' lemmas over them.
A frame types its initialized locals at their declared types under the
frame's semantic arguments; a state types global memory at its keys' types
and pending write-backs at their loans' referents, every loan below the
allocation frontier.
-/

namespace LeanerIR

open Validation SemanticOperations

/-- A frame of a body typed under semantic arguments: each initialized local
holds a value of its declared type. -/
def TypedFrame (unit : ValidatedUnit) (loans : LoanTypes) (ns : ValidatedNamespace)
    (locals : Array LocalDecl) (env : Array SemArg) (frame : RuntimeFrame) : Prop :=
  frame.locals.size = locals.size ∧
    ∀ (index : Nat) (declaration : LocalDecl) (value : RuntimeValue),
      locals[index]? = some declaration → frame.locals[index]? = some (some value) →
        ∃ type, Resolves ns.tables env declaration.type.typeId type ∧
          HasType unit loans value type

/-- A state typed under loans: every global slot at its key's type, every
pending write-back past the first `inert` at its loan's referent, every
loan below the frontier. A run carries the write-backs pending at its start
without reading them (`applyPendingFrom` applies only those its callees
add), so they need no type. -/
structure TypedState (unit : ValidatedUnit) (loans : LoanTypes) (inert : Nat)
    (state : RuntimeState) : Prop where
  globals : ∀ slot ∈ state.globals.entries,
    ∃ ns type, unit.namespaces[slot.key.namespaceId.index]? = some ns ∧
      Resolves ns.tables #[] slot.key.typeId type ∧ HasType unit loans slot.value type
  inert_le : inert ≤ state.pending.size
  pending : ∀ entry ∈ state.pending.extract inert state.pending.size, ∃ type,
    loans entry.1 = some type ∧ HasType unit loans entry.2 type
  bounded : ∀ (loan : Nat) (type : SemTy), loans loan = some type → loan < state.nextLoan

/-- A published resource has its key's type. -/
theorem TypedState.lookup {unit : ValidatedUnit} {loans : LoanTypes} {inert : Nat}
    {state : RuntimeState} {key : GlobalKey} {value : RuntimeValue}
    (typed : TypedState unit loans inert state)
    (lookup_eq : state.globals.lookup key = some value) :
    ∃ ns type, unit.namespaces[key.namespaceId.index]? = some ns ∧
      Resolves ns.tables #[] key.typeId type ∧ HasType unit loans value type := by
  unfold GlobalMap.lookup at lookup_eq
  simp only [Option.map_eq_some_iff] at lookup_eq
  obtain ⟨slot, found, rfl⟩ := lookup_eq
  have same : slot.key = key := by simpa using Array.find?_some found
  subst same
  exact typed.globals slot (Array.mem_of_find?_eq_some found)

/-! ## Constants -/

private theorem mapM_some_cons {α β : Type} {f : α → Option β} {x : α} {xs : List α}
    {ys : List β} :
    (x :: xs).mapM f = some ys ↔ ∃ y rest, f x = some y ∧ xs.mapM f = some rest ∧ ys = y :: rest := by
  simp only [List.mapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff, Option.pure_def,
    Option.some.injEq]
  constructor
  · rintro ⟨y, hy, rest, hrest, rfl⟩
    exact ⟨y, rest, hy, hrest, rfl⟩
  · rintro ⟨y, rest, hy, hrest, rfl⟩
    exact ⟨y, hy, rest, hrest, rfl⟩

private theorem attachWith_mapM {α β : Type} {f : α → Option β} :
    ∀ {xs : List α} {P : α → Prop} {H : ∀ x ∈ xs, P x},
    (xs.attachWith P H).mapM (fun x => f x.val) = xs.mapM f := by
  intro xs
  induction xs with
  | nil => intro P H; rfl
  | cons x xs ih =>
      intro P H
      simp only [List.attachWith, List.pmap, List.mapM_cons]
      rw [show (xs.pmap Subtype.mk _) = xs.attachWith P (fun a h => H a (by simp [h])) from rfl,
        ih]

/-- Reifying an attached array of literals is reifying its list. -/
private theorem constValues?_toList {elements : Array ConstValue} {out : Array RuntimeValue}
    (h : elements.attach.mapM (fun ⟨element, _⟩ => constValue? element) = some out) :
    elements.toList.mapM constValue? = some out.toList := by
  rw [Array.mapM_eq_mapM_toList, Array.toList_attach, attachWith_mapM] at h
  cases m : elements.toList.mapM constValue? with
  | none => simp [m] at h
  | some ys =>
      rw [m] at h
      have out_def : ys.toArray = out := by simpa using h
      subst out_def
      simp

private theorem mapM_constValue?_length :
    ∀ {literals : List ConstValue} {values : List RuntimeValue},
      literals.mapM constValue? = some values → values.length = literals.length
  | [], values, h => by cases h; rfl
  | literal :: literals, values, h => by
      obtain ⟨value, rest, -, tail, rfl⟩ := mapM_some_cons.mp h
      simpa using mapM_constValue?_length tail

private theorem sizeOf_lt_of_mem_array {elements : Array ConstValue} {member : ConstValue}
    (member_in : member ∈ elements.toList) (wrap : Array ConstValue → ConstValue)
    (wrap_size : ∀ elements, sizeOf (wrap elements) = 1 + sizeOf elements) :
    sizeOf member < sizeOf (wrap elements) := by
  have member_lt := List.sizeOf_lt_of_mem member_in
  rw [wrap_size]
  cases elements with
  | mk l => simp only [] at member_lt; simp; omega

/-- A value reified from a constant the checker accepts at a type inhabits
it. -/
theorem constValue?_typed {unit : ValidatedUnit} {loans : LoanTypes} :
    ∀ {literal : ConstValue} {type : SemTy} {value : RuntimeValue},
      StaticTyping.constTyped (targetPointerWidth? unit) literal type = true →
        constValue? literal = some value → HasType unit loans value type := by
  suffices general : ∀ (fuel : Nat) (literal : ConstValue), sizeOf literal ≤ fuel →
      ∀ {type : SemTy} {value : RuntimeValue},
        StaticTyping.constTyped (targetPointerWidth? unit) literal type = true →
        constValue? literal = some value → HasType unit loans value type by
    intro literal type value typed value_eq
    exact general _ literal (Nat.le_refl _) typed value_eq
  intro fuel
  induction fuel with
  | zero => intro literal size_le; cases literal <;> simp at size_le
  | succ fuel ih =>
      intro literal size_le type value typed value_eq
      -- Each element of an aggregate literal is below the fuel.
      have elementsTyped : ∀ (literals : List ConstValue), (∀ m ∈ literals, sizeOf m ≤ fuel) →
          ∀ (elementType : SemTy) (values : List RuntimeValue),
            StaticTyping.constsTypedEach (targetPointerWidth? unit) literals elementType = true →
            literals.mapM constValue? = some values →
            HasTypeEach unit loans values elementType := by
        intro literals
        induction literals with
        | nil => intro _ _ values _ mapped; cases mapped; exact .nil
        | cons head rest restIh =>
            intro bounds elementType values all mapped
            obtain ⟨value, others, value_eq, rest_eq, rfl⟩ := mapM_some_cons.mp mapped
            simp only [StaticTyping.constsTypedEach, Bool.and_eq_true] at all
            exact .cons (ih head (bounds head (by simp)) all.1 value_eq)
              (restIh (fun m hm => bounds m (by simp [hm])) elementType others all.2 rest_eq)
      have rowTyped : ∀ (literals : List ConstValue), (∀ m ∈ literals, sizeOf m ≤ fuel) →
          ∀ (types : List SemTy) (values : List RuntimeValue),
            StaticTyping.constsTyped (targetPointerWidth? unit) literals types = true →
            literals.mapM constValue? = some values → HasTypes unit loans values types := by
        intro literals
        induction literals with
        | nil =>
            intro _ types values all mapped
            cases mapped
            cases types with
            | nil => exact .nil
            | cons _ _ => simp [StaticTyping.constsTyped] at all
        | cons head rest restIh =>
            intro bounds types values all mapped
            obtain ⟨value, others, value_eq, rest_eq, rfl⟩ := mapM_some_cons.mp mapped
            cases types with
            | nil => simp [StaticTyping.constsTyped] at all
            | cons type types =>
                simp only [StaticTyping.constsTyped, Bool.and_eq_true] at all
                exact .cons (ih head (bounds head (by simp)) all.1 value_eq)
                  (restIh (fun m hm => bounds m (by simp [hm])) types others all.2 rest_eq)
      cases literal with
      | unit =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          simp only [StaticTyping.constTyped, Bool.or_eq_true, beq_iff_eq] at typed
          rcases typed with rfl | rfl
          · exact .unit
          · exact .unitEmptyTuple
      | bool b =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          simp only [StaticTyping.constTyped, beq_iff_eq] at typed
          subst typed; exact .bool b
      | character c =>
          simp only [StaticTyping.constTyped, Bool.and_eq_true, beq_iff_eq] at typed
          obtain ⟨rfl, valid⟩ := typed
          simp only [constValue?, valid, if_true, Option.some.injEq] at value_eq
          subst value_eq
          exact .character c valid
      | integer i =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          cases type <;> simp only [StaticTyping.constTyped, Bool.false_eq_true] at typed
          exact .integer i _ _ typed
      | address a =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          simp only [StaticTyping.constTyped, beq_iff_eq] at typed
          subst typed; exact .address a
      | string str =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          simp only [StaticTyping.constTyped, beq_iff_eq] at typed
          subst typed; exact .string str
      | bytes bs =>
          simp only [constValue?, Option.some.injEq] at value_eq
          subst value_eq
          simp only [StaticTyping.constTyped, beq_iff_eq] at typed
          subst typed; exact .bytes bs
      | profile _ => simp [constValue?] at value_eq
      | vector elements =>
          simp only [constValue?] at value_eq
          cases out_eq : Array.mapM (fun x => constValue? x.val) elements.attach with
          | none => simp [out_eq] at value_eq
          | some out =>
              rw [out_eq] at value_eq
              have value_def : RuntimeValue.vector out = value := by simpa using value_eq
              subst value_def
              have list_eq := constValues?_toList out_eq
              have bounds : ∀ member ∈ elements.toList, sizeOf member ≤ fuel := fun member member_in =>
                Nat.le_of_lt_succ (Nat.lt_of_lt_of_le
                  (sizeOf_lt_of_mem_array member_in ConstValue.vector (fun _ => by simp)) size_le)
              cases type with
              | vector element length =>
                  simp only [StaticTyping.constTyped, Bool.and_eq_true] at typed
                  have sizes : out.size = elements.size := by
                    simpa using mapM_constValue?_length list_eq
                  refine .vector out element length ?_ (elementsTyped _ bounds element _ typed.1 list_eq)
                  have lengthCheck := typed.2
                  cases length with
                  | none => trivial
                  | some c =>
                      cases c <;> simp at lengthCheck
                      simp only [VectorLengthMatches, sizes, lengthCheck]
                      rfl
              | _ => simp [StaticTyping.constTyped] at typed
      | tuple elements =>
          simp only [constValue?] at value_eq
          cases out_eq : Array.mapM (fun x => constValue? x.val) elements.attach with
          | none => simp [out_eq] at value_eq
          | some out =>
              rw [out_eq] at value_eq
              have value_def : RuntimeValue.tuple out = value := by simpa using value_eq
              subst value_def
              have list_eq := constValues?_toList out_eq
              have bounds : ∀ member ∈ elements.toList, sizeOf member ≤ fuel := fun member member_in =>
                Nat.le_of_lt_succ (Nat.lt_of_lt_of_le
                  (sizeOf_lt_of_mem_array member_in ConstValue.tuple (fun _ => by simp)) size_le)
              cases type with
              | tuple types =>
                  simp only [StaticTyping.constTyped] at typed
                  exact .tuple out types (rowTyped _ bounds types _ typed list_eq)
              | _ => simp [StaticTyping.constTyped] at typed

/-! ## Scalar primitives

A scalar evaluator computes its value at its result type through
`modularInteger` or `checkedInteger`; either yields an integer the type
holds. -/

/-- An integer value a fixed-width type holds. -/
def IntegerAt (type : Ty) (value : RuntimeValue) : Prop :=
  ∃ width signed integer, type = .integer (.bits width) signed ∧ value = .integer integer ∧
    (Ty.integer (.bits width) signed).integerValueFits? integer = some true

private theorem fits_of_bounds {width : Nat} {signed : Bool} {value lower upper : Int}
    (bounds_eq : (Ty.integer (.bits width) signed).integerBounds? = some (lower, upper))
    (lower_le : lower ≤ value) (le_upper : value ≤ upper) :
    (Ty.integer (.bits width) signed).integerValueFits? value = some true := by
  simp only [Ty.integerValueFits?, bounds_eq, bind, Option.bind, Option.some.injEq]
  simp [lower_le, le_upper]

theorem checkedInteger_integerAt {failure : ThrowKind} {type : Ty} {value : Int}
    {out : RuntimeValue} (ok_eq : checkedInteger failure type value = .ok out) :
    IntegerAt type out := by
  unfold checkedInteger at ok_eq
  cases bounds_eq : type.integerBounds? with
  | none => simp [bounds_eq] at ok_eq
  | some bounds =>
      obtain ⟨lower, upper⟩ := bounds
      simp only [bounds_eq] at ok_eq
      cases in_range : lower ≤ value && value ≤ upper with
      | false => simp [in_range] at ok_eq
      | true =>
          simp only [in_range, if_true, Except.ok.injEq] at ok_eq
          subst ok_eq
          simp only [Bool.and_eq_true, decide_eq_true_eq] at in_range
          cases type with
          | integer width signed =>
              cases width with
              | bits width =>
                  exact ⟨width, signed, value, rfl, rfl,
                    fits_of_bounds bounds_eq in_range.1 in_range.2⟩
              | _ => simp [Ty.integerBounds?] at bounds_eq
          | _ => simp [Ty.integerBounds?] at bounds_eq

theorem modularInteger_integerAt {type : Ty} {value : Int} {out : RuntimeValue}
    (mod_eq : modularInteger type value = some out) : IntegerAt type out := by
  unfold modularInteger at mod_eq
  cases type with
  | integer width signed =>
    cases width with
    | bits width =>
      cases zero : width == 0 with
      | true => simp [zero] at mod_eq
      | false =>
          have width_pos : 0 < width := Nat.pos_of_ne_zero (by simpa using zero)
          simp only [zero, Bool.false_eq_true, if_false, Option.some.injEq] at mod_eq
          have two_pow_pos : ∀ n : Nat, (0 : Int) < (2 : Int) ^ n := by
            intro n
            induction n with
            | zero => simp
            | succ m ih => rw [Int.pow_succ]; omega
          have modulus_pos : (0 : Int) < (2 : Int) ^ width := two_pow_pos width
          have residue_nonneg :
              0 ≤ ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width :=
            Int.emod_nonneg _ (by omega)
          have residue_lt :
              ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width
                < (2 : Int) ^ width :=
            Int.emod_lt_of_pos _ modulus_pos
          have split_pow : (2 : Int) ^ width = 2 * (2 : Int) ^ (width - 1) := by
            cases width with
            | zero => omega
            | succ m => rw [Int.pow_succ]; simp; omega
          rw [← mod_eq]
          refine ⟨width, signed, _, rfl, rfl, ?_⟩
          cases signed with
          | false =>
              simp only [Bool.false_and, Bool.false_eq_true, if_false]
              apply fits_of_bounds (lower := 0) (upper := (2 : Int) ^ width - 1)
                (bounds_eq := by simp [Ty.integerBounds?, Nat.pos_iff_ne_zero.mp width_pos])
              · exact residue_nonneg
              · omega
          | true =>
              simp only [Bool.true_and]
              by_cases big :
                  ((value % (2 : Int) ^ width) + (2 : Int) ^ width) % (2 : Int) ^ width
                    ≥ (2 : Int) ^ (width - 1)
              · rw [if_pos (decide_eq_true big)]
                apply fits_of_bounds (lower := -(2 : Int) ^ (width - 1))
                  (upper := (2 : Int) ^ (width - 1) - 1)
                  (bounds_eq := by simp [Ty.integerBounds?, Nat.pos_iff_ne_zero.mp width_pos])
                · omega
                · omega
              · rw [if_neg (by simpa using big)]
                apply fits_of_bounds (lower := -(2 : Int) ^ (width - 1))
                  (upper := (2 : Int) ^ (width - 1) - 1)
                  (bounds_eq := by simp [Ty.integerBounds?, Nat.pos_iff_ne_zero.mp width_pos])
                · omega
                · omega
    | _ => simp at mod_eq
  | _ => simp at mod_eq

/-- An integer a resolved result type holds inhabits the result's semantic
type at the unit's target width. -/
theorem IntegerAt.typed {unit : ValidatedUnit} {loans : LoanTypes} {width : IntWidth}
    {signed : Bool} {resolved : Ty} {value : RuntimeValue}
    (resolve_eq : resolveTargetIntegerType? (targetPointerWidth? unit) (.integer width signed) =
      some resolved)
    (at_ : IntegerAt resolved value) : HasType unit loans value (.integer width signed) := by
  obtain ⟨bits, signed', integer, rfl, rfl, fits⟩ := at_
  refine .integer integer width signed ?_
  cases width with
  | bits w =>
      simp only [resolveTargetIntegerType?, Option.some.injEq, Ty.integer.injEq,
        IntWidth.bits.injEq] at resolve_eq
      obtain ⟨rfl, rfl⟩ := resolve_eq
      simp [StaticTyping.holdsAt, StaticTyping.targetWidth, fits]
  | pointer =>
      simp only [resolveTargetIntegerType?, bind, Option.bind_eq_some_iff] at resolve_eq
      obtain ⟨w, width_eq, supported_eq⟩ := resolve_eq
      split at supported_eq
      · rename_i supported
        simp only [Option.some.injEq, Ty.integer.injEq, IntWidth.bits.injEq] at supported_eq
        obtain ⟨rfl, rfl⟩ := supported_eq
        simp [StaticTyping.holdsAt, StaticTyping.targetWidth, width_eq, supported, fits]
      · cases supported_eq
  | unbounded =>
      simp only [resolveTargetIntegerType?, Option.some.injEq] at resolve_eq
      cases resolve_eq

private theorem modularBinaryInteger_integerAt {type : Ty} {arguments : Array RuntimeValue}
    {value : RuntimeValue} {operation : Int → Int → Int}
    (ok_eq : modularBinaryInteger type arguments operation = some (.ok value)) :
    IntegerAt type value := by
  unfold modularBinaryInteger at ok_eq
  split at ok_eq
  · cases wrapped : modularInteger type _ with
    | none => rw [wrapped] at ok_eq; simp at ok_eq
    | some out =>
        rw [wrapped] at ok_eq
        have : out = value := by simpa using ok_eq
        subst this
        exact modularInteger_integerAt wrapped
  · cases ok_eq

private theorem modularUnaryInteger_integerAt {type : Ty} {arguments : Array RuntimeValue}
    {value : RuntimeValue} {operation : Int → Int}
    (ok_eq : modularUnaryInteger type arguments operation = some (.ok value)) :
    IntegerAt type value := by
  unfold modularUnaryInteger at ok_eq
  split at ok_eq
  · cases wrapped : modularInteger type _ with
    | none => rw [wrapped] at ok_eq; simp at ok_eq
    | some out =>
        rw [wrapped] at ok_eq
        have : out = value := by simpa using ok_eq
        subst this
        exact modularInteger_integerAt wrapped
  · cases ok_eq

private theorem checkedBinaryInteger_integerAt {failure : ThrowKind} {type : Ty}
    {arguments : Array RuntimeValue} {value : RuntimeValue} {operation : Int → Int → Int}
    (ok_eq : checkedBinaryInteger failure type arguments operation = some (.ok value)) :
    IntegerAt type value := by
  unfold checkedBinaryInteger at ok_eq
  split at ok_eq
  · exact checkedInteger_integerAt (Option.some.inj ok_eq)
  · cases ok_eq

private theorem checkedUnaryInteger_integerAt {failure : ThrowKind} {type : Ty}
    {arguments : Array RuntimeValue} {value : RuntimeValue} {operation : Int → Int}
    (ok_eq : checkedUnaryInteger failure type arguments operation = some (.ok value)) :
    IntegerAt type value := by
  unfold checkedUnaryInteger at ok_eq
  split at ok_eq
  · exact checkedInteger_integerAt (Option.some.inj ok_eq)
  · cases ok_eq

private theorem bitwiseBinaryInteger_integerAt {type : Ty} {arguments : Array RuntimeValue}
    {value : RuntimeValue} {operation : Nat → Nat → Nat}
    (ok_eq : bitwiseBinaryInteger type arguments operation = some (.ok value)) :
    IntegerAt type value := by
  unfold bitwiseBinaryInteger at ok_eq
  split at ok_eq
  · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
    obtain ⟨leftBits, -, rightBits, -, out, wrap_eq, out_def⟩ := ok_eq
    have : out = value := by simpa using out_def
    subst this
    exact modularInteger_integerAt wrap_eq
  · cases ok_eq

private theorem bitwiseNotInteger_integerAt {type : Ty} {arguments : Array RuntimeValue}
    {value : RuntimeValue} (ok_eq : bitwiseNotInteger type arguments = some (.ok value)) :
    IntegerAt type value := by
  unfold bitwiseNotInteger at ok_eq
  split at ok_eq
  · split at ok_eq
    · cases ok_eq
    · split at ok_eq
      · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
        obtain ⟨bits, -, out, wrap_eq, out_def⟩ := ok_eq
        have : out = value := by simpa using out_def
        subst this
        exact modularInteger_integerAt wrap_eq
      · cases ok_eq
  · cases ok_eq

private theorem checkedShiftInteger_integerAt {failure : ThrowKind} {left : Bool} {type : Ty}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (ok_eq : checkedShiftInteger failure left type arguments = some (.ok value)) :
    IntegerAt type value := by
  unfold checkedShiftInteger at ok_eq
  split at ok_eq
  · split at ok_eq
    · cases ok_eq
    · split at ok_eq
      · split at ok_eq
        · simp at ok_eq
        · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
          obtain ⟨bits, -, out, wrap_eq, out_def⟩ := ok_eq
          have : out = value := by simpa using out_def
          subst this
          exact modularInteger_integerAt wrap_eq
      · cases ok_eq
  · cases ok_eq

private theorem shiftInteger_integerAt {left : Bool} {type : Ty}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (ok_eq : shiftInteger left type arguments = some (.ok value)) : IntegerAt type value := by
  unfold shiftInteger at ok_eq
  split at ok_eq
  · rename_i inner_eq
    cases ok_eq
    exact checkedShiftInteger_integerAt inner_eq
  · cases ok_eq

private theorem divideIntegers?_integerAt {type : Ty} {arguments : Array RuntimeValue}
    {value : RuntimeValue} (ok_eq : divideIntegers? type arguments = some (.ok value)) :
    IntegerAt type value := by
  rw [divideIntegers?] at ok_eq
  split at ok_eq
  · simp only [bind, Option.bind_eq_some_iff] at ok_eq
    obtain ⟨quotient, -, out, wrap_eq, out_def⟩ := ok_eq
    have : out = value := by simpa using out_def
    subst this
    exact modularInteger_integerAt wrap_eq
  · cases ok_eq

private theorem moduloIntegers?_integerAt {type : Ty} {arguments : Array RuntimeValue}
    {value : RuntimeValue} (ok_eq : moduloIntegers? type arguments = some (.ok value)) :
    IntegerAt type value := by
  rw [moduloIntegers?] at ok_eq
  split at ok_eq
  · simp only [bind, Option.bind_eq_some_iff] at ok_eq
    obtain ⟨remainder, -, out, wrap_eq, out_def⟩ := ok_eq
    have : out = value := by simpa using out_def
    subst this
    exact modularInteger_integerAt wrap_eq
  · cases ok_eq

private theorem checkedDivideIntegers?_integerAt {failure : ThrowKind} {type : Ty}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (ok_eq : checkedDivideIntegers? failure type arguments = some (.ok value)) :
    IntegerAt type value := by
  unfold checkedDivideIntegers? at ok_eq
  split at ok_eq
  · cases ok_eq
  · simp only [bind, pure, Option.bind_eq_some_iff, Option.some.injEq] at ok_eq
    obtain ⟨quotient, -, checked_eq⟩ := ok_eq
    exact checkedInteger_integerAt checked_eq
  · cases ok_eq

private theorem checkedModuloIntegers?_integerAt {failure : ThrowKind} {type : Ty}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (ok_eq : checkedModuloIntegers? failure type arguments = some (.ok value)) :
    IntegerAt type value := by
  unfold checkedModuloIntegers? at ok_eq
  split at ok_eq
  · cases ok_eq
  · simp only [bind, pure, Option.bind_eq_some_iff] at ok_eq
    obtain ⟨quotient, -, checked_eq⟩ := ok_eq
    split at checked_eq
    · cases checked_eq
    · exact checkedInteger_integerAt (Option.some.inj checked_eq)
  · cases ok_eq

/-- A scalar result read at its node: the node is the integer type the
semantic type names, and the value fits it at the target width. -/
private theorem integer_result {unit : ValidatedUnit} {loans : LoanTypes} {tables : Tables}
    {env : Array SemArg} {node resolved : Ty} {type : SemTy} {value : RuntimeValue}
    (node_resolves : NodeResolves tables env node type)
    (resolve_eq : resolveTargetIntegerType? (targetPointerWidth? unit) node = some resolved)
    (at_ : IntegerAt resolved value) : HasType unit loans value type := by
  cases node with
  | integer width signed =>
      simp only [NodeResolves] at node_resolves
      subst node_resolves
      exact IntegerAt.typed resolve_eq at_
  | _ =>
      simp only [resolveTargetIntegerType?, Option.some.injEq] at resolve_eq
      subst resolve_eq
      obtain ⟨_, _, _, shape, -⟩ := at_
      cases shape

/-! ## Rows and elements -/

theorem HasTypeEach.mem {unit : ValidatedUnit} {loans : LoanTypes} {type : SemTy} :
    ∀ {values : List RuntimeValue}, HasTypeEach unit loans values type →
      ∀ value ∈ values, HasType unit loans value type
  | [], _, value, member => by simp at member
  | head :: rest, .cons typed others, value, member => by
      simp only [List.mem_cons] at member
      rcases member with rfl | member
      · exact typed
      · exact HasTypeEach.mem others value member

theorem HasTypeEach.of_forall {unit : ValidatedUnit} {loans : LoanTypes} {type : SemTy} :
    ∀ {values : List RuntimeValue}, (∀ value ∈ values, HasType unit loans value type) →
      HasTypeEach unit loans values type
  | [], _ => .nil
  | head :: rest, all => .cons (all head (by simp))
      (HasTypeEach.of_forall fun value member => all value (by simp [member]))

theorem HasTypes.length_eq {unit : ValidatedUnit} {loans : LoanTypes} :
    ∀ {values : List RuntimeValue} {types : List SemTy}, HasTypes unit loans values types →
      values.length = types.length
  | [], [], .nil => rfl
  | _ :: _, _ :: _, .cons _ rest => by simp [HasTypes.length_eq rest]

/-- A row of values at one type each is a homogeneous row. -/
theorem HasTypes.each {unit : ValidatedUnit} {loans : LoanTypes} {type : SemTy} :
    ∀ {values : List RuntimeValue} {types : List SemTy}, HasTypes unit loans values types →
      types.all (· == type) = true → HasTypeEach unit loans values type
  | [], [], .nil, _ => .nil
  | _ :: _, head :: rest, .cons typed others, all => by
      simp only [List.all_cons, Bool.and_eq_true, beq_iff_eq] at all
      obtain ⟨rfl, all⟩ := all
      exact .cons typed (HasTypes.each others (by simpa using all))

theorem HasTypes.one {unit : ValidatedUnit} {loans : LoanTypes} {first : RuntimeValue}
    {types : List SemTy} (typed : HasTypes unit loans [first] types) :
    ∃ firstType, types = [firstType] ∧ HasType unit loans first firstType := by
  cases typed with
  | cons head rest => cases rest; exact ⟨_, rfl, head⟩

theorem HasTypes.two {unit : ValidatedUnit} {loans : LoanTypes} {first second : RuntimeValue}
    {types : List SemTy} (typed : HasTypes unit loans [first, second] types) :
    ∃ firstType secondType, types = [firstType, secondType] ∧
      HasType unit loans first firstType ∧ HasType unit loans second secondType := by
  cases typed with
  | cons head rest =>
      cases rest with
      | cons second rest => cases rest; exact ⟨_, _, rfl, head, second⟩

theorem HasTypes.three {unit : ValidatedUnit} {loans : LoanTypes}
    {first second third : RuntimeValue} {types : List SemTy}
    (typed : HasTypes unit loans [first, second, third] types) :
    ∃ firstType secondType thirdType, types = [firstType, secondType, thirdType] ∧
      HasType unit loans first firstType ∧ HasType unit loans second secondType ∧
        HasType unit loans third thirdType := by
  cases typed with
  | cons head rest =>
      cases rest with
      | cons second rest =>
          cases rest with
          | cons third rest => cases rest; exact ⟨_, _, _, rfl, head, second, third⟩

/-- The elements of a typed vector. -/
theorem HasType.vector_elements {unit : ValidatedUnit} {loans : LoanTypes}
    {elements : Array RuntimeValue} {element : SemTy} {length : Option ConstValue}
    (typed : HasType unit loans (.vector elements) (.vector element length)) :
    HasTypeEach unit loans elements.toList element := by
  cases typed with
  | vector _ _ _ _ elements => exact elements

/-! ## Array moves

The vector primitives move elements, insert one, or drop some: what they
return is drawn from what they were given. -/

private theorem mem_insertIdxIfInBounds {xs : Array RuntimeValue} {i : Nat}
    {inserted value : RuntimeValue} (member : value ∈ xs.insertIdxIfInBounds i inserted) :
    value = inserted ∨ value ∈ xs := by
  unfold Array.insertIdxIfInBounds at member
  split at member
  · exact Array.mem_insertIdx.mp member
  · exact .inr member

private theorem mem_eraseIdxIfInBounds {xs : Array RuntimeValue} {i : Nat}
    {value : RuntimeValue} (member : value ∈ xs.eraseIdxIfInBounds i) : value ∈ xs := by
  rw [Array.eraseIdxIfInBounds_eq] at member
  split at member
  · exact Array.mem_of_mem_eraseIdx member
  · exact member

private theorem mem_extract {xs : Array RuntimeValue} {i j : Nat} {value : RuntimeValue}
    (member : value ∈ xs.extract i j) : value ∈ xs := by
  obtain ⟨k, bound, rfl⟩ := Array.mem_extract_iff_getElem.mp member
  exact Array.getElem_mem _

private theorem mem_set! {xs : Array RuntimeValue} {i : Nat} {replacement value : RuntimeValue}
    (member : value ∈ xs.set! i replacement) : value ∈ xs ∨ value = replacement := by
  rw [Array.set!_eq_setIfInBounds] at member
  exact Array.mem_or_eq_of_mem_setIfInBounds member

/-- Each element of a result drawn from typed elements and typed values is
typed. -/
private theorem each_of_drawn {unit : ValidatedUnit} {loans : LoanTypes} {type : SemTy}
    {result : Array RuntimeValue} (source : List RuntimeValue)
    (sourceTyped : HasTypeEach unit loans source type)
    (drawn : ∀ value ∈ result, value ∈ source) : HasTypeEach unit loans result.toList type :=
  HasTypeEach.of_forall fun value member =>
    sourceTyped.mem value (drawn value (by simpa using member))

/-- Target resolution changes only a pointer-sized integer, into a
fixed-width one. -/
private theorem resolveTarget_cases {width : Option Nat} {node resolved : Ty}
    (resolve_eq : resolveTargetIntegerType? width node = some resolved) :
    resolved = node ∨ ∃ signed bits, node = .integer .pointer signed ∧
      resolved = .integer (.bits bits) signed := by
  cases node with
  | integer w s =>
      cases w with
      | pointer =>
          simp only [resolveTargetIntegerType?, Option.bind_eq_bind, Option.bind_eq_some_iff]
            at resolve_eq
          obtain ⟨bits, -, supported⟩ := resolve_eq
          split at supported
          · simp only [Option.some.injEq] at supported
            exact .inr ⟨s, bits, rfl, supported.symm⟩
          · cases supported
      | _ => simp only [resolveTargetIntegerType?, Option.some.injEq] at resolve_eq; exact .inl resolve_eq.symm
  | _ => simp only [resolveTargetIntegerType?, Option.some.injEq] at resolve_eq; exact .inl resolve_eq.symm

/-- A resolved node of another shape than a fixed-width integer is the
node itself. -/
private theorem resolveTarget_same {width : Option Nat} {node resolved : Ty}
    (resolve_eq : resolveTargetIntegerType? width node = some resolved)
    (shape : ∀ bits signed, resolved ≠ .integer (.bits bits) signed) : resolved = node := by
  rcases resolveTarget_cases resolve_eq with same | ⟨signed, bits, -, rfl⟩
  · exact same
  · exact absurd rfl (shape bits signed)

private theorem ResolvesAll.two {tables : Tables} {env : Array SemArg} {first second : TypeId}
    {types : List SemTy} (resolved : ResolvesAll tables env [first, second] types) :
    ∃ firstType secondType, types = [firstType, secondType] ∧
      Resolves tables env first firstType ∧ Resolves tables env second secondType := by
  cases resolved with
  | cons head rest =>
      cases rest with
      | cons second rest => cases rest; exact ⟨_, _, rfl, head, second⟩

/-- Any integer inhabits an unbounded integer type. -/
private theorem unbounded_typed {unit : ValidatedUnit} {loans : LoanTypes} {tables : Tables}
    {env : Array SemArg} {signed : Bool} {type : SemTy} (value : Int)
    (node_resolves : NodeResolves tables env (.integer .unbounded signed) type) :
    HasType unit loans (.integer value) type := by
  simp only [NodeResolves] at node_resolves
  subst node_resolves
  exact .integer value _ _ (by simp [StaticTyping.holdsAt, StaticTyping.targetWidth,
    Ty.integerValueFits?])

/-! ## Primitive operations -/

/-- A primitive the checker accepts, evaluated over operands of the types it
was checked at, yields a value of its result's semantic type. -/
theorem evaluatePrimitive_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {ns : ValidatedNamespace} {env : Array SemArg} {resultType : TypeId} {type : SemTy}
    {operation : PrimitiveOperation} {arguments : Array RuntimeValue}
    {operandTypes : List SemTy} {value : RuntimeValue}
    (resolves : Resolves ns.tables env resultType type)
    (typed : StaticTyping.primitiveTyped (targetPointerWidth? unit) operation operandTypes type =
      true)
    (operands : HasTypes unit loans arguments.toList operandTypes)
    (eval : evaluatePrimitiveOperation? ns resultType operation arguments
      (targetPointerWidth? unit) = some (.ok value)) :
    HasType unit loans value type := by
  unfold evaluatePrimitiveOperation? at eval
  cases entry : ns.tables.types[resultType.index]? with
  | none => simp [entry] at eval
  | some node =>
    have nodeResolves := (Resolves.node entry type).mp resolves
    cases resolve_eq : resolveTargetIntegerType? (targetPointerWidth? unit) node with
    | none => simp [entry, resolve_eq] at eval
    | some resolved =>
      simp only [entry, resolve_eq, Option.bind_eq_bind, Option.bind_some] at eval
      have scalar := fun (at_ : IntegerAt resolved value) =>
        integer_result (loans := loans) nodeResolves resolve_eq at_
      cases operation
      all_goals dsimp only at eval
      case add | subtract | multiply => exact scalar (modularBinaryInteger_integerAt eval)
      case checkedAdd | checkedSubtract | checkedMultiply =>
        exact scalar (checkedBinaryInteger_integerAt eval)
      case divide => exact scalar (divideIntegers?_integerAt eval)
      case modulo => exact scalar (moduloIntegers?_integerAt eval)
      case checkedDivide => exact scalar (checkedDivideIntegers?_integerAt eval)
      case checkedModulo => exact scalar (checkedModuloIntegers?_integerAt eval)
      case negate => exact scalar (modularUnaryInteger_integerAt eval)
      case checkedNegate => exact scalar (checkedUnaryInteger_integerAt eval)
      case bitwiseNot => exact scalar (bitwiseNotInteger_integerAt eval)
      case checkedShiftLeft | checkedShiftRight => exact scalar (checkedShiftInteger_integerAt eval)
      case shiftLeft | shiftRight => exact scalar (shiftInteger_integerAt eval)
      case logicalAnd | logicalOr | logicalNot =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        split at eval
        · simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval; exact .bool _
        · cases eval
      case equal | notEqual =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        simp only [equalValues?, notEqualValues?] at eval
        split at eval
        · simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval; exact .bool _
        · cases eval
      case less | greater | lessEqual | greaterEqual =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        unfold compareOrdered at eval
        split at eval <;>
          first
          | (simp only [Option.some.injEq, Except.ok.injEq] at eval; subst eval; exact .bool _)
          | cases eval
      case containsVector =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        unfold containsVector? at eval
        split at eval
        · simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval; exact .bool _
        · cases eval
      case destroyEmptyVector | checkVectorIndex =>
        simp only [StaticTyping.primitiveTyped, StaticTyping.packs, Bool.or_eq_true,
          beq_iff_eq] at typed
        simp only [destroyEmptyVector?, checkVectorIndex?] at eval
        split at eval
        · split at eval
          · simp only [Option.some.injEq, Except.ok.injEq] at eval
            subst eval
            rcases typed with rfl | rfl
            · exact .unit
            · exact .unitEmptyTuple
          · simp at eval
        · cases eval
      case signerAddress =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        split at eval
        · simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval; exact .address _
        · cases eval
      case copyValue | moveValue =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        split at eval
        · rename_i only shape
          simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval
          rw [shape] at operands
          cases operands with
          | cons head _ => exact head
        · cases eval
      case range | implies | equivalent | identical => simp at eval
      case tuple =>
        simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
        subst typed
        simp only [Option.some.injEq, Except.ok.injEq] at eval
        subst eval
        exact .tuple _ _ operands
      case vector =>
        simp only [Option.some.injEq, Except.ok.injEq] at eval
        subst eval
        cases type with
        | vector element length =>
            simp only [StaticTyping.primitiveTyped, Bool.and_eq_true] at typed
            refine .vector _ element length ?_ (operands.each typed.1)
            have sizes := operands.length_eq
            have lengthCheck := typed.2
            cases length with
            | none => trivial
            | some count =>
                cases count <;> simp at lengthCheck
                simp only [VectorLengthMatches]
                rw [lengthCheck, ← sizes, Array.length_toList]
                rfl
        | _ => simp [StaticTyping.primitiveTyped] at typed
      case pushVector =>
        split at eval
        · rename_i elements pushed shape
          simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval
          rw [shape] at operands
          obtain ⟨vectorType, pushedType, rfl, vectorTyped, pushedTyped⟩ := operands.two
          cases vectorType with
          | vector element length =>
              simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
              obtain ⟨rfl, rfl⟩ := typed
              refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
              simp only [Array.toList_push, List.mem_append, List.mem_singleton] at member
              rcases member with member | rfl
              · exact vectorTyped.vector_elements.mem value member
              · exact pushedTyped
          | _ => simp [StaticTyping.primitiveTyped] at typed
        · cases eval
      case insertVector =>
        unfold insertVector? at eval
        split at eval
        · rename_i elements index inserted shape
          split at eval
          · simp at eval
          · simp only [Option.some.injEq, Except.ok.injEq] at eval
            subst eval
            rw [shape] at operands
            obtain ⟨vectorType, indexType, insertedType, rfl, vectorTyped, -, insertedTyped⟩ :=
              operands.three
            cases vectorType with
            | vector element length =>
                simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                obtain ⟨⟨-, rfl⟩, rfl⟩ := typed
                refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
                rcases mem_insertIdxIfInBounds (by simpa using member) with rfl | member
                · exact insertedTyped
                · exact vectorTyped.vector_elements.mem value (by simpa using member)
            | _ => simp [StaticTyping.primitiveTyped] at typed
        · cases eval
      case concatVector =>
        unfold concatVector? at eval
        split at eval
        · rename_i left right shape
          simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval
          rw [shape] at operands
          obtain ⟨leftType, rightType, rfl, leftTyped, rightTyped⟩ := operands.two
          cases leftType with
          | vector element length =>
              cases rightType with
              | vector other otherLength =>
                  simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                  obtain ⟨rfl, rfl⟩ := typed
                  refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
                  simp only [Array.toList_append, List.mem_append] at member
                  rcases member with member | member
                  · exact leftTyped.vector_elements.mem value member
                  · exact rightTyped.vector_elements.mem value member
              | _ => simp [StaticTyping.primitiveTyped] at typed
          | _ => simp [StaticTyping.primitiveTyped] at typed
        · cases eval
      case removeVector =>
        unfold removeVector? at eval
        split at eval
        · rename_i elements index shape
          split at eval
          · simp at eval
          · split at eval
            · rename_i removed removed_eq
              simp only [Option.some.injEq, Except.ok.injEq] at eval
              subst eval
              rw [shape] at operands
              obtain ⟨vectorType, indexType, rfl, vectorTyped, -⟩ := operands.two
              cases vectorType with
              | vector element length =>
                  simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                  obtain ⟨-, rfl⟩ := typed
                  have elementsTyped := vectorTyped.vector_elements
                  refine .tuple _ _ (.cons ?_ (.cons ?_ .nil))
                  · exact elementsTyped.mem _ (by simpa using Array.mem_of_getElem? removed_eq)
                  · refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
                    simp only [Array.toList_eraseIdxIfInBounds] at member
                    exact elementsTyped.mem value (List.mem_of_mem_eraseIdx member)
              | _ => simp [StaticTyping.primitiveTyped] at typed
            · simp at eval
        · cases eval
      case swapVector =>
        unfold swapVector? at eval
        split at eval
        · rename_i elements left right shape
          split at eval
          · simp at eval
          · split at eval
            · rename_i leftValue rightValue left_eq right_eq
              simp only [Option.some.injEq, Except.ok.injEq] at eval
              subst eval
              rw [shape] at operands
              obtain ⟨vectorType, firstType, secondType, rfl, vectorTyped, -, -⟩ := operands.three
              cases vectorType with
              | vector element length =>
                  simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                  obtain ⟨-, rfl⟩ := typed
                  have elementsTyped := vectorTyped.vector_elements
                  refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
                  have drawn : value ∈ elements := by
                    rcases mem_set! (Array.mem_def.mpr member) with member | rfl
                    · rcases mem_set! member with member | rfl
                      · exact member
                      · exact Array.mem_of_getElem? right_eq
                    · exact Array.mem_of_getElem? left_eq
                  exact elementsTyped.mem value (Array.mem_def.mp drawn)
              | _ => simp [StaticTyping.primitiveTyped] at typed
            · simp at eval
        · cases eval
      case reverseSliceVector =>
        unfold reverseSliceVector? at eval
        split at eval
        · rename_i elements start stop shape
          split at eval
          · simp at eval
          · simp only [Option.some.injEq, Except.ok.injEq] at eval
            subst eval
            rw [shape] at operands
            obtain ⟨vectorType, firstType, secondType, rfl, vectorTyped, -, -⟩ := operands.three
            cases vectorType with
            | vector element length =>
                simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                obtain ⟨-, rfl⟩ := typed
                refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
                exact vectorTyped.vector_elements.mem value
                  (Array.mem_def.mp (mem_reverseVectorRange _ _ _ _ _ (Array.mem_def.mpr member)))
            | _ => simp [StaticTyping.primitiveTyped] at typed
        · cases eval
      case slice =>
        unfold sliceVector? at eval
        split at eval
        · rename_i elements start stop shape
          split at eval
          · simp at eval
          · simp only [Option.some.injEq, Except.ok.injEq] at eval
            subst eval
            rw [shape] at operands
            obtain ⟨vectorType, firstType, secondType, rfl, vectorTyped, -, -⟩ := operands.three
            cases vectorType with
            | vector element length =>
                simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                obtain ⟨-, rfl⟩ := typed
                refine .vector _ _ none trivial (HasTypeEach.of_forall fun value member => ?_)
                exact vectorTyped.vector_elements.mem value
                  (Array.mem_def.mp (mem_extract (Array.mem_def.mpr member)))
            | _ => simp [StaticTyping.primitiveTyped] at typed
        · cases eval
      case index =>
        split at eval
        · rename_i elements position shape
          split at eval
          · simp at eval
          · split at eval
            · rename_i found found_eq
              simp only [Option.some.injEq, Except.ok.injEq] at eval
              subst eval
              rw [shape] at operands
              obtain ⟨vectorType, indexType, rfl, vectorTyped, -⟩ := operands.two
              cases vectorType with
              | vector element length =>
                  simp only [StaticTyping.primitiveTyped, Bool.and_eq_true, beq_iff_eq] at typed
                  obtain ⟨-, rfl⟩ := typed
                  exact vectorTyped.vector_elements.mem _
                    (by simpa using Array.mem_of_getElem? found_eq)
              | _ => simp [StaticTyping.primitiveTyped] at typed
            · simp at eval
        · cases eval
      case repeatVector =>
        split at eval
        · rename_i elementId length repeated shape
          split at eval
          · simp at eval
          · rename_i nonnegative
            simp only [Option.some.injEq, Except.ok.injEq] at eval
            subst eval
            have resolved_eq := resolveTarget_same resolve_eq (by intro _ _ h; cases h)
            subst resolved_eq
            simp only [NodeResolves] at nodeResolves
            obtain ⟨element, -, rfl⟩ := nodeResolves
            rw [shape] at operands
            obtain ⟨repeatedType, rfl, repeatedTyped⟩ := operands.one
            simp only [StaticTyping.primitiveTyped, beq_iff_eq] at typed
            subst typed
            refine .vector _ _ _ ?_ (HasTypeEach.of_forall fun value member => ?_)
            · simp only [VectorLengthMatches, Array.size_replicate]
              exact (Int.toNat_of_nonneg (Int.not_lt.mp nonnegative)).symm
            · simp only [Array.toList_replicate, List.mem_replicate] at member
              rw [member.2]; exact repeatedTyped
        · cases eval
      case length =>
        split at eval <;> (try cases eval) <;>
        · simp only [Option.map_eq_map, Option.map_eq_some_iff] at eval
          obtain ⟨out, out_eq, ok_eq⟩ := eval
          simp only [Except.ok.injEq] at ok_eq
          subst ok_eq
          split at out_eq
          · simp only [Option.some.injEq] at out_eq
            subst out_eq
            rename_i signed resolved_shape
            have resolved_eq := resolveTarget_same resolve_eq (by intro _ _ h; cases h)
            subst resolved_eq
            exact unbounded_typed _ nodeResolves
          · exact scalar (modularInteger_integerAt out_eq)
      case compare =>
        split at eval
        · simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval
          simp only [StaticTyping.primitiveTyped, Bool.and_eq_true] at typed
          obtain ⟨⟨below, zero⟩, above⟩ := typed
          cases type with
          | integer width signed =>
              refine .integer _ width signed ?_
              simp only [StaticTyping.holds] at below zero above
              generalize RuntimeValue.order _ _ _ = ordering
              cases ordering <;> simp [orderValue, below, zero, above]
          | _ => simp [StaticTyping.holds] at below
        · cases eval
      case bitwiseOr | bitwiseAnd | bitwiseXor =>
        unfold bitwiseBinary at eval
        split at eval
        · rename_i left right shape
          simp only [Option.some.injEq, Except.ok.injEq] at eval
          subst eval
          simp only [StaticTyping.primitiveTyped, Bool.or_eq_true, Bool.and_eq_true,
            beq_iff_eq] at typed
          rcases typed with ⟨rfl, -⟩ | ⟨integerResult, rfl⟩
          · exact .bool _
          · rw [shape] at operands
            obtain ⟨_, _, same, leftTyped, -⟩ := operands.two
            simp only [List.cons.injEq] at same
            obtain ⟨rfl, -⟩ := same
            cases leftTyped <;> simp [StaticTyping.fixedInteger] at integerResult
        · exact scalar (bitwiseBinaryInteger_integerAt eval)
      case cast | checkedCast =>
        split at eval
        · rename_i integer shape
          split at eval
          · have node_eq := resolveTarget_same resolve_eq (by intro _ _ h; cases h)
            subst node_eq
            simp only [NodeResolves] at nodeResolves
            subst nodeResolves
            split at eval
            · simp at eval
            · rename_i valid
              simp only [Option.some.injEq, Except.ok.injEq] at eval
              subst eval
              simp only [Bool.or_eq_true, decide_eq_true_eq, not_or, Bool.not_eq_true] at valid
              exact .character _ (by simpa using valid.2)
          · first
            | exact scalar (checkedInteger_integerAt (Option.some.inj eval))
            | (simp only [Option.map_eq_map, Option.map_eq_some_iff] at eval
               obtain ⟨out, out_eq, ok_eq⟩ := eval
               simp only [Except.ok.injEq] at ok_eq
               subst ok_eq
               exact scalar (modularInteger_integerAt out_eq))
        · first
          | exact scalar (checkedInteger_integerAt (Option.some.inj eval))
          | (simp only [Option.map_eq_map, Option.map_eq_some_iff] at eval
             obtain ⟨out, out_eq, ok_eq⟩ := eval
             simp only [Except.ok.injEq] at ok_eq
             subst ok_eq
             exact scalar (modularInteger_integerAt out_eq))
        · cases eval
      case indexOfVector =>
        split at eval
        · rename_i elementIds
          have node_eq := resolveTarget_same resolve_eq (by intro _ _ h; cases h)
          subst node_eq
          simp only [NodeResolves] at nodeResolves
          obtain ⟨types, resolvesAll, rfl⟩ := nodeResolves
          split at eval
          · rename_i first index ids_eq
            rw [ids_eq] at resolvesAll
            obtain ⟨firstType, indexSemType, rfl, -, indexResolves⟩ := resolvesAll.two
            simp only [StaticTyping.primitiveTyped] at typed
            cases firstType <;> simp at typed
            · simp only [Option.bind_eq_some_iff] at eval
              obtain ⟨indexType, indexType_eq, eval⟩ := eval
              obtain ⟨indexNode, indexEntry, indexResolve⟩ := indexType_eq
              have indexNodeResolves := (Resolves.node indexEntry _).mp indexResolves
              unfold indexOfVector? at eval
              split at eval
              · split at eval
                · simp only [bind, Option.bind_some, Option.some.injEq, Except.ok.injEq] at eval
                  subst eval
                  have same := resolveTarget_same indexResolve (by intro _ _ h; cases h)
                  subst same
                  exact .tuple _ _ (.cons (.bool _) (.cons (unbounded_typed _ indexNodeResolves) .nil))
                · simp only [bind, Option.bind_eq_some_iff, Option.some.injEq, Except.ok.injEq]
                    at eval
                  obtain ⟨position, position_eq, rfl⟩ := eval
                  exact .tuple _ _ (.cons (.bool _) (.cons (integer_result indexNodeResolves
                    indexResolve (modularInteger_integerAt position_eq)) .nil))
              · cases eval
          · cases eval
        · cases eval
      case overflowingAdd | overflowingSubtract | overflowingMultiply =>
        unfold overflowingBinaryInteger at eval
        split at eval
        · rename_i elementIds
          have node_eq := resolveTarget_same resolve_eq (by intro _ _ h; cases h)
          subst node_eq
          simp only [NodeResolves] at nodeResolves
          obtain ⟨types, resolvesAll, rfl⟩ := nodeResolves
          split at eval
          · rename_i valueId overflowId ids_eq
            rw [ids_eq] at resolvesAll
            obtain ⟨valueSemType, overflowSemType, rfl, valueResolves, -⟩ := resolvesAll.two
            simp only [StaticTyping.primitiveTyped] at typed
            cases overflowSemType <;> simp at typed
            · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
              obtain ⟨valueType, valueType_eq, overflowType, -, eval⟩ := eval
              obtain ⟨valueNode, valueEntry, valueResolve⟩ := valueType_eq
              have valueNodeResolves := (Resolves.node valueEntry _).mp valueResolves
              split at eval
              · cases eval
              · split at eval
                · simp only [Option.bind_eq_some_iff] at eval
                  obtain ⟨bounds, -, out, out_eq, eval⟩ := eval
                  simp only [Option.some.injEq, Except.ok.injEq] at eval
                  subst eval
                  exact .tuple _ _ (.cons (integer_result valueNodeResolves valueResolve
                    (modularInteger_integerAt out_eq)) (.cons (.bool _) .nil))
                · cases eval
          · cases eval
        · cases eval

/-! ## Nominal values -/

/-- A struct the runtime resolves, the checker's lookup finds: the
declaration the handle names. -/
theorem declarationTarget_of_resolve {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {ns : ValidatedNamespace} {reference : QualifiedRef} {handle : StructHandle}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (resolve : resolveStruct? unit sourceNamespace reference = some handle) :
    ∃ targetNs declaration,
      StaticTyping.declarationTarget? unit ns reference = some (targetNs, declaration) ∧
      unit.namespaces[handle.namespaceId.index]? = some targetNs ∧
      targetNs.structs[handle.structId]? = some declaration := by
  simp only [resolveStruct?, ns_eq, Option.bind_eq_bind, Option.bind_some, pure] at resolve
  obtain ⟨qualified, qualified_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  split at resolve
  · cases resolve
  rename_i same
  obtain ⟨typeId, typeId_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  obtain ⟨targetNs, targetNs_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  obtain ⟨declaration, declaration_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
  simp only [Option.some.injEq] at resolve
  subst resolve
  refine ⟨targetNs, declaration, ?_, targetNs_eq, declaration_eq⟩
  simp only [StaticTyping.declarationTarget?, qualified_eq, same, typeId_eq, targetNs_eq,
    declaration_eq, Option.bind_eq_bind, Option.bind_some, if_false, Bool.false_eq_true]

/-- A struct the checker resolves is the declaration the runtime's handle
names, under the name the runtime spells it by. -/
theorem structTarget_handle {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {ns targetNs : ValidatedNamespace} {reference : QualifiedRef} {declaration : StructDecl}
    {spelled : QualifiedName} {handle : StructHandle}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (resolve : resolveStruct? unit sourceNamespace reference = some handle) :
    unit.namespaces[handle.namespaceId.index]? = some targetNs ∧
      targetNs.structs[handle.structId]? = some declaration ∧
      structName? unit handle = some spelled := by
  obtain ⟨targetNs', declaration', declared, targetNs_eq, declaration_eq⟩ :=
    declarationTarget_of_resolve ns_eq resolve
  simp only [StaticTyping.structTarget?, declared, Option.bind_eq_bind, Option.bind_some]
    at target
  obtain ⟨spelled', spelled_eq, target⟩ := Option.bind_eq_some_iff.mp target
  obtain ⟨namespaceRef, namespaceRef_eq, target⟩ := Option.bind_eq_some_iff.mp target
  simp only [Option.some.injEq, Prod.mk.injEq] at target
  obtain ⟨rfl, rfl, rfl⟩ := target
  refine ⟨targetNs_eq, declaration_eq, ?_⟩
  simp [structName?, targetNs_eq, declaration_eq, declaredName?, spelled_eq, namespaceRef_eq]

/-- The fields the checker reads for a variant are the fields the runtime's
handle reads. -/
theorem fieldsOf_handleFields {unit : ValidatedUnit} {handle : StructHandle}
    {targetNs : ValidatedNamespace} {declaration : StructDecl} {variant : Option String}
    {fields : Array FieldDecl}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (fields_eq : StaticTyping.fieldsOf targetNs declaration variant = some fields) :
    handleFields? unit handle variant = some (targetNs, fields) := by
  unfold handleFields?
  simp only [namespace_eq, declaration_eq, Option.bind_eq_bind, Option.bind_some]
  cases variant with
  | none =>
      simp only [StaticTyping.fieldsOf] at fields_eq
      split at fields_eq
      · rename_i empty
        simp only [Option.some.injEq] at fields_eq
        simp [empty, fields_eq]
      · cases fields_eq
  | some variantName =>
      simp only [StaticTyping.fieldsOf, Option.bind_eq_bind, Option.bind_eq_some_iff,
        Option.some.injEq] at fields_eq
      obtain ⟨declared, declared_eq, rfl⟩ := fields_eq
      simp [declared_eq]

/-- Field types resolved by the checker, elementwise. -/
theorem fieldTypes_resolve {targetNs : ValidatedNamespace} {declaration : StructDecl}
    {variant : Option String} {arguments : List SemArg} {fieldTypes : List SemTy}
    (fieldTypes_eq : StaticTyping.fieldTypes? targetNs declaration variant arguments =
      some fieldTypes) :
    ∃ fields, StaticTyping.fieldsOf targetNs declaration variant = some fields ∧
      ResolvesAll targetNs.tables arguments.toArray (fields.toList.map (·.type.typeId))
        fieldTypes := by
  simp only [StaticTyping.fieldTypes?, Option.bind_eq_bind, Option.bind_eq_some_iff]
    at fieldTypes_eq
  obtain ⟨fields, fields_eq, resolved⟩ := fieldTypes_eq
  refine ⟨fields, fields_eq, ResolvesAll.of_mapM (fuel := targetNs.tables.types.size + 1) ?_⟩
  rw [List.mapM_map]
  exact resolved

/-- A constructed nominal value inhabits the nominal type of the arguments
its fields are typed under. -/
theorem constructNominal_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {variant : Option String} {fields : Array RuntimeValue}
    {declaration : StructDecl} {spelled : QualifiedName} {arguments : List SemArg}
    {fieldTypes : List SemTy} {value : RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (fieldTypes_eq : StaticTyping.fieldTypes? targetNs declaration variant arguments =
      some fieldTypes)
    (fieldsTyped : HasTypes unit loans fields.toList fieldTypes)
    (construct : constructNominal? unit sourceNamespace reference variant fields = some value) :
    HasType unit loans value (.nominal spelled arguments) := by
  simp only [constructNominal?, Option.bind_eq_bind, Option.bind_eq_some_iff] at construct
  obtain ⟨handle, resolve, expected, -, construct⟩ := construct
  split at construct
  · cases construct
  · simp only [pure, Option.some.injEq] at construct
    subst construct
    obtain ⟨namespace_eq, declaration_eq, name_eq⟩ := structTarget_handle ns_eq target resolve
    obtain ⟨declared, declared_eq, resolved⟩ := fieldTypes_resolve fieldTypes_eq
    exact .nominal handle variant fields spelled arguments targetNs declared fieldTypes name_eq
      (fieldsOf_handleFields namespace_eq declaration_eq declared_eq) resolved fieldsTyped

theorem ResolvesAll.unique {tables : Tables} {env : Array SemArg} :
    ∀ {typeIds : List TypeId} {left right : List SemTy}, ResolvesAll tables env typeIds left →
      ResolvesAll tables env typeIds right → left = right
  | [], [], [], .nil, .nil => rfl
  | _ :: _, _ :: _, _ :: _, .cons head rest, .cons head' rest' => by
      rw [Resolves.unique head head', ResolvesAll.unique rest rest']

/-- A typed nominal value is its declaration's, its fields typed at the
declared field types under the type's arguments. -/
theorem HasType.nominal_fields {unit : ValidatedUnit} {loans : LoanTypes}
    {source : StructHandle} {variant : Option String} {fields : Array RuntimeValue}
    {name : QualifiedName} {arguments : List SemArg}
    (typed : HasType unit loans (.nominal source variant fields) (.nominal name arguments)) :
    ∃ declaringNamespace declared fieldTypes, structName? unit source = some name ∧
      handleFields? unit source variant = some (declaringNamespace, declared) ∧
      ResolvesAll declaringNamespace.tables arguments.toArray
        (declared.toList.map (·.type.typeId)) fieldTypes ∧
      HasTypes unit loans fields.toList fieldTypes := by
  cases typed with
  | nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq declaration_eq resolved
      fieldsTyped => exact ⟨declaringNamespace, declared, fieldTypes, name_eq, declaration_eq,
        resolved, fieldsTyped⟩

/-- A value at a shared reference type is a value of the referent. -/
theorem HasType.shared_referent {unit : ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {referent : SemTy}
    (typed : HasType unit loans value (.reference .shared referent)) :
    HasType unit loans value referent := by
  cases typed with
  | shared _ _ typed => exact typed
  | hole loan loanReferent _ loan_eq same inhabited =>
      exact .hole loan loanReferent referent loan_eq (by simpa [SemTy.unshared] using same)
        inhabited

/-- A destructured nominal value's fields are typed at the field types the
checker resolved. -/
theorem destructNominal_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {variant : Option String} {value : RuntimeValue}
    {declaration : StructDecl} {spelled : QualifiedName} {arguments : List SemArg}
    {fieldTypes : List SemTy} {fields : Array RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (fieldTypes_eq : StaticTyping.fieldTypes? targetNs declaration variant arguments =
      some fieldTypes)
    (operand : HasType unit loans value (.nominal spelled arguments))
    (destruct : destructNominal? unit sourceNamespace reference variant value = some fields) :
    HasTypes unit loans fields.toList fieldTypes := by
  unfold destructNominal? at destruct
  split at destruct
  · rename_i actualSource actualVariant actualFields
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at destruct
    obtain ⟨handle, resolve, expected, -, destruct⟩ := destruct
    split at destruct
    · rename_i matched
      simp only [Option.some.injEq] at destruct
      subst destruct
      simp only [Bool.and_eq_true, beq_iff_eq] at matched
      obtain ⟨⟨rfl, rfl⟩, -⟩ := matched
      obtain ⟨namespace_eq, declaration_eq, -⟩ := structTarget_handle ns_eq target resolve
      obtain ⟨declared, declared_eq, resolved⟩ := fieldTypes_resolve fieldTypes_eq
      obtain ⟨declaringNamespace, declared', fieldTypes', -, handle_eq, resolved', typed⟩ :=
        operand.nominal_fields
      rw [fieldsOf_handleFields namespace_eq declaration_eq declared_eq] at handle_eq
      simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq
      obtain ⟨rfl, rfl⟩ := handle_eq
      rw [ResolvesAll.unique resolved resolved'] at *
      exact typed
    · cases destruct
  · cases destruct

/-! ## Field selection -/

theorem HasTypes.get {unit : ValidatedUnit} {loans : LoanTypes} :
    ∀ {values : List RuntimeValue} {types : List SemTy}, HasTypes unit loans values types →
      ∀ {index : Nat} {value : RuntimeValue}, values[index]? = some value →
        ∃ type, types[index]? = some type ∧ HasType unit loans value type
  | [], [], .nil, _, _, found => by simp at found
  | _ :: _, type :: _, .cons head rest, index, value, found => by
      cases index with
      | zero => simp only [List.getElem?_cons_zero, Option.some.injEq] at found; subst found
                exact ⟨type, rfl, head⟩
      | succ index => simpa using HasTypes.get rest (by simpa using found)

theorem ResolvesAll.get {tables : Tables} {env : Array SemArg} :
    ∀ {typeIds : List TypeId} {types : List SemTy}, ResolvesAll tables env typeIds types →
      ∀ {index : Nat} {type : SemTy}, types[index]? = some type →
        ∃ typeId, typeIds[index]? = some typeId ∧ Resolves tables env typeId type
  | [], [], .nil, _, _, found => by simp at found
  | typeId :: _, _ :: _, .cons head rest, index, type, found => by
      cases index with
      | zero => simp only [List.getElem?_cons_zero, Option.some.injEq] at found; subst found
                exact ⟨typeId, rfl, head⟩
      | succ index => simpa using ResolvesAll.get rest (by simpa using found)

private theorem mem_filterMapM {α β : Type} {f : α → Option (Option β)} :
    ∀ {xs : List α} {ys : List β}, xs.filterMapM f = some ys →
      ∀ x ∈ xs, ∀ y, f x = some (some y) → y ∈ ys := by
  intro xs
  induction xs with
  | nil => intro ys _ x member; simp at member
  | cons head rest ih =>
      intro ys mapped x member y image
      simp only [List.filterMapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff] at mapped
      obtain ⟨headImage, headImage_eq, mapped⟩ := mapped
      simp only [List.mem_cons] at member
      cases headImage with
      | none =>
          rcases member with rfl | member
          · rw [image] at headImage_eq; cases headImage_eq
          · exact ih (by simpa using mapped) x member y image
      | some headValue =>
          simp only [Option.bind_eq_some_iff, pure, Option.some.injEq] at mapped
          obtain ⟨tail, tail_eq, rfl⟩ := mapped
          rcases member with rfl | member
          · rw [image] at headImage_eq
            simp only [Option.some.injEq] at headImage_eq
            subst headImage_eq
            simp
          · exact List.mem_cons_of_mem _ (ih tail_eq x member y image)

private theorem filterMapM_some {α β : Type} {f : α → Option (Option β)} :
    ∀ {xs : List α} {ys : List β}, xs.filterMapM f = some ys →
      ∀ x ∈ xs, ∃ image, f x = some image := by
  intro xs
  induction xs with
  | nil => intro ys _ x member; simp at member
  | cons head rest ih =>
      intro ys mapped x member
      simp only [List.filterMapM_cons, Option.bind_eq_bind, Option.bind_eq_some_iff] at mapped
      obtain ⟨headImage, headImage_eq, mapped⟩ := mapped
      simp only [List.mem_cons] at member
      rcases member with rfl | member
      · exact ⟨headImage, headImage_eq⟩
      cases headImage with
      | none => exact ih (by simpa using mapped) x member
      | some _ =>
          simp only [Option.bind_eq_some_iff] at mapped
          obtain ⟨tail, tail_eq, -⟩ := mapped
          exact ih tail_eq x member

/-- A field found in a variant's fields has the variant's field type. -/
private theorem fieldType_of_find {targetNs : ValidatedNamespace} {declaration : StructDecl}
    {variant : Option String} {fields : Array FieldDecl} {field : FieldDecl}
    {fieldName : String} {arguments : List SemArg}
    (fields_eq : StaticTyping.fieldsOf targetNs declaration variant = some fields)
    (find_eq : fields.toList.find? (fun field =>
      (targetNs.tables.names[field.name.index]?).any (·.name == fieldName)) = some field) :
    StaticTyping.fieldType? targetNs declaration variant arguments fieldName =
      StaticTyping.resolveIn targetNs arguments.toArray field.type.typeId := by
  rw [Array.find?_toList] at find_eq
  simp only [StaticTyping.fieldType?, fields_eq, find_eq, Option.bind_eq_bind, Option.bind_some]

/-- The fields a handle reads: its declaration's, in its declaring namespace. -/
private theorem handleFields_namespace {unit : ValidatedUnit} {handle : StructHandle}
    {variant : Option String} {targetNs targetNs' : ValidatedNamespace}
    {fields : Array FieldDecl}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (handle_eq : handleFields? unit handle variant = some (targetNs', fields)) :
    targetNs' = targetNs := by
  simp only [handleFields?, namespace_eq, Option.bind_eq_bind, Option.bind_some,
    Option.bind_eq_some_iff] at handle_eq
  obtain ⟨declaration, -, handle_eq⟩ := handle_eq
  cases variant with
  | none =>
      simp only at handle_eq
      split at handle_eq
      · simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq
        exact handle_eq.1.symm
      · cases handle_eq
  | some _ =>
      simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at handle_eq
      obtain ⟨_, -, rfl, -⟩ := handle_eq
      rfl

/-- The fields a handle reads for a variant: the variant the checker finds
by that name, among the declaration's. -/
private theorem handleFields_variant {unit : ValidatedUnit} {handle : StructHandle}
    {targetNs : ValidatedNamespace} {declaration : StructDecl} {variantName : String}
    {targetNs' : ValidatedNamespace} {fields : Array FieldDecl}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (handle_eq : handleFields? unit handle (some variantName) = some (targetNs', fields)) :
    targetNs' = targetNs ∧ ∃ chosen spelled, chosen ∈ declaration.variants.toList ∧
      targetNs.tables.names[chosen.name.index]? = some spelled ∧
      StaticTyping.fieldsOf targetNs declaration (some spelled.name) = some fields ∧
      fields = chosen.fields := by
  simp only [handleFields?, namespace_eq, declaration_eq, Option.bind_eq_bind, Option.bind_some,
    Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at handle_eq
  obtain ⟨chosen, chosen_eq, rfl, rfl⟩ := handle_eq
  refine ⟨rfl, chosen, ?_⟩
  have named := List.find?_some chosen_eq
  simp only [Option.any_eq_true, beq_iff_eq] at named
  obtain ⟨spelled, spelled_eq, spelled_name⟩ := named
  refine ⟨spelled, List.mem_of_find?_eq_some chosen_eq, spelled_eq, ?_, rfl⟩
  simp only [StaticTyping.fieldsOf, spelled_name, chosen_eq, Option.bind_eq_bind,
    Option.bind_some]

/-- The type of a field in whichever variant a value has resolves, and is
among the types the checker requires to agree. -/
theorem fieldTypesAcross_mem {unit : ValidatedUnit} {handle : StructHandle}
    {targetNs targetNs' : ValidatedNamespace} {declaration : StructDecl} {variant : Option String}
    {fields : Array FieldDecl} {field : FieldDecl} {fieldName : String}
    {arguments : List SemArg} {types : List SemTy}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (handle_eq : handleFields? unit handle variant = some (targetNs', fields))
    (find_eq : fields.toList.find? (fun field =>
      (targetNs'.tables.names[field.name.index]?).any (·.name == fieldName)) = some field)
    (types_eq : StaticTyping.fieldTypesAcross? targetNs declaration arguments fieldName = some types) :
    ∃ type, StaticTyping.resolveIn targetNs arguments.toArray field.type.typeId = some type ∧
      type ∈ types := by
  cases variant with
  | none =>
      have fields_eq : StaticTyping.fieldsOf targetNs declaration none = some fields ∧
          targetNs' = targetNs ∧ declaration.variants.isEmpty = true := by
        simp only [handleFields?, namespace_eq, declaration_eq, Option.bind_eq_bind,
          Option.bind_some] at handle_eq
        split at handle_eq
        · rename_i empty
          simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq
          obtain ⟨rfl, rfl⟩ := handle_eq
          exact ⟨by simp [StaticTyping.fieldsOf, empty], rfl, empty⟩
        · cases handle_eq
      obtain ⟨fields_eq, rfl, empty⟩ := fields_eq
      simp only [StaticTyping.fieldTypesAcross?, empty, if_true,
        fieldType_of_find fields_eq find_eq, Option.map_eq_some_iff] at types_eq
      obtain ⟨type, resolved, rfl⟩ := types_eq
      exact ⟨type, resolved, List.mem_singleton_self type⟩
  | some variantName =>
      obtain ⟨rfl, chosen, spelled, chosen_mem, spelled_eq, fields_eq, rfl⟩ :=
        handleFields_variant namespace_eq declaration_eq handle_eq
      have nonempty : declaration.variants.isEmpty = false := by
        cases variants : declaration.variants with
        | mk list =>
            rw [variants] at chosen_mem
            cases list with
            | nil => simp at chosen_mem
            | cons _ _ => rfl
      simp only [StaticTyping.fieldTypesAcross?, nonempty, Bool.false_eq_true, if_false]
        at types_eq
      have has_field : chosen.fields.any (fun field =>
          (targetNs'.tables.names[field.name.index]?).any (·.name == fieldName)) = true := by
        rw [← Array.any_toList]
        have matched := List.find?_some find_eq
        exact List.any_eq_true.mpr ⟨field, List.mem_of_find?_eq_some find_eq, matched⟩
      have image_eq : ∀ image, (do
          let variantName ← targetNs'.tables.names[chosen.name.index]?
          let fields ← StaticTyping.fieldsOf targetNs' declaration (some variantName.name)
          if fields.any fun field =>
              (targetNs'.tables.names[field.name.index]?).any (·.name == fieldName) then
            some <$> StaticTyping.fieldType? targetNs' declaration (some variantName.name)
              arguments fieldName
          else some none) = some image →
          ∃ type, StaticTyping.resolveIn targetNs' arguments.toArray field.type.typeId =
            some type ∧ image = some type := by
        intro image image_eq
        simp only [spelled_eq, fields_eq, has_field, if_true,
          fieldType_of_find fields_eq find_eq, Option.bind_eq_bind, Option.bind_some,
          Option.map_eq_map, Option.map_eq_some_iff] at image_eq
        obtain ⟨type, resolved, rfl⟩ := image_eq
        exact ⟨type, resolved, rfl⟩
      obtain ⟨image, found⟩ := filterMapM_some types_eq chosen chosen_mem
      obtain ⟨type, resolved, rfl⟩ := image_eq image found
      exact ⟨type, resolved, mem_filterMapM types_eq chosen chosen_mem type found⟩

theorem ResolvesAll.getId {tables : Tables} {env : Array SemArg} :
    ∀ {typeIds : List TypeId} {types : List SemTy}, ResolvesAll tables env typeIds types →
      ∀ {index : Nat} {typeId : TypeId}, typeIds[index]? = some typeId →
        ∃ type, types[index]? = some type ∧ Resolves tables env typeId type
  | [], [], .nil, _, _, found => by simp at found
  | _ :: _, type :: _, .cons head rest, index, typeId, found => by
      cases index with
      | zero => simp only [List.getElem?_cons_zero, Option.some.injEq] at found; subst found
                exact ⟨type, rfl, head⟩
      | succ index => simpa using ResolvesAll.getId rest (by simpa using found)

/-- The resolved type at a declared field's position is its field's type. -/
private theorem declared_field_resolves {tables : Tables} {env : Array SemArg}
    {declared : Array FieldDecl} {fieldTypes : List SemTy} {index : Nat} {field : FieldDecl}
    (resolved : ResolvesAll tables env (declared.toList.map (·.type.typeId)) fieldTypes)
    (field_eq : declared[index]? = some field) :
    ∃ fieldType, fieldTypes[index]? = some fieldType ∧
      Resolves tables env field.type.typeId fieldType :=
  resolved.getId (by simp [field_eq])

/-- The type of a nominal value's field at its position is one of the types
the checker requires to agree for that field. -/
private theorem field_type_mem {unit : ValidatedUnit} {targetNs : ValidatedNamespace}
    {declaration : StructDecl} {arguments : List SemArg} {fieldName : String}
    {types : List SemTy} {handle : StructHandle} {variant : Option String} {index : Nat}
    {declaringNamespace : ValidatedNamespace} {declared : Array FieldDecl}
    {fieldTypes : List SemTy} {fieldType : SemTy} {field : FieldDecl}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (types_eq : StaticTyping.fieldTypesAcross? targetNs declaration arguments fieldName = some types)
    (handle_eq : handleFields? unit handle variant = some (declaringNamespace, declared))
    (resolved : ResolvesAll declaringNamespace.tables arguments.toArray
      (declared.toList.map (·.type.typeId)) fieldTypes)
    (field_eq : declared[index]? = some field)
    (find_eq : declared.toList.find? (fun field =>
      (declaringNamespace.tables.names[field.name.index]?).any (·.name == fieldName)) = some field)
    (fieldType_eq : fieldTypes[index]? = some fieldType) :
    fieldType ∈ types := by
  have same := handleFields_namespace namespace_eq handle_eq
  subst same
  obtain ⟨fieldType', fieldType'_eq, typeResolves⟩ := declared_field_resolves resolved field_eq
  rw [fieldType_eq, Option.some.injEq] at fieldType'_eq
  subst fieldType'_eq
  obtain ⟨type, resolvedIn, member⟩ :=
    fieldTypesAcross_mem namespace_eq declaration_eq handle_eq find_eq types_eq
  have agree := Resolves.unique typeResolves ⟨_, resolvedIn⟩
  subst agree
  exact member

/-- A nominal value's field at its position has one of the types the checker
requires to agree for that field. -/
private theorem nominal_field_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {declaration : StructDecl} {spelled : QualifiedName}
    {arguments : List SemArg} {fieldName : String} {types : List SemTy}
    {handle : StructHandle} {variant : Option String} {values : Array RuntimeValue}
    {index : Nat} {value : RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (resolve : resolveStruct? unit sourceNamespace reference = some handle)
    (types_eq : StaticTyping.fieldTypesAcross? targetNs declaration arguments fieldName = some types)
    (operand : HasType unit loans (.nominal handle variant values) (.nominal spelled arguments))
    {targetNs' : ValidatedNamespace} {fields : Array FieldDecl} {field : FieldDecl}
    (handle_eq' : handleFields? unit handle variant = some (targetNs', fields))
    (field_eq : fields[index]? = some field)
    (find_eq : fields.toList.find? (fun field =>
      (targetNs'.tables.names[field.name.index]?).any (·.name == fieldName)) = some field)
    (value_eq : values[index]? = some value) :
    ∃ type, type ∈ types ∧ HasType unit loans value type := by
  obtain ⟨declaringNamespace, declared, fieldTypes, -, handle_eq, resolved, typed⟩ :=
    operand.nominal_fields
  rw [handle_eq] at handle_eq'
  simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq'
  obtain ⟨rfl, rfl⟩ := handle_eq'
  obtain ⟨fieldType, fieldType_eq, valueTyped⟩ := typed.get (by simpa using value_eq)
  obtain ⟨namespace_eq, declaration_eq, -⟩ := structTarget_handle ns_eq target resolve
  exact ⟨fieldType, field_type_mem namespace_eq declaration_eq types_eq handle_eq resolved
    field_eq find_eq fieldType_eq, valueTyped⟩

private theorem selected_some {operandType result : SemTy} {field : SemTy → Bool}
    (selected : StaticTyping.Context.selected operandType result field = true) :
    ∃ type, field type = true := by
  unfold StaticTyping.Context.selected at selected
  simp only [Bool.or_eq_true] at selected
  rcases selected with direct | through
  · exact ⟨_, direct⟩
  · split at through
    · exact ⟨_, (Bool.and_eq_true _ _ ▸ through).2⟩
    · cases through

private theorem selected_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {operandType result : SemTy} {field : SemTy → Bool} {spelled : QualifiedName}
    {arguments : List SemArg} {source : StructHandle} {variant : Option String}
    {values : Array RuntimeValue} {value : RuntimeValue}
    (operand_eq : StaticTyping.Context.nominalOperand operandType = some (spelled, arguments))
    (selected : StaticTyping.Context.selected operandType result field = true)
    (typed : HasType unit loans (.nominal source variant values) operandType)
    (field_typed : HasType unit loans (.nominal source variant values)
        (.nominal spelled arguments) →
      ∃ type, HasType unit loans value type ∧ ∀ expected, field expected = true → type = expected) :
    HasType unit loans value result := by
  have nominal : HasType unit loans (.nominal source variant values) (.nominal spelled arguments) ∧
      ∀ kind referent, operandType = .reference kind referent → kind = .shared := by
    cases operandType with
    | nominal name arguments' =>
        simp only [StaticTyping.Context.nominalOperand, Option.some.injEq, Prod.mk.injEq]
          at operand_eq
        obtain ⟨rfl, rfl⟩ := operand_eq
        exact ⟨typed, fun _ _ same => by cases same⟩
    | reference kind referent =>
        cases referent <;> simp only [StaticTyping.Context.nominalOperand, reduceCtorEq]
          at operand_eq
        simp only [Option.some.injEq, Prod.mk.injEq] at operand_eq
        obtain ⟨rfl, rfl⟩ := operand_eq
        cases kind with
        | mutable => cases typed
        | shared =>
            refine ⟨typed.shared_referent, fun kind _ same => ?_⟩
            cases same
            rfl
    | _ => simp [StaticTyping.Context.nominalOperand] at operand_eq
  obtain ⟨type, typed', agrees⟩ := field_typed nominal.1
  unfold StaticTyping.Context.selected at selected
  simp only [Bool.or_eq_true] at selected
  rcases selected with direct | through
  · rw [← agrees _ direct]
    exact typed'
  · split at through
    · rename_i kind _ resultKind referent
      simp only [Bool.and_eq_true] at through
      have shared := nominal.2 kind _ rfl
      subst shared
      cases resultKind
      · rw [← agrees _ through.2]
        exact .shared _ _ typed'
      · exact absurd through.1 (by decide)
    · cases through

/-- A selected field has the node's type: the field's, or a shared
reference to it when the operand is a reference. -/
theorem select_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {declaration : StructDecl} {spelled : QualifiedName}
    {arguments : List SemArg} {fieldName : String} {types : List SemTy}
    {operandType result : SemTy} {operand value : RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (operand_eq : StaticTyping.Context.nominalOperand operandType = some (spelled, arguments))
    (types_eq : StaticTyping.fieldTypesAcross? targetNs declaration arguments fieldName = some types)
    (selected : StaticTyping.Context.selected operandType result
      (fun type => types.all (· == type)) = true)
    (typed : HasType unit loans operand operandType)
    (eval : evaluateDataOperation? unit sourceNamespace (.select reference fieldName) #[operand] =
      some value) :
    HasType unit loans value result := by
  cases operand with
  | nominal source variant values =>
      unfold evaluateDataOperation? at eval
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
      obtain ⟨handle, resolve, eval⟩ := eval
      split at eval
      · cases eval
      rename_i same
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
      subst same
      obtain ⟨index, index_eq, value_eq⟩ := Option.bind_eq_some_iff.mp eval
      obtain ⟨_, _, _, handle_eq, field_eq, find_eq⟩ := handleFieldIndex?_find index_eq
      refine selected_typed operand_eq selected typed fun nominal => ?_
      obtain ⟨type, member, typed⟩ := nominal_field_typed ns_eq target resolve types_eq nominal
        handle_eq field_eq find_eq value_eq
      exact ⟨type, typed, fun _ all => beq_iff_eq.mp (List.all_eq_true.mp all type member)⟩
  | _ => simp [evaluateDataOperation?] at eval

private theorem inj_of_nodup_map {α β : Type} {f : α → β} :
    ∀ {list : List α}, (list.map f).Nodup →
      ∀ {x y : α}, x ∈ list → y ∈ list → f x = f y → x = y
  | [], _, _, _, member, _, _ => by simp at member
  | head :: rest, nodup, x, y, x_mem, y_mem, same => by
      rw [List.map_cons, List.nodup_cons] at nodup
      obtain ⟨fresh, nodup⟩ := nodup
      rcases List.mem_cons.mp x_mem with rfl | x_rest <;>
        rcases List.mem_cons.mp y_mem with rfl | y_rest
      · rfl
      · exact absurd (same ▸ List.mem_map_of_mem y_rest) fresh
      · exact absurd (same.symm ▸ List.mem_map_of_mem x_rest) fresh
      · exact inj_of_nodup_map nodup x_rest y_rest same

/-- With distinct variant names, a variant's name reads its fields. -/
theorem handleFields_of_distinct {unit : ValidatedUnit} {handle : StructHandle}
    {targetNs : ValidatedNamespace} {declaration : StructDecl} {variant : VariantDecl}
    {name : QualifiedName}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (distinct : StaticTyping.variantsDistinct targetNs declaration = true)
    (member : variant ∈ declaration.variants)
    (name_eq : targetNs.tables.names[variant.name.index]? = some name) :
    handleFields? unit handle (some name.name) = some (targetNs, variant.fields) := by
  have member := Array.mem_toList_iff.mpr member
  have found : (declaration.variants.toList.find? fun candidate =>
      (targetNs.tables.names[candidate.name.index]?).any (·.name == name.name)).isSome := by
    exact List.find?_isSome.mpr ⟨variant, member, by simp [name_eq]⟩
  obtain ⟨chosen, chosen_eq⟩ := Option.isSome_iff_exists.mp found
  have chosen_mem := List.mem_of_find?_eq_some chosen_eq
  have named := List.find?_some chosen_eq
  simp only [Option.any_eq_true, beq_iff_eq] at named
  obtain ⟨chosenName, chosenName_eq, same⟩ := named
  have : chosen = variant := inj_of_nodup_map (of_decide_eq_true distinct) chosen_mem member
    (by simp [chosenName_eq, name_eq, same])
  subst this
  simp only [handleFields?, namespace_eq, declaration_eq, chosen_eq, Option.bind_eq_bind,
    Option.bind_some]

/-- A nominal value's field at its position, in the variant the value holds,
has the checker's type of that variant's field. -/
private theorem nominal_variant_field_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {targetNs : ValidatedNamespace} {declaration : StructDecl} {spelled : QualifiedName}
    {arguments : List SemArg} {variantName fieldName : String} {fieldType : SemTy}
    {handle : StructHandle} {values : Array RuntimeValue} {index : Nat} {value : RuntimeValue}
    {fields : Array FieldDecl} {field : FieldDecl}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (fieldType_eq : StaticTyping.fieldType? targetNs declaration (some variantName) arguments
      fieldName = some fieldType)
    (operand : HasType unit loans (.nominal handle (some variantName) values)
      (.nominal spelled arguments))
    (handle_eq' : handleFields? unit handle (some variantName) = some (targetNs, fields))
    (field_eq : fields[index]? = some field)
    (find_eq : fields.toList.find? (fun field =>
      (targetNs.tables.names[field.name.index]?).any (·.name == fieldName)) = some field)
    (value_eq : values[index]? = some value) :
    HasType unit loans value fieldType := by
  obtain ⟨declaringNamespace, declared, fieldTypes, -, handle_eq, resolved, typed⟩ :=
    operand.nominal_fields
  rw [handle_eq] at handle_eq'
  simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq'
  obtain ⟨rfl, rfl⟩ := handle_eq'
  obtain ⟨fieldType', fieldType'_eq, valueTyped⟩ := typed.get (by simpa using value_eq)
  obtain ⟨fieldType'', fieldType''_eq, typeResolves⟩ := declared_field_resolves resolved field_eq
  rw [fieldType'_eq, Option.some.injEq] at fieldType''_eq
  subst fieldType''_eq
  simp only [StaticTyping.fieldType?, Option.bind_eq_bind, Option.bind_eq_some_iff] at fieldType_eq
  obtain ⟨fields', fields'_eq, field', field'_eq, resolvedIn⟩ := fieldType_eq
  have handle_eq'' := fieldsOf_handleFields namespace_eq declaration_eq fields'_eq
  rw [handle_eq] at handle_eq''
  simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq''
  obtain ⟨-, rfl⟩ := handle_eq''
  rw [← Array.find?_toList, find_eq, Option.some.injEq] at field'_eq
  subst field'_eq
  rwa [Resolves.unique typeResolves ⟨_, resolvedIn⟩] at valueTyped

/-- A field selected across the listed variants has the node's type. -/
theorem selectVariants_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {declaration : StructDecl} {spelled : QualifiedName}
    {arguments : List SemArg} {fields : Array (String × String)}
    {operandType result : SemTy} {operand value : RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (distinct : StaticTyping.variantsDistinct targetNs declaration = true)
    (operand_eq : StaticTyping.Context.nominalOperand operandType = some (spelled, arguments))
    (selected : StaticTyping.Context.selected operandType result (fun type => fields.all
      fun selected => StaticTyping.fieldType? targetNs declaration (some selected.1) arguments
        selected.2 == some type) = true)
    (typed : HasType unit loans operand operandType)
    (eval : evaluateDataOperation? unit sourceNamespace (.selectVariants reference fields)
      #[operand] = some value) :
    HasType unit loans value result := by
  unfold evaluateDataOperation? at eval
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
  obtain ⟨handle, resolve, choices, choices_eq, eval⟩ := eval
  cases operand with
  | nominal source variant values =>
      cases variant with
      | none => simp [selectNominalVariantFieldAt?] at eval
      | some actualVariant =>
      simp only [selectNominalVariantFieldAt?] at eval
      split at eval
      · cases eval
      rename_i same
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
      subst same
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
      obtain ⟨⟨choiceName, index⟩, choice_eq, value_eq⟩ := eval
      obtain ⟨targetNs', declaration', chosen, name, fieldName, field, namespace_eq,
        declaration_eq, chosen_mem, name_eq, choiceName_eq, pair_mem, field_eq, find_eq⟩ :=
        variantFieldChoices?_mem choices_eq (Array.mem_of_find?_eq_some choice_eq)
      obtain ⟨namespace_eq', declaration_eq', -⟩ := structTarget_handle ns_eq target resolve
      rw [namespace_eq'] at namespace_eq
      cases namespace_eq
      rw [declaration_eq'] at declaration_eq
      cases declaration_eq
      have actual : choiceName = actualVariant := by
        simpa using Array.find?_some choice_eq
      simp only at choiceName_eq
      subst actual
      subst choiceName_eq
      have handle_eq := handleFields_of_distinct namespace_eq' declaration_eq' distinct chosen_mem
        name_eq
      have type_of : ∀ {expected}, (fields.all fun selected => StaticTyping.fieldType? targetNs
          declaration (some selected.1) arguments selected.2 == some expected) = true →
          StaticTyping.fieldType? targetNs declaration (some name.name) arguments fieldName =
            some expected := by
        intro expected all
        simpa using Array.all_eq_true'.mp all _ pair_mem
      obtain ⟨type, some_field⟩ := selected_some selected
      refine selected_typed operand_eq selected typed fun nominal => ?_
      refine ⟨type, nominal_variant_field_typed namespace_eq' declaration_eq' (type_of some_field)
        nominal handle_eq field_eq find_eq value_eq, fun expected all => ?_⟩
      exact Option.some.inj ((type_of some_field).symm.trans (type_of all))
  | _ => simp [selectNominalVariantFieldAt?] at eval

/-- A variant test is a Boolean. -/
theorem testVariants_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {reference : QualifiedRef} {variants : Array String}
    {arguments : Array RuntimeValue} {value : RuntimeValue}
    (eval : evaluateDataOperation? unit sourceNamespace (.testVariants reference variants)
      arguments = some value) :
    HasType unit loans value .bool := by
  unfold evaluateDataOperation? at eval
  generalize arguments.toList = operands at eval
  rcases operands with _ | ⟨operand, _ | ⟨_, _⟩⟩
  · cases eval
  · cases operand <;> try (cases eval; done)
    dsimp only at eval
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
    obtain ⟨_, -, eval⟩ := eval
    split at eval
    · cases eval
    · cases eval
      exact .bool _
  · simp at eval

/-- A discriminant is one the declaration declares, which the check makes
fit the node's type. -/
theorem discriminant_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns : ValidatedNamespace} {reference : QualifiedRef}
    {result : SemTy} {arguments : Array RuntimeValue} {value : RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (checked : (match StaticTyping.declarationTarget? unit ns reference with
      | some (_, declaration) => declaration.variants.all fun variant =>
          variant.discriminant.all (StaticTyping.holds (targetPointerWidth? unit) result ·)
      | none => true) = true)
    (eval : evaluateDataOperation? unit sourceNamespace (.discriminant reference) arguments =
      some value) :
    HasType unit loans value result := by
  unfold evaluateDataOperation? at eval
  generalize arguments.toList = operands at eval
  rcases operands with _ | ⟨operand, _ | ⟨_, _⟩⟩
  · cases eval
  · cases operand <;> try (cases eval; done)
    rename_i actualVariant _
    cases actualVariant
    · cases eval
    dsimp only at eval
    simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
    obtain ⟨handle, resolve, eval⟩ := eval
    split at eval
    · cases eval
    simp only [Option.map_eq_map, Option.map_eq_some_iff] at eval
    obtain ⟨discriminant, discriminant_eq, rfl⟩ := eval
    obtain ⟨targetNs, declaration, declared, namespace_eq, declaration_eq⟩ :=
      declarationTarget_of_resolve ns_eq resolve
    simp only [declared] at checked
    simp only [variantDiscriminant?, namespace_eq, declaration_eq, Option.bind_eq_bind,
      Option.bind_some, Option.bind_eq_some_iff] at discriminant_eq
    obtain ⟨chosen, chosen_eq, discriminant_eq⟩ := discriminant_eq
    have holds := Array.all_eq_true'.mp checked chosen (Array.mem_of_find?_eq_some chosen_eq)
    rw [discriminant_eq] at holds
    simp only [Option.all_some] at holds
    cases result with
    | integer width signed => exact .integer _ _ _ holds
    | _ => simp [StaticTyping.holds] at holds
  · simp at eval

theorem HasTypes.set {unit : ValidatedUnit} {loans : LoanTypes} :
    ∀ {values : List RuntimeValue} {types : List SemTy}, HasTypes unit loans values types →
      ∀ {index : Nat} {type : SemTy} {value : RuntimeValue}, types[index]? = some type →
        HasType unit loans value type → HasTypes unit loans (values.set index value) types
  | [], [], .nil, _, _, _, found, _ => by simp at found
  | _ :: _, _ :: _, .cons head rest, index, type, value, found, typed => by
      cases index with
      | zero =>
          simp only [List.getElem?_cons_zero, Option.some.injEq] at found
          subst found
          exact .cons typed rest
      | succ index => exact .cons head (HasTypes.set rest (by simpa using found) typed)

/-- An updated field keeps the nominal value's type: the replacement has
every type the field has across variants. -/
theorem updateField_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {sourceNamespace : NamespaceId} {ns targetNs : ValidatedNamespace}
    {reference : QualifiedRef} {declaration : StructDecl} {spelled : QualifiedName}
    {arguments : List SemArg} {fieldName : String} {types : List SemTy}
    {operand replacement value : RuntimeValue}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.structTarget? unit ns reference = some (targetNs, declaration, spelled))
    (types_eq : StaticTyping.fieldTypesAcross? targetNs declaration arguments fieldName = some types)
    (typed : HasType unit loans operand (.nominal spelled arguments))
    (replacement_typed : ∀ type ∈ types, HasType unit loans replacement type)
    (eval : evaluateDataOperation? unit sourceNamespace (.updateField reference fieldName)
      #[operand, replacement] = some value) :
    HasType unit loans value (.nominal spelled arguments) := by
  cases operand with
  | nominal source variant values =>
      unfold evaluateDataOperation? at eval
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at eval
      obtain ⟨handle, resolve, eval⟩ := eval
      split at eval
      · cases eval
      rename_i same
      simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
      subst same
      obtain ⟨index, index_eq, eval⟩ := Option.bind_eq_some_iff.mp eval
      split at eval
      · simp only [Option.some.injEq] at eval
        subst eval
        obtain ⟨_, _, field, handle_eq', field_eq, find_eq⟩ := handleFieldIndex?_find index_eq
        cases typed with
        | nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq resolved
            fieldsTyped =>
          rw [handle_eq] at handle_eq'
          simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq'
          obtain ⟨rfl, rfl⟩ := handle_eq'
          rename_i bound
          obtain ⟨fieldType, fieldType_eq, -⟩ := fieldsTyped.get (index := index)
            (value := values[index]) (by simp [bound])
          obtain ⟨namespace_eq, declaration_eq, -⟩ := structTarget_handle ns_eq target resolve
          have member := field_type_mem namespace_eq declaration_eq types_eq handle_eq resolved
            field_eq find_eq fieldType_eq
          refine .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq
            resolved ?_
          simpa [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds] using
            fieldsTyped.set fieldType_eq (replacement_typed fieldType member)
      · cases eval
  | _ => simp [evaluateDataOperation?] at eval

/-! ## Results -/

/-- Packed results have the type the checker packs their types into. -/
theorem packResults_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {values : Array RuntimeValue} {results : List SemTy} {type : SemTy}
    (typed : HasTypes unit loans values.toList results)
    (packed : StaticTyping.packs results type = true) :
    HasType unit loans (packResults values) type := by
  rcases values with ⟨_ | ⟨first, _ | ⟨second, rest⟩⟩⟩
  · cases typed
    simp only [StaticTyping.packs, Bool.or_eq_true, beq_iff_eq] at packed
    rcases packed with rfl | rfl
    · exact .unit
    · exact .unitEmptyTuple
  · obtain ⟨result, rfl, head⟩ := typed.one
    simp only [StaticTyping.packs, beq_iff_eq] at packed
    subst packed
    exact head
  · cases typed with
    | cons head typed =>
        cases typed with
        | cons second' rest' =>
            simp only [StaticTyping.packs, beq_iff_eq] at packed
            subst packed
            exact .tuple _ _ (.cons head (.cons second' rest'))

/-- A fall-through value unpacked into the declared results has their
types. -/
theorem unpackFallthrough_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {value : RuntimeValue} {results : List SemTy} {type : SemTy} {values : Array RuntimeValue}
    (typed : HasType unit loans value type)
    (packed : StaticTyping.packs results type = true)
    (unpacked : unpackFallthrough results.length value = some values) :
    HasTypes unit loans values.toList results := by
  rcases results with _ | ⟨result, _ | ⟨second, rest⟩⟩
  · simp only [List.length_nil, unpackFallthrough] at unpacked
    split at unpacked
    · cases unpacked
      exact .nil
    · cases unpacked
  · simp only [List.length_singleton, unpackFallthrough, Option.some.injEq] at unpacked
    subst unpacked
    simp only [StaticTyping.packs, beq_iff_eq] at packed
    subst packed
    exact .cons typed .nil
  · simp only [StaticTyping.packs, beq_iff_eq] at packed
    subst packed
    simp only [List.length_cons, unpackFallthrough] at unpacked
    split at unpacked
    · split at unpacked
      · cases unpacked
        cases typed with
        | tuple _ _ elements => exact elements
      · cases unpacked
    · cases unpacked

/-! ## Places -/

/-- A projection path from a value of `root` reaches a position of `type`:
what it reads there has `type`, and writing a value of `type` there keeps a
value of `root`. -/
structure PathTyped (unit : ValidatedUnit) (loans : LoanTypes) (value : RuntimeValue)
    (root : SemTy) (projections : List RuntimeProjection) (type : SemTy) : Prop where
  read : ∀ {part}, readProjections? value projections = some part → HasType unit loans part type
  write : ∀ {replacement updated}, HasType unit loans replacement type →
    writeProjections? value projections replacement = some updated →
      HasType unit loans updated root

theorem PathTyped.nil {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {type : SemTy} (typed : HasType unit loans value type) :
    PathTyped unit loans value type [] type where
  read read := by
    simp only [readProjections?, Option.some.injEq] at read
    exact read ▸ typed
  write replacement write := by
    simp only [writeProjections?, Option.some.injEq] at write
    exact write ▸ replacement

/-- A path to a shared reference reaches the value it observes. -/
theorem PathTyped.unshare {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {root type : SemTy} {projections : List RuntimeProjection}
    (path : PathTyped unit loans value root projections (.reference .shared type)) :
    PathTyped unit loans value root projections type where
  read read := (path.read read).shared_referent
  write replacement write := path.write (.shared _ _ replacement) write

theorem PathTyped.snoc {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {root type next : SemTy} {projections : List RuntimeProjection}
    {projection : RuntimeProjection}
    (path : PathTyped unit loans value root projections type)
    (step : ∀ part, readProjections? value projections = some part →
      PathTyped unit loans part type [projection] next) :
    PathTyped unit loans value root (projections ++ [projection]) next where
  read read := by
    rw [readProjections?_append] at read
    obtain ⟨part, part_eq, read⟩ := Option.bind_eq_some_iff.mp read
    exact (step part part_eq).read read
  write replacement write := by
    rw [writeProjections?_append] at write
    obtain ⟨part, part_eq, write⟩ := Option.bind_eq_some_iff.mp write
    obtain ⟨written, written_eq, write⟩ := Option.bind_eq_some_iff.mp write
    exact path.write ((step part part_eq).write replacement written_eq) write

theorem PathTyped.field {unit : ValidatedUnit} {loans : LoanTypes} {source : StructHandle}
    {variant : Option String} {fields : Array RuntimeValue} {name : QualifiedName}
    {arguments : List SemArg} {declaringNamespace : ValidatedNamespace}
    {declared : Array FieldDecl} {fieldTypes : List SemTy} {index : Nat} {fieldType : SemTy}
    (name_eq : structName? unit source = some name)
    (handle_eq : handleFields? unit source variant = some (declaringNamespace, declared))
    (resolved : ResolvesAll declaringNamespace.tables arguments.toArray
      (declared.toList.map (·.type.typeId)) fieldTypes)
    (typed : HasTypes unit loans fields.toList fieldTypes)
    (fieldType_eq : fieldTypes[index]? = some fieldType) :
    PathTyped unit loans (.nominal source variant fields) (.nominal name arguments)
      [.field index] fieldType where
  read read := by
    simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
    obtain ⟨part, part_eq, read⟩ := read
    simp only [Option.some.injEq] at read
    subst read
    obtain ⟨type, type_eq, part_typed⟩ := typed.get (by simpa using part_eq)
    rw [fieldType_eq, Option.some.injEq] at type_eq
    exact type_eq ▸ part_typed
  write replacement write := by
    simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at write
    obtain ⟨_, -, _, written_eq, rfl⟩ := write
    subst written_eq
    exact .nominal _ _ _ _ _ declaringNamespace declared fieldTypes name_eq handle_eq resolved
      (by simpa [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds] using
        typed.set fieldType_eq replacement)

theorem PathTyped.vectorIndex {unit : ValidatedUnit} {loans : LoanTypes}
    {elements : Array RuntimeValue} {element : SemTy} {length : Option ConstValue} {index : Nat}
    (typed : HasType unit loans (.vector elements) (.vector element length)) :
    PathTyped unit loans (.vector elements) (.vector element length) [.index index] element where
  read read := by
    simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
    obtain ⟨part, part_eq, read⟩ := read
    simp only [Option.some.injEq] at read
    subst read
    exact typed.vector_elements.mem part (by simpa using Array.mem_of_getElem? part_eq)
  write replacement write := by
    simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at write
    obtain ⟨_, -, _, written_eq, rfl⟩ := write
    subst written_eq
    cases typed with
    | vector _ _ _ length_matches elements =>
        refine .vector _ _ _ (by simpa using length_matches) (HasTypeEach.of_forall ?_)
        intro value member
        simp only [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds] at member
        rcases List.mem_or_eq_of_mem_set member with member | rfl
        · exact elements.mem value member
        · exact replacement

theorem PathTyped.tupleIndex {unit : ValidatedUnit} {loans : LoanTypes}
    {elements : Array RuntimeValue} {types : List SemTy} {index : Nat} {type : SemTy}
    (typed : HasType unit loans (.tuple elements) (.tuple types))
    (type_eq : types[index]? = some type) :
    PathTyped unit loans (.tuple elements) (.tuple types) [.index index] type where
  read read := by
    simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
    obtain ⟨part, part_eq, read⟩ := read
    simp only [Option.some.injEq] at read
    subst read
    cases typed with
    | tuple _ _ elements_typed =>
        obtain ⟨type', type'_eq, part_typed⟩ := elements_typed.get (by simpa using part_eq)
        rw [type_eq, Option.some.injEq] at type'_eq
        exact type'_eq ▸ part_typed
  write replacement write := by
    simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff, pure,
      Option.some.injEq] at write
    obtain ⟨_, -, _, written_eq, rfl⟩ := write
    subst written_eq
    cases typed with
    | tuple _ _ elements_typed =>
        exact .tuple _ _ (by simpa [Array.set!_eq_setIfInBounds, Array.toList_setIfInBounds]
          using elements_typed.set type_eq replacement)

theorem PathTyped.downcast {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {type : SemTy} {variant : String} (typed : HasType unit loans value type) :
    PathTyped unit loans value type [.downcast variant] type where
  read read := by
    simp only [readProjections?] at read
    split at read
    · split at read
      · simp only [Option.some.injEq] at read
        subst read
        exact typed
      · cases read
    · cases read
  write replacement write := by
    simp only [writeProjections?] at write
    split at write
    · split at write
      · simp only [Option.some.injEq] at write
        exact write ▸ replacement
      · cases write
    · cases write

theorem PathTyped.deref {unit : ValidatedUnit} {loans : LoanTypes} {loan : Nat}
    {current : RuntimeValue} {type : SemTy}
    (typed : HasType unit loans (.borrow loan current) (.reference .mutable type)) :
    PathTyped unit loans (.borrow loan current) (.reference .mutable type) [.deref] type where
  read read := by
    simp only [readProjections?, Option.some.injEq] at read
    subst read
    cases typed with
    | borrow _ _ _ _ current_typed => exact current_typed
  write replacement write := by
    simp only [writeProjections?, Option.bind_eq_bind, Option.bind_some, pure,
      Option.some.injEq] at write
    subst write
    cases typed with
    | borrow _ _ _ loan_eq _ => exact .borrow _ _ _ loan_eq replacement

theorem mem_of_mem_extract {α : Type} {xs : Array α} {a : α} {start stop : Nat}
    (member : a ∈ xs.extract start stop) : a ∈ xs := by
  obtain ⟨_, _, rfl⟩ := Array.mem_extract_iff_getElem.mp member
  exact Array.getElem_mem _

private theorem subsliceBounds?_some {size start stop first last : Nat} {fromEnd : Bool}
    (bounds : subsliceBounds? size start stop fromEnd = some (first, last)) :
    first = start ∧ first ≤ last ∧ last ≤ size ∧
      last = (if fromEnd then size - stop else stop) := by
  unfold subsliceBounds? at bounds
  cases fromEnd with
  | true =>
      simp only [if_true] at bounds
      split at bounds
      · simp only [Option.some.injEq, Prod.mk.injEq] at bounds
        obtain ⟨rfl, rfl⟩ := bounds
        refine ⟨rfl, by omega, by omega, by simp⟩
      · cases bounds
  | false =>
      simp only [Bool.false_eq_true, if_false] at bounds
      split at bounds
      · rename_i within
        simp only [Bool.and_eq_true, decide_eq_true_eq] at within
        simp only [Option.some.injEq, Prod.mk.injEq] at bounds
        obtain ⟨rfl, rfl⟩ := bounds
        exact ⟨rfl, within.1, within.2, by simp⟩
      · cases bounds

/-- A vector's length matches its subslice's static length. -/
private theorem subsliceLength_matches {length : Option ConstValue} {size start stop first last : Nat}
    {fromEnd : Bool} (matches_ : VectorLengthMatches length size)
    (bounds : subsliceBounds? size start stop fromEnd = some (first, last)) :
    VectorLengthMatches (StaticTyping.subsliceLength length start stop fromEnd) (last - first) := by
  obtain ⟨rfl, -, -, last_eq⟩ := subsliceBounds?_some bounds
  cases length with
  | none => trivial
  | some value =>
      cases value <;> simp only [VectorLengthMatches] at matches_
      case integer count =>
        subst matches_
        simp only [StaticTyping.subsliceLength, VectorLengthMatches, Int.toNat_natCast]
        cases fromEnd <;> simp_all

theorem PathTyped.subslice {unit : ValidatedUnit} {loans : LoanTypes}
    {elements : Array RuntimeValue} {element : SemTy} {length : Option ConstValue}
    {start stop : Nat} {fromEnd : Bool}
    (typed : HasType unit loans (.vector elements) (.vector element length)) :
    PathTyped unit loans (.vector elements) (.vector element length)
      [.subslice start stop fromEnd]
      (.vector element (StaticTyping.subsliceLength length start stop fromEnd)) where
  read read := by
    simp only [readProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff] at read
    obtain ⟨⟨first, last⟩, bounds, read⟩ := read
    simp only [Option.some.injEq] at read
    subst read
    cases typed with
    | vector _ _ _ length_matches elements_typed =>
        obtain ⟨-, ordered, bounded, -⟩ := subsliceBounds?_some bounds
        refine .vector _ _ _ ?_ (HasTypeEach.of_forall fun value member =>
          elements_typed.mem value (by
            simpa using mem_of_mem_extract (Array.mem_toList_iff.mp member)))
        have size_eq : (elements.extract first last).size = last - first := by
          simp only [Array.size_extract]
          omega
        rw [size_eq]
        exact subsliceLength_matches length_matches bounds
  write replacement write := by
    simp only [writeProjections?, Option.bind_eq_bind, Option.bind_eq_some_iff, pure] at write
    obtain ⟨⟨first, last⟩, bounds, written, written_eq, write⟩ := write
    simp only [Option.some.injEq] at written_eq
    subst written_eq
    split at write
    · rename_i inner
      split at write
      · cases write
      · rename_i same
        simp only [Option.some.injEq] at write
        subst write
        cases typed with
        | vector _ _ _ length_matches elements_typed =>
            obtain ⟨-, ordered, bounded, -⟩ := subsliceBounds?_some bounds
            have inner_typed := replacement.vector_elements
            refine .vector _ _ _ ?_ (HasTypeEach.of_forall fun value member => ?_)
            · have size_eq : (elements.extract 0 first ++ inner ++
                  elements.extract last elements.size).size = elements.size := by
                simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
                simp only [Array.size_append, Array.size_extract, same]
                omega
              rw [size_eq]
              exact length_matches
            · simp only [Array.toList_append, List.mem_append] at member
              rcases member with (member | member) | member
              · exact elements_typed.mem value
                  (by simpa using mem_of_mem_extract (Array.mem_toList_iff.mp member))
              · exact inner_typed.mem value member
              · exact elements_typed.mem value
                  (by simpa using mem_of_mem_extract (Array.mem_toList_iff.mp member))
    · cases write

/-- A nominal type a field place names is the declaration the runtime's
handle names, under the name the runtime spells it by. -/
theorem nominalTarget_handle {unit : ValidatedUnit} {sourceNamespace : NamespaceId}
    {ns targetNs : ValidatedNamespace} {reference : QualifiedRef} {declaration : StructDecl}
    {spelled : QualifiedName} {handle : StructHandle}
    (ns_eq : unit.namespaces[sourceNamespace.index]? = some ns)
    (target : StaticTyping.nominalTarget? unit ns reference.name =
      some (targetNs, declaration, spelled))
    (resolve : resolveStruct? unit sourceNamespace reference = some handle) :
    unit.namespaces[handle.namespaceId.index]? = some targetNs ∧
      targetNs.structs[handle.structId]? = some declaration ∧
      structName? unit handle = some spelled := by
  refine structTarget_handle ns_eq ?_ resolve
  obtain ⟨targetNs', declaration', declared, -, -⟩ := declarationTarget_of_resolve ns_eq resolve
  simp only [StaticTyping.declarationTarget?, Option.bind_eq_bind, Option.bind_eq_some_iff]
    at declared
  obtain ⟨qualified, qualified_eq, declared⟩ := declared
  split at declared
  · cases declared
  rename_i same
  simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same
  simp only [StaticTyping.nominalTarget?, qualified_eq, same, Option.bind_eq_bind,
    Option.bind_some] at target
  simp only [StaticTyping.structTarget?, StaticTyping.declarationTarget?, qualified_eq, same,
    bne_self_eq_false, Bool.false_eq_true, if_false, Option.bind_eq_bind, Option.bind_some]
  simpa only [Option.bind_assoc, Option.bind_some] using target

/-- A field step reaches the field at the checker's type for the place: the
variant's when the place is downcast, else the type its variants agree on. -/
theorem PathTyped.fieldAt {unit : ValidatedUnit} {loans : LoanTypes}
    {targetNs : ValidatedNamespace} {declaration : StructDecl} {spelled : QualifiedName}
    {arguments : List SemArg} {fieldName : String} {staticVariant : Option String}
    {fieldType : SemTy} {handle : StructHandle} {variant : Option String}
    {fields : Array RuntimeValue} {index : Nat}
    (namespace_eq : unit.namespaces[handle.namespaceId.index]? = some targetNs)
    (declaration_eq : targetNs.structs[handle.structId]? = some declaration)
    (fieldType_eq : StaticTyping.fieldTypeAt? targetNs declaration staticVariant arguments
      fieldName = some fieldType)
    (variant_eq : ∀ name, staticVariant = some name → variant = some name)
    (typed : HasType unit loans (.nominal handle variant fields) (.nominal spelled arguments))
    (index_eq : handleFieldIndex? unit handle variant fieldName = some index) :
    PathTyped unit loans (.nominal handle variant fields) (.nominal spelled arguments)
      [.field index] fieldType := by
  obtain ⟨declaringNamespace, declared, fieldTypes, name_eq, handle_eq, resolved, fieldsTyped⟩ :=
    typed.nominal_fields
  obtain ⟨_, _, field, handle_eq', field_eq, find_eq⟩ := handleFieldIndex?_find index_eq
  rw [handle_eq] at handle_eq'
  simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq'
  obtain ⟨rfl, rfl⟩ := handle_eq'
  have same := handleFields_namespace namespace_eq handle_eq
  subst same
  obtain ⟨fieldType', fieldType'_eq, typeResolves⟩ := declared_field_resolves resolved field_eq
  refine PathTyped.field name_eq handle_eq resolved fieldsTyped ?_
  rw [fieldType'_eq]
  congr 1
  cases staticVariant with
  | none =>
      simp only [StaticTyping.fieldTypeAt?] at fieldType_eq
      split at fieldType_eq
      · rename_i first rest types_eq
        split at fieldType_eq
        · rename_i agree
          simp only [Option.some.injEq] at fieldType_eq
          subst fieldType_eq
          have member := field_type_mem namespace_eq declaration_eq types_eq handle_eq resolved
            field_eq find_eq fieldType'_eq
          rcases List.mem_cons.mp member with rfl | member
          · rfl
          · exact beq_iff_eq.mp (List.all_eq_true.mp agree fieldType' member)
        · cases fieldType_eq
      · cases fieldType_eq
  | some variantName =>
      obtain rfl := variant_eq variantName rfl
      simp only [StaticTyping.fieldTypeAt?, StaticTyping.fieldType?, Option.bind_eq_bind,
        Option.bind_eq_some_iff] at fieldType_eq
      obtain ⟨fields', fields'_eq, field', field'_eq, resolvedIn⟩ := fieldType_eq
      have handle_eq'' := fieldsOf_handleFields namespace_eq declaration_eq fields'_eq
      rw [handle_eq] at handle_eq''
      simp only [Option.some.injEq, Prod.mk.injEq] at handle_eq''
      obtain ⟨-, rfl⟩ := handle_eq''
      rw [← Array.find?_toList, find_eq, Option.some.injEq] at field'_eq
      subst field'_eq
      exact Resolves.unique typeResolves ⟨_, resolvedIn⟩

/-- A local's value has the local's type. -/
theorem TypedFrame.read {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {frame : RuntimeFrame} {localId : LocalId} {type : SemTy}
    {value : RuntimeValue}
    (typed : TypedFrame unit loans context.ns context.locals context.env frame)
    (local_eq : context.localType localId = some type)
    (read : readLocal? frame localId = some value) : HasType unit loans value type := by
  simp only [StaticTyping.Context.localType, StaticTyping.Context.typeOf, Option.bind_eq_bind,
    Option.bind_eq_some_iff] at local_eq
  obtain ⟨declaration, declaration_eq, resolved⟩ := local_eq
  simp only [readLocal?, Option.join_eq_some_iff] at read
  obtain ⟨type', resolves, value_typed⟩ := typed.2 localId.index declaration value declaration_eq read
  rw [Resolves.unique resolves ⟨_, resolved⟩] at value_typed
  exact value_typed

private theorem readRuntimePlace_local {frame : RuntimeFrame} {state : RuntimeState}
    {place : RuntimePlace} {localId : LocalId} {rootValue : RuntimeValue}
    (root_eq : place.root = .local localId) (read : readLocal? frame localId = some rootValue) :
    readRuntimePlace? frame state place = readProjections? rootValue place.projections.toList := by
  simp [readRuntimePlace?, root_eq, readRoot?, read]

private theorem readRuntimePlace_local_some {frame : RuntimeFrame} {state : RuntimeState}
    {place : RuntimePlace} {localId : LocalId} {value : RuntimeValue}
    (root_eq : place.root = .local localId)
    (read : readRuntimePlace? frame state place = some value) :
    ∃ rootValue, readLocal? frame localId = some rootValue ∧
      readProjections? rootValue place.projections.toList = some value := by
  simp only [readRuntimePlace?, root_eq, readRoot?, Option.bind_eq_bind,
    Option.bind_eq_some_iff] at read
  exact read

/-- What a resolved place guarantees (`resolvePlace_typed`): it is a path
from a local that reads and writes at the place's type; a downcast place
reads its variant; a path without projections writes the local itself, at a
type the place's embeds in. -/
abbrev PlaceTyped (unit : ValidatedUnit) (loans : LoanTypes) (context : StaticTyping.Context)
    (frame : RuntimeFrame) (place : RuntimePlace) (type : SemTy) (variant : Option String) :
    Prop :=
  ∃ localId rootType, place.root = .local localId ∧ context.localType localId = some rootType ∧
    (place.projections = #[] →
      ∀ value, HasType unit loans value type → HasType unit loans value rootType) ∧
    ∀ rootValue, readLocal? frame localId = some rootValue →
      PathTyped unit loans rootValue rootType place.projections.toList type ∧
      ∀ name, variant = some name → ∀ part,
        readProjections? rootValue place.projections.toList = some part →
        ∃ source fields, part = .nominal source (some name) fields

/-- A typed place extended by a projection step from its value. -/
theorem PlaceTyped.extend {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {frame : RuntimeFrame} {place : RuntimePlace}
    {type next : SemTy} {variant : Option String} {projection : RuntimeProjection}
    (typed : PlaceTyped unit loans context frame place type variant)
    (step : ∀ part, HasType unit loans part type →
      (∀ name, variant = some name → ∃ source fields, part = .nominal source (some name) fields) →
      (∀ rootValue, readLocal? frame (match place.root with | .local l => l | .global _ => ⟨0⟩) =
        some rootValue → readProjections? rootValue place.projections.toList = some part) →
      PathTyped unit loans part type [projection] next) :
    PlaceTyped unit loans context frame
      { place with projections := place.projections.push projection } next none := by
  obtain ⟨localId, rootType, root_eq, local_eq, -, paths⟩ := typed
  refine ⟨localId, rootType, root_eq, local_eq, by simp, fun rootValue read => ⟨?_, by simp⟩⟩
  simp only [Array.toList_push]
  refine (paths rootValue read).1.snoc fun part part_eq => step part
    ((paths rootValue read).1.read part_eq)
    (fun name variant_eq => (paths rootValue read).2 name variant_eq part part_eq) ?_
  intro rootValue' read'
  simp only [root_eq] at read'
  rw [read, Option.some.injEq] at read'
  exact read' ▸ part_eq

/-- A subslice step from any value of a vector type: only a vector has
one. -/
theorem PathTyped.subsliceOf {unit : ValidatedUnit} {loans : LoanTypes} {value : RuntimeValue}
    {element : SemTy} {length : Option ConstValue} {start stop : Nat} {fromEnd : Bool}
    (typed : HasType unit loans value (.vector element length)) :
    PathTyped unit loans value (.vector element length) [.subslice start stop fromEnd]
      (.vector element (StaticTyping.subsliceLength length start stop fromEnd)) := by
  cases value with
  | vector elements => exact PathTyped.subslice typed
  | _ => exact {
      read := fun read => by simp [readProjections?] at read
      write := fun _ write => by simp [writeProjections?] at write }

/-- A place the checker types resolves to a typed place (`PlaceTyped`). -/
theorem resolvePlace_typed {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {frame : RuntimeFrame} {state : RuntimeState}
    (unit_eq : context.unit = unit)
    (ns_eq : unit.namespaces[context.ns.identity.index]? = some context.ns)
    (frameTyped : TypedFrame unit loans context.ns context.locals context.env frame) :
    ∀ (fuel fuel' : Nat) {placeId : PlaceId} {type : SemTy} {variant : Option String}
      {place : RuntimePlace},
      context.placeType fuel placeId = some (type, variant) →
      resolvePlaceFuel? unit context.ns frame state fuel' placeId = some place →
      PlaceTyped unit loans context frame place type variant := by
  intro fuel
  induction fuel with
  | zero => intro _ _ _ _ _ static; simp [StaticTyping.Context.placeType] at static
  | succ fuel ih =>
  intro fuel' placeId type variant place static resolve
  cases fuel' with
  | zero => simp [resolvePlaceFuel?] at resolve
  | succ fuel' =>
  unfold StaticTyping.Context.placeType at static
  unfold resolvePlaceFuel? at resolve
  simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at resolve
  obtain ⟨node, node_eq, resolve⟩ := resolve
  rw [node_eq] at static
  cases node with
  | localVar localId =>
      dsimp only at static resolve
      simp only [Functor.map, Option.map_eq_some_iff, Prod.mk.injEq] at static
      obtain ⟨rootType, local_eq, rfl, rfl⟩ := static
      split at resolve
      · simp only [Option.some.injEq] at resolve
        subst resolve
        exact ⟨localId, rootType, rfl, local_eq, fun _ _ typed => typed, fun rootValue read =>
          ⟨PathTyped.nil (frameTyped.read local_eq read), by simp⟩⟩
      · cases resolve
  | deref base =>
      dsimp only at static resolve
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at static
      obtain ⟨⟨baseType, baseVariant⟩, base_eq, static⟩ := static
      cases baseType with
      | reference kind referent =>
          simp only at static
          split at static
          · rename_i agrees
            simp only [Option.some.injEq, Prod.mk.injEq] at static
            obtain ⟨rfl, rfl⟩ := static
            rw [unit_eq] at agrees
            split at resolve
            · rename_i shared
              rw [shared] at agrees
              cases kind with
              | mutable => exact absurd agrees (by decide)
              | shared =>
              obtain ⟨localId, rootType, root_eq, local_eq, embeds, paths⟩ :=
                ih fuel' base_eq resolve
              refine ⟨localId, rootType, root_eq, local_eq,
                fun empty value typed => embeds empty value (.shared _ _ typed),
                fun rootValue read => ⟨(paths rootValue read).1.unshare, by simp⟩⟩
            · rename_i unshared
              simp only [Bool.not_eq_true] at unshared
              rw [unshared] at agrees
              cases kind with
              | shared => exact absurd agrees (by decide)
              | mutable =>
              obtain ⟨basePlace, base_resolved, resolve⟩ := Option.bind_eq_some_iff.mp resolve
              obtain ⟨localId, rootType, root_eq, local_eq, -, paths⟩ :=
                ih fuel' base_eq base_resolved
              obtain ⟨borrowed, borrowed_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
              cases borrowed with
              | borrow loan current =>
                  simp only [Option.some.injEq] at resolve
                  subst resolve
                  refine ⟨localId, rootType, root_eq, local_eq, by simp, fun rootValue read => ?_⟩
                  refine ⟨?_, by simp⟩
                  simp only [Array.toList_push]
                  refine (paths rootValue read).1.snoc fun part part_eq => ?_
                  simp only [readRuntimePlace?, root_eq, readRoot?, read, Option.bind_eq_bind,
                    Option.bind_some, part_eq, Option.some.injEq] at borrowed_eq
                  subst borrowed_eq
                  exact PathTyped.deref ((paths rootValue read).1.read part_eq)
              | _ => simp at resolve
          · cases static
      | _ => simp at static
  | field base owner field =>
      dsimp only at static resolve
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at static
      obtain ⟨⟨baseType, baseVariant⟩, base_eq, static⟩ := static
      cases baseType with
      | nominal name arguments =>
          simp only [Option.bind_eq_some_iff] at static
          obtain ⟨⟨targetNs, declaration, spelled⟩, target, static⟩ := static
          split at static
          · cases static
          rename_i same_name
          simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same_name
          subst same_name
          simp only [Option.bind_eq_some_iff] at static
          obtain ⟨fieldName, fieldName_eq, fieldType, fieldType_eq, static⟩ := static
          simp only [Option.some.injEq, Prod.mk.injEq] at static
          obtain ⟨rfl, rfl⟩ := static
          obtain ⟨basePlace, base_resolved, resolve⟩ := Option.bind_eq_some_iff.mp resolve
          obtain ⟨localId, rootType, root_eq, local_eq, -, paths⟩ :=
            ih fuel' base_eq base_resolved
          obtain ⟨baseValue, baseValue_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
          cases baseValue with
          | nominal source actualVariant values =>
              dsimp only at resolve
              obtain ⟨declared, declared_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
              split at resolve
              · cases resolve
              rename_i same_source
              simp only [bne_iff_ne, ne_eq, Decidable.not_not] at same_source
              subst same_source
              obtain ⟨fname, fname_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
              obtain ⟨index, index_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
              simp only [Option.some.injEq] at resolve
              subst resolve
              rw [unit_eq] at target
              obtain ⟨namespace_eq, declaration_eq, -⟩ :=
                nominalTarget_handle ns_eq target declared_eq
              have fname_def : fname = fieldName.name := by
                simp only [sourceFieldName?, fieldName_eq, Option.map_some,
                  Option.some.injEq] at fname_eq
                exact fname_eq.symm
              subst fname_def
              refine ⟨localId, rootType, root_eq, local_eq, by simp, fun rootValue read =>
                ⟨?_, by simp⟩⟩
              simp only [Array.toList_push]
              refine (paths rootValue read).1.snoc fun part part_eq => ?_
              rw [readRuntimePlace_local root_eq read, part_eq, Option.some.injEq] at baseValue_eq
              subst baseValue_eq
              refine PathTyped.fieldAt namespace_eq declaration_eq fieldType_eq ?_
                ((paths rootValue read).1.read part_eq) index_eq
              intro name variant_eq
              obtain ⟨_, _, same⟩ := (paths rootValue read).2 name variant_eq _ part_eq
              simp only [RuntimeValue.nominal.injEq] at same
              exact same.2.1
          | _ => simp at resolve
      | _ => simp at static
  | index base indexExpression =>
      dsimp only at static resolve
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at static
      obtain ⟨⟨baseType, baseVariant⟩, base_eq, static⟩ := static
      obtain ⟨basePlace, base_resolved, resolve⟩ := Option.bind_eq_some_iff.mp resolve
      have typed := ih fuel' base_eq base_resolved
      obtain ⟨localId, -, root_eq, -⟩ := id typed
      obtain ⟨index, index_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
      obtain ⟨baseValue, baseValue_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
      obtain ⟨rootValue, read, part_eq⟩ := readRuntimePlace_local_some root_eq baseValue_eq
      have baseTyped : HasType unit loans baseValue baseType := by
        obtain ⟨localId', rootType, root_eq', -, -, paths⟩ := id typed
        rw [root_eq] at root_eq'
        cases root_eq'
        exact (paths rootValue read).1.read part_eq
      have extend := fun next (step : ∀ part, HasType unit loans part baseType →
          readProjections? rootValue basePlace.projections.toList = some part →
          PathTyped unit loans part baseType [.index index] next) =>
        typed.extend (projection := .index index) (next := next) fun part partTyped _ reads =>
          step part partTyped (reads rootValue (by simpa [root_eq] using read))
      cases baseType with
      | vector element length =>
          simp only at static
          split at static
          · split at static
            · simp only [Option.some.injEq, Prod.mk.injEq] at static
              obtain ⟨rfl, rfl⟩ := static
              cases baseValue with
              | vector elements =>
                  simp only at resolve
                  split at resolve
                  · simp only [Option.some.injEq] at resolve
                    subst resolve
                    exact extend element fun part partTyped part_eq' => by
                      rw [part_eq, Option.some.injEq] at part_eq'
                      subst part_eq'
                      exact PathTyped.vectorIndex partTyped
                  · cases resolve
              | tuple elements => cases baseTyped
              | _ => simp at resolve
            · cases static
          · cases static
      | tuple elementTypes =>
          simp only at static
          split at static
          · rename_i expression position _ expression_eq
            split at static
            · cases static
            rename_i nonnegative
            simp only [Functor.map, Option.map_eq_some_iff, Prod.mk.injEq] at static
            obtain ⟨_, type_eq, rfl, rfl⟩ := static
            have index_def : index = position.toNat := by
              cases fuel' with
              | zero => simp [simpleIndexFuel?] at index_eq
              | succ fuel' =>
                  simp only [simpleIndexFuel?, Validation.placeIndexForm?, expression_eq,
                    Option.bind_eq_bind, Option.bind_some, nonnegative, if_false,
                    Option.some.injEq] at index_eq
                  exact index_eq.symm
            subst index_def
            cases baseValue with
            | tuple elements =>
                simp only at resolve
                split at resolve
                · simp only [Option.some.injEq] at resolve
                  subst resolve
                  exact extend _ fun part partTyped part_eq' => by
                    rw [part_eq, Option.some.injEq] at part_eq'
                    subst part_eq'
                    exact PathTyped.tupleIndex partTyped type_eq
                · cases resolve
            | vector elements => cases baseTyped
            | _ => simp at resolve
          · cases static
      | _ => simp at static
  | subslice base start stop fromEnd =>
      dsimp only at static resolve
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at static
      obtain ⟨⟨baseType, baseVariant⟩, base_eq, static⟩ := static
      obtain ⟨basePlace, base_resolved, resolve⟩ := Option.bind_eq_some_iff.mp resolve
      simp only [Option.some.injEq] at resolve
      subst resolve
      have typed := ih fuel' base_eq base_resolved
      cases baseType with
      | vector element length =>
          simp only [Option.some.injEq, Prod.mk.injEq] at static
          obtain ⟨rfl, rfl⟩ := static
          exact typed.extend fun _ partTyped _ _ => PathTyped.subsliceOf partTyped
      | _ => simp at static
  | downcast base variantId =>
      dsimp only at static resolve
      simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at static
      obtain ⟨⟨baseType, baseVariant⟩, base_eq, static⟩ := static
      cases baseType with
      | nominal name arguments =>
          simp only [Option.bind_eq_some_iff, Option.some.injEq, Prod.mk.injEq] at static
          obtain ⟨variantName, variantName_eq, rfl, rfl⟩ := static
          obtain ⟨basePlace, base_resolved, resolve⟩ := Option.bind_eq_some_iff.mp resolve
          obtain ⟨localId, rootType, root_eq, local_eq, -, paths⟩ :=
            ih fuel' base_eq base_resolved
          obtain ⟨baseValue, -, resolve⟩ := Option.bind_eq_some_iff.mp resolve
          cases baseValue with
          | nominal source actualVariant values =>
              cases actualVariant with
              | none => simp at resolve
              | some actual =>
                  simp only at resolve
                  obtain ⟨expected, expected_eq, resolve⟩ := Option.bind_eq_some_iff.mp resolve
                  have expected_def : expected = variantName.name := by
                    simp only [sourceFieldName?, variantName_eq, Option.map_some,
                      Option.some.injEq] at expected_eq
                    exact expected_eq.symm
                  subst expected_def
                  split at resolve
                  · simp only [Option.some.injEq] at resolve
                    subst resolve
                    refine ⟨localId, rootType, root_eq, local_eq, by simp, fun rootValue read =>
                      ⟨?_, ?_⟩⟩
                    · simp only [Array.toList_push]
                      exact (paths rootValue read).1.snoc fun part part_eq =>
                        PathTyped.downcast ((paths rootValue read).1.read part_eq)
                    · intro name name_eq part' read'
                      simp only [Option.some.injEq] at name_eq
                      subst name_eq
                      simp only [Array.toList_push, readProjections?_append,
                        Option.bind_eq_some_iff] at read'
                      obtain ⟨part, -, read'⟩ := read'
                      cases part with
                      | nominal partSource partVariant partValues =>
                          cases partVariant with
                          | none => simp [readProjections?] at read'
                          | some partActual =>
                              simp only [readProjections?] at read'
                              split at read'
                              · rename_i same
                                simp only [Option.some.injEq] at read'
                                subst read'
                                exact ⟨partSource, partValues, by simp [beq_iff_eq.mp same]⟩
                              · cases read'
                      | _ => simp [readProjections?] at read'
                  · cases resolve
          | _ => simp at resolve
      | _ => simp at static

/-- A value read at a typed place has the place's type. -/
theorem PlaceTyped.read {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {frame : RuntimeFrame} {state : RuntimeState}
    {place : RuntimePlace} {type : SemTy} {variant : Option String} {value : RuntimeValue}
    (typed : PlaceTyped unit loans context frame place type variant)
    (read : readRuntimePlace? frame state place = some value) :
    HasType unit loans value type := by
  obtain ⟨localId, rootType, root_eq, -, -, paths⟩ := typed
  obtain ⟨rootValue, rootRead, value_eq⟩ := readRuntimePlace_local_some root_eq read
  exact (paths rootValue rootRead).1.read value_eq

/-- Storing a value of a local's type keeps the frame typed. -/
theorem TypedFrame.set {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {frame : RuntimeFrame} {localId : LocalId} {type : SemTy}
    {value : RuntimeValue}
    (typed : TypedFrame unit loans context.ns context.locals context.env frame)
    (local_eq : context.localType localId = some type)
    (value_typed : HasType unit loans value type) :
    TypedFrame unit loans context.ns context.locals context.env
      { frame with locals := frame.locals.set! localId.index (some value) } := by
  obtain ⟨size_eq, locals_typed⟩ := typed
  simp only [StaticTyping.Context.localType, StaticTyping.Context.typeOf, Option.bind_eq_bind,
    Option.bind_eq_some_iff] at local_eq
  obtain ⟨declaration, declaration_eq, resolved⟩ := local_eq
  refine ⟨by simp [size_eq], fun index declaration' stored declaration'_eq stored_eq => ?_⟩
  simp only [Array.set!_eq_setIfInBounds, Array.getElem?_setIfInBounds] at stored_eq
  split at stored_eq
  · rename_i same
    subst same
    split at stored_eq
    · simp only [Option.some.injEq] at stored_eq
      subst stored_eq
      rw [declaration_eq, Option.some.injEq] at declaration'_eq
      subst declaration'_eq
      exact ⟨type, ⟨_, resolved⟩, value_typed⟩
    · cases stored_eq
  · exact locals_typed index declaration' stored declaration'_eq stored_eq

/-- Writing a value of a typed place's type keeps the frame typed and the
state unchanged. -/
theorem PlaceTyped.write {unit : ValidatedUnit} {loans : LoanTypes}
    {context : StaticTyping.Context} {frame frame' : RuntimeFrame} {state state' : RuntimeState}
    {place : RuntimePlace} {type : SemTy} {variant : Option String} {value : RuntimeValue}
    (typed : PlaceTyped unit loans context frame place type variant)
    (frameTyped : TypedFrame unit loans context.ns context.locals context.env frame)
    (value_typed : HasType unit loans value type)
    (write : writeRuntimePlace? frame state place value = some (frame', state')) :
    TypedFrame unit loans context.ns context.locals context.env frame' ∧ state' = state := by
  obtain ⟨localId, rootType, root_eq, local_eq, embeds, paths⟩ := typed
  have store : ∀ stored, HasType unit loans stored rootType →
      writeRoot? frame state place.root stored = some (frame', state') →
      TypedFrame unit loans context.ns context.locals context.env frame' ∧ state' = state := by
    intro stored stored_typed stored_eq
    simp only [writeRoot?, root_eq] at stored_eq
    split at stored_eq
    · cases stored_eq
    · simp only [Option.some.injEq, Prod.mk.injEq] at stored_eq
      obtain ⟨rfl, rfl⟩ := stored_eq
      exact ⟨frameTyped.set local_eq stored_typed, rfl⟩
  simp only [writeRuntimePlace?] at write
  split at write
  · cases write
  split at write
  · rename_i empty
    exact store value (embeds (Array.isEmpty_iff.mp empty) value value_typed) write
  · simp only [Option.bind_eq_bind, Option.bind_eq_some_iff] at write
    obtain ⟨rootValue, rootRead, updated, updated_eq, write⟩ := write
    simp only [root_eq, readRoot?] at rootRead
    exact store updated ((paths rootValue rootRead).1.write value_typed updated_eq) write

end LeanerIR
