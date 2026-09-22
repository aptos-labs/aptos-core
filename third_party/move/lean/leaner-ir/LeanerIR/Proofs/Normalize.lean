-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Tree
import LeanerIR.Proofs.DenotationWP
import LeanerIR.Semantics.Focus
import LeanerIR.Proofs.Plain
import LeanerIR.Proofs.RuntimeEquality

/-!
# Normalization

The generic route's engine: a body's weakest precondition is reached by
`simp` alone.  `lir_wp_norm` turns the tree's computation into the
weakest preconditions of its evaluations; the evaluations at literal rows
are computed by the ground-evaluation simprocs below, which unfold an
evaluator's own definition closure at exactly the applications whose row
is written out; and the reads, writes, and identifier comparisons the
evaluators leave behind are decided by the `lir_eval` set.  No per-shape
law and no stepping is involved: the same `leaner_normalize` closes the
body of every supported tree to its verification condition.
-/

register_option leaner.lazyComputations : Bool := {
  defValue := false
  descr := "normalize call-free computations without visiting unselected continuations"
}

register_option leaner.branchBoundaries : Bool := {
  defValue := false
  descr := "retain branches until their guards are introduced at invariant boundaries"
}

namespace LeanerIR.Proofs.Denotation.RowSpec

open LeanerIR LeanerIR.Validation LeanerIR.SemanticOperations

initialize Lean.registerTraceClass `leaner.normalize

/-! ## The row state as data -/

@[simp, lir_eval] theorem ofFrame_rowFrame (row : Row) (registries : Registries)
    (state : RuntimeState) :
    RowState.ofFrame (rowFrame row registries) state = ⟨row, registries, state⟩ := rfl

attribute [lir_reconcile] RowState.rowFrame_ofFrame

/-- The row spelling of the generic plain-value finalization rule.  Keeping
`rowFrame` folded lets the native normalizer discharge a typed aggregate in
one rewrite, independent of its payload size. -/
@[lir_reconcile] theorem exportFrameLoans_rowFrame_plainValue
    (state : RuntimeState) (value : RuntimeValue)
    (registries : Registries) (plain : Plain value) :
    exportFrameLoans (rowFrame #[some value] registries) state = state := by
  exact exportFrameLoans_plainValue_state state value plain
    registries.activeLoans registries.loanLocations registries.typeInstantiation

/-- A storage take consumes its bound payload before function exit, leaving
only the address parameter and the cleared binding.  This common generic
shape is borrow-free independently of the payload codec. -/
@[simp, lir_eval] theorem exportFrameLoans_address_none
    (state : RuntimeState) (address : String)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (typeInstantiation : Array (TypeId × TypeId)) :
    exportFrameLoans
      { locals := #[some (.address address), none]
        activeLoans, loanLocations, typeInstantiation } state = state := by
  apply exportFrameLoans_borrowFree
  simp [frameBorrows, outermostBorrows, borrowEntry?, collectPruned]

@[simp, lir_eval] theorem ofFrame_mk (locals : Row) (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (typeInstantiation : Array (TypeId × TypeId)) (state : RuntimeState) :
    RowState.ofFrame ⟨locals, activeLoans, loanLocations, typeInstantiation⟩ state =
      ⟨locals, ⟨activeLoans, loanLocations, typeInstantiation⟩, state⟩ := rfl

@[simp, lir_eval] theorem frame_mk (row : Row) (registries : Registries)
    (state : RuntimeState) :
    RowState.frame ⟨row, registries, state⟩ = rowFrame row registries := rfl

/-! ## Weakest preconditions of the primitives -/

/- The default inventory remains available to modular normalization and
hand proofs. Call-free normalization filters it from its local simp context. -/
attribute [lir_wp_norm] value localVar valuesNil valuesCons statementsNil statementsCons
  blockUnit blockResult operation branch throw_ letValue call callAt


/-- A branch on a decided proposition normalizes to its two implications. -/
@[lir_wp_norm] theorem wp_ite_prop (p : Prop) [Decidable p] (t e : RowSpec α)
    (post : α → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (if p then t else e) post aborts s ↔
      (p → wp t post aborts s) ∧ (¬p → wp e post aborts s) := by
  by_cases h : p <;> simp [h, lir_wp_norm]

/-- The weakest precondition of one evaluation: the evaluator at the
current state selects the value or the throw, at the state it left. -/
@[lir_wp_norm] theorem wp_evaluate (evaluator : NativeEvaluator) (values : Array RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate evaluator values) post aborts s ↔
      match evaluator values s.frame s.state with
      | some (.value frame state runtimeValue) =>
          post (.value runtimeValue) (RowState.ofFrame frame state)
      | some (.throw_ frame state kind thrown) =>
          post (.throw_ kind thrown) (RowState.ofFrame frame state)
      | none => True := by
  simp only [RowSpec.evaluate, RowSpec.bind_def, RowSpec.pure_def, wp_bind, RowSpec.wp_get]
  rcases h : evaluator values s.frame s.state with _ | ⟨f, st, v⟩ | ⟨f, st, k, a⟩ <;>
    simp [lir_wp_norm]

/- A global operation has one dynamic storage choice.  Preserve it as two
implications instead of unfolding its evaluator to an `Option` match: the
closing procedure can then introduce the selected case directly, with work
bounded by the two possible states of the slot. -/
attribute [lir_wp_norm]
  LeanerIR.Proofs.Denotation.globalOperationPost
  LeanerIR.Proofs.Denotation.storageOptionPost

@[lir_wp_norm high] theorem wp_evaluate_publish
    (resource : ResourceLocation) (values : Array RuntimeValue)
    (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (GlobalLocationOperation.publish resource).evaluate? values)
        post aborts s ↔
      LeanerIR.Proofs.Denotation.globalOperationPost (.publish resource) values
        s.frame s.state
        (fun frame state control => post control (RowState.ofFrame frame state)) := by
  rw [wp_evaluate]
  have equivalence := LeanerIR.Proofs.Denotation.globalOperation_post_iff
    (.publish resource) values s.frame s.state
      (fun frame state control => post control (RowState.ofFrame frame state))
  cases result : (GlobalLocationOperation.publish resource).evaluate?
      values s.frame s.state with
  | none => simpa [result] using equivalence
  | some resultValue =>
      cases resultValue <;> simpa [result] using equivalence

/-- Checking an index does not copy elements or perform modular length
arithmetic. Retain its branch boundary so loop continuations normalize only
after the successful bounds (or their negation) enter the local context. -/
@[lir_wp_norm high] theorem wp_evaluate_checkVectorIndex (failure : ThrowKind)
    (elements : Array RuntimeValue) (index : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.checkVectorIndex failure).evaluate?
        #[.vector elements, .integer index]) post aborts s ↔
      wp (if 0 ≤ index ∧ index < Int.ofNat elements.size then
          pure (.value .unit)
        else pure (.throw_ failure #[.integer 1])) post aborts s := by
  rw [wp_evaluate]
  by_cases h : 0 ≤ index ∧ index < Int.ofNat elements.size <;>
    simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
      checkVectorIndex?, List.toList_toArray, h, ↓reduceIte] <;>
    simp [h, lir_wp_norm]

/-- A symbolic vector index exposes its checked success/abort split instead
of leaving the evaluator's option match for the closing procedure. -/
@[lir_wp_norm] theorem wp_evaluate_index (resultType : Ty)
    (elements : Array RuntimeValue) (index : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.index resultType).evaluate?
        #[.vector elements, .integer index]) post aborts s ↔
      ((index < 0 ∨ elements.size ≤ index.toNat) →
          post (.throw_ .abort #[.integer index]) s) ∧
        ((0 ≤ index ∧ index.toNat < elements.size) →
          post (.value (elements[index.toNat]?.getD .unit)) s) := by
  rw [wp_evaluate]
  calc
    _ ↔ LeanerIR.Proofs.Denotation.vectorIndexPost
        #[.vector elements, .integer index] s.frame s.state
        (fun frame state control => post control (RowState.ofFrame frame state)) :=
      LeanerIR.Proofs.Denotation.vectorIndexEvaluator_post_iff_row resultType
        #[.vector elements, .integer index] s.frame s.state _
    _ ↔ ((index < 0 ∨ elements.size ≤ index.toNat) →
          post (.throw_ .abort #[.integer index]) s) ∧
        ((0 ≤ index ∧ index.toNat < elements.size) →
          post (.value (elements[index.toNat]?.getD .unit)) s) := by
      simp [LeanerIR.Proofs.Denotation.vectorIndexPost]

/-- The common three-element literal form is expanded to fixed-value cases.
This keeps decoding and equality reasoning free of symbolic array indexing. -/
@[lir_wp_norm high] theorem wp_evaluate_index_three (resultType : Ty)
    (first second third : RuntimeValue) (index : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.index resultType).evaluate?
        #[.vector #[first, second, third], .integer index]) post aborts s ↔
      ((index < 0 ∨ 3 ≤ index.toNat) →
          post (.throw_ .abort #[.integer index]) s) ∧
        (index = 0 → post (.value first) s) ∧
        (index = 1 → post (.value second) s) ∧
        (index = 2 → post (.value third) s) := by
  rw [wp_evaluate_index]
  change
    (((index < 0 ∨ 3 ≤ index.toNat) →
        post (.throw_ .abort #[.integer index]) s) ∧
      ((0 ≤ index ∧ index.toNat < 3) →
        post (.value (#[first, second, third][index.toNat]?.getD .unit)) s)) ↔ _
  constructor
  · rintro ⟨abortCase, valueCase⟩
    refine ⟨abortCase, ?_, ?_, ?_⟩
    · rintro rfl
      simpa using valueCase ⟨by omega, by omega⟩
    · rintro rfl
      simpa using valueCase ⟨by omega, by omega⟩
    · rintro rfl
      simpa using valueCase ⟨by omega, by omega⟩
  · rintro ⟨abortCase, firstCase, secondCase, thirdCase⟩
    refine ⟨abortCase, ?_⟩
    rintro ⟨nonnegative, inBounds⟩
    have position : index = 0 ∨ index = 1 ∨ index = 2 := by
      omega
    rcases position with position_eq | position_eq | position_eq
    · simpa [position_eq] using firstCase position_eq
    · simpa [position_eq] using secondCase position_eq
    · simpa [position_eq] using thirdCase position_eq

/-- Indexed assignment to the common three-element local literal is likewise
expanded to fixed rows.  The source assignment has no step when its place is
out of bounds, so only the three successful positions constrain its WP. -/
@[lir_wp_norm high] theorem wp_assignLocalIndex_three
    (first second third replacement : RuntimeValue) (index : Int)
    (registries : Registries) (state : RuntimeState)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (RowSpec.assignLocalIndex (⟨1⟩ : LocalId) (⟨0⟩ : LocalId)
        (LeanerIR.Proofs.Spec.pure (.value replacement))) post aborts
        ⟨#[some (.integer index), some (.vector #[first, second, third])],
          registries, state⟩ ↔
      (index = 0 →
          post (.value .unit)
            ⟨#[some (.integer index), some (.vector #[replacement, second, third])],
              registries, state⟩) ∧
        (index = 1 →
          post (.value .unit)
            ⟨#[some (.integer index), some (.vector #[first, replacement, third])],
              registries, state⟩) ∧
        (index = 2 →
          post (.value .unit)
            ⟨#[some (.integer index), some (.vector #[first, second, replacement])],
              registries, state⟩) := by
  by_cases negative : index < 0
  · have not_nonnegative : ¬ 0 ≤ index := by omega
    have not_zero : index ≠ 0 := by omega
    have not_one : index ≠ 1 := by omega
    have not_two : index ≠ 2 := by omega
    simp [RowSpec.assignLocalIndex, resolveLocalIndex?,
      readLocal?, rowFrame, writeRuntimePlace?, readRoot?,
      writeRoot?, lir_wp_norm, negative,
      not_zero, not_one, not_two]
  · have nonnegative : 0 ≤ index := by omega
    by_cases zero : index = 0
    · subst index
      simp [RowSpec.assignLocalIndex, resolveLocalIndex?,
        readLocal?, rowFrame, writeRuntimePlace?, readRoot?,
        writeProjections?, writeRoot?, lir_wp_norm]
    · by_cases one : index = 1
      · subst index
        simp [RowSpec.assignLocalIndex, resolveLocalIndex?,
          readLocal?, rowFrame, writeRuntimePlace?, readRoot?,
          writeProjections?, writeRoot?, lir_wp_norm]
      · by_cases two : index = 2
        · subst index
          simp [RowSpec.assignLocalIndex, resolveLocalIndex?,
            readLocal?, rowFrame, writeRuntimePlace?, readRoot?,
            writeProjections?, writeRoot?, lir_wp_norm]
        · have out_of_bounds : 3 ≤ index := by omega
          have not_in_bounds : ¬ index < 3 := by omega
          have not_zero : index.toNat ≠ 0 := by omega
          have not_one : index.toNat ≠ 1 := by omega
          have not_two : index.toNat ≠ 2 := by omega
          simp [RowSpec.assignLocalIndex, resolveLocalIndex?,
            readLocal?, rowFrame, writeRuntimePlace?, readRoot?,
            writeRoot?, lir_wp_norm, negative, not_in_bounds, zero, one, two]

/-! ## Checked arithmetic -/

/-- A checked binary primitive at integer operands: the bounds of the
result type select the value or the throw. -/
theorem checkedBinaryInteger_integers (failure : ThrowKind) (resultType : Ty)
    (operation : Int → Int → Int) (left right : Int) :
    checkedBinaryInteger failure resultType #[.integer left, .integer right] operation =
      some (match resultType.integerBounds? with
        | some (lower, upper) =>
            if lower ≤ operation left right ∧ operation left right ≤ upper then
              .ok (.integer (operation left right))
            else .error (failure, #[.integer (operation left right)])
        | none => .error (failure, #[])) := by
  simp only [checkedBinaryInteger, checkedInteger]
  cases h : resultType.integerBounds? with
  | none => rfl
  | some bounds =>
      obtain ⟨lower, upper⟩ := bounds
      by_cases inRange : lower ≤ operation left right ∧ operation left right ≤ upper
      · simp [inRange]
      · simp [inRange]

/-- The weakest precondition of a checked binary primitive, keyed on the
primitive and its integer operands. -/
theorem wp_evaluate_checked {operation : PrimitiveLocationOperation} {failure : ThrowKind}
    {resultType : Ty} {combine : Int → Int → Int}
    (evaluation : ∀ arguments frame state, operation.evaluate? arguments frame state =
      (do
        let value ← checkedBinaryInteger failure resultType arguments combine
        match value with
        | .ok value => some (.value frame state value)
        | .error (kind, thrown) => some (.throw_ frame state kind thrown)))
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate operation.evaluate? #[.integer left, .integer right]) post aborts s ↔
      match resultType.integerBounds? with
      | some (lower, upper) =>
          (lower ≤ combine left right ∧ combine left right ≤ upper →
            post (.value (.integer (combine left right))) s) ∧
          (¬ (lower ≤ combine left right ∧ combine left right ≤ upper) →
            post (.throw_ failure #[.integer (combine left right)]) s)
      | none => post (.throw_ failure #[]) s := by
  rw [wp_evaluate, evaluation, checkedBinaryInteger_integers]
  cases h : resultType.integerBounds? with
  | none => simp
  | some bounds =>
      obtain ⟨lower, upper⟩ := bounds
      by_cases inRange : lower ≤ combine left right ∧ combine left right ≤ upper
      · simp [inRange]
      · simp [inRange]

@[lir_wp_norm] theorem wp_evaluate_checkedAdd (failure : ThrowKind) (resultType : Ty)
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.checkedAdd failure resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      match resultType.integerBounds? with
      | some (lower, upper) =>
          (lower ≤ left + right ∧ left + right ≤ upper →
            post (.value (.integer (left + right))) s) ∧
          (¬ (lower ≤ left + right ∧ left + right ≤ upper) →
            post (.throw_ failure #[.integer (left + right)]) s)
      | none => post (.throw_ failure #[]) s :=
  wp_evaluate_checked (fun _ _ _ => rfl) left right post aborts s

@[lir_wp_norm] theorem wp_evaluate_checkedSubtract (failure : ThrowKind) (resultType : Ty)
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.checkedSubtract failure resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      match resultType.integerBounds? with
      | some (lower, upper) =>
          (lower ≤ left - right ∧ left - right ≤ upper →
            post (.value (.integer (left - right))) s) ∧
          (¬ (lower ≤ left - right ∧ left - right ≤ upper) →
            post (.throw_ failure #[.integer (left - right)]) s)
      | none => post (.throw_ failure #[]) s :=
  wp_evaluate_checked (fun _ _ _ => rfl) left right post aborts s

@[lir_wp_norm] theorem wp_evaluate_checkedMultiply (failure : ThrowKind) (resultType : Ty)
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.checkedMultiply failure resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      match resultType.integerBounds? with
      | some (lower, upper) =>
          (lower ≤ left * right ∧ left * right ≤ upper →
            post (.value (.integer (left * right))) s) ∧
          (¬ (lower ≤ left * right ∧ left * right ≤ upper) →
            post (.throw_ failure #[.integer (left * right)]) s)
      | none => post (.throw_ failure #[]) s :=
  wp_evaluate_checked (fun _ _ _ => rfl) left right post aborts s

@[lir_wp_norm high] theorem wp_evaluate_checkedCast (failure : ThrowKind)
    (width : IntWidth) (signed : Bool) (value : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate
        (PrimitiveLocationOperation.checkedCast failure (.integer width signed)).evaluate?
        #[.integer value]) post aborts s ↔
      match (Ty.integer width signed).integerBounds? with
      | some (lower, upper) =>
          (lower ≤ value ∧ value ≤ upper → post (.value (.integer value)) s) ∧
          (¬ (lower ≤ value ∧ value ≤ upper) →
            post (.throw_ failure #[.integer value]) s)
      | none => post (.throw_ failure #[]) s := by
  rw [wp_evaluate]
  simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    List.toList_toArray, checkedInteger]
  cases h : (Ty.integer width signed).integerBounds? with
  | none => simp
  | some bounds =>
      obtain ⟨lower, upper⟩ := bounds
      by_cases inRange : lower ≤ value ∧ value ≤ upper
      · simp [inRange]
      · simp [inRange]

theorem checkedShiftInteger_unsigned (failure : ThrowKind) (left : Bool)
    (width : Nat) (value distance : Int) (widthPos : 0 < width) :
    checkedShiftInteger failure left (.integer (.bits width) false)
        #[.integer value, .integer distance] =
      some (if distance < 0 ∨ width ≤ distance.toNat then
        .error (failure, #[.integer distance])
      else .ok (.integer
        (Int.ofNat (if left then (value % (2 : Int) ^ width).toNat <<< distance.toNat
          else (value % (2 : Int) ^ width).toNat >>> distance.toNat) % (2 : Int) ^ width))) := by
  have widthNonzero : (width == 0) = false := by
    simp [Nat.ne_of_gt widthPos]
  simp only [checkedShiftInteger, widthNonzero, Bool.false_eq_true, ↓reduceIte,
    List.toList_toArray, Bool.or_eq_true, decide_eq_true_eq]
  by_cases invalid : distance < 0 ∨ width ≤ distance.toNat
  · simp [invalid]
  · simp [invalid, integerBitPattern?, widthNonzero, modularInteger_unsigned _ _ widthPos,
      Int.emod_add_emod, Int.add_emod, Int.emod_emod]

@[lir_wp_norm high] theorem wp_evaluate_checkedShiftLeft_unsigned (failure : ThrowKind)
    (width : Nat) (value distance : Int) (widthPos : 0 < width)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate
        (PrimitiveLocationOperation.checkedShiftLeft failure (.integer (.bits width) false)).evaluate?
        #[.integer value, .integer distance]) post aborts s ↔
      ((distance < 0 ∨ width ≤ distance.toNat) →
        post (.throw_ failure #[.integer distance]) s) ∧
      (¬ (distance < 0 ∨ width ≤ distance.toNat) →
        post (.value (.integer (Int.ofNat
          ((value % (2 : Int) ^ width).toNat <<< distance.toNat) % (2 : Int) ^ width))) s) := by
  rw [wp_evaluate]
  simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    checkedShiftInteger_unsigned failure true width value distance widthPos]
  by_cases invalid : distance < 0 ∨ width ≤ distance.toNat <;> simp [invalid]

@[lir_wp_norm high] theorem wp_evaluate_checkedShiftRight_unsigned (failure : ThrowKind)
    (width : Nat) (value distance : Int) (widthPos : 0 < width)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate
        (PrimitiveLocationOperation.checkedShiftRight failure (.integer (.bits width) false)).evaluate?
        #[.integer value, .integer distance]) post aborts s ↔
      ((distance < 0 ∨ width ≤ distance.toNat) →
        post (.throw_ failure #[.integer distance]) s) ∧
      (¬ (distance < 0 ∨ width ≤ distance.toNat) →
        post (.value (.integer (Int.ofNat
          ((value % (2 : Int) ^ width).toNat >>> distance.toNat) % (2 : Int) ^ width))) s) := by
  rw [wp_evaluate]
  simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator,
    checkedShiftInteger_unsigned failure false width value distance widthPos]
  by_cases invalid : distance < 0 ∨ width ≤ distance.toNat <;> simp [invalid]

/-- Checked truncating division at integer operands: a zero divisor throws
with no payload; otherwise the bounds of the result type select the
quotient or the throw. -/
theorem wp_evaluate_checkedDivide (failure : ThrowKind) (resultType : Ty)
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.checkedDivide failure resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      (right = 0 → post (.throw_ failure #[]) s) ∧
      (right ≠ 0 →
        match resultType.integerBounds? with
        | some (lower, upper) =>
            (lower ≤ left.tdiv right ∧ left.tdiv right ≤ upper →
              post (.value (.integer (left.tdiv right))) s) ∧
            (¬ (lower ≤ left.tdiv right ∧ left.tdiv right ≤ upper) →
              post (.throw_ failure #[.integer (left.tdiv right)]) s)
        | none => post (.throw_ failure #[]) s) := by
  rw [wp_evaluate]
  by_cases zero : right = 0
  · subst zero
    simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, checkedDivideIntegers?]
  · have step : checkedDivideIntegers? failure resultType #[.integer left, .integer right] =
        some (checkedInteger failure resultType (left.tdiv right)) := by
      unfold checkedDivideIntegers?
      split
      · rename_i h
        simp at h
        exact absurd h.2 zero
      · rename_i h
        simp only [List.cons.injEq, RuntimeValue.integer.injEq, and_true] at h
        obtain ⟨rfl, rfl⟩ := h
        simp [truncatingQuotient?_eq, zero]
        try (cases checkedInteger failure resultType (left.tdiv right) <;> rfl)
      · simp_all
    simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, step,
      checkedInteger]
    cases h : resultType.integerBounds? with
    | none => simp [zero]
    | some bounds =>
        obtain ⟨lower, upper⟩ := bounds
        by_cases inRange : lower ≤ left.tdiv right ∧ left.tdiv right ≤ upper
        · simp [inRange, zero]
        · simp [inRange, zero]

/-- Checked truncating remainder at integer operands: a zero divisor
throws with no payload; otherwise the quotient's range is checked before
the remainder's. -/
theorem wp_evaluate_checkedModulo (failure : ThrowKind) (resultType : Ty)
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.checkedModulo failure resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      (right = 0 → post (.throw_ failure #[]) s) ∧
      (right ≠ 0 →
        match resultType.integerBounds? with
        | some (lower, upper) =>
            (¬ (lower ≤ left.tdiv right ∧ left.tdiv right ≤ upper) →
              post (.throw_ failure #[.integer (left.tdiv right)]) s) ∧
            (lower ≤ left.tdiv right ∧ left.tdiv right ≤ upper →
              (lower ≤ left.tmod right ∧ left.tmod right ≤ upper →
                post (.value (.integer (left.tmod right))) s) ∧
              (¬ (lower ≤ left.tmod right ∧ left.tmod right ≤ upper) →
                post (.throw_ failure #[.integer (left.tmod right)]) s))
        | none => post (.throw_ failure #[]) s) := by
  rw [wp_evaluate]
  by_cases zero : right = 0
  · subst zero
    simp [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, checkedModuloIntegers?]
  · have step : checkedModuloIntegers? failure resultType #[.integer left, .integer right] =
        some (match checkedInteger failure resultType (left.tdiv right) with
          | .error error => .error error
          | .ok _ => checkedInteger failure resultType (left - left.tdiv right * right)) := by
      unfold checkedModuloIntegers?
      split
      · rename_i h
        simp at h
        exact absurd h.2 zero
      · rename_i h
        simp only [List.cons.injEq, RuntimeValue.integer.injEq, and_true] at h
        obtain ⟨rfl, rfl⟩ := h
        simp [truncatingQuotient?_eq, zero]
        try (cases checkedInteger failure resultType (left.tdiv right) <;> rfl)
      · simp_all
    have remainder : left - left.tdiv right * right = left.tmod right := by
      rw [Int.tmod_def, Int.mul_comm]
    simp only [PrimitiveLocationOperation.evaluate?, liftPrimitiveEvaluator, step,
      checkedInteger, remainder]
    cases h : resultType.integerBounds? with
    | none => simp [zero]
    | some bounds =>
        obtain ⟨lower, upper⟩ := bounds
        by_cases quotientRange : lower ≤ left.tdiv right ∧ left.tdiv right ≤ upper
        · by_cases remainderRange : lower ≤ left.tmod right ∧ left.tmod right ≤ upper
          · simp [quotientRange, remainderRange, zero]
          · simp [quotientRange, remainderRange, zero]
        · simp [quotientRange, zero]

/-- A modular binary primitive at integer operands: the result type wraps
the mathematical result, or the primitive is stuck at a non-integer type. -/
theorem wp_evaluate_modular {operation : PrimitiveLocationOperation} {resultType : Ty}
    {combine : Int → Int → Int}
    (evaluation : ∀ arguments frame state, operation.evaluate? arguments frame state =
      (do
        let value ← modularBinaryInteger resultType arguments combine
        match value with
        | .ok value => some (.value frame state value)
        | .error (kind, thrown) => some (.throw_ frame state kind thrown)))
    (left right : Int) (post : Control → RowState → Prop) (aborts : Failure → Prop)
    (s : RowState) :
    wp (RowSpec.evaluate operation.evaluate? #[.integer left, .integer right]) post aborts s ↔
      ∀ wrapped, modularInteger resultType (combine left right) = some wrapped →
        post (.value wrapped) s := by
  rw [wp_evaluate, evaluation]
  simp only [modularBinaryInteger, List.toList_toArray]
  cases h : modularInteger resultType (combine left right) with
  | none => simp
  | some wrapped => simp

@[lir_wp_norm] theorem wp_evaluate_add (resultType : Ty) (left right : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.add resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      ∀ wrapped, modularInteger resultType (left + right) = some wrapped →
        post (.value wrapped) s :=
  wp_evaluate_modular (fun _ _ _ => rfl) left right post aborts s

@[lir_wp_norm] theorem wp_evaluate_subtract (resultType : Ty) (left right : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.subtract resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      ∀ wrapped, modularInteger resultType (left - right) = some wrapped →
        post (.value wrapped) s :=
  wp_evaluate_modular (fun _ _ _ => rfl) left right post aborts s

@[lir_wp_norm] theorem wp_evaluate_multiply (resultType : Ty) (left right : Int)
    (post : Control → RowState → Prop) (aborts : Failure → Prop) (s : RowState) :
    wp (RowSpec.evaluate (PrimitiveLocationOperation.multiply resultType).evaluate?
        #[.integer left, .integer right]) post aborts s ↔
      ∀ wrapped, modularInteger resultType (left * right) = some wrapped →
        post (.value wrapped) s :=
  wp_evaluate_modular (fun _ _ _ => rfl) left right post aborts s

/- The checked lemmas are keyed on the primitive and must be tried before
the generic evaluation lemma, which would otherwise unfold the primitive
under the match; a set carries no priority from the attribute itself. -/
attribute [lir_wp_norm high] wp_evaluate_checkedAdd wp_evaluate_checkedSubtract
  wp_evaluate_checkedMultiply wp_evaluate_checkedDivide wp_evaluate_checkedModulo
  wp_evaluate_add wp_evaluate_subtract wp_evaluate_multiply

attribute [lir_eval] modularInteger
  bitwiseBinary bitwiseBinaryInteger integerBitPattern?

/-! ## Reads, writes, and binds over literal rows -/

/-- Binding a variable pattern, keyed on the literal binder: any positive
fuel binds. -/
theorem bindVariable_rowFrame' (fuel : Nat) (localId : LocalId) (row : Row)
    (registries : Registries) (value : RuntimeValue)
    (fuelPos : 0 < fuel) (inBounds : localId.index < row.size) :
    NativePatternBinder.bind ⟨fuel, .variable localId⟩ (rowFrame row registries) value =
      some (rowFrame (row.set! localId.index (some value)) registries) := by
  cases fuel with
  | zero => exact absurd fuelPos (Nat.lt_irrefl 0)
  | succ fuel => exact bindVariable_rowFrame fuel localId row registries value inBounds

@[lir_eval] theorem readProjections?_deref_borrow (loan : Nat) (current : RuntimeValue)
    (rest : List RuntimeProjection) :
    readProjections? (.borrow loan current) (.deref :: rest) = readProjections? current rest := by
  simp [readProjections?]

/-- A symbolic vector read exposes just its bounds choice, without unfolding
the vector payload or the remainder of the resolved projection path. -/
@[lir_eval] theorem readProjections?_vector_index (elements : Array RuntimeValue)
    (index : Nat) (rest : List RuntimeProjection) :
    readProjections? (.vector elements) (.index index :: rest) =
      if h : index < elements.size then readProjections? elements[index] rest else none := by
  by_cases h : index < elements.size <;> simp [readProjections?, h]

@[lir_eval] theorem writeProjections?_deref_borrow (loan : Nat) (current replacement : RuntimeValue)
    (rest : List RuntimeProjection) :
    writeProjections? (.borrow loan current) (.deref :: rest) replacement =
      (writeProjections? current rest replacement).map (.borrow loan) := by
  simp [writeProjections?]
  cases writeProjections? current rest replacement <;> rfl

@[lir_eval] theorem Array.filter_empty {α : Type} (p : α → Bool) :
    Array.filter p #[] 0 0 = #[] := rfl

/-! ## The pruned walk

`collectPruned` is well-founded over the value; its constructor equations
are what evaluation rewrites with. -/

@[lir_eval] theorem collectPrunedList_nil {α : Type} (f : RuntimeValue → Option α) :
    collectPrunedList f [] = #[] := by
  rw [collectPrunedList.eq_def]

@[lir_eval] theorem collectPrunedList_cons {α : Type} (f : RuntimeValue → Option α)
    (element : RuntimeValue) (rest : List RuntimeValue) :
    collectPrunedList f (element :: rest) = collectPruned f element ++ collectPrunedList f rest := by
  rw [collectPrunedList.eq_def]

@[lir_eval] theorem collectPruned_nominal {α : Type} (f : RuntimeValue → Option α)
    (source : StructHandle) (variant : Option String) (fields : Array RuntimeValue) :
    collectPruned f (.nominal source variant fields) =
      match f (.nominal source variant fields) with
      | some found => #[found]
      | none => collectPrunedList f fields.toList := by
  rw [collectPruned.eq_def]
  exact rfl

@[lir_eval] theorem collectPruned_borrow {α : Type} (f : RuntimeValue → Option α)
    (loan : Nat) (current : RuntimeValue) :
    collectPruned f (.borrow loan current) =
      match f (.borrow loan current) with
      | some found => #[found]
      | none => collectPruned f current := by
  rw [collectPruned.eq_def]
  exact rfl

@[lir_eval] theorem collectPruned_integer {α : Type} (f : RuntimeValue → Option α) (value : Int) :
    collectPruned f (.integer value) =
      match f (.integer value) with
      | some found => #[found]
      | none => #[] := by
  rw [collectPruned.eq_def]
  exact rfl

@[lir_eval] theorem collectPruned_bool {α : Type} (f : RuntimeValue → Option α) (value : Bool) :
    collectPruned f (.bool value) =
      match f (.bool value) with
      | some found => #[found]
      | none => #[] := by
  rw [collectPruned.eq_def]
  exact rfl

@[lir_eval] theorem collectPruned_address {α : Type} (f : RuntimeValue → Option α)
    (value : String) :
    collectPruned f (.address value) =
      match f (.address value) with
      | some found => #[found]
      | none => #[] := by
  rw [collectPruned.eq_def]
  exact rfl

@[lir_eval] theorem collectPruned_unit {α : Type} (f : RuntimeValue → Option α) :
    collectPruned f .unit =
      match f .unit with
      | some found => #[found]
      | none => #[] := by
  rw [collectPruned.eq_def]
  exact rfl

@[lir_eval] theorem collectPruned_loanHole {α : Type} (f : RuntimeValue → Option α) (loan : Nat) :
    collectPruned f (.loanHole loan) =
      match f (.loanHole loan) with
      | some found => #[found]
      | none => #[] := by
  rw [collectPruned.eq_def]
  exact rfl

attribute [lir_eval] outermostBorrows borrowEntry?

@[lir_eval high] theorem returnedBorrowIds_singleBorrow (loan : Nat) (value : RuntimeValue) :
    returnedBorrowIds #[.borrow loan value] = #[loan] := by
  simp [returnedBorrowIds, outermostBorrows, collectPruned, borrowEntry?]

@[lir_eval] theorem maskReturnedBorrows_borrow (loans : Array Nat) (loan : Nat)
    (value : RuntimeValue) :
    maskReturnedBorrows loans (.borrow loan value) =
      if loans.contains loan then .unit else .borrow loan value := by
  rw [maskReturnedBorrows.eq_def]

/-- Distinct handles do not alias, independently of the borrowed payload. -/
theorem maskReturnedBorrows_single_other (returned loan : Nat) (value : RuntimeValue)
    (different : returned ≠ loan) :
    maskReturnedBorrows #[returned] (.borrow loan value) = .borrow loan value := by
  simp [maskReturnedBorrows_borrow, Ne.symm different]

/-- Finalization depends on the frame only through its outer-borrow
collection and the holes those loans can see.  This one-entry equation is
structural in that collection, rather than in the number or layout of local
slots; it covers a scalar borrow, a returned reborrow, and a borrow nested in
an aggregate with the same rule. -/
theorem exportFrameLoans_singleBorrow
    (frame : RuntimeFrame) (state : RuntimeState) (loan : Nat)
    (current : RuntimeValue)
    (borrows : frameBorrows frame = #[(loan, current)])
    (noLocalHole : holeInFrame frame loan = false)
    (noGlobalHole : globalLoanKey? state loan = none) :
    exportFrameLoans frame state =
      { state with pending := state.pending.push (loan, current) } := by
  simp [exportFrameLoans, exportSettledLoans, borrows, noLocalHole,
    applyWriteBack_empty_export, noGlobalHole]

/-! ## Returned references resolved into an export

A callee's returned mutable references fill their holes in the parameter
export; at literal rows the resolution is one fill per returned borrow. -/

attribute [lir_eval] resolveReturnedBorrows_singleBorrow resolveReturnedBorrows_empty
  resolveReturnedBorrows_integer

/-! ## The loan registry

The registry's key lookup stays folded: a body's borrows head the
registry, whose tail is the initial state's, about which the contract
states the facts (freshness of unminted ids, the parameters' loans). -/

/-- A registration of another loan is walked past. -/
@[lir_eval] theorem globalLoanKeyIn?_cons_ne (other loan : Nat) (key : GlobalKey)
    (rest : List (Nat × GlobalKey)) (distinct : other ≠ loan) :
    globalLoanKeyIn? ((other, key) :: rest) loan = globalLoanKeyIn? rest loan := by
  simp [globalLoanKeyIn?, List.find?, beq_eq_false_iff_ne.mpr distinct]

attribute [lir_eval] globalLoanKeyIn?_head globalLoanKey?_registry

/-! ## Identifier comparisons

The identifiers are structures deriving `BEq`; their comparisons decide
by their indices. -/

instance : LawfulBEq ExprId where
  eq_of_beq {a b} h := by
    cases a; cases b
    simp [BEq.beq, instBEqExprId.beq] at h
    rw [h]
  rfl {a} := by
    cases a
    simp [BEq.beq, instBEqExprId.beq]

instance : LawfulBEq LocalId where
  eq_of_beq {a b} h := by
    cases a; cases b
    simp [BEq.beq, instBEqLocalId.beq] at h
    rw [h]
  rfl {a} := by
    cases a
    simp [BEq.beq, instBEqLocalId.beq]

instance : LawfulBEq LoanId where
  eq_of_beq {a b} h := by
    cases a; cases b
    simp [BEq.beq, instBEqLoanId.beq] at h
    rw [h]
  rfl {a} := by
    cases a
    simp [BEq.beq, instBEqLoanId.beq]

attribute [lir_eval] beq_iff_eq bne_iff_ne ExprId.mk.injEq LocalId.mk.injEq LoanId.mk.injEq
  List.toList_toArray Array.toList_append List.cons_append List.nil_append
  List.getElem?_toArray List.getElem?_cons_zero List.getElem?_cons_succ Option.join_some
  List.push_toArray List.filter_toArray List.filter_nil List.singleton_append
  Array.set!_eq_setIfInBounds List.setIfInBounds_toArray List.set_cons_succ List.set_cons_zero
  List.size_toArray List.length_cons List.length_nil Nat.lt_add_one Nat.zero_lt_succ
  Nat.succ_lt_succ_iff
  rowFrame_locals rowFrame_activeLoans rowFrame_loanLocations readLocal?_rowFrame
  dereference_evaluate less_evaluate
  integerBounds_u8 integerBounds_u64
  finishControl? finalizeFunctionState unpackFallthrough
  returnedBorrowIds maskReturnedBorrows maskReturnedBorrowList
  parameterLoanLocations
  findFirst_unit findFirst_bool findFirst_character findFirst_integer findFirst_address
  findFirst_signer findFirst_string findFirst_bytes findFirst_loanHole findFirst_borrow
  findFirst_nominal findFirstList_nil findFirstList_cons
  rewriteFirst_unit rewriteFirst_bool rewriteFirst_character rewriteFirst_integer
  rewriteFirst_address rewriteFirst_signer rewriteFirst_string rewriteFirst_bytes
  rewriteFirst_loanHole rewriteFirst_borrow rewriteFirst_nominal rewriteFirstList_nil
  rewriteFirstList_cons holeInFrame_mk holeWithin fillHole? Array.find?
  List.findSome?_nil List.findSome?_cons

-- Plain results reuse the existing boundary rule without scanning the row
-- once to choose a branch and again to export its loans.
attribute [lir_eval high] exportReturnedFrameLoans_empty

/-! ## Closed comparisons

The runtime's descriptors derive `BEq`; a comparison of two closed
descriptors is decided by evaluating it. -/

/-- Reduce a closed Boolean-valued application by evaluation. -/
def reduceClosedBool (e : Lean.Expr) : Lean.Meta.SimpM Lean.Meta.Simp.Step := do
  if e.hasFVar || e.hasMVar then return .continue
  let reduced ← Lean.Meta.whnfD e
  if reduced.isConstOf ``true || reduced.isConstOf ``false then
    return .done { expr := reduced }
  return .continue

simproc [lir_eval] reduceBEq (@BEq.beq _ _ _ _) := reduceClosedBool

simproc [lir_eval] reduceBne (@bne _ _ _ _) := reduceClosedBool

/-! ## Loan comparisons

An evaluator compares loan ids the contract keeps symbolic; their
distinctness or equality follows from the bounds and distinctness facts
in the context, by arithmetic. -/

/-- Negative results are scoped to the hypotheses that justified them.
A callee's loan discipline can make a previously unknown comparison
decidable. Reusing a negative result across that boundary loses the fact. -/
initialize undecidedEqualities :
    IO.Ref (Lean.PersistentHashMap (Lean.Expr × Array Lean.FVarId) Unit) ← IO.mkRef {}

/-- Nested semantic simplifications share these certificates for one
normalization pass. Keep only proofs whose free variables remain in scope:
the same payload can be encountered under different continuation binders. -/
initialize leafCertificates : IO.Ref (Lean.ExprMap Lean.Expr) ← IO.mkRef {}

private def cachedLeaf? (proposition : Lean.Expr) : Lean.MetaM (Option Lean.Expr) := do
  let some proof := (← leafCertificates.get)[proposition]? | return none
  let context ← Lean.getLCtx
  if (Lean.collectFVars {} proof).fvarIds.all context.contains then
    return some proof
  return none

/-- Decide an equality of naturals with free variables by arithmetic over
the context. -/
def decideNatEquality (e : Lean.Expr) : Lean.Meta.SimpM Lean.Meta.Simp.Step := do
  unless e.isAppOfArity ``Eq 3 do return .continue
  unless (e.getArg! 0).isConstOf ``Nat do return .continue
  unless e.hasFVar && !e.hasMVar && !e.hasLooseBVars do return .continue
  if e.getArg! 1 == e.getArg! 2 then
    return .done {
      expr := Lean.mkConst ``True
      proof? := some (← Lean.Meta.mkEqTrue (← Lean.Meta.mkEqRefl (e.getArg! 1))) }
  /- Only loan ids are compared symbolically: an id minted in the body
  (`initial.nextLoan + k`) or a parameter's loan the contract bounds. -/
  let mentionsNextLoan (side : Lean.Expr) : Bool :=
    (side.find? (·.isConstOf ``RuntimeState.nextLoan)).isSome
  unless mentionsNextLoan (e.getArg! 1) || mentionsNextLoan (e.getArg! 2) ||
      (e.getArg! 1).isFVar || (e.getArg! 2).isFVar do return .continue
  let context := (← Lean.getLCtx).foldl (init := #[]) fun ids declaration =>
    ids.push declaration.fvarId
  let cacheKey := (e, context)
  if (← undecidedEqualities.get).contains cacheKey then return .continue
  let decide (proposition : Lean.Expr) : Lean.MetaM (Option Lean.Expr) := do
    if let some proof ← cachedLeaf? proposition then return some proof
    let goal ← Lean.Meta.mkFreshExprMVar proposition
    try
      let remaining ← Lean.Elab.Term.TermElabM.run' do
        Lean.Elab.Tactic.run goal.mvarId! do
          Lean.Elab.Tactic.withoutRecover <|
            Lean.Elab.Tactic.evalTactic (← `(tactic| omega))
      if remaining.isEmpty then
        let proof ← Lean.instantiateMVars goal
        unless proof.hasMVar do leafCertificates.modify (·.insert proposition proof)
        return some proof
      else return none
    catch _ => return none
  -- Fresh children and older lenders are usually distinct. Syntactically
  -- identical ids were handled above; try separation before equality.
  if let some proof ← decide (Lean.mkNot e) then
    return .done { expr := Lean.mkConst ``False, proof? := some (← Lean.Meta.mkEqFalse proof) }
  if let some proof ← decide e then
    return .done { expr := Lean.mkConst ``True, proof? := some (← Lean.Meta.mkEqTrue proof) }
  undecidedEqualities.modify (·.insert cacheKey ())
  return .continue

simproc [lir_eval] decideLoanEquality (@Eq Nat _ _) := decideNatEquality

/-- Index resolution mixes signed source integers with natural array sizes.
Use only the small range query, not the borrow evaluator containing it, as
an arithmetic goal. Negative results are local to the current branch. -/
private def decideIndexBound (e : Lean.Expr) : Lean.Meta.Simp.SimpM Lean.Meta.Simp.Step := do
  unless e.isAppOfArity ``LT.lt 4 && e.hasFVar && !e.hasMVar && !e.hasLooseBVars do
    return .continue
  let carrier := e.getArg! 0
  let left := e.getArg! 2
  let right := e.getArg! 3
  let naturalIndex := carrier.isConstOf ``Nat && left.isAppOfArity ``Int.toNat 1
  let negativeIndex ← if carrier.isConstOf ``Int && left.isFVar then
      Lean.Meta.isDefEq right (Lean.toExpr (0 : Int))
    else pure false
  unless naturalIndex || negativeIndex do return .continue
  let context := (← Lean.getLCtx).foldl (init := #[]) fun ids declaration =>
    ids.push declaration.fvarId
  let key := (e, context)
  if (← undecidedEqualities.get).contains key then return .continue
  let decide (proposition : Lean.Expr) : Lean.MetaM (Option Lean.Expr) := do
    if let some proof ← cachedLeaf? proposition then return some proof
    let goal ← Lean.Meta.mkFreshExprMVar proposition
    try
      let remaining ← Lean.Elab.Term.TermElabM.run' do
        Lean.Elab.Tactic.run goal.mvarId! do
          Lean.Elab.Tactic.withoutRecover <|
            Lean.Elab.Tactic.evalTactic (← `(tactic|
              (simp (failIfUnchanged := false) only [Array.size_map, Int.reduceAbs] at * <;> omega)))
      if !remaining.isEmpty then return none
      let proof ← Lean.instantiateMVars goal
      unless proof.hasMVar do leafCertificates.modify (·.insert proposition proof)
      return some proof
    catch _ => return none
  if let some proof ← decide e then
    return .done { expr := Lean.mkConst ``True, proof? := some (← Lean.Meta.mkEqTrue proof) }
  if let some proof ← decide (Lean.mkNot e) then
    return .done { expr := Lean.mkConst ``False, proof? := some (← Lean.Meta.mkEqFalse proof) }
  undecidedEqualities.modify (·.insert key ())
  return .continue

simproc ↓ [lir_eval] decideDynamicIndexBound (@LT.lt _ _ _ _) := decideIndexBound

/-! ## Ground evaluation

An evaluator computes only at a ground application: under a binder its
row and state are the bound state of a later step, and unfolding it there
produces the interpreter's search symbolically.  Each simproc runs the
ambient simp set plus the evaluator's own definition closure on the ground
application alone. -/

private def plainValueCertificate? (value : Lean.Expr) : Lean.MetaM (Option Lean.Expr) := do
  -- Scalar constructors are proof evidence already. Do not build a simp
  -- goal or scan a caller's context to rediscover that their payload cannot
  -- contain references; the payload itself can stay completely symbolic.
  let constructor? := match value.getAppFn.constName? with
    | some ``RuntimeValue.unit => some ``Plain.unit
    | some ``RuntimeValue.bool => some ``Plain.bool
    | some ``RuntimeValue.character => some ``Plain.character
    | some ``RuntimeValue.integer => some ``Plain.integer
    | some ``RuntimeValue.address => some ``Plain.address
    | some ``RuntimeValue.signer => some ``Plain.signer
    | some ``RuntimeValue.string => some ``Plain.string
    | some ``RuntimeValue.bytes => some ``Plain.bytes
    | _ => none
  if let some constructor := constructor? then
    return some (Lean.mkAppN (Lean.mkConst constructor) value.getAppArgs)
  -- Neither constructor can be plain; do not simplify its potentially
  -- large current value while attempting an impossible certificate.
  if value.isAppOf ``RuntimeValue.borrow || value.isAppOf ``RuntimeValue.loanHole then
    return none
  let proposition ← Lean.Meta.mkAppM ``Plain #[value]
  if let some proof ← cachedLeaf? proposition then return some proof
  for declaration in ← Lean.getLCtx do
    if declaration.isImplementationDetail then continue
    unless declaration.type.consumeMData.isAppOfArity ``Plain 1 do continue
    if (← Lean.instantiateMVars declaration.type) == proposition then
      let proof := declaration.toExpr
      leafCertificates.modify (·.insert proposition proof)
      return some proof
  -- An opaque slot has no structural loan-freedom proof to search for.
  -- Its loop-header assumption is the certificate, when one is present.
  if value.isFVar then return none
  if value.isAppOfArity ``Option.getD 3 then
    let slot ← Lean.Meta.whnf (value.getArg! 1)
    unless slot.isAppOfArity ``Option.some 2 || slot.isAppOfArity ``Option.none 1 do
      return none
    if slot.isAppOfArity ``Option.some 2 then
      let payload ← Lean.Meta.whnf (slot.getArg! 1)
      -- The optional wrapper must not turn a known borrow into a Plain
      -- search that traverses its arbitrarily large current value.
      if payload.isAppOf ``RuntimeValue.borrow || payload.isAppOf ``RuntimeValue.loanHole then
        return none
  do
    let goal ← Lean.Meta.mkFreshExprMVar proposition
    try
      let remaining ← Lean.Elab.Term.TermElabM.run' do
        Lean.Elab.Tactic.run goal.mvarId! do
          Lean.Elab.Tactic.withoutRecover <|
            Lean.Elab.Tactic.evalTactic (← `(tactic| leaner_plain))
      if remaining.isEmpty then
        let proof ← Lean.instantiateMVars goal
        unless proof.hasMVar do leafCertificates.modify (·.insert proposition proof)
        pure (some proof)
      else pure none
    catch _ => pure none

/-- Use the existing typed loan-freedom algebra before opening a walker.
In particular, a symbolic vector of native values is not a search space. -/
private def evalPlainWalk (e : Lean.Expr) : Lean.Meta.Simp.SimpM Lean.Meta.Simp.Step := do
  let isFind := e.isAppOfArity ``findFirst 3
  let isRewrite := e.isAppOfArity ``rewriteFirst 2
  unless isFind || isRewrite do return .continue
  let value := e.getAppArgs.back!
  unless [``RuntimeValue.vector, ``RuntimeValue.tuple, ``RuntimeValue.nominal,
      ``RuntimeValue.closure, ``Option.getD, ``Codec.encode].any value.isAppOf || value.isFVar do
    return .continue
  let query := e.getArg! (if isFind then 1 else 0)
  let matcherLaw? := ([(``holeMark?, ``LoanMatcher.holeMark?),
    (``anyHole?, ``LoanMatcher.anyHole?), (``holeFill?, ``LoanMatcher.holeFill?),
    (``borrowCurrent?, ``LoanMatcher.borrowCurrent?),
    (``borrowRewrite?, ``LoanMatcher.borrowRewrite?),
    (``borrowClear?, ``LoanMatcher.borrowClear?)]).find? fun pair => query.isAppOf pair.1
  let some (_, law) := matcherLaw? | return .continue
  let plain? ← plainValueCertificate? value
  let some plain := plain? | return .continue
  let matcher ← Lean.Meta.mkAppM law query.getAppArgs
  let proof ← Lean.Meta.mkAppM
    (if isFind then ``findFirst_eq_none_of_plain else ``rewriteFirst_eq_none_of_plain)
    #[matcher, plain]
  let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
  return .done { expr := rhs, proof? := some proof }

simproc ↓ [lir_eval] findPlainAggregate (findFirst _ _) := evalPlainWalk
simproc ↓ [lir_eval] rewritePlainAggregate (rewriteFirst _ _) := evalPlainWalk

simproc ↓ [lir_eval] maskReturnedPlainAggregate (maskReturnedBorrows _ _) := fun e => do
  let value := e.getAppArgs.back!
  let some plain ← plainValueCertificate? value | return .continue
  let proof ← Lean.Meta.mkAppM ``Plain.maskReturnedBorrows_eq_self #[plain, e.getArg! 0]
  let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
  return .done { expr := rhs, proof? := some proof }

/-- Finalization's pruned borrow collector uses the same typed certificate
as the search and rewrite walkers. Its cost does not depend on vector length. -/
simproc ↓ [lir_eval] collectPlainAggregate (collectPruned borrowEntry? _) := fun e => do
  let value := e.getAppArgs.back!
  unless [``RuntimeValue.vector, ``RuntimeValue.tuple, ``RuntimeValue.nominal,
      ``RuntimeValue.closure, ``Option.getD, ``Codec.encode].any value.isAppOf || value.isFVar do
    return .continue
  let some plain ← plainValueCertificate? value | return .continue
  let proof ← Lean.Meta.mkAppM ``Plain.collectPruned_borrowEntry?_eq_empty #[plain]
  let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
  return .done { expr := rhs, proof? := some proof }

/-- Missing slots contribute no borrows. Use a value observer rather than
splitting optional slots: loop headers can carry arbitrarily many inactive,
loan-free locals, and branching on their presence would be exponential. -/
theorem frameBorrows_getD (frame : RuntimeFrame) :
    frameBorrows frame = frame.locals.foldl (init := #[]) (fun borrows slot =>
      borrows ++ outermostBorrows (slot.getD .unit)) := by
  unfold frameBorrows
  congr 1
  funext borrows slot
  cases slot <;> simp [outermostBorrows, collectPruned, borrowEntry?]

theorem holeWithin_slot (slot : Option RuntimeValue) (loan : Nat) :
    (slot.map (holeWithin loan)).getD false = holeWithin loan (slot.getD .unit) := by
  cases slot <;> simp [holeWithin, findFirst, holeMark?]

simproc ↓ [lir_eval] holeWithinOptionalSlot
    (Option.getD (Option.map (holeWithin _) _) false) := fun e => do
  unless e.isAppOfArity ``Option.getD 3 do return .continue
  let mapped := e.getArg! 1
  unless mapped.isAppOfArity ``Option.map 4 do
    trace[leaner.normalize] "optional hole map: {mapped}"
    return .continue
  let query := mapped.getArg! 2
  unless query.isAppOfArity ``holeWithin 1 do
    trace[leaner.normalize] "optional hole query: {query}"
    return .continue
  let slot := mapped.getArg! 3
  let proof ← Lean.Meta.mkAppM ``holeWithin_slot #[slot, query.getArg! 0]
  let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
  return .done { expr := rhs, proof? := some proof }

@[simp, lir_eval] theorem readProjections_nil (value : RuntimeValue) :
    readProjections? value [] = some value := rfl

/-- A native aggregate cannot hide a loan when its element encoder cannot.
Prove that once for the encoder, without inspecting a symbolic list. -/
@[simp, lir_eval] theorem findFirstList_map_none {α β : Type}
    (query : RuntimeValue → Option β) (encode : α → RuntimeValue) (values : List α)
    (absent : ∀ value, findFirst query (encode value) = none) :
    findFirstList query (values.map encode) = none := by
  induction values with
  | nil => simp only [List.map_nil, findFirstList]
  | cons value values ih => simp only [List.map_cons, findFirstList, absent, ih]

@[simp, lir_eval] theorem rewriteFirstList_map_none {α : Type}
    (rewrite : RuntimeValue → Option RuntimeValue) (encode : α → RuntimeValue)
    (values : List α) (absent : ∀ value, rewriteFirst rewrite (encode value) = none) :
    rewriteFirstList rewrite (values.map encode) = none := by
  induction values with
  | nil => simp only [List.map_nil, rewriteFirstList]
  | cons value values ih =>
      simp only [List.map_cons, rewriteFirstList, absent, ih, Option.map_none]

/- Updating a loan-free aggregate preserves the absence of other matches.
These certificates visit neither its symbolic prefix nor its suffix. -/
@[simp high, lir_eval high] theorem rewriteFirstList_set_none
    (rewrite : RuntimeValue → Option RuntimeValue) (values : List RuntimeValue)
    (index : Nat) (replacement : RuntimeValue)
    (absent : rewriteFirstList rewrite values = none)
    (replaced : rewriteFirst rewrite replacement = none) :
    rewriteFirstList rewrite (values.set index replacement) = none := by
  induction values generalizing index with
  | nil => simp [List.set, rewriteFirstList]
  | cons value rest ih =>
    cases head : rewriteFirst rewrite value with
    | some found => simp [rewriteFirstList, head] at absent
    | none =>
      have restNone : rewriteFirstList rewrite rest = none := by
        simpa [rewriteFirstList, head] using absent
      cases index with
      | zero => simp [List.set, rewriteFirstList, replaced, restNone]
      | succ index => simp [List.set, rewriteFirstList, head, ih index restNone]
@[simp high, lir_eval high] theorem findFirstList_set_of_absent {α : Type}
    (query : RuntimeValue → Option α) (values : List RuntimeValue)
    (index : Nat) (replacement : RuntimeValue)
    (absent : findFirstList query values = none) (bound : index < values.length) :
    findFirstList query (values.set index replacement) = findFirst query replacement := by
  induction values generalizing index with
  | nil => simp at bound
  | cons value rest ih =>
    cases head : findFirst query value with
    | some found => simp [findFirstList, head] at absent
    | none =>
      have restNone : findFirstList query rest = none := by
        simpa [findFirstList, head] using absent
      cases index with
      | zero =>
        cases selected : findFirst query replacement <;>
          simp [List.set, findFirstList, selected, restNone]
      | succ index =>
        simp only [List.length_cons, Nat.add_one_lt_add_one_iff] at bound
        simp [List.set, findFirstList, head, ih index restNone bound]

@[simp high, lir_eval high] theorem rewriteFirstList_set_of_absent
    (rewrite : RuntimeValue → Option RuntimeValue) (values : List RuntimeValue)
    (index : Nat) (replacement : RuntimeValue)
    (absent : rewriteFirstList rewrite values = none) (bound : index < values.length) :
    rewriteFirstList rewrite (values.set index replacement) =
      (rewriteFirst rewrite replacement).map (values.set index) := by
  induction values generalizing index with
  | nil => simp at bound
  | cons value rest ih =>
    cases head : rewriteFirst rewrite value with
    | some found => simp [rewriteFirstList, head] at absent
    | none =>
      have restNone : rewriteFirstList rewrite rest = none := by
        simpa [rewriteFirstList, head] using absent
      cases index with
      | zero =>
        cases selected : rewriteFirst rewrite replacement <;>
          simp [List.set, rewriteFirstList, selected, restNone]
      | succ index =>
        simp only [List.length_cons, Nat.add_one_lt_add_one_iff] at bound
        simp [List.set, rewriteFirstList, head, ih index restNone bound, Option.map_map,
          Function.comp_def]

@[simp, lir_eval] theorem findFirstList_borrowCurrent_integer_map {α : Type}
    (loan : Nat) (value : α → Int) (values : List α) :
    findFirstList (borrowCurrent? loan) (values.map fun x => .integer (value x)) = none := by
  apply findFirstList_map_none
  intro x
  simp [findFirst_integer]

@[simp, lir_eval] theorem findFirstList_holeMark_integer_map {α : Type}
    (loan : Nat) (value : α → Int) (values : List α) :
    findFirstList (holeMark? loan) (values.map fun x => .integer (value x)) = none := by
  apply findFirstList_map_none
  intro x
  simp [findFirst_integer]

@[simp, lir_eval] theorem rewriteFirstList_borrowClear_integer_map {α : Type}
    (loan : Nat) (value : α → Int) (values : List α) :
    rewriteFirstList (borrowClear? loan) (values.map fun x => .integer (value x)) = none := by
  apply rewriteFirstList_map_none
  intro x
  simp [rewriteFirst_integer]

/-- Lift a completed `endLoans?` computation through the reference
operation wrapper.  The ground evaluator uses this to compose a structural
loan law without unfolding the generic loan search. -/
theorem endLoan_evaluate_of_endLoans {loans : Array LoanId}
    {arguments : Array RuntimeValue} {frame finalFrame : RuntimeFrame}
    {state finalState : RuntimeState} {value : RuntimeValue}
    (evaluated : endLoans? loans arguments frame state =
      some (finalFrame, finalState, value)) :
    (ReferenceLocationOperation.endLoan loans).evaluate? arguments frame state =
      some (.value finalFrame finalState value) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, evaluated]

/-- A lexical death marker needs no reconciliation when every listed loan
has already been returned by a callee. Unrelated active loans are allowed. -/

def listedLoansRetired (loans : Array LoanId) (active : Array (ExprId × Nat)) : Bool :=
  loans.all fun lexical => (active.find? (·.1 == ⟨lexical.index⟩)).isNone

theorem endLoans_listedRetired (loans : Array LoanId) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) (inactive : listedLoansRetired loans frame.activeLoans = true) :
    endLoans? loans arguments frame state =
      match arguments.toList with
      | [value] => some (frame, state, value)
      | [] => some (frame, state, .unit)
      | _ => none := by
  unfold endLoans?
  generalize folded : loans.foldr _ _ = result
  have fold : result = (frame, state) := by
    rw [← folded]
    apply Array.foldr_induction (fun _ pair => pair = (frame, state)) rfl
    intro i pair same
    subst pair
    have absent := (Array.all_eq_true.mp inactive) i.val i.isLt
    have absent := Option.isNone_iff_eq_none.mp absent
    simp only [Fin.getElem_fin] at *
    simp only [absent]
  rw [fold]
  rfl

/-- The definitions an evaluator's definition reaches, restricted to the
semantics: the instances, projections, matchers, and the abstract global
map stay folded. -/
partial def unfoldClosureWith (root : Lean.Name) (keep : Lean.Name → Bool) :
    Lean.MetaM (Array Lean.Name) := do
  let env ← Lean.getEnv
  let mut seen : Lean.NameSet := {}
  let mut queue : Array Lean.Name := #[root]
  let mut out : Array Lean.Name := #[]
  while h : queue.size > 0 do
    let name := queue[queue.size - 1]
    queue := queue.pop
    if seen.contains name then continue
    seen := seen.insert name
    let some (.defnInfo info) := env.find? name | continue
    if ← Lean.Meta.isInstance name then continue
    if env.isProjectionFn name then continue
    if (← Lean.Meta.getMatcherInfo? name).isSome then continue
    out := out.push name
    for used in info.value.getUsedConstants do
      if keep used then
        queue := queue.push used
  return out

/-- The semantics' own closure: the abstract global map and the loan
registry's key lookup stay folded, the contract stating facts about
them. -/
def unfoldClosure (root : Lean.Name) : Lean.MetaM (Array Lean.Name) :=
  unfoldClosureWith root fun used =>
    (`LeanerIR).isPrefixOf (Lean.privateToUserName used) &&
      !(`LeanerIR.GlobalMap).isPrefixOf (Lean.privateToUserName used) &&
      used != ``globalLoanKeyIn? && used != ``globalLoanKey? &&
      used != ``instantiatedTypeId

/-- The closures, computed once. -/
initialize unfoldClosures : IO.Ref (Lean.NameMap (Array Lean.Name)) ← IO.mkRef {}

/-- A literal row: one written out. -/
def isLiteralRow (row : Lean.Expr) : Bool :=
  row.isAppOfArity ``List.toArray 2

/-- A literal frame: a row written out. -/
def isLiteralFrame (frame : Lean.Expr) : Bool :=
  (frame.isAppOfArity ``rowFrame 2 && isLiteralRow (frame.getArg! 0)) ||
    (frame.isAppOfArity ``RuntimeFrame.mk 4 && isLiteralRow (frame.getArg! 0))

/-- The sole element of an array literal. -/
def literalSingleton? (array : Lean.Expr) : Option Lean.Expr := do
  let array := array.consumeMData
  guard (array.isAppOfArity ``List.toArray 2 || array.isAppOfArity ``Array.mk 2)
  let list := (array.getArg! 1).consumeMData
  guard (list.isAppOfArity ``List.cons 3)
  guard ((list.getArg! 2).consumeMData.isAppOfArity ``List.nil 1)
  return (list.getArg! 1).consumeMData

/-- Simplify one ground semantic computation with its own definition
closure, retaining the local facts and evaluator rules but omitting the
ambient WP inventory. -/
def simplifyGround (head : Lean.Name) (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM Lean.Meta.Simp.Result := do
  let closure ← do
    match (← unfoldClosures.get).find? head with
    | some closure => pure closure
    | none =>
        let closure ← unfoldClosure head
        unfoldClosures.modify (·.insert head closure)
        pure closure
  let ctx ← Lean.Meta.Simp.getContext
  let mut extra : Lean.Meta.SimpTheorems := {}
  for name in closure do
    extra ← extra.addDeclToUnfold name
  let evaluatorRules ← match ← Lean.Meta.getSimpExtension? `lir_eval with
    | some extension => extension.getTheorems
    | none => pure {}
  let evaluatorTheorems := if h : ctx.simpTheorems.size > 1 then
      #[ctx.simpTheorems[0], evaluatorRules, ctx.simpTheorems[ctx.simpTheorems.size - 1], extra]
    else (ctx.simpTheorems.push evaluatorRules).push extra
  let ctx' := ctx.setSimpTheorems evaluatorTheorems
  let builtinSimprocs ← Lean.Meta.Simp.getSimprocs
  /- Open the selected operation once before installing evaluator
  simprocs, so its own simproc cannot re-enter it. Nested operations and
  loan-free aggregates then settle in this pass, sharing simp's cache. -/
  let opened : Lean.Meta.Simp.Result ← if head == ``frameBorrows then do
    let proof ← Lean.Meta.mkAppM ``frameBorrows_getD #[e.getArg! 0]
    let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq?
      | throwError "invalid frame borrow observation certificate"
    pure { expr := rhs, proof? := some proof }
  else pure { expr := ← Lean.Meta.withTransparency .all (Lean.Meta.unfoldDefinition e) }
  let simprocs ← match ← Lean.Meta.Simp.getSimprocExtension? `lir_eval with
    | some extension => extension.getSimprocs
    | none => pure {}
  let (result, _) ← Lean.Meta.simp opened.expr ctx' #[builtinSimprocs, simprocs]
  opened.mkEqTrans result

/-- Evaluate `e`, an application of `head`, once its literal argument is
available. -/
def evalGround (head : Lean.Name) (gateArg : Nat) (gate : Lean.Expr → Bool) (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM Lean.Meta.Simp.Step := do
  unless e.getAppNumArgs > gateArg && gate (e.getArg! gateArg) do return .continue
  let start ← IO.getNumHeartbeats
  let result ← simplifyGround head e
  let cost := (← IO.getNumHeartbeats) - start
  if cost > 500000 then trace[leaner.normalize] "ground {head}: {cost}"
  if result.expr == e then return .continue
  return .done result

simproc ↓ [lir_eval] frameBorrowObservation (frameBorrows _) :=
  evalGround ``frameBorrows 0 isLiteralFrame

/-- Evaluate a partial operation only when simplification reaches an actual
`Option` result.  Retaining the folded application on a symbolic residue is
important for loan retirement: its unfolded fold is exponentially larger
than the operation that a later specialized row can recognize. -/
def evalGroundOption (head : Lean.Name) (gateArg : Nat)
    (gate : Lean.Expr → Bool) (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM Lean.Meta.Simp.Step := do
  unless e.getAppNumArgs > gateArg && gate (e.getArg! gateArg) do return .continue
  let start ← IO.getNumHeartbeats
  let result ← simplifyGround head e
  let cost := (← IO.getNumHeartbeats) - start
  if cost > 500000 then trace[leaner.normalize] "ground option {head}: {cost}"
  let reduced := result.expr.consumeMData
  unless reduced.isAppOfArity ``Option.some 2 ||
      reduced.isAppOfArity ``Option.none 1 do
    trace[leaner.normalize] "option residue: {reduced}"
    return .done { expr := e }
  let hasResidualFold ← result.expr.foldlM (init := false) fun found subterm =>
    pure (found || subterm.isConstOf ``forIn)
  if hasResidualFold then
    trace[leaner.normalize] "fold residue: {result.expr}"
    return .done { expr := e }
  return .done result

/-- Export a frame with one outer borrow through the structural
single-borrow equation. -/
def evalSingleBorrowExport (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  unless e.isAppOfArity ``exportFrameLoans 2 do return none
  let frame := (e.getArg! 0).consumeMData
  unless frame.isAppOfArity ``rowFrame 2 && isLiteralRow (frame.getArg! 0) do
    return none
  let state := (e.getArg! 1).consumeMData
  let borrowsApplication ← Lean.Meta.mkAppM ``frameBorrows #[frame]
  let borrowsResult ← simplifyGround ``frameBorrows borrowsApplication
  let some entry := literalSingleton? borrowsResult.expr | return none
  unless entry.isAppOfArity ``Prod.mk 4 do return none
  let loan := (entry.getArg! 2).consumeMData
  let current := (entry.getArg! 3).consumeMData
  let localApplication ← Lean.Meta.mkAppM ``holeInFrame #[frame, loan]
  let localResult ← simplifyGround ``holeInFrame localApplication
  unless localResult.expr.consumeMData.isConstOf ``Bool.false do return none
  let globalApplication ← Lean.Meta.mkAppM ``globalLoanKey? #[state, loan]
  let globalResult ← simplifyGround ``globalLoanKey? globalApplication
  unless globalResult.expr.consumeMData.isAppOfArity ``Option.none 1 do return none
  let proofOf (application : Lean.Expr) (result : Lean.Meta.Simp.Result) :
      Lean.MetaM Lean.Expr := match result.proof? with
    | some proof => pure proof
    | none => Lean.Meta.mkEqRefl application
  let proof ← Lean.Meta.mkAppM ``exportFrameLoans_singleBorrow
    #[frame, state, loan, current, ← proofOf borrowsApplication borrowsResult,
      ← proofOf localApplication localResult, ← proofOf globalApplication globalResult]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private partial def literalArrayElements? (e : Lean.Expr) : Option (Array Lean.Expr) := do
  let e := e.consumeMData
  guard (e.isAppOfArity ``List.toArray 2 || e.isAppOfArity ``Array.mk 2)
  go (e.getArg! 1).consumeMData #[]
where
  go (list : Lean.Expr) (elements : Array Lean.Expr) : Option (Array Lean.Expr) :=
    let list := list.consumeMData
    if list.isAppOfArity ``List.nil 1 then some elements
    else if list.isAppOfArity ``List.cons 3 then
      go (list.getArg! 2) (elements.push (list.getArg! 1).consumeMData)
    else none

private def constructorArg? (e : Lean.Expr) (constructor : Lean.Name)
    (arity index : Nat) : Option Lean.Expr := do
  let e := e.consumeMData
  guard (e.isAppOfArity constructor arity)
  return (e.getArg! index).consumeMData

/-- Keep literal array swaps literal even when their elements are symbolic.
The replacement is certified by reduction; no payload equality is decided. -/
simproc ↓ [lir_eval] evalLiteralArraySwap (Array.swapIfInBounds _ _ _) := fun e => do
  let some elements := literalArrayElements? (e.getArg! 1) | return .continue
  let some left ← Lean.Meta.evalNat (e.getArg! 2) |>.run | return .continue
  let some right ← Lean.Meta.evalNat (e.getArg! 3) |>.run | return .continue
  let result ← Lean.Meta.mkArrayLit (e.getArg! 0)
    (elements.swapIfInBounds left right).toList
  return .done { expr := result }

/-- Recognize the six-local frame produced by two independent mutable
borrows of flat global fields.  Applying the composite law before generic
simplification avoids expanding four nested loan searches. -/
private def evalEndLoansTwoFocusedGlobals (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let law := `LeanerIR.Proofs.Denotation.endLoans?_twoFocusedGlobals_singleField
  unless (← Lean.getEnv).contains law do return none
  let e := e.consumeMData
  unless e.isAppOfArity ``endLoans? 4 do return none
  let some argument := literalSingleton? (e.getArg! 1) | return none
  let frame := (e.getArg! 2).consumeMData
  unless frame.isAppOfArity ``rowFrame 2 do return none
  let some locals := literalArrayElements? (frame.getArg! 0) | return none
  unless locals.size == 6 do return none
  let some local0 := locals[0]? | return none
  let some local1 := locals[1]? | return none
  let some local2 := locals[2]? | return none
  let some local3 := locals[3]? | return none
  let some local4 := locals[4]? | return none
  let some local5 := locals[5]? | return none
  let some addressValue := constructorArg? local0 ``Option.some 2 1 | return none
  let some address := constructorArg? addressValue ``RuntimeValue.address 1 0 | return none
  let some amountValue := constructorArg? local1 ``Option.some 2 1 | return none
  let some amount := constructorArg? amountValue ``RuntimeValue.integer 1 0 | return none
  let some fieldBorrow₁ := constructorArg? local2 ``Option.some 2 1 | return none
  let fieldBorrow₁ := fieldBorrow₁.consumeMData
  unless fieldBorrow₁.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let some value₁ := constructorArg? (fieldBorrow₁.getArg! 1)
      ``RuntimeValue.integer 1 0 | return none
  let some holderBorrow₁ := constructorArg? local4 ``Option.some 2 1 | return none
  let holderBorrow₁ := holderBorrow₁.consumeMData
  unless holderBorrow₁.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let loan := (holderBorrow₁.getArg! 0).consumeMData
  let holder₁ := (holderBorrow₁.getArg! 1).consumeMData
  unless holder₁.isAppOfArity ``RuntimeValue.nominal 3 do return none
  let source₁ := (holder₁.getArg! 0).consumeMData
  let some fieldBorrow₂ := constructorArg? local3 ``Option.some 2 1 | return none
  let fieldBorrow₂ := fieldBorrow₂.consumeMData
  unless fieldBorrow₂.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let some value₂ := constructorArg? (fieldBorrow₂.getArg! 1)
      ``RuntimeValue.integer 1 0 | return none
  let some holderBorrow₂ := constructorArg? local5 ``Option.some 2 1 | return none
  let holderBorrow₂ := holderBorrow₂.consumeMData
  unless holderBorrow₂.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let holder₂ := (holderBorrow₂.getArg! 1).consumeMData
  unless holder₂.isAppOfArity ``RuntimeValue.nominal 3 do return none
  let source₂ := (holder₂.getArg! 0).consumeMData
  let state := (e.getArg! 3).consumeMData
  unless state.isAppOfArity ``RuntimeState.mk 4 do return none
  let stored₂ := (state.getArg! 0).consumeMData
  unless stored₂.isAppOfArity ``GlobalMap.insert 3 do return none
  let stored₁ := (stored₂.getArg! 0).consumeMData
  let key₂ := (stored₂.getArg! 1).consumeMData
  unless stored₁.isAppOfArity ``GlobalMap.insert 3 do return none
  let globals := (stored₁.getArg! 0).consumeMData
  let key₁ := (stored₁.getArg! 1).consumeMData
  let globalLoans₂ := (state.getArg! 1).consumeMData
  unless globalLoans₂.isAppOfArity ``List.cons 3 do return none
  let globalLoans₁ := (globalLoans₂.getArg! 2).consumeMData
  unless globalLoans₁.isAppOfArity ``List.cons 3 do return none
  let rest := (globalLoans₁.getArg! 2).consumeMData
  let nextLoan := (state.getArg! 2).consumeMData
  let pending := (state.getArg! 3).consumeMData
  let proof ← Lean.Meta.mkAppM law
    #[globals, rest, nextLoan, loan, pending, key₁, key₂, address,
      source₁, source₂, amount, value₁, value₂, argument]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalEndLoansPaddedFocusedGlobal (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let law := `LeanerIR.Proofs.Denotation.endLoans?_focusedGlobalPadded_singleField
  unless (← Lean.getEnv).contains law do return none
  let e := e.consumeMData
  unless e.isAppOfArity ``endLoans? 4 do return none
  let some argument := literalSingleton? (e.getArg! 1) | return none
  let frame := (e.getArg! 2).consumeMData
  unless frame.isAppOfArity ``rowFrame 2 do return none
  let some locals := literalArrayElements? (frame.getArg! 0) | return none
  unless locals.size == 6 do return none
  let some local0 := locals[0]? | return none
  let some local1 := locals[1]? | return none
  let some local2 := locals[2]? | return none
  let some local3 := locals[3]? | return none
  let some local4 := locals[4]? | return none
  let some local5 := locals[5]? | return none
  let some addressValue := constructorArg? local0 ``Option.some 2 1 | return none
  let some address := constructorArg? addressValue ``RuntimeValue.address 1 0 | return none
  let some amountValue := constructorArg? local1 ``Option.some 2 1 | return none
  let some amount := constructorArg? amountValue ``RuntimeValue.integer 1 0 | return none
  let some unit2 := constructorArg? local2 ``Option.some 2 1 | return none
  unless unit2.isConstOf ``RuntimeValue.unit do return none
  let some fieldBorrow := constructorArg? local3 ``Option.some 2 1 | return none
  let fieldBorrow := fieldBorrow.consumeMData
  unless fieldBorrow.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let some value := constructorArg? (fieldBorrow.getArg! 1)
      ``RuntimeValue.integer 1 0 | return none
  let some unit4 := constructorArg? local4 ``Option.some 2 1 | return none
  unless unit4.isConstOf ``RuntimeValue.unit do return none
  let some holderBorrow := constructorArg? local5 ``Option.some 2 1 | return none
  let holderBorrow := holderBorrow.consumeMData
  unless holderBorrow.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let holder := (holderBorrow.getArg! 1).consumeMData
  unless holder.isAppOfArity ``RuntimeValue.nominal 3 do return none
  let source := (holder.getArg! 0).consumeMData
  let registries := (frame.getArg! 1).consumeMData
  unless registries.isAppOfArity ``Registries.mk 3 do return none
  let some locations := literalArrayElements? (registries.getArg! 1) | return none
  unless locations.size == 4 do return none
  let some firstLocation := locations[0]? | return none
  unless firstLocation.isAppOfArity ``Prod.mk 4 do return none
  let loan := (firstLocation.getArg! 2).consumeMData
  let previousPlace := (firstLocation.getArg! 3).consumeMData
  unless previousPlace.isAppOfArity ``RuntimePlace.mk 3 do return none
  let previousRoot := (previousPlace.getArg! 0).consumeMData
  let some previousKey := constructorArg? previousRoot
      ``RuntimePlaceRoot.global 1 0 | return none
  let state := (e.getArg! 3).consumeMData
  unless state.isAppOfArity ``RuntimeState.mk 4 do return none
  let stored := (state.getArg! 0).consumeMData
  unless stored.isAppOfArity ``GlobalMap.insert 3 do return none
  let globals := (stored.getArg! 0).consumeMData
  let key := (stored.getArg! 1).consumeMData
  let globalLoans := (state.getArg! 1).consumeMData
  unless globalLoans.isAppOfArity ``List.cons 3 do return none
  let rest := (globalLoans.getArg! 2).consumeMData
  let nextLoan := (state.getArg! 2).consumeMData
  let pending := (state.getArg! 3).consumeMData
  let proof ← Lean.Meta.mkAppM law
    #[globals, rest, nextLoan, loan, pending, previousKey, key, address,
      source, amount, value, argument]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalEndLoansReservedFocusedGlobal (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let law := `LeanerIR.Proofs.Denotation.endLoans?_focusedGlobalReserved_singleField
  unless (← Lean.getEnv).contains law do return none
  let e := e.consumeMData
  unless e.isAppOfArity ``endLoans? 4 do return none
  let some arguments := literalArrayElements? (e.getArg! 1) | return none
  unless arguments.isEmpty do return none
  let frame := (e.getArg! 2).consumeMData
  unless frame.isAppOfArity ``rowFrame 2 do return none
  let some locals := literalArrayElements? (frame.getArg! 0) | return none
  unless locals.size == 6 do return none
  let some local0 := locals[0]? | return none
  let some local1 := locals[1]? | return none
  let some local2 := locals[2]? | return none
  let some local3 := locals[3]? | return none
  let some local4 := locals[4]? | return none
  let some local5 := locals[5]? | return none
  let some addressValue := constructorArg? local0 ``Option.some 2 1 | return none
  let some address := constructorArg? addressValue ``RuntimeValue.address 1 0 | return none
  let some amountValue := constructorArg? local1 ``Option.some 2 1 | return none
  let some amount := constructorArg? amountValue ``RuntimeValue.integer 1 0 | return none
  let some fieldBorrow := constructorArg? local2 ``Option.some 2 1 | return none
  let fieldBorrow := fieldBorrow.consumeMData
  unless fieldBorrow.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let some value := constructorArg? (fieldBorrow.getArg! 1)
      ``RuntimeValue.integer 1 0 | return none
  unless local3.consumeMData.isAppOfArity ``Option.none 1 do return none
  let some holderBorrow := constructorArg? local4 ``Option.some 2 1 | return none
  let holderBorrow := holderBorrow.consumeMData
  unless holderBorrow.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let loan := (holderBorrow.getArg! 0).consumeMData
  let holder := (holderBorrow.getArg! 1).consumeMData
  unless holder.isAppOfArity ``RuntimeValue.nominal 3 do return none
  let source := (holder.getArg! 0).consumeMData
  unless local5.consumeMData.isAppOfArity ``Option.none 1 do return none
  let state := (e.getArg! 3).consumeMData
  unless state.isAppOfArity ``RuntimeState.mk 4 do return none
  let stored := (state.getArg! 0).consumeMData
  unless stored.isAppOfArity ``GlobalMap.insert 3 do return none
  let globals := (stored.getArg! 0).consumeMData
  let key := (stored.getArg! 1).consumeMData
  let globalLoans := (state.getArg! 1).consumeMData
  unless globalLoans.isAppOfArity ``List.cons 3 do return none
  let rest := (globalLoans.getArg! 2).consumeMData
  let nextLoan := (state.getArg! 2).consumeMData
  let pending := (state.getArg! 3).consumeMData
  let proof ← Lean.Meta.mkAppM law
    #[globals, rest, nextLoan, loan, pending, key, address, source,
      amount, value]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

/-- Recognize the closed five-local frame produced by a shared nominal read
followed by a mutable borrow of a flat resource field, and instantiate the
corresponding composite evaluator law directly.  Syntactic extraction keeps
this hot path linear; asking the general simplifier to unify the full frame
law at every recursive loan-search node is substantially more expensive. -/
private def evalEndLoansAfterSharedNominal (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let law :=
    `LeanerIR.Proofs.Denotation.endLoans?_focusedGlobalAfterSharedNominal_singleField
  unless (← Lean.getEnv).contains law do return none
  let e := e.consumeMData
  unless e.isAppOfArity ``endLoans? 4 do return none
  let some argument := literalSingleton? (e.getArg! 1) | return none
  let frame := (e.getArg! 2).consumeMData
  unless frame.isAppOfArity ``rowFrame 2 do return none
  let some locals := literalArrayElements? (frame.getArg! 0) | return none
  unless locals.size == 5 do return none
  let some local0 := locals[0]? | return none
  let some local1 := locals[1]? | return none
  let some local3 := locals[3]? | return none
  let some local4 := locals[4]? | return none
  let some addressValue := constructorArg? local0 ``Option.some 2 1 | return none
  let some address := constructorArg? addressValue ``RuntimeValue.address 1 0 | return none
  let some savedValue := constructorArg? local1 ``Option.some 2 1 | return none
  let savedValue := savedValue.consumeMData
  unless savedValue.isAppOfArity ``RuntimeValue.nominal 3 do return none
  let source := (savedValue.getArg! 0).consumeMData
  let some savedField := literalSingleton? (savedValue.getArg! 2) | return none
  let some saved := constructorArg? savedField ``RuntimeValue.integer 1 0 | return none
  let some fieldBorrow := constructorArg? local3 ``Option.some 2 1 | return none
  let fieldBorrow := fieldBorrow.consumeMData
  unless fieldBorrow.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let some fieldValue := constructorArg? (fieldBorrow.getArg! 1)
      ``RuntimeValue.integer 1 0 | return none
  let some holderBorrow := constructorArg? local4 ``Option.some 2 1 | return none
  let holderBorrow := holderBorrow.consumeMData
  unless holderBorrow.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let loan := (holderBorrow.getArg! 0).consumeMData
  let holder := (holderBorrow.getArg! 1).consumeMData
  unless holder.isAppOfArity ``RuntimeValue.nominal 3 do return none
  let focusSource := (holder.getArg! 0).consumeMData
  let state := (e.getArg! 3).consumeMData
  unless state.isAppOfArity ``RuntimeState.mk 4 do return none
  let stored := (state.getArg! 0).consumeMData
  unless stored.isAppOfArity ``GlobalMap.insert 3 do return none
  let globals := (stored.getArg! 0).consumeMData
  let key := (stored.getArg! 1).consumeMData
  let globalLoans := (state.getArg! 1).consumeMData
  unless globalLoans.isAppOfArity ``List.cons 3 do return none
  let rest := (globalLoans.getArg! 2).consumeMData
  let nextLoan := (state.getArg! 2).consumeMData
  let pending := (state.getArg! 3).consumeMData
  let proof ← Lean.Meta.mkAppM law
    #[globals, rest, nextLoan, loan, pending, key, address, source, focusSource,
      saved, fieldValue, argument]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalReferenceEndLoansAfterSharedNominal (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let e := e.consumeMData
  unless e.isAppOfArity ``ReferenceLocationOperation.evaluate? 4 do return none
  let operation := (e.getArg! 0).consumeMData
  unless operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 do return none
  let endLoans ← Lean.Meta.mkAppM ``endLoans?
    #[operation.getArg! 0, e.getArg! 1, e.getArg! 2, e.getArg! 3]
  let some evaluated ← evalEndLoansAfterSharedNominal endLoans | return none
  let some evaluatedProof := evaluated.proof? | return none
  let proof ← Lean.Meta.mkAppM ``endLoan_evaluate_of_endLoans #[evaluatedProof]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalReferenceEndLoansTwoFocusedGlobals (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let e := e.consumeMData
  unless e.isAppOfArity ``ReferenceLocationOperation.evaluate? 4 do return none
  let operation := (e.getArg! 0).consumeMData
  unless operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 do return none
  let endLoans ← Lean.Meta.mkAppM ``endLoans?
    #[operation.getArg! 0, e.getArg! 1, e.getArg! 2, e.getArg! 3]
  let some evaluated ← evalEndLoansTwoFocusedGlobals endLoans | return none
  let some evaluatedProof := evaluated.proof? | return none
  let proof ← Lean.Meta.mkAppM ``endLoan_evaluate_of_endLoans #[evaluatedProof]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalReferenceEndLoansPaddedFocusedGlobal (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let e := e.consumeMData
  unless e.isAppOfArity ``ReferenceLocationOperation.evaluate? 4 do return none
  let operation := (e.getArg! 0).consumeMData
  unless operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 do return none
  let endLoans ← Lean.Meta.mkAppM ``endLoans?
    #[operation.getArg! 0, e.getArg! 1, e.getArg! 2, e.getArg! 3]
  let some evaluated ← evalEndLoansPaddedFocusedGlobal endLoans | return none
  let some evaluatedProof := evaluated.proof? | return none
  let proof ← Lean.Meta.mkAppM ``endLoan_evaluate_of_endLoans #[evaluatedProof]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalReferenceEndLoansReservedFocusedGlobal (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  let e := e.consumeMData
  unless e.isAppOfArity ``ReferenceLocationOperation.evaluate? 4 do return none
  let operation := (e.getArg! 0).consumeMData
  unless operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 do return none
  let endLoans ← Lean.Meta.mkAppM ``endLoans?
    #[operation.getArg! 0, e.getArg! 1, e.getArg! 2, e.getArg! 3]
  let some evaluated ← evalEndLoansReservedFocusedGlobal endLoans | return none
  let some evaluatedProof := evaluated.proof? | return none
  let proof ← Lean.Meta.mkAppM ``endLoan_evaluate_of_endLoans #[evaluatedProof]
  let some (_, lhs, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  unless ← Lean.Meta.isDefEq lhs e do return none
  return some { expr := rhs, proof? := some proof }

private def evalRetiredEndLoans (e : Lean.Expr) :
    Lean.Meta.Simp.SimpM (Option Lean.Meta.Simp.Result) := do
  unless e.isAppOfArity ``endLoans? 4 do return none
  let frame := e.getArg! 2
  unless isLiteralFrame frame do return none
  let active ← Lean.Meta.mkAppM ``RuntimeFrame.activeLoans #[frame]
  let check ← Lean.Meta.mkAppM ``listedLoansRetired #[e.getArg! 0, active]
  let checked ← simplifyGround ``listedLoansRetired check
  unless checked.expr.isConstOf ``Bool.true do return none
  let proof ← Lean.Meta.mkAppM ``endLoans_listedRetired
    #[e.getArg! 0, e.getArg! 1, frame, e.getArg! 3, ← checked.getProof]
  let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return none
  let rhs ← Lean.Meta.whnf rhs
  unless rhs.isAppOfArity ``Option.some 2 || rhs.isAppOfArity ``Option.none 1 do return none
  return some { expr := rhs, proof? := some proof }

simproc [lir_eval] evalEndLoans (endLoans? _ _ _ _) := fun e => do
  if let some result ← evalRetiredEndLoans e then return .done result
  if let some result ← evalEndLoansReservedFocusedGlobal e then return .done result
  if let some result ← evalEndLoansPaddedFocusedGlobal e then return .done result
  if let some result ← evalEndLoansTwoFocusedGlobals e then return .done result
  if let some result ← evalEndLoansAfterSharedNominal e then return .done result
  evalGroundOption ``endLoans? 2 isLiteralFrame e

simproc [lir_eval] evalMutate (mutateBorrow? _ _ _) :=
  evalGround ``mutateBorrow? 1 isLiteralFrame

simproc [lir_eval] evalExport (exportFrameLoans _ _) := fun e => do
  if let some result ← evalSingleBorrowExport e then return .done result
  evalGround ``exportFrameLoans 0 isLiteralFrame e

simproc [lir_eval] evalReturnedExport (exportReturnedFrameLoans _ _ _) :=
  evalGround ``exportReturnedFrameLoans 1 isLiteralFrame

simproc ↓ [lir_eval] exportPlainResult (exportReturnedFrameLoans _ _ _) := fun e => do
  let results := (e.getArg! 0).consumeMData
  if (results.isAppOfArity ``List.toArray 2 || results.isAppOfArity ``Array.mk 2) &&
      (results.getArg! 1).isAppOf ``List.nil then
    let proof ← Lean.Meta.mkAppM ``exportReturnedFrameLoans_empty #[e.getArg! 1, e.getArg! 2]
    let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
    return .done { expr := rhs, proof? := some proof }
  let some value := literalSingleton? (e.getArg! 0) | return .continue
  let some plain ← plainValueCertificate? value | return .continue
  let proof ← Lean.Meta.mkAppM ``exportReturnedFrameLoans_singlePlain
    #[plain, e.getArg! 1, e.getArg! 2]
  let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
  return .done { expr := rhs, proof? := some proof }

simproc [lir_eval] evalGlobal (GlobalLocationOperation.evaluate? _ _ _ _) :=
  evalGround ``GlobalLocationOperation.evaluate? 2 isLiteralFrame

simproc [lir_eval] evalLocal (LocalLocationOperation.evaluate? _ _ _ _) :=
  evalGround ``LocalLocationOperation.evaluate? 2 isLiteralFrame

simproc [lir_eval] evalDerefLocalBorrow (DerefLocalBorrowOperation.evaluate? _ _ _ _) :=
  evalGround ``DerefLocalBorrowOperation.evaluate? 2 isLiteralFrame

theorem indexedBorrow_unresolved (operation : IndexedLocalBorrowOperation)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState)
    (missing : operation.resolve? frame state = none) :
    operation.evaluate? arguments frame state = none := by
  simp [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator, missing]

theorem indexedFieldBorrow_unresolved (operation : IndexedLocalFieldBorrowOperation)
    (arguments : Array RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState)
    (missing : operation.resolve? frame state = none) :
    operation.evaluate? arguments frame state = none := by
  simp [IndexedLocalFieldBorrowOperation.evaluate?, liftPlaceEvaluator, missing]

/- Resolve the place before opening mutation/loan machinery. Invalid indices
and wrong nominal tags need no state traversal or borrower definition closure. -/
private def evalIndexedBorrow (resolver unresolved evaluator : Lean.Name)
    (e : Lean.Expr) : Lean.Meta.Simp.SimpM Lean.Meta.Simp.Step := do
  unless isLiteralFrame (e.getArg! 2) do return .continue
  let call ← Lean.Meta.mkAppM resolver #[e.getArg! 0, e.getArg! 2, e.getArg! 3]
  /- Closed failures reduce directly, without constructing even the read
  evaluator's simplifier closure. Symbolic bounds use the local facts. -/
  let reduced ← Lean.Meta.withTransparency .all <| Lean.Meta.whnf call
  let resolved : Lean.Meta.Simp.Result ← if reduced.isAppOfArity ``Option.none 1 then
      pure { expr := reduced }
    else simplifyGround resolver call
  if resolved.expr.isAppOfArity ``Option.none 1 then
    let proof ← Lean.Meta.mkAppM unresolved
      #[e.getArg! 0, e.getArg! 1, e.getArg! 2, e.getArg! 3, ← resolved.getProof]
    let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
    return .done { expr := rhs, proof? := some proof }
  evalGround evaluator 2 isLiteralFrame e

simproc [lir_eval] evalIndexedLocalBorrow (IndexedLocalBorrowOperation.evaluate? _ _ _ _) :=
  evalIndexedBorrow ``IndexedLocalBorrowOperation.resolve? ``indexedBorrow_unresolved
    ``IndexedLocalBorrowOperation.evaluate?

simproc [lir_eval] evalIndexedLocalFieldBorrow
    (IndexedLocalFieldBorrowOperation.evaluate? _ _ _ _) :=
  evalIndexedBorrow ``IndexedLocalFieldBorrowOperation.resolve? ``indexedFieldBorrow_unresolved
    ``IndexedLocalFieldBorrowOperation.evaluate?

simproc [lir_eval] evalReference (ReferenceLocationOperation.evaluate? _ _ _ _) := fun e => do
  if e.isAppOfArity ``ReferenceLocationOperation.evaluate? 4 then
    let operation := (e.getArg! 0).consumeMData
    if operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 then
      let call ← Lean.Meta.mkAppM ``endLoans?
        #[operation.getArg! 0, e.getArg! 1, e.getArg! 2, e.getArg! 3]
      if let some evaluated ← evalRetiredEndLoans call then
        if evaluated.expr.isAppOfArity ``Option.some 2 then
          let proof ← Lean.Meta.mkAppM ``endLoan_evaluate_of_endLoans #[← evaluated.getProof]
          let some (_, _, rhs) := (← Lean.Meta.inferType proof).eq? | return .continue
          return .done { expr := rhs, proof? := some proof }
  if let some result ← evalReferenceEndLoansReservedFocusedGlobal e then return .done result
  if let some result ← evalReferenceEndLoansPaddedFocusedGlobal e then return .done result
  if let some result ← evalReferenceEndLoansTwoFocusedGlobals e then return .done result
  if let some result ← evalReferenceEndLoansAfterSharedNominal e then return .done result
  let operation := (e.getArg! 0).consumeMData
  if operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 then
    evalGroundOption ``ReferenceLocationOperation.evaluate? 2 isLiteralFrame e
  else
    evalGround ``ReferenceLocationOperation.evaluate? 2 isLiteralFrame e

simproc [lir_eval] evalFieldSelect (NominalFieldLocation.evaluateSelect? _ _ _ _) :=
  evalGround ``NominalFieldLocation.evaluateSelect? 2 isLiteralFrame

simproc [lir_eval] evalVariantFieldSelect
    (NominalVariantFieldLocation.evaluateSelect? _ _ _ _) :=
  evalGround ``NominalVariantFieldLocation.evaluateSelect? 2 isLiteralFrame

simproc [lir_eval] evalVariantTest (NominalVariantTest.evaluate? _ _ _ _) :=
  evalGround ``NominalVariantTest.evaluate? 2 isLiteralFrame

simproc [lir_eval] evalConstructor (NominalConstructor.evaluate? _ _ _ _) :=
  evalGround ``NominalConstructor.evaluate? 2 isLiteralFrame

simproc [lir_eval] evalPrimitive (PrimitiveLocationOperation.evaluate? _ _ _ _) :=
  evalGround ``PrimitiveLocationOperation.evaluate? 1 isLiteralRow

simproc [lir_eval] evalBind (NativePatternBinder.bind _ _ _) :=
  evalGround ``NativePatternBinder.bind 1 isLiteralFrame

/-- The elements of a literal list. -/
partial def literalList? (e : Lean.Expr) : Option (Array Lean.Expr) :=
  go e #[]
where
  go (e : Lean.Expr) (acc : Array Lean.Expr) : Option (Array Lean.Expr) :=
    if e.isAppOfArity ``List.nil 1 then some acc
    else if e.isAppOfArity ``List.cons 3 then go (e.getArg! 2) (acc.push (e.getArg! 1))
    else none

/-- The entry row of a literal count and literal arguments, written out;
the equation holds by reduction. -/
simproc [lir_eval] evalInitialLocals (initialLocals _ _) := fun e => do
  let some count ← Lean.Meta.evalNat (e.getArg! 0) |>.run | return .continue
  let row := e.getArg! 1
  unless isLiteralRow row do return .continue
  let some arguments := literalList? (row.getArg! 1) | return .continue
  let valueType := Lean.mkConst ``RuntimeValue
  let optionType := Lean.mkApp (Lean.mkConst ``Option [Lean.Level.zero]) valueType
  let mut elements : Array Lean.Expr := #[]
  for index in [0:count] do
    if h : index < arguments.size then
      elements := elements.push
        (Lean.mkApp2 (Lean.mkConst ``Option.some [Lean.Level.zero]) valueType arguments[index])
    else
      elements := elements.push (Lean.mkApp (Lean.mkConst ``Option.none [Lean.Level.zero]) valueType)
  let list ← Lean.Meta.mkListLit optionType elements.toList
  let result ← Lean.Meta.mkAppM ``List.toArray #[list]
  return .done { expr := result }

/-! ## The tactic -/

/-- Normalize the goal: the tree's computation, the weakest preconditions
of its primitives, the evaluations at the literal rows they reach, and the
data laws that decide the residue.  The facts given are the goal's own
(the resources present at the keys it reads); they join the set. -/
syntax "leaner_normalize" (" [" (Lean.Parser.Tactic.simpStar <|> Lean.Parser.Tactic.simpErase <|>
  Lean.Parser.Tactic.simpLemma),* "]")? : tactic

syntax "leaner_normalize_lazy" (" [" (Lean.Parser.Tactic.simpStar <|>
  Lean.Parser.Tactic.simpErase <|> Lean.Parser.Tactic.simpLemma),* "]")? : tactic

macro_rules
  | `(tactic| leaner_normalize_lazy $[[$facts,*]]?) =>
      `(tactic| set_option leaner.lazyComputations true in leaner_normalize $[[$facts,*]]?)

/-- The facts of the context normalization reads through: the equations
about the initial state — what the keys hold, which loans are registered
where. -/
def presentFacts : Lean.MetaM (Array (Lean.FVarId × Bool)) := do
  let mut initial? : Option Lean.FVarId := none
  for declaration in ← Lean.getLCtx do
    if declaration.isImplementationDetail then continue
    if declaration.userName == `initial then initial? := some declaration.fvarId
  let some initial := initial? | return #[]
  let mut names := #[]
  for declaration in ← Lean.getLCtx do
    if declaration.isImplementationDetail then continue
    let type ← Lean.instantiateMVars declaration.type
    if type.isAppOfArity ``Eq 3 && (type.getArg! 1).containsFVar initial then
      names := names.push (declaration.fvarId, false)
    else if type.isAppOfArity ``FreshGlobalLoanIds 1 then
      names := names.push (declaration.fvarId, false)
    else if type.isAppOfArity ``IntegerValueFits 3 then
      /- A literal field decoder selects its successful branch from the
      certificate already carried by a typed integer parameter. -/
      names := names.push (declaration.fvarId, false)
    else if type.isAppOfArity ``Ne 3 || type.isAppOfArity ``Not 1 ||
        type.isAppOfArity ``LT.lt 4 || type.isAppOfArity ``LE.le 4 then
      /- The distinctness and bounds of the loans the contract states: a
      registry walk compares loan ids, and these decide it; a distinctness
      is read in both orientations. -/
      let inner := if type.isAppOfArity ``Not 1 then type.getArg! 0 else type
      if inner.isAppOfArity ``Eq 3 || inner.isAppOfArity ``Ne 3 ||
          inner.isAppOfArity ``LT.lt 4 || inner.isAppOfArity ``LE.le 4 then
        let carrier ← Lean.Meta.inferType (inner.getAppArgs.back!)
        if carrier.isConstOf ``Nat || carrier.isConstOf ``Int then
          let distinct := inner.isAppOfArity ``Eq 3 || inner.isAppOfArity ``Ne 3
          names := names.push (declaration.fvarId, distinct)
  return names

open Lean Elab Tactic in
elab_rules : tactic
  | `(tactic| leaner_normalize $[[$facts,*]]?) => do
      leafCertificates.set {}
      undecidedEqualities.set {}
      let present ← (← getMainGoal).withContext presentFacts
      let mut presentLemmas : Array (TSyntax `Lean.Parser.Tactic.simpLemma) := #[]
      for (id, distinct) in present do
        -- Nested generated guards can share a user-facing name. Embed the
        -- exact local proof rather than resolving that name again later.
        let fact ← withMainContext do Term.exprToSyntax (mkFVar id)
        presentLemmas := presentLemmas.push
          (← `(Lean.Parser.Tactic.simpLemma| $fact:term))
        if distinct then
          presentLemmas := presentLemmas.push
            (← `(Lean.Parser.Tactic.simpLemma| (Ne.symm $fact:term)))
      let given : Array (TSyntax [`Lean.Parser.Tactic.simpStar, `Lean.Parser.Tactic.simpErase,
          `Lean.Parser.Tactic.simpLemma]) := match facts with
        | some facts => facts.getElems
        | none => #[]
      let presentGiven : Array (TSyntax [`Lean.Parser.Tactic.simpStar,
          `Lean.Parser.Tactic.simpErase, `Lean.Parser.Tactic.simpLemma]) :=
        presentLemmas.map fun lemma => ⟨lemma.raw⟩
      let given := given ++ presentGiven
      if ← leaner.lazyComputations.getM then
        let invocation ← `(tactic|
          simp [-Bool.exists_bool, -Array.map_eq_singleton_iff,
            RowState.frame, lir_wp_norm, lir_eval, $given,*])
        withMainContext do withSimpDiagnostics do
          let r ← mkSimpContext invocation (eraseLocal := false)
          /- `simp [-definition]` only erases from the default set. Filter
          every added set, preserving the surrounding environment and the
          normalizer's supplied facts, procedures, and diagnostics. -/
          let unfoldings := #[``value, ``localVar, ``valuesNil, ``valuesCons,
            ``statementsNil, ``statementsCons, ``blockUnit, ``blockResult,
            ``operation, ``branch, ``throw_, ``letValue, ``call, ``callAt]
          let ctx := r.ctx.setSimpTheorems (r.ctx.simpTheorems.map fun theorems =>
            unfoldings.foldl (fun theorems name => theorems.eraseCore (.decl name)) theorems)
          let r := { r with ctx }
          let stats ← r.dischargeWrapper.with fun discharge? =>
            withLoopChecking r <| simpLocation ctx r.simprocs discharge? (.targets #[] true)
          if tactic.simp.trace.get (← getOptions) then
            traceSimpCall invocation stats.usedTheorems
          else if Lean.Linter.getLinterValue linter.unusedSimpArgs
              (← Lean.Linter.getLinterOptions) then
            warnUnusedSimpArgs r.simpArgs stats.usedTheorems
          return stats.diag
      else
        evalTactic (← `(tactic|
          -- A decoded Boolean result has one determined witness, not two
          -- alternatives to prove before its result codec is exposed.
          -- Preserve vector equality guards as equations. Inverting a mapped
          -- singleton here introduces element existentials that the contract
          -- does not use, obscuring the same equality in its postcondition.
          simp [-Bool.exists_bool, -Array.map_eq_singleton_iff,
            Tree.compute, Operands.compute, Statements.compute, RowState.frame,
            lir_wp_norm, lir_eval, $given,*]))

/-- An invariant is selected before the loop body or its continuation is
normalized. This boundary never unfolds the finite loop relation. -/
simproc ↓ [lir_wp_norm] stopAtLoop (wp (loop _ _) _ _ _) := fun e =>
  return .done { expr := e }

/-- Loop branch guards must be available before resolving dynamic places.
Keep only this computation boundary folded; globally contextual simp would
also inspect every quantified contract and continuation. -/
simproc ↓ [lir_wp_norm] stopAtLoopBranch (wp (ite _ _ _) _ _ _) := fun e => do
  if (← leaner.branchBoundaries.getM) && e.isAppOfArity ``wp 7 &&
      (e.getArg! 3).isAppOfArity ``ite 5 then return .done { expr := e }
  return .continue

/- Consume sequencing before visiting its continuation. The syntactic
guard matters: applying `wp_bind` through definitional equality could
unfold an entire evaluator instead of using its dedicated WP law. -/
simproc ↓ [lir_wp_norm] sequenceBeforePost (wp _ _ _ _) := fun e => do
  let arguments := e.getAppArgs
  unless arguments.size ≥ 4 do return .continue
  let index := arguments.size - 4
  let action := arguments[index]!
  let head := action.getAppFn
  let guardedMatcher ← if ← leaner.branchBoundaries.getM then
      match head.constName? with
      | some name => pure (← Lean.Meta.getMatcherInfo? name).isSome
      | none => pure false
    else pure false
  if !(← leaner.lazyComputations.getM) && !guardedMatcher then
    if head.isConstOf ``evaluate then
      -- Let operation-level focus rules see their boundary before applying
      -- the generic evaluator law. Other operations retain eager sequencing.
      let evaluator := action.getArg! 0
      if evaluator.isAppOfArity ``DerefLocalBorrowOperation.evaluate? 1 ||
          evaluator.isAppOfArity ``IndexedLocalBorrowOperation.evaluate? 1 then
        return .continue
      if evaluator.isAppOfArity ``ReferenceLocationOperation.evaluate? 1 &&
          (evaluator.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate then
        return .continue
      return ← Lean.Meta.Simp.rewritePost (rflOnly := false) e
    if [``Spec.bind, ``Bind.bind, ``Spec.pure, ``Pure.pure, ``get, ``set,
        ``invoke].any head.isConstOf then
      return ← Lean.Meta.Simp.rewritePost (rflOnly := false) e
    if [``value, ``localVar, ``valuesNil, ``valuesCons, ``statementsNil,
        ``statementsCons, ``blockUnit, ``blockResult, ``operation, ``branch,
        ``throw_, ``letValue, ``call, ``callAt].any head.isConstOf then
      if let some opened ← Lean.Meta.unfoldDefinition? action (ignoreTransparency := true) then
        return .visit { expr := Lean.mkAppN e.getAppFn (arguments.set! index opened) }
    return .continue
  let monadicNotation := [``Bind.bind, ``Pure.pure].any head.isConstOf
  let matcher ← match head.constName? with
    | some name => pure (← Lean.Meta.getMatcherInfo? name).isSome
    | none => pure false
  if monadicNotation || matcher then
    /- Resolve notation before `wp_bind`, and the selected match before
    `wp_pure`. Ordinary congruence would simplify the action and then visit
    its still-symbolic postcondition before retrying the enclosing WP rule.
    Reduce just one match, not its whole selected computation. -/
    let step ← if monadicNotation then
        Lean.Meta.Simp.rewritePost (rflOnly := false) action
      else Lean.Meta.Simp.simpMatch action
    let result ← match step with
      | .visit result | .done result => pure result
      | .continue _ => return .continue
    if result.expr == action then return .continue
    let proof? ← match result.proof? with
      | none => pure none
      | some proof =>
        Lean.Meta.withLocalDeclD `action (← Lean.Meta.inferType action) fun boundAction => do
          let fn ← Lean.Meta.mkLambdaFVars #[boundAction]
            (Lean.mkAppN e.getAppFn (arguments.set! index boundAction))
          pure (some (← Lean.Meta.mkCongrArg fn proof))
    return .visit {
      expr := Lean.mkAppN e.getAppFn (arguments.set! index result.expr), proof? }
  if head.isConstOf ``evaluate then
    -- Let operation-level focus rules see their boundary before applying
    -- the generic evaluator law. Other operations retain eager sequencing.
    let evaluator := action.getArg! 0
    if evaluator.isAppOfArity ``DerefLocalBorrowOperation.evaluate? 1 ||
        evaluator.isAppOfArity ``IndexedLocalBorrowOperation.evaluate? 1 then
      return .continue
    if evaluator.isAppOfArity ``ReferenceLocationOperation.evaluate? 1 &&
        (evaluator.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate then
      return .continue
    return ← Lean.Meta.Simp.rewritePost (rflOnly := false) e
  if [``Spec.bind, ``Spec.pure, ``get, ``set,
      ``invoke].any head.isConstOf then
    return ← Lean.Meta.Simp.rewritePost (rflOnly := false) e
  if [``Tree.compute, ``Operands.compute, ``Statements.compute,
      ``value, ``localVar, ``valuesNil, ``valuesCons, ``statementsNil,
      ``statementsCons, ``blockUnit, ``blockResult, ``operation, ``branch,
      ``throw_, ``letValue, ``call, ``callAt].any head.isConstOf then
    -- Smart unfolding must see the constructor behind a generated tree
    -- definition, not just unfold `compute` itself at reducible transparency.
    if let some opened ← Lean.Meta.withTransparency .default <|
        Lean.Meta.unfoldDefinition? action (ignoreTransparency := true) then
      return .visit { expr := Lean.mkAppN e.getAppFn (arguments.set! index opened) }
  return .continue

/-- Expose only the boundary of a legacy hand script. Ordinary export stays
folded, and returned handles use the shape-independent no-alias certificate. -/
syntax "leaner_finalize" : tactic

open Lean Meta in
simproc ↓ [lir_eval] exportUnaliasedResult (exportReturnedFrameLoans _ _ _) := fun e => do
  let some value := literalSingleton? (e.getArg! 0) | return .continue
  unless value.isAppOfArity ``RuntimeValue.borrow 2 do return .continue
  let frame := e.getArg! 1
  let locals ← if frame.isAppOfArity ``rowFrame 2 then pure (frame.getArg! 0)
    else whnf (← mkAppM ``RuntimeFrame.locals #[frame])
  unless locals.isAppOfArity ``List.toArray 2 || locals.isAppOfArity ``Array.mk 2 do
    return .continue
  let step ← mkAppM ``exportReturnedFrameLoans_noAliases e.getAppArgs
  let .forallE _ unchanged _ _ ← inferType step | return .continue
  let certificate ← mkFreshExprMVar unchanged
  let saved ← (Lean.Meta.saveState : MetaM Lean.Meta.SavedState)
  try
    let remaining ← Lean.Elab.Term.TermElabM.run' do
      Lean.Elab.Tactic.run certificate.mvarId! do
        Lean.Elab.Tactic.withoutRecover <| Lean.Elab.Tactic.evalTactic (← `(tactic|
          simp (disch := omega) [rowFrame_locals, returnedBorrowIds_singleBorrow,
            maskReturnedBorrows_single_other, maskReturnedPlainAggregate]))
    unless remaining.isEmpty do
      saved.restore
      return .continue
    let proof := mkApp step (← instantiateMVars certificate)
    let some (_, _, rhs) := (← inferType proof).eq? | return .continue
    return .done { expr := rhs, proof? := some proof }
  catch _ =>
    saved.restore
    return .continue

macro_rules
  | `(tactic| leaner_finalize) =>
      `(tactic| simp only [finalizeFunctionState, exportPlainResult, exportUnaliasedResult])

end LeanerIR.Proofs.Denotation.RowSpec
