-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.NativeCall
import LeanerIR.Semantics.Focus
import LeanerIR.Proofs.Plain

/-!
# Storage over the frame-free row: the loan-bracketed segment

A storage body is one *loan-bracketed segment*: a mutable global borrow
(and, for a field, a reborrow along a nominal path) mints the loans and
registers them, an inner scalar block runs at those registries, and the
paired `endLoan` retires the loans, writing the resource back under its
key.  Inside the bracket the registries are fixed, so the ordinary row
rules drive the inner block; the bracket's entry and exit are closed
computations, and its exit is stated through the family representation —
the typed contents, not the map — which is what makes the obligations the
closer sees purely typed.
-/

namespace LeanerIR.Proofs.Denotation

open LeanerIR.SemanticOperations

/-- Native vector construction is a closed row operation. -/
theorem vector_evaluate (values : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.vector.evaluate? values frame state =
      some (.value frame state (.vector values)) := rfl

/-- Native vector append is a closed row operation. -/
theorem pushVector_evaluate (values : Array RuntimeValue)
    (value : RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    PrimitiveLocationOperation.pushVector.evaluate?
        #[.vector values, value] frame state =
      some (.value frame state (.vector (values.push value))) := rfl

/- Keep focused aggregate projections opaque until the focus laws can
rewrite them.  Unfolding `RuntimeValue.field` first exposes symbolic array
lookups and makes the arithmetic closer pay for irrelevant tails. -/
attribute [lir_data_norm high]
  VectorFocus.runtimeField_fill
  VectorFocus.runtimeAsIntField_fill
  VectorFocus.runtimeField_fill_zero
  VectorFocus.runtimeAsIntField_fill_zero
  VectorFocus.runtimeField_vector_zero
  VectorFocus.runtimeAsIntField_vector_zero

/-! ## Entry: the global borrow and the field reborrow, computed -/

/-- A mutable borrow of a present resource, over a row whose local 0 holds
the address: mint, hole the slot, register the loan under its key. -/
theorem globalBorrow_evaluate_rowFrame (namespaceId : NamespaceId)
    (typeId : TypeId) (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (address : String) (rest : List (Option RuntimeValue))
    (registries : Registries) (state : RuntimeState) (resource : RuntimeValue)
    (present : state.globals.lookup
      (globalKey namespaceId
        (instantiatedTypeId registries.typeInstantiation typeId)
        (.address address)) = some resource) :
    (GlobalLocationOperation.borrow
      { resource := ⟨namespaceId, typeId⟩, referenceType, kind := .mutable
        lexicalLoan := lex }).evaluate?
      #[.address address]
      (rowFrame (some (.address address) :: rest).toArray registries) state =
    some (.value
      (rowFrame (some (.address address) :: rest).toArray
        { activeLoans :=
            (registries.activeLoans.filter (·.1 != ⟨lex⟩)).push
              (⟨lex⟩, state.nextLoan)
          loanLocations := registries.loanLocations.push
            (state.nextLoan,
              { root := .global (globalKey namespaceId
                  (instantiatedTypeId registries.typeInstantiation typeId)
                  (.address address)) })
          typeInstantiation := registries.typeInstantiation })
      { state with
        globals := state.globals.insert
          (globalKey namespaceId
            (instantiatedTypeId registries.typeInstantiation typeId)
            (.address address))
          (.loanHole state.nextLoan)
        globalLoans :=
          (state.nextLoan, globalKey namespaceId
            (instantiatedTypeId registries.typeInstantiation typeId)
            (.address address))
            :: state.globalLoans
        nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan resource)) := by
  simp [GlobalLocationOperation.evaluate?, borrowGlobalAt?,
    borrowGlobalUsing?, globalValue?, present, rowFrame,
    RuntimeValue.storageKey?,
    borrowRuntimePlaceAt?_global_mutable_of_lookup lex referenceType _ _ _
      resource mutableKind present]

/-- The field reborrow along a focused nominal path, through a local
holding the resource borrow: read the focus, mint the loan, leave the hole
at the focus, register the loan and its location. -/
theorem derefLocalBorrow_evaluate_nominalPath (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (localId : LocalId) (steps : List FocusStep) (outerLoan : Nat)
    (leaf : RuntimeValue) (row : Row) (registries : Registries)
    (state : RuntimeState)
    (slot : row[localId.index]? =
      some (some (.borrow outerLoan (focusValue steps leaf)))) :
    (({ location := ⟨localId⟩
        fields := focusFields steps
        referenceType, kind := .mutable, lexicalLoan := lex } :
      DerefLocalBorrowOperation)).evaluate? #[]
      (rowFrame row registries) state =
    some (.value
      (rowFrame
        (row.set! localId.index
          (some (.borrow outerLoan (focusValue steps (.loanHole state.nextLoan)))))
        { activeLoans :=
            (registries.activeLoans.filter (·.1 != ⟨lex⟩)).push
              (⟨lex⟩, state.nextLoan)
          loanLocations := registries.loanLocations.push
            (state.nextLoan,
              ⟨.local localId, #[.deref] ++ focusProjections steps, true⟩)
          typeInstantiation := registries.typeInstantiation })
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan leaf)) := by
  have inBounds : localId.index < row.size :=
    (Array.getElem?_eq_some_iff.mp slot).1
  have slotElem := (Array.getElem?_eq_some_iff.mp slot).2
  have notBeyond : ¬ (row.size ≤ localId.index) := Nat.not_le.mpr inBounds
  have resolved : resolveNominalFieldSteps?
      { locals := row, activeLoans := registries.activeLoans,
        loanLocations := registries.loanLocations,
        typeInstantiation := registries.typeInstantiation }
      state (focusFields steps) { root := .local localId, projections := #[.deref] } =
      some { root := .local localId, projections := #[.deref] ++ focusProjections steps } := by
    have resolved := resolveNominalFieldSteps?_focus (rowFrame row registries) state
      steps leaf ⟨.local localId, #[.deref], true⟩
      (by simp [readRuntimePlace?, readRoot?, readLocal?, rowFrame, slot,
        readProjections?])
    simpa [rowFrame] using resolved
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    DerefLocalBorrowOperation.resolve?, resolveDerefLocalFieldPath?,
    resolved, readLocal?, borrowRuntimePlaceAt?,
    readRuntimePlace?, readRoot?, readProjections?, readProjections?_focusValue,
    writeRuntimePlace?, writeRoot?, writeProjections?, writeProjections?_focusValue,
    rowFrame, mutableKind, inBounds, slotElem, notBeyond,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-- A literal-index reborrow through a mutable vector parameter.  The
element is replaced by the fresh loan hole and the compact indexed path is
registered without consulting the source place arena. -/
theorem indexedLocalBorrow_evaluate_derefVector
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (outerLoan : Nat) (focus : VectorFocus) (current : RuntimeValue)
    (registries : Registries) (state : RuntimeState) :
    (({ location := ⟨⟨0⟩⟩, dereference := true, index := focus.index,
        referenceType, kind := .mutable, lexicalLoan := lex } :
      IndexedLocalBorrowOperation)).evaluate? #[]
      (rowFrame #[some (.borrow outerLoan (focus.fill current)), none]
        registries) state =
    some (.value
      (rowFrame #[some (.borrow outerLoan
            (focus.fill (.loanHole state.nextLoan))), none]
        { activeLoans :=
            (registries.activeLoans.filter (·.1 != ⟨lex⟩)).push
              (⟨lex⟩, state.nextLoan)
          loanLocations := registries.loanLocations.push
            (state.nextLoan,
              ⟨.local ⟨0⟩, #[.deref, .index focus.index], true⟩)
          typeInstantiation := registries.typeInstantiation })
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan current)) := by
  have at_ := focus.getElem?_fill current
  have inBounds := (Array.getElem?_eq_some_iff.mp at_).1
  have elementEq := (Array.getElem?_eq_some_iff.mp at_).2
  have focusBound : focus.before.size <
      focus.before.size + 1 + focus.after.size := by omega
  simp [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    IndexedLocalBorrowOperation.resolve?, resolveLocalLiteralIndex?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, rowFrame,
    borrowRuntimePlaceAt?, mutableKind, inBounds, elementEq,
    writeRuntimePlace?, writeRoot?, writeProjections?, Array.set!,
    VectorFocus.fill, VectorFocus.index, VectorFocus.set!_fill,
    VectorFocus.setIfInBounds_push, focusBound,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-! ## A field below an indexed owned local

The source path `local[0].field` is lowered to two fixed numeric indices.
These closed rows keep the borrow, mutation, and retirement constant-time:
none of them searches the source place arena or a symbolic aggregate. -/

/-- Mutable entry through `local0[0].field0` for the single-element pair
shape used by an owned-vector borrow. -/
theorem indexedLocalFieldBorrow_evaluate_ownedPair
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (source : StructHandle) (left right : Int) (state : RuntimeState) :
    (({ location := ⟨⟨0⟩⟩, index := 0,
        field := ⟨source, none, 0⟩,
        referenceType, kind := .mutable, lexicalLoan := lex } :
      IndexedLocalFieldBorrowOperation)).evaluate? #[]
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.integer left, .integer right]]), none, none, none]
        { activeLoans := #[], loanLocations := #[] }) state =
    some (.value
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.loanHole state.nextLoan, .integer right]]),
          none, none, none]
        { activeLoans := #[(⟨lex⟩, state.nextLoan)]
          loanLocations := #[(state.nextLoan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan (.integer left))) := by
  simp [IndexedLocalFieldBorrowOperation.evaluate?, liftPlaceEvaluator,
    IndexedLocalFieldBorrowOperation.resolve?, resolveLocalLiteralIndexField?,
    resolveLocalLiteralIndex?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, borrowRuntimePlaceAt?, mutableKind,
    writeRuntimePlace?, writeRoot?, writeProjections?, rowFrame]
  rw [show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]
  rfl

/-- Lift the owned indexed-field entry into the throw-aware row WP. -/
theorem wpRowThrow_indexedLocalFieldBorrow0_ownedPair
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (source : StructHandle) (left right : Int) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : postValue
      #[some (.vector #[.nominal source none
          #[.loanHole state.nextLoan, .integer right]]),
        none, none, none]
      { activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations := #[(state.nextLoan,
          ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan (.integer left)))) :
    wpRowThrow
      (nativeIndexedLocalFieldBorrowOperation
        { location := ⟨⟨0⟩⟩, index := 0,
          field := ⟨source, none, 0⟩,
          referenceType, kind := .mutable, lexicalLoan := lex }
        valuesNil)
      #[some (.vector #[.nominal source none
          #[.integer left, .integer right]]), none, none, none]
      { activeLoans := #[], loanLocations := #[] }
      state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨operandFrame, operandState, propagated, nilStep, -⟩ |
      ⟨operandFrame, operandState, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with frameEq stateEq valuesEq
    subst operandFrame operandState values
    rcases evaluated with ⟨runtimeValue, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [indexedLocalFieldBorrow_evaluate_ownedPair referenceType mutableKind
        lex source left right state] at evaluation
    · cases Option.some.inj evaluation
      exact ⟨_, _, rfl, exit⟩
    · cases Option.some.inj evaluation

/-- Write the new scalar into the borrow resting in local 1. -/
theorem mutate_evaluate_ownedPairField (source : StructHandle)
    (state : RuntimeState) (lex loan : Nat) (left right written : Int) :
    ReferenceLocationOperation.mutate.evaluate?
      #[.borrow loan (.integer left), .integer written]
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.loanHole loan, .integer right]]),
          some (.borrow loan (.integer left)), none, none]
        { activeLoans := #[(⟨lex⟩, loan)]
          loanLocations := #[(loan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      state =
    some (.value
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.loanHole loan, .integer right]]),
          some (.borrow loan (.integer written)), none, none]
        { activeLoans := #[(⟨lex⟩, loan)]
          loanLocations := #[(loan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      state .unit) := by
  have untouched : rewriteFirst (borrowRewrite? loan (.integer written))
      (.vector #[.nominal source none #[.loanHole loan, .integer right]]) = none := by
    simp [rewriteFirst.eq_def, rewriteFirstList.eq_def]
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?, readProjections?,
    writeRuntimePlace?, writeRoot?, writeProjections?, rewriteFirst, untouched]

/-- Retire the indexed-field loan, restore the field, and clear local 1. -/
theorem endLoan_evaluate_ownedPairField (source : StructHandle)
    (state : RuntimeState) (loan : Nat) (right written : Int) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[]
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.loanHole loan, .integer right]]),
          some (.borrow loan (.integer written)), none, none]
        { activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(loan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      state =
    some (.value
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.integer written, .integer right]]),
          some .unit, none, none]
        { activeLoans := #[]
          loanLocations := #[(loan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      state .unit) := by
  have siteSelf : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have siteNotDifferent : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame,
    endLoans?, findBorrowValue?, findFirst, findFirstList, List.findSome?,
    clearBorrowValue, rewriteFirst, rewriteFirstList, applyWriteBack,
    fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    globalLoanKey?, globalLoanKeyIn?, transferGlobalLoan, transferredLoan?,
    siteSelf, siteNotDifferent]

/-- Retire the indexed-field loan on the throw-aware statement spine.
Keeping the concrete row parameters in this rule prevents rewriting the
generic evaluator equation from leaving unresolved row metavariables in
the following expression. -/
theorem wpRowThrow_endLoan_ownedPairField (source : StructHandle)
    (state : RuntimeState) (loan : Nat) (right written : Int)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : postValue
      #[some (.vector #[.nominal source none
          #[.integer written, .integer right]]),
        some .unit, none, none]
      { activeLoans := #[]
        loanLocations := #[(loan,
          ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] }
      state (.value .unit)) :
    wpRowThrow
      (nativeReferenceOperation (.endLoan #[⟨0⟩]) valuesNil)
      #[some (.vector #[.nominal source none
          #[.loanHole loan, .integer right]]),
        some (.borrow loan (.integer written)), none, none]
      { activeLoans := #[(⟨0⟩, loan)]
        loanLocations := #[(loan,
          ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] }
      state postValue postThrow := by
  apply wpRowThrow_endLoanStatement
  intro finalFrame finalState retired evaluated
  rw [endLoan_evaluate_ownedPairField source state loan right written]
    at evaluated
  injection evaluated with inner
  injection inner with frameEq stateEq retiredEq
  subst finalFrame finalState retired
  exact ⟨_, _, rfl, exit⟩

/-- A shared read of the restored element leaves the row unchanged. -/
theorem indexedLocalBorrow_evaluate_ownedPair
    (referenceType : ReferenceType)
    (sharedKind : referenceType.kind = .shared) (lex loan : Nat)
    (source : StructHandle) (left right : Int) (state : RuntimeState) :
    (({ location := ⟨⟨0⟩⟩, dereference := false, index := 0,
        referenceType, kind := .immutable, lexicalLoan := lex } :
      IndexedLocalBorrowOperation)).evaluate? #[]
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.integer left, .integer right]]),
          some .unit, none, none]
        { activeLoans := #[]
          loanLocations := #[(loan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      state =
    some (.value
      (rowFrame
        #[some (.vector #[.nominal source none
            #[.integer left, .integer right]]),
          some .unit, none, none]
        { activeLoans := #[]
          loanLocations := #[(loan,
            ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] })
      state (.nominal source none #[.integer left, .integer right])) := by
  simp [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    IndexedLocalBorrowOperation.resolve?, resolveLocalLiteralIndex?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?,
    borrowRuntimePlaceAt?, sharedKind, rowFrame]
  rw [show (ReferenceKind.shared != ReferenceKind.shared) = false from rfl]
  rfl

/-- Lift the shared owned-element read into the throw-aware row WP. -/
theorem wpRowThrow_indexedLocalBorrow0_ownedPair
    (referenceType : ReferenceType)
    (sharedKind : referenceType.kind = .shared) (lex loan : Nat)
    (source : StructHandle) (left right : Int) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : postValue
      #[some (.vector #[.nominal source none
          #[.integer left, .integer right]]),
        some .unit, none, none]
      { activeLoans := #[]
        loanLocations := #[(loan,
          ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] }
      state (.value (.nominal source none #[.integer left, .integer right]))) :
    wpRowThrow
      (nativeIndexedLocalBorrowOperation
        { location := ⟨⟨0⟩⟩, dereference := false, index := 0,
          referenceType, kind := .immutable, lexicalLoan := lex }
        valuesNil)
      #[some (.vector #[.nominal source none
          #[.integer left, .integer right]]),
        some .unit, none, none]
      { activeLoans := #[]
        loanLocations := #[(loan,
          ⟨.local ⟨0⟩, #[.index 0, .field 0], true⟩)] }
      state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨operandFrame, operandState, propagated, nilStep, -⟩ |
      ⟨operandFrame, operandState, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with frameEq stateEq valuesEq
    subst operandFrame operandState values
    rcases evaluated with ⟨runtimeValue, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [indexedLocalBorrow_evaluate_ownedPair referenceType sharedKind lex
        loan source left right state] at evaluation
    · cases Option.some.inj evaluation
      exact ⟨_, _, rfl, exit⟩
    · cases Option.some.inj evaluation

/-- Any immutable compact indexed-local borrow preserves the row; failure
also leaves no result to account for. -/
theorem evaluatorRowStable_indexedLocalBorrow_immutable
    (operation : IndexedLocalBorrowOperation)
    (immutable : operation.kind = .immutable)
    (sharedKind : operation.referenceType.kind = .shared) :
    EvaluatorRowStable operation.evaluate? := by
  intro operands row registries state result evaluated
  simp only [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator]
    at evaluated
  rw [immutable] at evaluated
  split at evaluated
  · cases evaluated
  · cases resolved : operation.resolve? (rowFrame row registries) state with
    | none => simp [resolved] at evaluated
    | some place =>
      simp only [resolved] at evaluated
      simp [borrowRuntimePlaceAt?, sharedKind] at evaluated
      rw [show (ReferenceKind.shared != ReferenceKind.shared) = false from rfl,
        show (ReferenceKind.shared == ReferenceKind.mutable) = false from rfl]
        at evaluated
      cases read : readRuntimePlace? (rowFrame row registries) state place with
      | none => simp [read] at evaluated
      | some value =>
        simp [read] at evaluated
        cases evaluated
        exact ⟨row, rfl⟩

/-- Lift the closed indexed-borrow evaluation into the throw-aware row
weakest precondition.  This is the entry law used by vector-element borrow
scopes; its operand row is empty, so no generic evaluator reduction is
needed at the call site. -/
theorem wpRowThrow_indexedLocalBorrow0
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (outerLoan : Nat) (focus : VectorFocus) (current : RuntimeValue)
    (registries : Registries) (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : postValue
      #[some (.borrow outerLoan
          (focus.fill (.loanHole state.nextLoan))), none]
      { activeLoans :=
          (registries.activeLoans.filter (·.1 != ⟨lex⟩)).push
            (⟨lex⟩, state.nextLoan)
        loanLocations := registries.loanLocations.push
          (state.nextLoan,
            ⟨.local ⟨0⟩, #[.deref, .index focus.index], true⟩)
        typeInstantiation := registries.typeInstantiation }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan current))) :
    wpRowThrow
      (nativeIndexedLocalBorrowOperation
        { location := ⟨⟨0⟩⟩, dereference := true, index := focus.index,
          referenceType, kind := .mutable, lexicalLoan := lex }
        valuesNil)
      #[some (.borrow outerLoan (focus.fill current)), none]
      registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨operandFrame, operandState, propagated, nilStep, -⟩ |
      ⟨operandFrame, operandState, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with frameEq stateEq valuesEq
    subst operandFrame operandState values
    rcases evaluated with ⟨runtimeValue, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [indexedLocalBorrow_evaluate_derefVector referenceType mutableKind lex
        outerLoan focus current registries state] at evaluation
    · cases Option.some.inj evaluation
      exact ⟨_, _, rfl, exit⟩
    · cases Option.some.inj evaluation

/-- The indexed-borrow entry specialized to a single mutable parameter.
Its statement keeps the two registry rows literal, which lets the scalar
inner script match its mutation law without reducing array programs. -/
theorem wpRowThrow_indexedLocalBorrow0_parameter
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (outerLoan : Nat) (focus : VectorFocus) (current : RuntimeValue)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : postValue
      #[some (.borrow outerLoan
          (focus.fill (.loanHole state.nextLoan))), none]
      { activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations := #[(outerLoan, ⟨.local ⟨0⟩, #[], true⟩),
          (state.nextLoan,
            ⟨.local ⟨0⟩, #[.deref, .index focus.index], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan current))) :
    wpRowThrow
      (nativeIndexedLocalBorrowOperation
        { location := ⟨⟨0⟩⟩, dereference := true, index := focus.index,
          referenceType, kind := .mutable, lexicalLoan := lex }
        valuesNil)
      #[some (.borrow outerLoan (focus.fill current)), none]
      { activeLoans := #[]
        loanLocations := #[(outerLoan, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postValue postThrow := by
  apply wpRowThrow_indexedLocalBorrow0 referenceType mutableKind lex outerLoan
    focus current _ state
  simpa using exit

/-! ## An indexed loan of an owned vector beside a live parameter -/

/-- Borrow `local1[0]` while local 0 contains an unrelated mutable
parameter.  The row is the exact three-local shape emitted for the loans
regression, so locating the owned vector remains a constant computation. -/
theorem indexedLocalBorrow_evaluate_ownedVectorBesideParameter
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (outer : Nat) (outerCurrent current : RuntimeValue)
    (state : RuntimeState) :
    (({ location := ⟨⟨1⟩⟩, dereference := false, index := 0,
        referenceType, kind := .mutable, lexicalLoan := lex } :
      IndexedLocalBorrowOperation)).evaluate? #[]
      (rowFrame #[some (.borrow outer outerCurrent),
          some (.vector #[current]), none]
        { activeLoans := #[]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }) state =
    some (.value
      (rowFrame #[some (.borrow outer outerCurrent),
          some (.vector #[.loanHole state.nextLoan]), none]
        { activeLoans := #[(⟨lex⟩, state.nextLoan)]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (state.nextLoan, ⟨.local ⟨1⟩, #[.index 0], true⟩)] })
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan current)) := by
  simp [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    IndexedLocalBorrowOperation.resolve?, resolveLocalLiteralIndex?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?,
    borrowRuntimePlaceAt?, mutableKind, writeRuntimePlace?, writeRoot?,
    writeProjections?, rowFrame]
  rw [show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]
  rfl

/-- Throw-aware entry rule for the owned-vector element loan. -/
theorem wpRowThrow_indexedLocalBorrow1_ownedVectorBesideParameter
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (outer : Nat) (outerCurrent current : RuntimeValue)
    (state : RuntimeState)
    (postValue : Row → Registries → RuntimeState → Control → Prop)
    (postThrow : ThrowKind → Array RuntimeValue → Prop)
    (exit : postValue
      #[some (.borrow outer outerCurrent),
        some (.vector #[.loanHole state.nextLoan]), none]
      { activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨1⟩, #[.index 0], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan current))) :
    wpRowThrow
      (nativeIndexedLocalBorrowOperation
        { location := ⟨⟨1⟩⟩, dereference := false, index := 0,
          referenceType, kind := .mutable, lexicalLoan := lex }
        valuesNil)
      #[some (.borrow outer outerCurrent), some (.vector #[current]), none]
      { activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨operandFrame, operandState, propagated, nilStep, -⟩ |
      ⟨operandFrame, operandState, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with frameEq stateEq valuesEq
    subst operandFrame operandState values
    rcases evaluated with ⟨runtimeValue, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [indexedLocalBorrow_evaluate_ownedVectorBesideParameter
        referenceType mutableKind lex outer outerCurrent current state]
        at evaluation
    · cases Option.some.inj evaluation
      exact ⟨_, _, rfl, exit⟩
    · cases Option.some.inj evaluation

/-- Write through the element borrow in local 2 without inspecting the
unrelated parameter's typed current value. -/
theorem mutate_evaluate_ownedVectorBesideParameter
    (state : RuntimeState) (outer loan : Nat)
    (outerCurrent current written : RuntimeValue)
    (outerPlain : Plain outerCurrent) (separate : outer ≠ loan) :
    ReferenceLocationOperation.mutate.evaluate?
      #[.borrow loan current, written]
      (rowFrame #[some (.borrow outer outerCurrent),
          some (.vector #[.loanHole loan]), some (.borrow loan current)]
        { activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨1⟩, #[.index 0], true⟩)] }) state =
    some (.value
      (rowFrame #[some (.borrow outer outerCurrent),
          some (.vector #[.loanHole loan]), some (.borrow loan written)]
        { activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨1⟩, #[.index 0], true⟩)] })
      state .unit) := by
  have outerUntouched : rewriteFirst (borrowRewrite? loan written)
      (.borrow outer outerCurrent) = none := by
    rw [rewriteFirst.eq_def]
    simp [borrowRewrite?, separate,
      rewriteFirst_eq_none_of_plain (LoanMatcher.borrowRewrite? loan written)
        outerPlain]
  have holeUntouched : rewriteFirstList (borrowRewrite? loan written)
      [.loanHole loan] = none := by
    simp [rewriteFirstList.eq_def, rewriteFirst.eq_def, borrowRewrite?]
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, writeRuntimePlace?, writeRoot?, writeProjections?,
    rewriteFirst, rewriteFirstList, outerUntouched, holeUntouched, separate]

/-- Retire the owned element loan, restoring the vector in local 1 and
clearing its temporary borrow in local 2. -/
theorem endLoan_evaluate_ownedVectorBesideParameter
    (state : RuntimeState) (outer loan : Nat)
    (outerCurrent written : RuntimeValue)
    (outerPlain : Plain outerCurrent) (writtenPlain : Plain written)
    (separate : outer ≠ loan) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[]
      (rowFrame #[some (.borrow outer outerCurrent),
          some (.vector #[.loanHole loan]), some (.borrow loan written)]
        { activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨1⟩, #[.index 0], true⟩)] }) state =
    some (.value
      (rowFrame #[some (.borrow outer outerCurrent),
          some (.vector #[written]), some .unit]
        { activeLoans := #[]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨1⟩, #[.index 0], true⟩)] })
      state .unit) := by
  have siteSelf : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have siteNotDifferent : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have firstUntouched : rewriteFirst (borrowClear? loan)
      (.borrow outer outerCurrent) = none := by
    rw [rewriteFirst.eq_def]
    simp [borrowClear?, separate,
      rewriteFirst_eq_none_of_plain (LoanMatcher.borrowClear? loan) outerPlain]
  have noOuterBorrow := findFirst_eq_none_of_plain
    (LoanMatcher.borrowCurrent? loan) outerPlain
  have noOuterHole := findFirst_eq_none_of_plain
    (LoanMatcher.holeMark? loan) outerPlain
  have noWrittenHole := findFirst_eq_none_of_plain
    LoanMatcher.anyHole? writtenPlain
  have outerFillUntouched := rewriteFirst_eq_none_of_plain
    (LoanMatcher.holeFill? loan written) outerPlain
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame,
    endLoans?, findBorrowValue?, findFirst, findFirstList, List.findSome?,
    clearBorrowValue, rewriteFirst, rewriteFirstList, applyWriteBack,
    fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    globalLoanKey?, globalLoanKeyIn?, transferGlobalLoan, transferredLoan?,
    firstUntouched, noOuterBorrow, noOuterHole, noWrittenHole,
    outerFillUntouched, separate, siteSelf, siteNotDifferent]

/-- Once the independent element loan has retired, writing the unrelated
mutable parameter uses its cached local-zero address.  The stale element
location is harmless and deliberately retained, matching the runtime. -/
theorem mutate_evaluate_parameterBesideRetiredOwnedVector
    (state : RuntimeState) (outer retired : Nat)
    (current replacement element : RuntimeValue) (separate : outer ≠ retired) :
    ReferenceLocationOperation.mutate.evaluate?
      #[.borrow outer current, replacement]
      (rowFrame #[some (.borrow outer current),
          some (.vector #[element]), some .unit]
        { activeLoans := #[]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (retired, ⟨.local ⟨1⟩, #[.index 0], true⟩)] }) state =
    some (.value
      (rowFrame #[some (.borrow outer replacement),
          some (.vector #[element]), some .unit]
        { activeLoans := #[]
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (retired, ⟨.local ⟨1⟩, #[.index 0], true⟩)] })
      state .unit) := by
  have retiredSeparate : retired ≠ outer := Ne.symm separate
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, writeRuntimePlace?, writeRoot?, writeProjections?,
    rewriteFirst, separate, retiredSeparate]

/-- Retire a literal-index reborrow after its body: the element's current
value fills the hole in the outer vector borrow and the temporary local is
cleared. -/
theorem endLoans?_indexedReborrow_zero
    (state : RuntimeState) (outerLoan loan : Nat)
    (focus : VectorFocus) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (plain : focus.Plain) (separate : outerLoan ≠ loan) :
    endLoans? #[(⟨0⟩ : LoanId)] #[argument]
        { locals := #[some (.borrow outerLoan
              (focus.fill (.loanHole loan))),
            some (.borrow loan current)]
          activeLoans := #[(⟨0⟩, loan)]
          loanLocations } state =
      some
        ({ locals := #[some (.borrow outerLoan
              (focus.fill current)), some .unit]
           activeLoans := #[]
           loanLocations },
         state, argument) := by
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by decide
  obtain ⟨holeMark, anyHole, borrowCurrent, holeFill, borrowRewrite,
      borrowClear⟩ := focus.walks plain
  simp [endLoans?, findBorrowValue?, findFirst, clearBorrowValue,
    rewriteFirst, applyWriteBack, fillVisibleHole,
    holeInFrame, holeWithin, fillHole?, Array.filter,
    holeMark, anyHole, borrowCurrent, holeFill, borrowRewrite, borrowClear,
    separate, siteSelf, siteNotDifferent]

theorem endLoan_evaluate_indexedReborrow
    (state : RuntimeState) (outerLoan loan : Nat)
    (focus : VectorFocus) (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace))
    (plain : focus.Plain) (separate : outerLoan ≠ loan) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[argument]
      (rowFrame #[some (.borrow outerLoan
            (focus.fill (.loanHole loan))),
          some (.borrow loan current)]
        { activeLoans := #[(⟨0⟩, loan)], loanLocations }) state =
      some (.value
        (rowFrame #[some (.borrow outerLoan
              (focus.fill current)), some .unit]
          { activeLoans := #[], loanLocations })
        state argument) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  rw [endLoans?_indexedReborrow_zero state outerLoan loan focus current
    argument loanLocations plain separate]
  rfl

/-! ## Two disjoint field reborrows of one parameter -/

/-- Retire the right-hand field loan while the left-hand sibling loan is
still live.  The row is the exact five-local shape generated by
`read_siblings`; keeping it literal makes the two-hole walk a closed
computation rather than a symbolic search. -/
theorem endLoan_evaluate_siblingRight (source : StructHandle)
    (state : RuntimeState) (outer first second : Nat) (left right : Int)
    (outerFirst : outer ≠ first) (outerSecond : outer ≠ second)
    (distinct : first ≠ second) :
    (ReferenceLocationOperation.endLoan #[⟨1⟩]).evaluate? #[]
      (rowFrame
        #[some (.borrow outer (.nominal source none
              #[.loanHole first, .loanHole second])),
          some (.borrow first (.integer left)),
          some (.borrow second (.integer right)),
          some (.integer right), none]
        { activeLoans := #[(⟨0⟩, first), (⟨1⟩, second)]
          loanLocations :=
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (first, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩),
              (second, ⟨.local ⟨0⟩, #[.deref, .field 1], true⟩)] })
      state =
    some (.value
      (rowFrame
        #[some (.borrow outer (.nominal source none
              #[.loanHole first, .integer right])),
          some (.borrow first (.integer left)), some .unit,
          some (.integer right), none]
        { activeLoans := #[(⟨0⟩, first)]
          loanLocations :=
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (first, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩),
              (second, ⟨.local ⟨0⟩, #[.deref, .field 1], true⟩)] })
      state .unit) := by
  have zeroOne : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have oneSelf : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zeroNeOne : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  have oneNe : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have secondFirst : second ≠ first := Ne.symm distinct
  have firstOuter : first ≠ outer := Ne.symm outerFirst
  have secondOuter : second ≠ outer := Ne.symm outerSecond
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, findBorrowValue?, findFirst, findFirstList, List.findSome?,
    clearBorrowValue, rewriteFirst, rewriteFirstList,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, Array.filter, globalLoanKey?, globalLoanKeyIn?,
    transferGlobalLoan, transferredLoan?, zeroOne, oneSelf, zeroNeOne, oneNe,
    outerFirst, outerSecond, distinct, secondFirst, firstOuter, secondOuter]

/-- Retire the remaining left-hand field loan after the right sibling has
already been restored. -/
theorem endLoan_evaluate_siblingLeft (source : StructHandle)
    (state : RuntimeState) (outer first second : Nat) (left right : Int)
    (outerFirst : outer ≠ first) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[]
      (rowFrame
        #[some (.borrow outer (.nominal source none
              #[.loanHole first, .integer right])),
          some (.borrow first (.integer left)), some .unit,
          some (.integer right), some (.integer left)]
        { activeLoans := #[(⟨0⟩, first)]
          loanLocations :=
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (first, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩),
              (second, ⟨.local ⟨0⟩, #[.deref, .field 1], true⟩)] })
      state =
    some (.value
      (rowFrame
        #[some (.borrow outer (.nominal source none
              #[.integer left, .integer right])),
          some .unit, some .unit, some (.integer right), some (.integer left)]
        { activeLoans := #[]
          loanLocations :=
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (first, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩),
              (second, ⟨.local ⟨0⟩, #[.deref, .field 1], true⟩)] })
      state .unit) := by
  have zeroSelf : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have zeroNe : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have firstOuter : first ≠ outer := Ne.symm outerFirst
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, findBorrowValue?, findFirst, findFirstList, List.findSome?,
    clearBorrowValue, rewriteFirst, rewriteFirstList,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?,
    Array.filter, globalLoanKey?, globalLoanKeyIn?, transferGlobalLoan,
    transferredLoan?, zeroSelf, zeroNe, outerFirst, firstOuter]

/-- Export the restored pair parameter after both temporary field loans
have been cleared.  Read-only scalar locals cannot contribute write-backs. -/
theorem exportFrameLoans_rowFrame_siblingReads (source : StructHandle)
    (state : RuntimeState) (outer first second : Nat) (left right : Int)
    (noGlobal : globalLoanKeyIn? state.globalLoans outer = none) :
    exportFrameLoans
      (rowFrame
        #[some (.borrow outer (.nominal source none
              #[.integer left, .integer right])),
          some .unit, some .unit, some (.integer right), some (.integer left)]
        { activeLoans := #[]
          loanLocations :=
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (first, ⟨.local ⟨0⟩, #[.deref, .field 0], true⟩),
              (second, ⟨.local ⟨0⟩, #[.deref, .field 1], true⟩)] })
      state =
    { state with pending := state.pending.push (outer,
        RuntimeValue.nominal source none #[.integer left, .integer right]) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, findFirstList, List.findSome?, applyWriteBack_empty,
    globalLoanKey?, noGlobal]

/-- Write through a focused vector-element borrow resting beside its
outer mutable vector parameter. -/
theorem mutate_evaluate_indexedLocal1 (state : RuntimeState)
    (outer loan : Nat) (focus : VectorFocus) (plain : focus.Plain)
    (current written : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) (separate : outer ≠ loan) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow loan current, written]
      (rowFrame #[some (.borrow outer (focus.fill (.loanHole loan))),
          some (.borrow loan current)]
        { activeLoans
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨0⟩, #[.deref, .index focus.index], true⟩)] })
      state =
    some (.value
      (rowFrame #[some (.borrow outer (focus.fill (.loanHole loan))),
          some (.borrow loan written)]
        { activeLoans
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (loan, ⟨.local ⟨0⟩, #[.deref, .index focus.index], true⟩)] })
      state .unit) := by
  obtain ⟨-, -, -, -, borrowRewrite, -⟩ := focus.walks plain
  have focusedHole : readProjections? (focus.fill (.loanHole loan))
      [.index focus.index] = some (.loanHole loan) := by
    rw [VectorFocus.fill, readProjections?]
    rw [focus.getElem?_fill]
    rfl
  have focusedBorrow : readProjections?
      (.borrow outer (focus.fill (.loanHole loan)))
      [.deref, .index focus.index] = some (.loanHole loan) := by
    rw [readProjections?]
    exact focusedHole
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    borrowRewrite, rewriteFirst, writeRuntimePlace?, writeRoot?,
    separate, focusedBorrow]

/-- Finalize one mutable parameter whose current value is any loan-free
runtime value.  Vector-element scopes use this after the temporary element
loan has been retired and the vector has been reconstructed. -/
theorem exportFrameLoans_rowFrame_singlePlainBorrow (state : RuntimeState)
    (loan : Nat) (value : RuntimeValue) (plain : Plain value)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan value), some .unit]
          { activeLoans, loanLocations })
        state =
      { state with pending := state.pending.push (loan, value) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have noNested := plain.outermostBorrows_eq_empty
  have noHole := findFirst_eq_none_of_plain
    (LoanMatcher.holeMark? loan) plain
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, noNested,
    holeInFrame, holeWithin, findFirst, applyWriteBack_empty_export,
    globalLoanKey?, noGlobal, noHole]

/-- A write through the focused field borrow of the bracket: the borrow's
current is replaced where it rests, in local 2.  The registries are the
bracket's own — the field loan's registered location is the lender's path,
which holds the hole, so the write lands on the resting borrow. -/
theorem mutate_evaluate_focusedField (state : RuntimeState) (loan outer : Nat)
    (key : GlobalKey) (address : String) (amount : Int)
    (steps : List FocusStep) (current written : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow loan current, written]
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan current),
          some (.borrow outer
            (focusValue steps (.loanHole loan)))]
        { activeLoans
          loanLocations := #[(outer, { root := .global key }),
            (loan, ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      state =
    some (.value
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan written),
          some (.borrow outer
            (focusValue steps (.loanHole loan)))]
        { activeLoans
          loanLocations := #[(outer, { root := .global key }),
            (loan, ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?, readProjections?,
    readProjections?_focusValue, writeRuntimePlace?, writeRoot?,
    rewriteFirst_integer, rewriteFirst_address, rewriteFirst_borrow,
    rewriteFirst_loanHole]

/-- The saved-read variant of `mutate_evaluate_focusedField`: a plain local
sits between the field borrow and its holder. -/
theorem mutate_evaluate_focusedFieldSaved (state : RuntimeState)
    (loan outer : Nat) (key : GlobalKey) (address : String)
    (amount saved : Int) (steps : List FocusStep)
    (current written : RuntimeValue) (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow loan current, written]
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan current), some (.integer saved),
          some (.borrow outer
            (focusValue steps (.loanHole loan)))]
        { activeLoans
          loanLocations := #[(outer, { root := .global key }),
            (loan, ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      state =
    some (.value
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan written), some (.integer saved),
          some (.borrow outer
            (focusValue steps (.loanHole loan)))]
        { activeLoans
          loanLocations := #[(outer, { root := .global key }),
            (loan, ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?, readProjections?,
    readProjections?_focusValue, writeRuntimePlace?, writeRoot?,
    rewriteFirst_integer, rewriteFirst_address, rewriteFirst_borrow,
    rewriteFirst_loanHole]

/-! ## The bracket: deposit's shape

`endLoan [0,1] (let value := (let coin := &mut Coin[addr]; &mut coin.f.g);
inner)`, over the four-local row of two parameters and two lets.  The
entry is the two computations above, chained through the bindings; the
inner block is driven by the ordinary row rules at the bracket's
registries and state; the exit is left as the generic death-marker step
on whatever row the inner block reached, for the script to close with the
closed exit row once the drive has made that row literal. -/

theorem wpRowThrow_focusedFieldBracket (namespaceId : NamespaceId)
    (typeId : TypeId) (borrowType fieldType : ReferenceType)
    (borrowMutable : borrowType.kind = .mutable)
    (fieldMutable : fieldType.kind = .mutable)
    (steps : List FocusStep) (fuelOuter fuelInner : Nat)
    (inner : ExprDenotation) {address : String} {amount fieldValue : Int}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (present : state.globals.lookup
        (globalKey namespaceId typeId (.address address)) =
      some (focusValue steps (.integer fieldValue)))
    (innerStable : RowStable inner)
    (innerWp : wpRow inner
      #[some (.address address), some (.integer amount),
        some (.borrow (state.nextLoan + 1) (.integer fieldValue)),
        some (.borrow state.nextLoan
          (focusValue steps (.loanHole (state.nextLoan + 1))))]
      { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
        loanLocations := #[(state.nextLoan,
            { root := .global (globalKey namespaceId typeId (.address address)) }),
          (state.nextLoan + 1, ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] }
      { state with
        globals := state.globals.insert
          (globalKey namespaceId typeId (.address address))
          (.loanHole state.nextLoan)
        globalLoans :=
          (state.nextLoan, globalKey namespaceId typeId (.address address))
            :: state.globalLoans
        nextLoan := state.nextLoan + 1 + 1 }
      fun innerRow innerState control =>
        match control with
        | .throw_ kind thrown => postThrow kind thrown
        | .value produced =>
            ∀ finalFrame finalState retired,
              (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate?
                  #[produced]
                  (rowFrame innerRow
                    { activeLoans := #[(⟨0⟩, state.nextLoan),
                        (⟨1⟩, state.nextLoan + 1)]
                      loanLocations := #[(state.nextLoan,
                          { root := .global
                              (globalKey namespaceId typeId (.address address)) }),
                        (state.nextLoan + 1,
                          ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] })
                  innerState =
                some (.value finalFrame finalState retired) →
              ∃ finalRow finalRegistries,
                finalFrame = rowFrame finalRow finalRegistries ∧
                postValue finalRow finalRegistries finalState (.value retired)
        | _ =>
            postValue innerRow
              { activeLoans := #[(⟨0⟩, state.nextLoan),
                  (⟨1⟩, state.nextLoan + 1)]
                loanLocations := #[(state.nextLoan,
                    { root := .global
                        (globalKey namespaceId typeId (.address address)) }),
                  (state.nextLoan + 1,
                    ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] }
              innerState control) :
    wpRowThrow
      (nativeReferenceOperation (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩])
        (valuesCons
          (letNativeValue ⟨fuelOuter + 1, .variable ⟨2⟩⟩
            (letNativeValue ⟨fuelInner + 1, .variable ⟨3⟩⟩
              (nativeGlobalOperation
                (GlobalLocationOperation.borrow
                  { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                    kind := .mutable, lexicalLoan := 0 })
                (valuesCons (localVar ⟨0⟩) valuesNil))
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨3⟩⟩, fields := focusFields steps,
                  referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
                valuesNil))
            inner)
          valuesNil))
      #[some (.address address), some (.integer amount), none, none]
      { activeLoans := #[], loanLocations := #[] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  /- The resource borrow: its operand row reads local 0, then the closed
  computation. -/
  have borrowRun :
      ∀ {bF : RuntimeFrame} {bS : RuntimeState} {c : Control},
        (nativeGlobalOperation
          (GlobalLocationOperation.borrow
            { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
              kind := .mutable, lexicalLoan := 0 })
          (valuesCons (localVar ⟨0⟩) valuesNil))
          (rowFrame #[some (.address address), some (.integer amount), none, none]
            { activeLoans := #[], loanLocations := #[] })
          state bF bS c →
        bF = rowFrame #[some (.address address), some (.integer amount), none, none]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] } ∧
        bS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan
          (focusValue steps (.integer fieldValue))) := by
    rintro bF bS c
      (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
        ⟨oF, oS, values, operandStep, evaluated⟩)
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · simp at resultEq
      · simp [valuesNil] at nilStep
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
        subst rF rS
        simp only [readLocal?_rowFrame, List.getElem?_toArray,
          List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
          at readEq
        subst readEq
        injection valueEq with valueEq
        subst rv
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (#[some (.address address), some (.integer amount), none,
              none] : Row) =
              (some (.address address) ::
                [some (.integer amount), none, none]).toArray from rfl]
            at evaluation <;>
          rw [globalBorrow_evaluate_rowFrame namespaceId typeId borrowType
            borrowMutable 0 address [some (.integer amount), none, none]
            { activeLoans := #[], loanLocations := #[], typeInstantiation := #[] }
            state _ (by simpa only [instantiatedTypeId_empty] using present)]
            at evaluation <;>
          simp only [instantiatedTypeId_empty] at evaluation <;>
          cases Option.some.inj evaluation
        refine ⟨?_, rfl, rfl⟩
        simp [rowFrame, Array.filter]
      · simp [valuesNil] at nilStep
  /- The field reborrow through local 3, at the frame the resource borrow
  left after its binding. -/
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨3⟩⟩, fields := focusFields steps,
            referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
          valuesNil)
          (rowFrame #[some (.address address), some (.integer amount), none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] })
          { state with
            globals := state.globals.insert
              (globalKey namespaceId typeId (.address address))
              (.loanHole state.nextLoan)
            globalLoans :=
              (state.nextLoan, globalKey namespaceId typeId (.address address))
                :: state.globalLoans
            nextLoan := state.nextLoan + 1 }
          rF rS c →
        rF = rowFrame #[some (.address address), some (.integer amount), none,
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
              loanLocations := #[(state.nextLoan,
                  { root := .global (globalKey namespaceId typeId (.address address)) }),
                (state.nextLoan + 1,
                  ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] } ∧
        rS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 + 1 } ∧
        c = .value (.borrow (state.nextLoan + 1) (.integer fieldValue)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_nominalPath fieldType fieldMutable 1 ⟨3⟩
          steps state.nextLoan (.integer fieldValue) _ _ _ rfl]
          at evaluation <;>
        cases Option.some.inj evaluation
      refine ⟨?_, rfl, rfl⟩
      simp [rowFrame, Array.filter, Array.set!,
        show ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true from rfl]
  /- The bracket's operand row is the outer let; run it to the inner block
  and read the inner block's step off `innerWp`. -/
  have letRun :
      ∀ {lF : RuntimeFrame} {lS : RuntimeState} {c : Control},
        (letNativeValue ⟨fuelOuter + 1, .variable ⟨2⟩⟩
          (letNativeValue ⟨fuelInner + 1, .variable ⟨3⟩⟩
            (nativeGlobalOperation
              (GlobalLocationOperation.borrow
                { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                  kind := .mutable, lexicalLoan := 0 })
              (valuesCons (localVar ⟨0⟩) valuesNil))
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨3⟩⟩, fields := focusFields steps,
                referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
              valuesNil))
          inner)
          (rowFrame #[some (.address address), some (.integer amount), none, none]
            { activeLoans := #[], loanLocations := #[] })
          state lF lS c →
        ∃ innerRow,
          lF = rowFrame innerRow
            { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
              loanLocations := #[(state.nextLoan,
                  { root := .global (globalKey namespaceId typeId (.address address)) }),
                (state.nextLoan + 1,
                  ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] } ∧
          inner
            (rowFrame #[some (.address address), some (.integer amount),
                some (.borrow (state.nextLoan + 1) (.integer fieldValue)),
                some (.borrow state.nextLoan
                  (focusValue steps (.loanHole (state.nextLoan + 1))))]
              { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
                loanLocations := #[(state.nextLoan,
                    { root := .global (globalKey namespaceId typeId (.address address)) }),
                  (state.nextLoan + 1,
                    ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] })
            { state with
              globals := state.globals.insert
                (globalKey namespaceId typeId (.address address))
                (.loanHole state.nextLoan)
              globalLoans :=
                (state.nextLoan, globalKey namespaceId typeId (.address address))
                  :: state.globalLoans
              nextLoan := state.nextLoan + 1 + 1 }
            lF lS c := by
    rintro lF lS c
      (⟨initStep, abrupt⟩ |
        ⟨iF, iS, bound, boundFrame, initStep, bindEq, bodyStep⟩)
    · /- The initializer is the inner let; its own initializer and body are
      value-only computations, so it cannot raise abrupt control. -/
      rcases initStep with ⟨borrowStep, abrupt'⟩ |
        ⟨bF, bS, borrowed, boundFrame', borrowStep, bindEq', reborrowStep⟩
      · obtain ⟨-, -, rfl⟩ := borrowRun borrowStep
        cases abrupt'
      · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
        injection valueEq with valueEq
        subst borrowed
        rw [bindVariable_rowFrame fuelInner ⟨3⟩ _ _ _ (by simp)] at bindEq'
        cases Option.some.inj bindEq'
        rw [show (#[some (.address address), some (.integer amount), none,
            none] : Row).set! 3 (some (.borrow state.nextLoan
              (focusValue steps (.integer fieldValue)))) =
            #[some (.address address), some (.integer amount), none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            from rfl] at reborrowStep
        obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
    · rcases initStep with ⟨borrowStep, abrupt'⟩ |
        ⟨bF, bS, borrowed, boundFrame', borrowStep, bindEq', reborrowStep⟩
      · cases abrupt'
      · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
        injection valueEq with valueEq
        subst borrowed
        rw [bindVariable_rowFrame fuelInner ⟨3⟩ _ _ _ (by simp)] at bindEq'
        cases Option.some.inj bindEq'
        rw [show (#[some (.address address), some (.integer amount), none,
            none] : Row).set! 3 (some (.borrow state.nextLoan
              (focusValue steps (.integer fieldValue)))) =
            #[some (.address address), some (.integer amount), none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            from rfl] at reborrowStep
        obtain ⟨rfl, rfl, valueEq'⟩ := reborrowRun reborrowStep
        injection valueEq' with valueEq'
        subst bound
        rw [bindVariable_rowFrame fuelOuter ⟨2⟩ _ _ _ (by simp)] at bindEq
        cases Option.some.inj bindEq
        rw [show (#[some (.address address), some (.integer amount), none,
            some (.borrow state.nextLoan
              (focusValue steps (.loanHole (state.nextLoan + 1))))] : Row).set! 2
              (some (.borrow (state.nextLoan + 1) (.integer fieldValue))) =
            #[some (.address address), some (.integer amount),
              some (.borrow (state.nextLoan + 1) (.integer fieldValue)),
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            from rfl] at bodyStep
        obtain ⟨innerRow, rfl⟩ := innerStable _ _ _ _ _ _ bodyStep
        exact ⟨innerRow, rfl, bodyStep⟩
  /- The statement: the death marker over the let's value. -/
  rcases step with
    ⟨oF, oS, propagated, operandsStep, frameEq, stateEq, controlEq⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · subst finalFrame finalState control
    rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · obtain ⟨innerRow, rfl, innerStep⟩ := letRun letStep
      injection resultEq with h1 h2 h3
      subst oS oF propagated
      have applied := innerWp innerRow lS lc innerStep
      cases lc with
      | throw_ kind thrown => exact applied
      | value produced => cases abrupt
      | return_ values => exact ⟨innerRow, _, rfl, applied⟩
      | break_ label => exact ⟨innerRow, _, rfl, applied⟩
      | continue_ label => exact ⟨innerRow, _, rfl, applied⟩
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · simp at resultEq
    · obtain ⟨innerRow, rfl, innerStep⟩ := letRun letStep
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      have applied := innerWp innerRow lS (.value lv) innerStep
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩
      · exact applied finalFrame finalState rv evaluation
      · exact absurd evaluation
          (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)
    · simp [valuesNil] at nilStep

/-- Both loans of the field bracket retire over the row the inner block
reached: the field loan's current fills the hole at the focus, and the
resource loan's current returns to its key.  The walks through the resource
are read off its focus, so the path is any focused path. -/
theorem endLoans?_focusedGlobal (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan loan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (amount value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (argument : RuntimeValue) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)),
            some (.borrow loan (focusValue steps (.loanHole (loan + 1))))]
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨3⟩ : LocalId), #[.deref] ++ focusProjections steps, true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some .unit, some .unit]
           activeLoans := #[]
           loanLocations := #[(loan, { root := .global key }),
             (loan + 1,
               (⟨.local (⟨3⟩ : LocalId), #[.deref] ++ focusProjections steps, true⟩ :
                 RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (focusValue steps (.integer value))
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have zero_eq_zero : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zero_eq_one : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_zero : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_one : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  obtain ⟨holeMark, anyHole, -, holeFill, -, -⟩ := focus_walks plainSteps
  simp [endLoans?, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    holeMark, anyHole, holeFill,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan,
    transferGlobalLoan, transferredLoan?, Array.filter,
    zero_eq_zero, one_eq_one, zero_eq_one, zero_ne_zero, one_ne_one, zero_ne_one]

/-- The exit of the field bracket, over the row the inner block reached:
both loans retire, the resource returns to its key with the written field,
and the marker's rows are gone. -/
theorem endLoan_evaluate_focusedField (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan loan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (amount value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps) (argument : RuntimeValue) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate? #[argument]
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow (loan + 1) (.integer value)),
          some (.borrow loan
            (focusValue steps (.loanHole (loan + 1))))]
        { activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1, ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      { globals := globals.insert key (.loanHole loan)
        globalLoans := (loan, key) :: rest
        nextLoan
        pending } =
    some (.value
      (rowFrame #[some (.address address), some (.integer amount),
          some .unit, some .unit]
        { activeLoans := #[]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1, ⟨.local ⟨3⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      { globals := (globals.insert key (.loanHole loan)).insert key
          (focusValue steps (.integer value))
        globalLoans := rest
        nextLoan
        pending }
      argument) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    rowFrame]
  rw [endLoans?_focusedGlobal _ _ _ _ _ _ _ _ _ steps plainSteps]
  rfl

/-! ## An absent resource: the bracket throws at its borrow

A contract that declares the missing resource as an abort direction, instead
of requiring it, meets the bracket on both sides of the key.  On the absent
side the resource borrow throws the missing-resource abort, and every
combinator around it — the lets, the operand row, the death marker —
propagates that throw untouched: the bracket's throw obligation is the
borrow's. -/

/-- The mutable borrow of an absent resource: the missing-resource abort. -/
theorem globalBorrow_evaluate_absent (namespaceId : NamespaceId)
    (typeId : TypeId) (referenceType : ReferenceType) (lex : Nat)
    (address : String) (frame : RuntimeFrame) (state : RuntimeState)
    (absent : state.globals.lookup
      (globalKey namespaceId (instantiatedTypeId frame.typeInstantiation typeId)
        (.address address)) = none) :
    (GlobalLocationOperation.borrow
      { resource := ⟨namespaceId, typeId⟩, referenceType, kind := .mutable,
        lexicalLoan := lex }).evaluate? #[.address address] frame state =
    some (.throw_ frame state .abort #[]) := by
  simp [GlobalLocationOperation.evaluate?, borrowGlobalAt?, borrowGlobalUsing?,
    globalValue?, absent, RuntimeValue.storageKey?]

/-- A denotation that can only throw `kind thrown` from this frame and
state. -/
def OnlyThrows (denotation : ExprDenotation) (frame : RuntimeFrame)
    (state : RuntimeState) (kind : ThrowKind) (thrown : Array RuntimeValue) : Prop :=
  ∀ finalFrame finalState control,
    denotation frame state finalFrame finalState control →
      control = .throw_ kind thrown

/-- A let whose initializer can only throw can only throw. -/
theorem OnlyThrows.letNativeValue {initializer body : ExprDenotation}
    {frame : RuntimeFrame} {state : RuntimeState} {kind : ThrowKind}
    {thrown : Array RuntimeValue} (binder : NativePatternBinder)
    (only : OnlyThrows initializer frame state kind thrown) :
    OnlyThrows (letNativeValue binder initializer body) frame state kind thrown := by
  rintro finalFrame finalState control
    (⟨initStep, -⟩ | ⟨iF, iS, bound, boundFrame, initStep, -, -⟩)
  · exact only _ _ _ initStep
  · cases only _ _ _ initStep

/-- A reference operation whose one operand can only throw throws it. -/
theorem wpRowThrow_of_onlyThrows_operand {operation : ReferenceLocationOperation}
    {head : ExprDenotation} {row : Row} {registries : Registries}
    {state : RuntimeState} {kind : ThrowKind} {thrown : Array RuntimeValue}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (only : OnlyThrows head (rowFrame row registries) state kind thrown)
    (throws : postThrow kind thrown) :
    wpRowThrow (nativeReferenceOperation operation (valuesCons head valuesNil))
      row registries state postValue postThrow := by
  intro finalFrame finalState control step
  rcases step with
    ⟨oF, oS, propagated, operandsStep, frameEq, stateEq, controlEq⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · subst frameEq stateEq controlEq
    rcases operandsStep with
      ⟨lF, lS, lc, headStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, headStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, headStep, nilStep, resultEq⟩
    · have controlEq := only _ _ _ headStep
      subst controlEq
      injection resultEq with _ _ propagatedEq
      subst propagatedEq
      exact throws
    · cases only _ _ _ headStep
    · cases only _ _ _ headStep
  · rcases operandsStep with
      ⟨lF, lS, lc, headStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, headStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, headStep, nilStep, resultEq⟩
    · simp at resultEq
    · cases only _ _ _ headStep
    · cases only _ _ _ headStep

/-- The resource borrow over local 0, at an absent key, can only throw the
missing-resource abort. -/
theorem globalBorrow_onlyThrows (namespaceId : NamespaceId) (typeId : TypeId)
    (borrowType : ReferenceType) (lex : Nat) (address : String)
    (rest : List (Option RuntimeValue)) (registries : Registries)
    (state : RuntimeState)
    (absent : state.globals.lookup
      (globalKey namespaceId
        (instantiatedTypeId registries.typeInstantiation typeId)
        (.address address)) = none) :
    OnlyThrows
      (nativeGlobalOperation
        (GlobalLocationOperation.borrow
          { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
            kind := .mutable, lexicalLoan := lex })
        (valuesCons (localVar ⟨0⟩) valuesNil))
      (rowFrame (some (.address address) :: rest).toArray registries) state
      .abort #[] := by
  rintro bF bS c
    (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
      ⟨oF, oS, values, operandStep, evaluated⟩)
  · rcases operandStep with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandStep with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
      subst rF rS
      simp only [readLocal?_rowFrame, List.getElem?_toArray,
        List.getElem?_cons_zero, Option.join_some, Option.some.injEq] at readEq
      subst readEq
      injection valueEq with valueEq
      subst rv
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [globalBorrow_evaluate_absent namespaceId typeId borrowType lex address
          _ _ absent] at evaluation <;>
        cases Option.some.inj evaluation
      rfl
    · simp [valuesNil] at nilStep

/-- The field brackets over an absent resource: the borrow's abort is the
bracket's, whatever the reborrow and the inner block. -/
theorem wpRowThrow_fieldBracketAbsent (namespaceId : NamespaceId) (typeId : TypeId)
    (borrowType : ReferenceType) (loans : Array LoanId)
    (outerBinder innerBinder : NativePatternBinder) (reborrow inner : ExprDenotation)
    {address : String} {rest : List (Option RuntimeValue)} {registries : Registries}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (absent : state.globals.lookup
      (globalKey namespaceId
        (instantiatedTypeId registries.typeInstantiation typeId)
        (.address address)) = none)
    (thrown : postThrow .abort #[]) :
    wpRowThrow
      (nativeReferenceOperation (ReferenceLocationOperation.endLoan loans)
        (valuesCons
          (letNativeValue outerBinder
            (letNativeValue innerBinder
              (nativeGlobalOperation
                (GlobalLocationOperation.borrow
                  { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                    kind := .mutable, lexicalLoan := 0 })
                (valuesCons (localVar ⟨0⟩) valuesNil))
              reborrow)
            inner)
          valuesNil))
      (some (.address address) :: rest).toArray registries state postValue postThrow :=
  wpRowThrow_of_onlyThrows_operand
    (OnlyThrows.letNativeValue _ (OnlyThrows.letNativeValue _
      (globalBorrow_onlyThrows namespaceId typeId borrowType 0 address rest registries
        state absent)))
    thrown

/-- The whole-resource bracket over an absent resource. -/
theorem wpRowThrow_wholeBracketAbsent (namespaceId : NamespaceId) (typeId : TypeId)
    (borrowType : ReferenceType) (loans : Array LoanId)
    (outerBinder : NativePatternBinder) (inner : ExprDenotation)
    {address : String} {rest : List (Option RuntimeValue)} {registries : Registries}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (absent : state.globals.lookup
      (globalKey namespaceId
        (instantiatedTypeId registries.typeInstantiation typeId)
        (.address address)) = none)
    (thrown : postThrow .abort #[]) :
    wpRowThrow
      (nativeReferenceOperation (ReferenceLocationOperation.endLoan loans)
        (valuesCons
          (letNativeValue outerBinder
            (nativeGlobalOperation
              (GlobalLocationOperation.borrow
                { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                  kind := .mutable, lexicalLoan := 0 })
              (valuesCons (localVar ⟨0⟩) valuesNil))
            inner)
          valuesNil))
      (some (.address address) :: rest).toArray registries state postValue postThrow :=
  wpRowThrow_of_onlyThrows_operand
    (OnlyThrows.letNativeValue _
      (globalBorrow_onlyThrows namespaceId typeId borrowType 0 address rest registries
        state absent))
    thrown

/-! ## The whole-resource bracket

`replace` borrows the resource mutably and writes a constructed value
through the borrow: one loan, no reborrow, the write landing on the borrow
resting in local 2 and the exit installing it at the key. -/

/-- A write through the resource borrow resting in local 2: the loan's
registered location is the global key, which holds the hole, so the update
finds the borrow in the locals and replaces its current. -/
theorem mutate_evaluate_globalBorrow (state : RuntimeState) (loan : Nat)
    (key : GlobalKey) (address : String) (amount : Int)
    (current written : RuntimeValue) (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow loan current, written]
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan current)]
        { activeLoans
          loanLocations := #[(loan, { root := .global key })] })
      state =
    some (.value
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan written)]
        { activeLoans
          loanLocations := #[(loan, { root := .global key })] })
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, rewriteFirst]

/-- The exit of the whole-resource bracket: the loan retires and the
constructed resource returns to its key. -/
theorem endLoan_evaluate_wholeResource (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan loan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (amount : Int) (resource : RuntimeValue)
    (plain : Plain resource) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[.unit]
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow loan resource)]
        { activeLoans := #[(⟨0⟩, loan)]
          loanLocations := #[(loan, { root := .global key })] })
      { globals := globals.insert key (.loanHole loan)
        globalLoans := (loan, key) :: rest
        nextLoan
        pending } =
    some (.value
      (rowFrame #[some (.address address), some (.integer amount), some .unit]
        { activeLoans := #[]
          loanLocations := #[(loan, { root := .global key })] })
      { globals := (globals.insert key (.loanHole loan)).insert key resource
        globalLoans := rest
        nextLoan
        pending }
      .unit) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    rowFrame]
  rw [endLoans?_globalBorrowThirdLocal]
  simp [transferGlobalLoan, transferredLoan?, removeGlobalLoan,
    findFirst_eq_none_of_plain LoanMatcher.anyHole? plain]

/-- The whole-resource bracket: the global mutable borrow bound to local 2,
the inner block at the bracket's registries, and the death marker left as
a generic step for the exit row. -/
theorem wpRowThrow_wholeResourceBracket (namespaceId : NamespaceId)
    (typeId : TypeId) (borrowType : ReferenceType)
    (borrowMutable : borrowType.kind = .mutable) (fuelOuter : Nat)
    (inner : ExprDenotation) {address : String} {amount : Int}
    {resource : RuntimeValue} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (present : state.globals.lookup
        (globalKey namespaceId typeId (.address address)) = some resource)
    (innerStable : RowStable inner)
    (innerWp : wpRow inner
      #[some (.address address), some (.integer amount),
        some (.borrow state.nextLoan resource)]
      { activeLoans := #[(⟨0⟩, state.nextLoan)]
        loanLocations := #[(state.nextLoan,
          { root := .global (globalKey namespaceId typeId (.address address)) })] }
      { state with
        globals := state.globals.insert
          (globalKey namespaceId typeId (.address address))
          (.loanHole state.nextLoan)
        globalLoans :=
          (state.nextLoan, globalKey namespaceId typeId (.address address))
            :: state.globalLoans
        nextLoan := state.nextLoan + 1 }
      fun innerRow innerState control =>
        match control with
        | .throw_ kind thrown => postThrow kind thrown
        | .value produced =>
            ∀ finalFrame finalState retired,
              (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate?
                  #[produced]
                  (rowFrame innerRow
                    { activeLoans := #[(⟨0⟩, state.nextLoan)]
                      loanLocations := #[(state.nextLoan,
                        { root := .global
                            (globalKey namespaceId typeId (.address address)) })] })
                  innerState =
                some (.value finalFrame finalState retired) →
              ∃ finalRow finalRegistries,
                finalFrame = rowFrame finalRow finalRegistries ∧
                postValue finalRow finalRegistries finalState (.value retired)
        | _ =>
            postValue innerRow
              { activeLoans := #[(⟨0⟩, state.nextLoan)]
                loanLocations := #[(state.nextLoan,
                  { root := .global
                      (globalKey namespaceId typeId (.address address)) })] }
              innerState control) :
    wpRowThrow
      (nativeReferenceOperation (ReferenceLocationOperation.endLoan #[⟨0⟩])
        (valuesCons
          (letNativeValue ⟨fuelOuter + 1, .variable ⟨2⟩⟩
            (nativeGlobalOperation
              (GlobalLocationOperation.borrow
                { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                  kind := .mutable, lexicalLoan := 0 })
              (valuesCons (localVar ⟨0⟩) valuesNil))
            inner)
          valuesNil))
      #[some (.address address), some (.integer amount), none]
      { activeLoans := #[], loanLocations := #[] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  have borrowRun :
      ∀ {bF : RuntimeFrame} {bS : RuntimeState} {c : Control},
        (nativeGlobalOperation
          (GlobalLocationOperation.borrow
            { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
              kind := .mutable, lexicalLoan := 0 })
          (valuesCons (localVar ⟨0⟩) valuesNil))
          (rowFrame #[some (.address address), some (.integer amount), none]
            { activeLoans := #[], loanLocations := #[] })
          state bF bS c →
        bF = rowFrame #[some (.address address), some (.integer amount), none]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] } ∧
        bS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan resource) := by
    rintro bF bS c
      (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
        ⟨oF, oS, values, operandStep, evaluated⟩)
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · simp at resultEq
      · simp [valuesNil] at nilStep
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
        subst rF rS
        simp only [readLocal?_rowFrame, List.getElem?_toArray,
          List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
          at readEq
        subst readEq
        injection valueEq with valueEq
        subst rv
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (#[some (.address address), some (.integer amount), none] : Row) =
              (some (.address address) ::
                [some (.integer amount), none]).toArray from rfl]
            at evaluation <;>
          rw [globalBorrow_evaluate_rowFrame namespaceId typeId borrowType
            borrowMutable 0 address [some (.integer amount), none]
            { activeLoans := #[], loanLocations := #[], typeInstantiation := #[] }
            state _ (by simpa only [instantiatedTypeId_empty] using present)]
            at evaluation <;>
          simp only [instantiatedTypeId_empty] at evaluation <;>
          cases Option.some.inj evaluation
        refine ⟨?_, rfl, rfl⟩
        simp [rowFrame, Array.filter]
      · simp [valuesNil] at nilStep
  have letRun :
      ∀ {lF : RuntimeFrame} {lS : RuntimeState} {c : Control},
        (letNativeValue ⟨fuelOuter + 1, .variable ⟨2⟩⟩
          (nativeGlobalOperation
            (GlobalLocationOperation.borrow
              { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                kind := .mutable, lexicalLoan := 0 })
            (valuesCons (localVar ⟨0⟩) valuesNil))
          inner)
          (rowFrame #[some (.address address), some (.integer amount), none]
            { activeLoans := #[], loanLocations := #[] })
          state lF lS c →
        ∃ innerRow,
          lF = rowFrame innerRow
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] } ∧
          inner
            (rowFrame #[some (.address address), some (.integer amount),
                some (.borrow state.nextLoan resource)]
              { activeLoans := #[(⟨0⟩, state.nextLoan)]
                loanLocations := #[(state.nextLoan,
                  { root := .global (globalKey namespaceId typeId (.address address)) })] })
            { state with
              globals := state.globals.insert
                (globalKey namespaceId typeId (.address address))
                (.loanHole state.nextLoan)
              globalLoans :=
                (state.nextLoan, globalKey namespaceId typeId (.address address))
                  :: state.globalLoans
              nextLoan := state.nextLoan + 1 }
            lF lS c := by
    rintro lF lS c
      (⟨borrowStep, abrupt⟩ |
        ⟨bF, bS, borrowed, boundFrame, borrowStep, bindEq, bodyStep⟩)
    · obtain ⟨-, -, rfl⟩ := borrowRun borrowStep
      cases abrupt
    · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
      injection valueEq with valueEq
      subst borrowed
      rw [bindVariable_rowFrame fuelOuter ⟨2⟩ _ _ _ (by simp)] at bindEq
      cases Option.some.inj bindEq
      rw [show (#[some (.address address), some (.integer amount), none] : Row).set! 2
            (some (.borrow state.nextLoan resource)) =
          #[some (.address address), some (.integer amount),
            some (.borrow state.nextLoan resource)] from rfl] at bodyStep
      obtain ⟨innerRow, rfl⟩ := innerStable _ _ _ _ _ _ bodyStep
      exact ⟨innerRow, rfl, bodyStep⟩
  rcases step with
    ⟨oF, oS, propagated, operandsStep, frameEq, stateEq, controlEq⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · subst finalFrame finalState control
    rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · obtain ⟨innerRow, rfl, innerStep⟩ := letRun letStep
      injection resultEq with h1 h2 h3
      subst oS oF propagated
      have applied := innerWp innerRow lS lc innerStep
      cases lc with
      | throw_ kind thrown => exact applied
      | value produced => cases abrupt
      | return_ values => exact ⟨innerRow, _, rfl, applied⟩
      | break_ label => exact ⟨innerRow, _, rfl, applied⟩
      | continue_ label => exact ⟨innerRow, _, rfl, applied⟩
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · simp at resultEq
    · obtain ⟨innerRow, rfl, innerStep⟩ := letRun letStep
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      have applied := innerWp innerRow lS (.value lv) innerStep
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩
      · exact applied finalFrame finalState rv evaluation
      · exact absurd evaluation
          (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)
    · simp [valuesNil] at nilStep

/-- A resolved constructor over a literal operand row builds the nominal
value in place. -/
theorem NominalConstructor.evaluate?_literal (source : StructHandle)
    (variant : Option String) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (⟨source, variant, arguments.size⟩ : NominalConstructor).evaluate?
        arguments frame state =
      some (.value frame state (.nominal source variant arguments)) := by
  simp [NominalConstructor.evaluate?]

/-! ## The saved bracket: withdraw's shape

`endLoan [0,1] (let value := (let coin := &mut Coin[addr]; &mut coin.f.g);
let current := *value; inner)`, over five locals: the holder sits in local
4 and the saved read in local 3.  The inner block branches on the saved
read and may throw after retiring both loans, so it runs on the
throw-aware spine: the law takes it as `wpRowThrow` and needs no stability
of it. -/

/-- The saved-read variant of `endLoans?_focusedGlobal`: one scalar local
sits between the field borrow and its holder. -/
theorem endLoans?_focusedGlobalSaved (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan loan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (amount saved value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (argument : RuntimeValue) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[argument]
        { locals := #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)), some (.integer saved),
            some (.borrow loan (focusValue steps (.loanHole (loan + 1))))]
          activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref] ++ focusProjections steps, true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address), some (.integer amount),
              some .unit, some (.integer saved), some .unit]
           activeLoans := #[]
           loanLocations := #[(loan, { root := .global key }),
             (loan + 1,
               (⟨.local (⟨4⟩ : LocalId), #[.deref] ++ focusProjections steps, true⟩ :
                 RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (focusValue steps (.integer value))
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have zero_eq_zero : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zero_eq_one : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_zero : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_one : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  obtain ⟨holeMark, anyHole, -, holeFill, -, -⟩ := focus_walks plainSteps
  simp [endLoans?, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    holeMark, anyHole, holeFill,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan,
    transferGlobalLoan, transferredLoan?, Array.filter,
    zero_eq_zero, one_eq_one, zero_eq_one, zero_ne_zero, one_ne_one, zero_ne_one]

/-- The exit of the saved bracket, over the row the inner block reached. -/
theorem endLoan_evaluate_focusedFieldSaved (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan loan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (amount saved value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps) (argument : RuntimeValue) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate? #[argument]
      (rowFrame #[some (.address address), some (.integer amount),
          some (.borrow (loan + 1) (.integer value)), some (.integer saved),
          some (.borrow loan
            (focusValue steps (.loanHole (loan + 1))))]
        { activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1, ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      { globals := globals.insert key (.loanHole loan)
        globalLoans := (loan, key) :: rest
        nextLoan
        pending } =
    some (.value
      (rowFrame #[some (.address address), some (.integer amount),
          some .unit, some (.integer saved), some .unit]
        { activeLoans := #[]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1, ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      { globals := (globals.insert key (.loanHole loan)).insert key
          (focusValue steps (.integer value))
        globalLoans := rest
        nextLoan
        pending }
      argument) := by
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    rowFrame]
  rw [endLoans?_focusedGlobalSaved _ _ _ _ _ _ _ _ _ _ steps plainSteps]
  rfl

/-- Retire a focused mutable-storage bracket whose lexical ids follow a
shared read.  Shared global borrows mint no runtime loan, but they still
occupy lexical id `0`; consequently the mutable resource and field loans
are registered at ids `1` and `2`.  Keeping this common composition as a
closed evaluator law prevents normalization from unfolding the generic
loan search through the two plain nominal locals. -/
theorem endLoans?_focusedGlobalAfterSharedNominal
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (source : StructHandle)
    (saved value : Int) (steps : List FocusStep)
    (plainSteps : PlainSteps steps) (argument : RuntimeValue) :
    endLoans? #[(⟨1⟩ : LoanId), (⟨2⟩ : LoanId)] #[argument]
        { locals := #[some (.address address),
            some (.nominal source none #[.integer saved]),
            some (.nominal source none #[.integer saved]),
            some (.borrow (loan + 1) (.integer value)),
            some (.borrow loan (focusValue steps (.loanHole (loan + 1))))]
          activeLoans := #[(⟨1⟩, loan), (⟨2⟩, loan + 1)]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref] ++ focusProjections steps, true⟩ :
                RuntimePlace))] }
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        ({ locals := #[some (.address address),
              some (.nominal source none #[.integer saved]),
              some (.nominal source none #[.integer saved]),
              some .unit, some .unit]
           activeLoans := #[]
           loanLocations := #[(loan, { root := .global key }),
             (loan + 1,
               (⟨.local (⟨4⟩ : LocalId), #[.deref] ++ focusProjections steps, true⟩ :
                 RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (focusValue steps (.integer value))
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have one_eq_one : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have two_eq_two : (((⟨2⟩ : ExprId) == (⟨2⟩ : ExprId)) = true) := by decide
  have one_eq_two : (((⟨1⟩ : ExprId) == (⟨2⟩ : ExprId)) = false) := by decide
  have one_ne_one : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have two_ne_two : (((⟨2⟩ : ExprId) != (⟨2⟩ : ExprId)) = false) := by decide
  have one_ne_two : (((⟨1⟩ : ExprId) != (⟨2⟩ : ExprId)) = true) := by decide
  obtain ⟨holeMark, anyHole, -, holeFill, -, -⟩ := focus_walks plainSteps
  simp [endLoans?, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    findFirst_nominal, findFirstList_nil, findFirstList_cons,
    holeMark, anyHole, holeFill,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    rewriteFirst_nominal, rewriteFirstList_nil, rewriteFirstList_cons,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan,
    transferGlobalLoan, transferredLoan?, Array.filter,
    one_eq_one, two_eq_two, one_eq_two, one_ne_one, two_ne_two, one_ne_two]

/-- The flat, one-field instance used by generated field borrows.  Unlike
`endLoans?_focusedGlobalAfterSharedNominal`, this has no `PlainSteps` side
condition for the simplifier to discover while normalizing a closed frame. -/
theorem endLoans?_focusedGlobalAfterSharedNominal_singleField
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (source focusSource : StructHandle)
    (saved value : Int) (argument : RuntimeValue) :
    endLoans? #[(⟨1⟩ : LoanId), (⟨2⟩ : LoanId)] #[argument]
        (rowFrame #[some (.address address),
            some (.nominal source none #[.integer saved]),
            some (.nominal source none #[.integer saved]),
            some (.borrow (loan + 1) (.integer value)),
            some (.borrow loan
              (.nominal focusSource none #[.loanHole (loan + 1)]))]
          { activeLoans := #[(⟨1⟩, loan), (⟨2⟩, loan + 1)]
            loanLocations := #[(loan, { root := .global key }),
            (loan + 1,
              (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                RuntimePlace))] })
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        (rowFrame #[some (.address address),
              some (.nominal source none #[.integer saved]),
              some (.nominal source none #[.integer saved]),
              some .unit, some .unit]
           { activeLoans := #[]
             loanLocations := #[(loan, { root := .global key }),
             (loan + 1,
               (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                 RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (.nominal focusSource none #[.integer value])
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  simpa [rowFrame, focusValue, FocusStep.fill, focusProjections,
      FocusStep.index] using
    (endLoans?_focusedGlobalAfterSharedNominal
      (globals := globals) (rest := rest) (nextLoan := nextLoan)
      (loan := loan) (pending := pending) (key := key) (address := address)
      (source := source) (saved := saved) (value := value)
      (steps := [⟨focusSource, #[], #[], none⟩])
      (plainSteps := by simp [PlainSteps, FocusStep.Plain])
      (argument := argument))

/-- Retire two independent mutable global-field brackets in one death
marker.  The generated row keeps each field borrow immediately before its
resource holder, while the global-loan stack is newest first.  A dedicated
closed equation keeps this common multi-resource case linear instead of
unfolding four nested searches through the full symbolic frame. -/
theorem endLoans?_twoFocusedGlobals_singleField
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key₁ key₂ : GlobalKey) (address : String)
    (source₁ source₂ : StructHandle) (amount value₁ value₂ : Int)
    (argument : RuntimeValue) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId),
        (⟨2⟩ : LoanId), (⟨3⟩ : LoanId)] #[argument]
        (rowFrame #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value₁)),
            some (.borrow (loan + 3) (.integer value₂)),
            some (.borrow loan
              (.nominal source₁ none #[.loanHole (loan + 1)])),
            some (.borrow (loan + 2)
              (.nominal source₂ none #[.loanHole (loan + 3)]))]
          { activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1),
              (⟨2⟩, loan + 2), (⟨3⟩, loan + 3)]
            loanLocations := #[(loan, { root := .global key₁ }),
              (loan + 1,
                (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                  RuntimePlace)),
              (loan + 2, { root := .global key₂ }),
              (loan + 3,
                (⟨.local (⟨5⟩ : LocalId), #[.deref, .field 0], true⟩ :
                  RuntimePlace))] })
        { globals := (globals.insert key₁ (.loanHole loan)).insert key₂
              (.loanHole (loan + 2))
          globalLoans := (loan + 2, key₂) :: (loan, key₁) :: rest
          nextLoan
          pending } =
      some
        (rowFrame #[some (.address address), some (.integer amount),
              some .unit, some .unit, some .unit, some .unit]
           { activeLoans := #[]
             loanLocations := #[(loan, { root := .global key₁ }),
               (loan + 1,
                 (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                   RuntimePlace)),
               (loan + 2, { root := .global key₂ }),
               (loan + 3,
                 (⟨.local (⟨5⟩ : LocalId), #[.deref, .field 0], true⟩ :
                   RuntimePlace))] },
         { globals :=
             ((((globals.insert key₁ (.loanHole loan)).insert key₂
                 (.loanHole (loan + 2))).insert key₂
                 (.nominal source₂ none #[.integer value₂])).insert key₁
                 (match
                   (((globals.insert key₁ (.loanHole loan)).insert key₂
                       (.loanHole (loan + 2))).insert key₂
                       (.nominal source₂ none #[.integer value₂])).lookup key₁ with
                  | some stored =>
                      (rewriteFirst
                        (holeFill? loan
                          (.nominal source₁ none #[.integer value₁]))
                        stored).getD
                          (.nominal source₁ none #[.integer value₁])
                  | none => .nominal source₁ none #[.integer value₁]))
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have zero_eq_zero : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have two_eq_two : (((⟨2⟩ : ExprId) == (⟨2⟩ : ExprId)) = true) := by decide
  have three_eq_three : (((⟨3⟩ : ExprId) == (⟨3⟩ : ExprId)) = true) := by decide
  have zero_eq_one : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have zero_eq_two : (((⟨0⟩ : ExprId) == (⟨2⟩ : ExprId)) = false) := by decide
  have zero_eq_three : (((⟨0⟩ : ExprId) == (⟨3⟩ : ExprId)) = false) := by decide
  have one_eq_two : (((⟨1⟩ : ExprId) == (⟨2⟩ : ExprId)) = false) := by decide
  have one_eq_three : (((⟨1⟩ : ExprId) == (⟨3⟩ : ExprId)) = false) := by decide
  have two_eq_three : (((⟨2⟩ : ExprId) == (⟨3⟩ : ExprId)) = false) := by decide
  have zero_ne_zero : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have two_ne_two : (((⟨2⟩ : ExprId) != (⟨2⟩ : ExprId)) = false) := by decide
  have three_ne_three : (((⟨3⟩ : ExprId) != (⟨3⟩ : ExprId)) = false) := by decide
  have zero_ne_one : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  have zero_ne_two : (((⟨0⟩ : ExprId) != (⟨2⟩ : ExprId)) = true) := by decide
  have zero_ne_three : (((⟨0⟩ : ExprId) != (⟨3⟩ : ExprId)) = true) := by decide
  have one_ne_two : (((⟨1⟩ : ExprId) != (⟨2⟩ : ExprId)) = true) := by decide
  have one_ne_three : (((⟨1⟩ : ExprId) != (⟨3⟩ : ExprId)) = true) := by decide
  have two_ne_three : (((⟨2⟩ : ExprId) != (⟨3⟩ : ExprId)) = true) := by decide
  have loan_ne_one : loan ≠ loan + 1 := by omega
  have loan_ne_two : loan ≠ loan + 2 := by omega
  have loan_ne_three : loan ≠ loan + 3 := by omega
  have loan_one_ne_two : loan + 1 ≠ loan + 2 := by omega
  have loan_one_ne_three : loan + 1 ≠ loan + 3 := by omega
  have loan_two_ne_three : loan + 2 ≠ loan + 3 := by omega
  simp [endLoans?, rowFrame, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    findFirst_nominal, findFirstList_nil, findFirstList_cons,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    rewriteFirst_nominal, rewriteFirstList_nil, rewriteFirstList_cons,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan,
    transferGlobalLoan, transferredLoan?, Array.filter,
    zero_eq_zero, one_eq_one, two_eq_two, three_eq_three,
    zero_eq_one, zero_eq_two, zero_eq_three, one_eq_two, one_eq_three,
    two_eq_three, zero_ne_zero, one_ne_one, two_ne_two, three_ne_three,
    zero_ne_one, zero_ne_two, zero_ne_three, one_ne_two, one_ne_three,
    two_ne_three,
    loan_ne_one, loan_ne_two, loan_ne_three, loan_one_ne_two,
    loan_one_ne_three, loan_two_ne_three]
  rfl

/-- Retire a flat global-field bracket after an earlier independent bracket
has already been cleared.  Hidden resource holders remain at the end of the
fixed local row and old loan locations remain as harmless provenance, while
only lexical loans `2` and `3` are active. -/
theorem endLoans?_focusedGlobalPadded_singleField
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (previousKey key : GlobalKey) (address : String)
    (source : StructHandle) (amount value : Int) (argument : RuntimeValue) :
    endLoans? #[(⟨2⟩ : LoanId), (⟨3⟩ : LoanId)] #[argument]
        (rowFrame #[some (.address address), some (.integer amount), some .unit,
            some (.borrow (loan + 3) (.integer value)), some .unit,
            some (.borrow (loan + 2)
              (.nominal source none #[.loanHole (loan + 3)]))]
          { activeLoans := #[(⟨2⟩, loan + 2), (⟨3⟩, loan + 3)]
            loanLocations := #[(loan, { root := .global previousKey }),
              (loan + 1,
                (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                  RuntimePlace)),
              (loan + 2, { root := .global key }),
              (loan + 3,
                (⟨.local (⟨5⟩ : LocalId), #[.deref, .field 0], true⟩ :
                  RuntimePlace))] })
        { globals := globals.insert key (.loanHole (loan + 2))
          globalLoans := (loan + 2, key) :: rest
          nextLoan
          pending } =
      some
        (rowFrame #[some (.address address), some (.integer amount), some .unit,
              some .unit, some .unit, some .unit]
           { activeLoans := #[]
             loanLocations := #[(loan, { root := .global previousKey }),
               (loan + 1,
                 (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                   RuntimePlace)),
               (loan + 2, { root := .global key }),
               (loan + 3,
                 (⟨.local (⟨5⟩ : LocalId), #[.deref, .field 0], true⟩ :
                   RuntimePlace))] },
         { globals := (globals.insert key (.loanHole (loan + 2))).insert key
               (.nominal source none #[.integer value])
           globalLoans := rest
           nextLoan
           pending },
         argument) := by
  have two_eq_two : (((⟨2⟩ : ExprId) == (⟨2⟩ : ExprId)) = true) := by decide
  have three_eq_three : (((⟨3⟩ : ExprId) == (⟨3⟩ : ExprId)) = true) := by decide
  have two_eq_three : (((⟨2⟩ : ExprId) == (⟨3⟩ : ExprId)) = false) := by decide
  have two_ne_two : (((⟨2⟩ : ExprId) != (⟨2⟩ : ExprId)) = false) := by decide
  have three_ne_three : (((⟨3⟩ : ExprId) != (⟨3⟩ : ExprId)) = false) := by decide
  have two_ne_three : (((⟨2⟩ : ExprId) != (⟨3⟩ : ExprId)) = true) := by decide
  simp [endLoans?, rowFrame, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    findFirst_nominal, findFirstList_nil, findFirstList_cons,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    rewriteFirst_nominal, rewriteFirstList_nil, rewriteFirstList_cons,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan,
    transferGlobalLoan, transferredLoan?, Array.filter,
    two_eq_two, three_eq_three, two_eq_three, two_ne_two, three_ne_three,
    two_ne_three]

/-- Retire the first flat global-field bracket in a frame whose later user
and hidden-resource slots have already been reserved but are still empty. -/
theorem endLoans?_focusedGlobalReserved_singleField
    (globals : GlobalMap) (rest : List (Nat × GlobalKey))
    (nextLoan loan : Nat) (pending : Array (Nat × RuntimeValue))
    (key : GlobalKey) (address : String) (source : StructHandle)
    (amount value : Int) :
    endLoans? #[(⟨0⟩ : LoanId), (⟨1⟩ : LoanId)] #[]
        (rowFrame #[some (.address address), some (.integer amount),
            some (.borrow (loan + 1) (.integer value)), none,
            some (.borrow loan
              (.nominal source none #[.loanHole (loan + 1)])), none]
          { activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
            loanLocations := #[(loan, { root := .global key }),
              (loan + 1,
                (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                  RuntimePlace))] })
        { globals := globals.insert key (.loanHole loan)
          globalLoans := (loan, key) :: rest
          nextLoan
          pending } =
      some
        (rowFrame #[some (.address address), some (.integer amount), some .unit,
              none, some .unit, none]
           { activeLoans := #[]
             loanLocations := #[(loan, { root := .global key }),
               (loan + 1,
                 (⟨.local (⟨4⟩ : LocalId), #[.deref, .field 0], true⟩ :
                   RuntimePlace))] },
         { globals := (globals.insert key (.loanHole loan)).insert key
               (.nominal source none #[.integer value])
           globalLoans := rest
           nextLoan
           pending },
         .unit) := by
  have zero_eq_zero : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zero_eq_one : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_zero : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_one : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  simp [endLoans?, rowFrame, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    findFirst_nominal, findFirstList_nil, findFirstList_cons,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    rewriteFirst_nominal, rewriteFirstList_nil, rewriteFirstList_cons,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, globalLoanKeyIn?, removeGlobalLoan,
    transferGlobalLoan, transferredLoan?, Array.filter,
    zero_eq_zero, one_eq_one, zero_eq_one, zero_ne_zero, one_ne_one,
    zero_ne_one]

/-- The saved bracket law: the resource borrow bound to local 4, the field
reborrow bound to local 2, the inner block on the throw-aware spine at the
bracket's registries, and the death marker left as a generic step. -/
theorem wpRowThrow_focusedFieldBracketSaved (namespaceId : NamespaceId)
    (typeId : TypeId) (borrowType fieldType : ReferenceType)
    (borrowMutable : borrowType.kind = .mutable)
    (fieldMutable : fieldType.kind = .mutable)
    (steps : List FocusStep) (fuelOuter fuelInner : Nat)
    (inner : ExprDenotation) {address : String} {amount fieldValue : Int}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (present : state.globals.lookup
        (globalKey namespaceId typeId (.address address)) =
      some (focusValue steps (.integer fieldValue)))
    (innerWp : wpRowThrow inner
      #[some (.address address), some (.integer amount),
        some (.borrow (state.nextLoan + 1) (.integer fieldValue)), none,
        some (.borrow state.nextLoan
          (focusValue steps (.loanHole (state.nextLoan + 1))))]
      { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
        loanLocations := #[(state.nextLoan,
            { root := .global (globalKey namespaceId typeId (.address address)) }),
          (state.nextLoan + 1, ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] }
      { state with
        globals := state.globals.insert
          (globalKey namespaceId typeId (.address address))
          (.loanHole state.nextLoan)
        globalLoans :=
          (state.nextLoan, globalKey namespaceId typeId (.address address))
            :: state.globalLoans
        nextLoan := state.nextLoan + 1 + 1 }
      (fun innerRow innerRegistries innerState control =>
        match control with
        | .value produced =>
            ∀ finalFrame finalState retired,
              (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate?
                  #[produced] (rowFrame innerRow innerRegistries) innerState =
                some (.value finalFrame finalState retired) →
              ∃ finalRow finalRegistries,
                finalFrame = rowFrame finalRow finalRegistries ∧
                postValue finalRow finalRegistries finalState (.value retired)
        | _ => postValue innerRow innerRegistries innerState control)
      postThrow) :
    wpRowThrow
      (nativeReferenceOperation (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩])
        (valuesCons
          (letNativeValue ⟨fuelOuter + 1, .variable ⟨2⟩⟩
            (letNativeValue ⟨fuelInner + 1, .variable ⟨4⟩⟩
              (nativeGlobalOperation
                (GlobalLocationOperation.borrow
                  { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                    kind := .mutable, lexicalLoan := 0 })
                (valuesCons (localVar ⟨0⟩) valuesNil))
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨4⟩⟩, fields := focusFields steps,
                  referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
                valuesNil))
            inner)
          valuesNil))
      #[some (.address address), some (.integer amount), none, none, none]
      { activeLoans := #[], loanLocations := #[] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  have borrowRun :
      ∀ {bF : RuntimeFrame} {bS : RuntimeState} {c : Control},
        (nativeGlobalOperation
          (GlobalLocationOperation.borrow
            { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
              kind := .mutable, lexicalLoan := 0 })
          (valuesCons (localVar ⟨0⟩) valuesNil))
          (rowFrame #[some (.address address), some (.integer amount), none, none, none]
            { activeLoans := #[], loanLocations := #[] })
          state bF bS c →
        bF = rowFrame #[some (.address address), some (.integer amount), none, none, none]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] } ∧
        bS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan
          (focusValue steps (.integer fieldValue))) := by
    rintro bF bS c
      (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
        ⟨oF, oS, values, operandStep, evaluated⟩)
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · simp at resultEq
      · simp [valuesNil] at nilStep
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
        subst rF rS
        simp only [readLocal?_rowFrame, List.getElem?_toArray,
          List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
          at readEq
        subst readEq
        injection valueEq with valueEq
        subst rv
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (#[some (.address address), some (.integer amount), none,
              none, none] : Row) =
              (some (.address address) ::
                [some (.integer amount), none, none, none]).toArray from rfl]
            at evaluation <;>
          rw [globalBorrow_evaluate_rowFrame namespaceId typeId borrowType
            borrowMutable 0 address [some (.integer amount), none, none, none]
            { activeLoans := #[], loanLocations := #[], typeInstantiation := #[] }
            state _ (by simpa only [instantiatedTypeId_empty] using present)]
            at evaluation <;>
          simp only [instantiatedTypeId_empty] at evaluation <;>
          cases Option.some.inj evaluation
        refine ⟨?_, rfl, rfl⟩
        simp [rowFrame, Array.filter]
      · simp [valuesNil] at nilStep
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨4⟩⟩, fields := focusFields steps,
            referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
          valuesNil)
          (rowFrame #[some (.address address), some (.integer amount), none, none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] })
          { state with
            globals := state.globals.insert
              (globalKey namespaceId typeId (.address address))
              (.loanHole state.nextLoan)
            globalLoans :=
              (state.nextLoan, globalKey namespaceId typeId (.address address))
                :: state.globalLoans
            nextLoan := state.nextLoan + 1 }
          rF rS c →
        rF = rowFrame #[some (.address address), some (.integer amount), none, none,
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
              loanLocations := #[(state.nextLoan,
                  { root := .global (globalKey namespaceId typeId (.address address)) }),
                (state.nextLoan + 1,
                  ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] } ∧
        rS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 + 1 } ∧
        c = .value (.borrow (state.nextLoan + 1) (.integer fieldValue)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_nominalPath fieldType fieldMutable 1 ⟨4⟩
          steps state.nextLoan (.integer fieldValue) _ _ _ rfl]
          at evaluation <;>
        cases Option.some.inj evaluation
      refine ⟨?_, rfl, rfl⟩
      simp [rowFrame, Array.filter, Array.set!,
        show ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true from rfl]
  have letRun :
      ∀ {lF : RuntimeFrame} {lS : RuntimeState} {c : Control},
        (letNativeValue ⟨fuelOuter + 1, .variable ⟨2⟩⟩
          (letNativeValue ⟨fuelInner + 1, .variable ⟨4⟩⟩
            (nativeGlobalOperation
              (GlobalLocationOperation.borrow
                { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                  kind := .mutable, lexicalLoan := 0 })
              (valuesCons (localVar ⟨0⟩) valuesNil))
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨4⟩⟩, fields := focusFields steps,
                referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
              valuesNil))
          inner)
          (rowFrame #[some (.address address), some (.integer amount), none, none, none]
            { activeLoans := #[], loanLocations := #[] })
          state lF lS c →
        inner
          (rowFrame #[some (.address address), some (.integer amount),
              some (.borrow (state.nextLoan + 1) (.integer fieldValue)), none,
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
              loanLocations := #[(state.nextLoan,
                  { root := .global (globalKey namespaceId typeId (.address address)) }),
                (state.nextLoan + 1,
                  ⟨.local ⟨4⟩, #[.deref] ++ focusProjections steps, true⟩)] })
          { state with
            globals := state.globals.insert
              (globalKey namespaceId typeId (.address address))
              (.loanHole state.nextLoan)
            globalLoans :=
              (state.nextLoan, globalKey namespaceId typeId (.address address))
                :: state.globalLoans
            nextLoan := state.nextLoan + 1 + 1 }
          lF lS c := by
    rintro lF lS c
      (⟨initStep, abrupt⟩ |
        ⟨iF, iS, bound, boundFrame, initStep, bindEq, bodyStep⟩)
    · rcases initStep with ⟨borrowStep, abrupt'⟩ |
        ⟨bF, bS, borrowed, boundFrame', borrowStep, bindEq', reborrowStep⟩
      · obtain ⟨-, -, rfl⟩ := borrowRun borrowStep
        cases abrupt'
      · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
        injection valueEq with valueEq
        subst borrowed
        rw [bindVariable_rowFrame fuelInner ⟨4⟩ _ _ _ (by simp)] at bindEq'
        cases Option.some.inj bindEq'
        rw [show (#[some (.address address), some (.integer amount), none,
            none, none] : Row).set! 4 (some (.borrow state.nextLoan
              (focusValue steps (.integer fieldValue)))) =
            #[some (.address address), some (.integer amount), none, none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            from rfl] at reborrowStep
        obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
    · rcases initStep with ⟨borrowStep, abrupt'⟩ |
        ⟨bF, bS, borrowed, boundFrame', borrowStep, bindEq', reborrowStep⟩
      · cases abrupt'
      · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
        injection valueEq with valueEq
        subst borrowed
        rw [bindVariable_rowFrame fuelInner ⟨4⟩ _ _ _ (by simp)] at bindEq'
        cases Option.some.inj bindEq'
        rw [show (#[some (.address address), some (.integer amount), none,
            none, none] : Row).set! 4 (some (.borrow state.nextLoan
              (focusValue steps (.integer fieldValue)))) =
            #[some (.address address), some (.integer amount), none, none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            from rfl] at reborrowStep
        obtain ⟨rfl, rfl, valueEq'⟩ := reborrowRun reborrowStep
        injection valueEq' with valueEq'
        subst bound
        rw [bindVariable_rowFrame fuelOuter ⟨2⟩ _ _ _ (by simp)] at bindEq
        cases Option.some.inj bindEq
        rw [show (#[some (.address address), some (.integer amount), none, none,
            some (.borrow state.nextLoan
              (focusValue steps (.loanHole (state.nextLoan + 1))))] : Row).set! 2
              (some (.borrow (state.nextLoan + 1) (.integer fieldValue))) =
            #[some (.address address), some (.integer amount),
              some (.borrow (state.nextLoan + 1) (.integer fieldValue)), none,
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            from rfl] at bodyStep
        exact bodyStep
  rcases step with
    ⟨oF, oS, propagated, operandsStep, frameEq, stateEq, controlEq⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · subst finalFrame finalState control
    rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · have innerStep := letRun letStep
      injection resultEq with h1 h2 h3
      subst oS oF propagated
      have applied := innerWp lF lS lc innerStep
      cases lc with
      | throw_ kind thrown => exact applied
      | value produced => cases abrupt
      | return_ values => exact applied
      | break_ label => exact applied
      | continue_ label => exact applied
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · simp at resultEq
    · have innerStep := letRun letStep
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      obtain ⟨innerRow, innerRegistries, rfl, exitWp⟩ :=
        innerWp lF lS (.value lv) innerStep
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩
      · exact exitWp finalFrame finalState rv evaluation
      · exact absurd evaluation
          (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)
    · simp [valuesNil] at nilStep

/-! ## Reading storage without a bracket

A shared global borrow hands the resource itself to the operation that
reads it — no loan is minted and no hole is left — or aborts when the key
is empty; a copy is the identity; a field selection indexes a literal
nominal; the existence test is the key's presence.  None moves the frame,
so all four are row-stable evaluators. -/

/-- A shared global borrow at a present key hands the resource over. -/
theorem globalBorrowShared_evaluate_present (namespaceId : NamespaceId)
    (typeId : TypeId) (referenceType : ReferenceType) (lex : Nat)
    (sharedKind : referenceType.kind = .shared) (address : String)
    (frame : RuntimeFrame) (state : RuntimeState) (resource : RuntimeValue)
    (present : state.globals.lookup
      (globalKey namespaceId (instantiatedTypeId frame.typeInstantiation typeId)
        (.address address)) = some resource) :
    (GlobalLocationOperation.borrow
      { resource := ⟨namespaceId, typeId⟩, referenceType, kind := .immutable,
        lexicalLoan := lex }).evaluate? #[.address address] frame state =
    some (.value frame state resource) := by
  simp [GlobalLocationOperation.evaluate?, borrowGlobalAt?, borrowGlobalUsing?,
    globalValue?, present, RuntimeValue.storageKey?,
    borrowRuntimePlaceAt?_global_immutable_of_lookup lex referenceType _ _ _
      resource sharedKind present]

/-- A shared global borrow at an empty key aborts. -/
theorem globalBorrowShared_evaluate_absent (namespaceId : NamespaceId)
    (typeId : TypeId) (referenceType : ReferenceType) (lex : Nat)
    (address : String) (frame : RuntimeFrame) (state : RuntimeState)
    (absent : state.globals.lookup
      (globalKey namespaceId (instantiatedTypeId frame.typeInstantiation typeId)
        (.address address)) = none) :
    (GlobalLocationOperation.borrow
      { resource := ⟨namespaceId, typeId⟩, referenceType, kind := .immutable,
        lexicalLoan := lex }).evaluate? #[.address address] frame state =
    some (.throw_ frame state .abort #[]) := by
  simp [GlobalLocationOperation.evaluate?, borrowGlobalAt?, borrowGlobalUsing?,
    globalValue?, absent, RuntimeValue.storageKey?]

/-- The existence test, computed. -/
theorem contains_evaluate (namespaceId : NamespaceId) (typeId : TypeId)
    (address : String) (frame : RuntimeFrame) (state : RuntimeState) :
    (GlobalLocationOperation.contains ⟨namespaceId, typeId⟩).evaluate?
        #[.address address] frame state =
      some (.value frame state (.bool
        (state.globals.lookup (globalKey namespaceId
          (instantiatedTypeId frame.typeInstantiation typeId)
          (.address address))).isSome)) := by
  simp [GlobalLocationOperation.evaluate?, containsGlobalAt?, globalExists,
    globalValue?, RuntimeValue.storageKey?]

/-- A copy is the identity. -/
theorem copyValue_evaluate (resultType : Ty) (value : RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    (PrimitiveLocationOperation.copyValue resultType).evaluate? #[value] frame state =
      some (.value frame state value) := rfl

/-- Selecting a field of a literal nominal. -/
theorem select_evaluate (source : StructHandle) (variant : Option String)
    (index : Nat) (fields : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) (field : RuntimeValue)
    (at_ : fields[index]? = some field) :
    (NominalFieldLocation.mk source variant index).evaluateSelect?
        #[.nominal source variant fields] frame state =
      some (.value frame state field) := by
  simp [NominalFieldLocation.evaluateSelect?, liftConstructorEvaluator,
    selectNominalFieldAt?, at_]

/-! ## A call through the focused field, in a one-parameter bracket

`bump_counter`'s shape: `let value := &mut Counter[addr].value; bump(value)`
over the three-local row of one parameter and two lets — the field borrow
in local 1, the resource borrow in local 2 — with the callee called through
a reborrow of local 1.  The callee's contract leaves its global loan
registry abstract, so the bracket's exit is stated over an abstract list
with the lookups the callee's discipline preserves. -/

/-- The bracket's registries for the one-parameter row. -/
abbrev bracketOneRegistries (key : GlobalKey) (loan : Nat)
    (steps : List FocusStep) : Registries :=
  { activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1)]
    loanLocations := #[(loan, { root := .global key }),
      (loan + 1, ⟨.local ⟨2⟩, #[.deref] ++ focusProjections steps, true⟩)] }

/-- The reborrow of the field borrow in local 1, minting the call's loan. -/
theorem derefLocalBorrow_evaluate_bracketOneField (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (distinctZero : lex ≠ 0) (distinctOne : lex ≠ 1)
    (key : GlobalKey) (loan : Nat) (steps : List FocusStep)
    (address : String) (current : Int) (holder : RuntimeValue)
    (state : RuntimeState) :
    (({ location := ⟨⟨1⟩⟩, fields := [], referenceType,
        kind := .mutable, lexicalLoan := lex } :
      DerefLocalBorrowOperation)).evaluate? #[]
      (rowFrame #[some (.address address), some (.borrow (loan + 1) (.integer current)),
          some holder]
        (bracketOneRegistries key loan steps)) state =
    some (.value
      { locals := #[some (.address address),
          some (.borrow (loan + 1) (.loanHole state.nextLoan)), some holder]
        activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1), (⟨lex⟩, state.nextLoan)]
        loanLocations := #[(loan, { root := .global key }),
          (loan + 1, ⟨.local ⟨2⟩, #[.deref] ++ focusProjections steps, true⟩),
          (state.nextLoan, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 }
      (.borrow state.nextLoan (.integer current))) := by
  have lexZero : ((⟨0⟩ : ExprId) != ⟨lex⟩) = true := by
    simp [bne, show ((⟨0⟩ : ExprId) == ⟨lex⟩) = (0 == lex) from rfl, Ne.symm distinctZero]
  have lexOne : ((⟨1⟩ : ExprId) != ⟨lex⟩) = true := by
    simp [bne, show ((⟨1⟩ : ExprId) == ⟨lex⟩) = (1 == lex) from rfl, Ne.symm distinctOne]
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    DerefLocalBorrowOperation.resolve?, resolveDerefLocalFieldPath?,
    resolveNominalFieldSteps?, readLocal?, borrowRuntimePlaceAt?,
    readRuntimePlace?, readRoot?, readProjections?, writeRuntimePlace?,
    writeRoot?, writeProjections?, rowFrame, mutableKind, Array.filter,
    lexZero, lexOne,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-- The callee's write-back through the call's loan lands in the field
borrow, and the call's rows retire from the registries. -/
theorem applyPendingWriteBack_bracketOneField (key : GlobalKey) (loan lex : Nat)
    (steps : List FocusStep) (address : String) (holder : RuntimeValue)
    (callLoan : Nat) (replacement : Int) (state : RuntimeState)
    (separateOuter : callLoan ≠ loan) (separateInner : callLoan ≠ loan + 1) :
    applyPendingWriteBack
      { locals := #[some (.address address),
          some (.borrow (loan + 1) (.loanHole callLoan)), some holder]
        activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1), (⟨lex⟩, callLoan)]
        loanLocations := #[(loan, { root := .global key }),
          (loan + 1, ⟨.local ⟨2⟩, #[.deref] ++ focusProjections steps, true⟩),
          (callLoan, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
      state callLoan (.integer replacement) =
    (rowFrame #[some (.address address), some (.borrow (loan + 1) (.integer replacement)),
        some holder]
      (bracketOneRegistries key loan steps),
     state) := by
  have outerNe : (loan == callLoan) = false := by
    simp [Ne.symm separateOuter]
  have innerNe : (loan + 1 == callLoan) = false := by
    simp [Ne.symm separateInner]
  have outerNe' : (callLoan == loan) = false := by simp [separateOuter]
  have innerNe' : (callLoan == loan + 1) = false := by simp [separateInner]
  have outerNeP : ¬ loan = callLoan := Ne.symm separateOuter
  have innerNeP : ¬ loan + 1 = callLoan := Ne.symm separateInner
  simp [applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?,
    transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    rowFrame, Array.filter, outerNe, innerNe, outerNe', innerNe', outerNeP, innerNeP]

/-- The call's lexical marker after its loan already retired: nothing to
end, and the argument passes through. -/
theorem endLoan_evaluate_bracketOneRetired (lex : Nat)
    (distinctZero : lex ≠ 0) (distinctOne : lex ≠ 1)
    (key : GlobalKey) (loan : Nat) (steps : List FocusStep) (row : Row)
    (argument : RuntimeValue) (state : RuntimeState) :
    (ReferenceLocationOperation.endLoan #[⟨lex⟩]).evaluate? #[argument]
      (rowFrame row (bracketOneRegistries key loan steps)) state =
    some (.value (rowFrame row (bracketOneRegistries key loan steps)) state argument) := by
  have lexZero : ((⟨0⟩ : ExprId) == ⟨lex⟩) = false := by
    simp [show ((⟨0⟩ : ExprId) == ⟨lex⟩) = (0 == lex) from rfl, Ne.symm distinctZero]
  have lexOne : ((⟨1⟩ : ExprId) == ⟨lex⟩) = false := by
    simp [show ((⟨1⟩ : ExprId) == ⟨lex⟩) = (1 == lex) from rfl, Ne.symm distinctOne]
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, endLoans?,
    rowFrame, Array.find?, lexZero, lexOne]

/-- Removing one loan's registration leaves every other loan's lookup. -/
theorem globalLoanKeyIn?_removeGlobalLoan_ne (loans : List (Nat × GlobalKey))
    (removed other : Nat) (separate : other ≠ removed) :
    globalLoanKeyIn? (removeGlobalLoan loans removed) other =
      globalLoanKeyIn? loans other := by
  induction loans with
  | nil => rfl
  | cons entry rest ih =>
      unfold removeGlobalLoan
      by_cases removedHead : entry.1 = removed
      · have removedOther : (removed == other) = false := by
          simp [Ne.symm separate]
        simp [removedHead, removedOther, globalLoanKeyIn?, List.find?_cons]
      · simp only [beq_iff_eq, removedHead, if_false]
        simp only [globalLoanKeyIn?, List.find?_cons] at ih ⊢
        cases otherHead : (entry.1 == other)
        · simpa [otherHead] using ih
        · simp [otherHead]

/-- The exit of the one-parameter field bracket over an abstract global
loan registry: the resource loan is looked up and retired, the field loan
lives in the frame and is not registered, and the written field returns to
the resource at its key. -/
theorem endLoan_evaluate_focusedFieldOne (globals : GlobalMap)
    (loans : List (Nat × GlobalKey)) (nextLoan loan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps) (argument : RuntimeValue)
    (lookupOuter : globalLoanKeyIn? loans loan = some key)
    (lookupInner : globalLoanKeyIn? loans (loan + 1) = none) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate? #[argument]
      (rowFrame #[some (.address address),
          some (.borrow (loan + 1) (.integer value)),
          some (.borrow loan (focusValue steps (.loanHole (loan + 1))))]
        (bracketOneRegistries key loan steps))
      { globals := globals.insert key (.loanHole loan)
        globalLoans := loans
        nextLoan
        pending } =
    some (.value
      (rowFrame #[some (.address address), some .unit, some .unit]
        { activeLoans := #[]
          loanLocations := #[(loan, { root := .global key }),
            (loan + 1, ⟨.local ⟨2⟩, #[.deref] ++ focusProjections steps, true⟩)] })
      { globals := (globals.insert key (.loanHole loan)).insert key
          (focusValue steps (.integer value))
        globalLoans := removeGlobalLoan loans loan
        nextLoan
        pending }
      argument) := by
  have zero_eq_zero : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have one_eq_one : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zero_eq_one : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_zero : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have one_ne_one : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zero_ne_one : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  obtain ⟨holeMark, anyHole, -, holeFill, -, -⟩ := focus_walks plainSteps
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, Array.find?, findBorrowValue?, findFirst_unit,
    findFirst_integer, findFirst_address, findFirst_borrow, findFirst_loanHole,
    holeMark, anyHole, holeFill,
    clearBorrowValue, rewriteFirst_unit, rewriteFirst_integer,
    rewriteFirst_address, rewriteFirst_borrow, rewriteFirst_loanHole,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin,
    fillHole?, globalLoanKey?, lookupOuter, lookupInner, transferGlobalLoan,
    transferredLoan?, Array.filter,
    zero_eq_zero, one_eq_one, zero_eq_one, zero_ne_zero, one_ne_one, zero_ne_one]

/-- `core.call f(&mut *value)` through the field borrow in local 1, as one
statement inside the bracket: the reborrow mints the call's loan, the
callee's export lands in the field borrow, and the marker retires the
call's row.  The bracket's registries return to their entry values. -/
theorem callFocusedStatement_step
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (distinctZero : lex ≠ 0) (distinctOne : lex ≠ 1)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (key : GlobalKey) (loan : Nat) (steps : List FocusStep)
    (address : String) (cur : Int) (holder : RuntimeValue)
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (innerPrior : loan + 1 < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan (.integer cur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ newValue : Int,
              calleeFinal.pending =
                state.pending.push (state.nextLoan, .integer newValue) ∧
              postValue
                #[some (.address address),
                  some (.borrow (loan + 1) (.integer newValue)), some holder]
                (bracketOneRegistries key loan steps)
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (packResults results))
        | .threw kind thrown => postThrow kind thrown)) :
    ∀ {finalFrame : RuntimeFrame} {finalState : RuntimeState} {control : Control},
      (nativeReferenceOperation (.endLoan #[⟨lex⟩])
        (valuesCons
          (nativeCall handle none
            (nativeFunctionRelation calleeUnit calleeShape calleeBody)
            (valuesCons
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨1⟩⟩, fields := []
                  referenceType, kind := .mutable, lexicalLoan := lex }
                valuesNil)
              valuesNil))
          valuesNil))
        (rowFrame #[some (.address address), some (.borrow (loan + 1) (.integer cur)),
            some holder]
          (bracketOneRegistries key loan steps))
        state finalFrame finalState control →
      (∃ kind thrown, control = .throw_ kind thrown ∧ postThrow kind thrown) ∨
      ∃ (results : Array RuntimeValue) (newValue : Int),
        control = .value (packResults results) ∧
        finalFrame = rowFrame
          #[some (.address address), some (.borrow (loan + 1) (.integer newValue)),
            some holder]
          (bracketOneRegistries key loan steps) ∧
        postValue
          #[some (.address address), some (.borrow (loan + 1) (.integer newValue)),
            some holder]
          (bracketOneRegistries key loan steps)
          finalState (.value (packResults results)) := by
  have separateOuter : state.nextLoan ≠ loan := by omega
  have separateInner : state.nextLoan ≠ loan + 1 := by omega
  unfold wpFunction at calleeWp
  intro finalFrame finalState control step
  let minted : RuntimeFrame :=
    { locals := #[some (.address address),
        some (.borrow (loan + 1) (.loanHole state.nextLoan)), some holder]
      activeLoans := #[(⟨0⟩, loan), (⟨1⟩, loan + 1), (⟨lex⟩, state.nextLoan)]
      loanLocations := #[(loan, { root := .global key }),
        (loan + 1, ⟨.local ⟨2⟩, #[.deref] ++ focusProjections steps, true⟩),
        (state.nextLoan, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨1⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lex }
          valuesNil)
          (rowFrame #[some (.address address), some (.borrow (loan + 1) (.integer cur)),
              some holder]
            (bracketOneRegistries key loan steps))
          state rF rS c →
        rF = minted ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan (.integer cur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_bracketOneField referenceType mutableKind lex
          distinctZero distinctOne key loan steps address cur holder state] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  have callRun :
      ∀ {cF : RuntimeFrame} {cS : RuntimeState} {c : Control},
        (nativeCall handle none
          (nativeFunctionRelation calleeUnit calleeShape calleeBody)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨1⟩⟩, fields := []
                referenceType, kind := .mutable, lexicalLoan := lex }
              valuesNil)
            valuesNil))
          (rowFrame #[some (.address address), some (.borrow (loan + 1) (.integer cur)),
              some holder]
            (bracketOneRegistries key loan steps))
          state cF cS c →
        (∃ kind thrown, c = .throw_ kind thrown ∧ postThrow kind thrown) ∨
        ∃ (results : Array RuntimeValue) (newValue : Int)
            (calleeFinal : RuntimeState),
          c = .value (packResults results) ∧
          cF = rowFrame
            #[some (.address address), some (.borrow (loan + 1) (.integer newValue)),
              some holder]
            (bracketOneRegistries key loan steps) ∧
          cS = { globals := calleeFinal.globals
                 globalLoans := calleeFinal.globalLoans
                 nextLoan := calleeFinal.nextLoan
                 pending := state.pending } ∧
          postValue
            #[some (.address address), some (.borrow (loan + 1) (.integer newValue)),
              some holder]
            (bracketOneRegistries key loan steps)
            { globals := calleeFinal.globals
              globalLoans := calleeFinal.globalLoans
              nextLoan := calleeFinal.nextLoan
              pending := state.pending }
            (.value (packResults results)) := by
    rintro cF cS c
      (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
        ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
          rfl, rfl, rfl⟩)
    · rcases operandControl with
        ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
      · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
      · simp [valuesNil] at nilStep
        obtain ⟨rfl, rfl, rfl⟩ := nilStep
        simp at resultEq
      · simp [valuesNil] at nilStep
    · rcases operandValues with
        ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
      · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
      · obtain ⟨rfl, rfl, veq⟩ := reborrowRun reborrowStep
        injection veq with veq
        subst veq
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        have applied := calleeWp calleeState outcome (by simpa using calleeStep)
        cases outcome with
        | threw kind thrown =>
            exact .inl ⟨kind, thrown, rfl, applied⟩
        | returned results =>
            obtain ⟨newValue, pendingShape, continuation⟩ := applied
            refine .inr ⟨results, newValue, calleeState, rfl, ?_, ?_, continuation⟩
            · rw [callFrame_returned]
              simp only [registerReturnedLoan]
              rw [applyPendingFrom_single (inherited := state.pending)
                (loan := state.nextLoan) (current := .integer newValue) pendingShape]
              rw [show minted = _ from rfl,
                applyPendingWriteBack_bracketOneField key loan lex steps address holder
                  state.nextLoan newValue _ separateOuter separateInner]
            · rw [applyPendingFrom_single (inherited := state.pending)
                (loan := state.nextLoan) (current := .integer newValue) pendingShape]
              rw [show minted = _ from rfl,
                applyPendingWriteBack_bracketOneField key loan lex steps address holder
                  state.nextLoan newValue _ separateOuter separateInner]
      · obtain ⟨rfl, rfl, -⟩ := reborrowRun reborrowStep
        simp [valuesNil] at nilStep
  rcases step with
    ⟨oF, oS, propagated, operandsStep, rfl, rfl, rfl⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · rcases operandsStep with
      ⟨cF, cS, cc, callStep, abrupt, resultEq⟩ |
      ⟨cF, cS, cv, f, s, vs, callStep, nilStep, resultEq⟩ |
      ⟨cF, cS, cv, f, s, cc, callStep, nilStep, resultEq⟩
    · rcases callRun callStep with ⟨kind, thrown, rfl, thrown_post⟩ |
        ⟨results, newValue, calleeFinal, rfl, -, -, -⟩
      · injection resultEq with h1 h2 h3
        subst h1 h2 h3
        exact .inl ⟨kind, thrown, rfl, thrown_post⟩
      · cases abrupt
    · simp [valuesNil] at nilStep
      obtain ⟨rfl, rfl, rfl⟩ := nilStep
      simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨cF, cS, cc, callStep, abrupt, resultEq⟩ |
      ⟨cF, cS, cv, f, s, vs, callStep, nilStep, resultEq⟩ |
      ⟨cF, cS, cv, f, s, cc, callStep, nilStep, resultEq⟩
    · simp at resultEq
    · rcases callRun callStep with ⟨kind, thrown, absurdEq, -⟩ |
        ⟨results, newValue, calleeFinal, ceq, feq, seq, continuation⟩
      · cases absurdEq
      · injection ceq with ceq
        subst ceq
        subst feq seq
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (ReferenceLocationOperation.endLoan
              #[(⟨lex⟩ : LoanId)]).evaluate?
              [packResults results].toArray _ _ = _ from
            endLoan_evaluate_bracketOneRetired lex distinctZero distinctOne key loan steps
              #[some (.address address), some (.borrow (loan + 1) (.integer newValue)),
                some holder]
              (packResults results) _] at evaluation <;>
          cases Option.some.inj evaluation
        exact .inr ⟨results, newValue, rfl, rfl, continuation⟩
    · simp [valuesNil] at nilStep

/-- The statement-spine rule for the focused call statement. -/
theorem wpStatementsRowThrow_consCallFocused
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (distinctZero : lex ≠ 0) (distinctOne : lex ≠ 1)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (key : GlobalKey) (loan : Nat) (steps : List FocusStep)
    (address : String) (cur : Int) (holder : RuntimeValue)
    {state : RuntimeState}
    (tail : StatementsDenotation)
    {postDone : Row → Registries → RuntimeState → Prop}
    {postControl : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (innerPrior : loan + 1 < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan (.integer cur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned _ =>
            ∃ newValue : Int,
              calleeFinal.pending =
                state.pending.push (state.nextLoan, .integer newValue) ∧
              wpStatementsRowThrow tail
                #[some (.address address),
                  some (.borrow (loan + 1) (.integer newValue)), some holder]
                (bracketOneRegistries key loan steps)
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                postDone postControl postThrow
        | .threw kind thrown => postThrow kind thrown)) :
    wpStatementsRowThrow
      (statementsCons
        (nativeReferenceOperation (.endLoan #[⟨lex⟩])
          (valuesCons
            (nativeCall handle none
              (nativeFunctionRelation calleeUnit calleeShape calleeBody)
              (valuesCons
                (nativeDerefLocalBorrowOperation
                  { location := ⟨⟨1⟩⟩, fields := []
                    referenceType, kind := .mutable, lexicalLoan := lex }
                  valuesNil)
                valuesNil))
            valuesNil))
        tail)
      #[some (.address address), some (.borrow (loan + 1) (.integer cur)), some holder]
      (bracketOneRegistries key loan steps)
      state postDone postControl postThrow := by
  rintro result
    (⟨finalFrame, finalState, raised, headStep, abrupt, rfl⟩ |
      ⟨headFrame, headState, runtimeValue, headStep, tailStep⟩)
  · rcases callFocusedStatement_step
        (postValue := fun continuationRow continuationRegistries
            continuationState _ =>
          wpStatementsRowThrow tail continuationRow continuationRegistries
            continuationState postDone postControl postThrow)
        handle lex distinctZero distinctOne referenceType mutableKind key loan steps
        address cur holder innerPrior calleeWp headStep with
      ⟨kind, thrown, rfl, thrown_post⟩ |
      ⟨results, newValue, rfl, -, -⟩
    · exact thrown_post
    · cases abrupt
  · rcases callFocusedStatement_step
        (postValue := fun continuationRow continuationRegistries
            continuationState _ =>
          wpStatementsRowThrow tail continuationRow continuationRegistries
            continuationState postDone postControl postThrow)
        handle lex distinctZero distinctOne referenceType mutableKind key loan steps
        address cur holder innerPrior calleeWp headStep with
      ⟨kind, thrown, absurdEq, -⟩ |
      ⟨results, newValue, valueEq, rfl, continuation⟩
    · cases absurdEq
    · exact continuation result tailStep

/-- The one-parameter field bracket with a throw-aware inner block:
`endLoan [0,1] (let value := (let holder := &mut R[addr]; &mut holder.f);
inner)` over the three-local row.  The inner block runs on the spine, so a
call inside it is at home; the exit is left as the death-marker step over
whatever row and registries the inner block reached. -/
theorem wpRowThrow_focusedFieldBracketOneThrow (namespaceId : NamespaceId)
    (typeId : TypeId) (borrowType fieldType : ReferenceType)
    (borrowMutable : borrowType.kind = .mutable)
    (fieldMutable : fieldType.kind = .mutable)
    (steps : List FocusStep) (fuelOuter fuelInner : Nat)
    (inner : ExprDenotation) {address : String} {fieldValue : Int}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (present : state.globals.lookup
        (globalKey namespaceId typeId (.address address)) =
      some (focusValue steps (.integer fieldValue)))
    (innerWp : wpRowThrow inner
      #[some (.address address),
        some (.borrow (state.nextLoan + 1) (.integer fieldValue)),
        some (.borrow state.nextLoan
          (focusValue steps (.loanHole (state.nextLoan + 1))))]
      (bracketOneRegistries (globalKey namespaceId typeId (.address address))
        state.nextLoan steps)
      { state with
        globals := state.globals.insert
          (globalKey namespaceId typeId (.address address))
          (.loanHole state.nextLoan)
        globalLoans :=
          (state.nextLoan, globalKey namespaceId typeId (.address address))
            :: state.globalLoans
        nextLoan := state.nextLoan + 1 + 1 }
      (fun innerRow innerRegistries innerState control =>
        match control with
        | .value produced =>
            ∀ finalFrame finalState retired,
              (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate?
                  #[produced] (rowFrame innerRow innerRegistries) innerState =
                some (.value finalFrame finalState retired) →
              ∃ finalRow finalRegistries,
                finalFrame = rowFrame finalRow finalRegistries ∧
                postValue finalRow finalRegistries finalState (.value retired)
        | _ => postValue innerRow innerRegistries innerState control)
      postThrow) :
    wpRowThrow
      (nativeReferenceOperation (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩])
        (valuesCons
          (letNativeValue ⟨fuelOuter + 1, .variable ⟨1⟩⟩
            (letNativeValue ⟨fuelInner + 1, .variable ⟨2⟩⟩
              (nativeGlobalOperation
                (GlobalLocationOperation.borrow
                  { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                    kind := .mutable, lexicalLoan := 0 })
                (valuesCons (localVar ⟨0⟩) valuesNil))
              (nativeDerefLocalBorrowOperation
                { location := ⟨⟨2⟩⟩, fields := focusFields steps,
                  referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
                valuesNil))
            inner)
          valuesNil))
      #[some (.address address), none, none]
      { activeLoans := #[], loanLocations := #[] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  have borrowRun :
      ∀ {bF : RuntimeFrame} {bS : RuntimeState} {c : Control},
        (nativeGlobalOperation
          (GlobalLocationOperation.borrow
            { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
              kind := .mutable, lexicalLoan := 0 })
          (valuesCons (localVar ⟨0⟩) valuesNil))
          (rowFrame #[some (.address address), none, none]
            { activeLoans := #[], loanLocations := #[] })
          state bF bS c →
        bF = rowFrame #[some (.address address), none, none]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] } ∧
        bS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan
          (focusValue steps (.integer fieldValue))) := by
    rintro bF bS c
      (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
        ⟨oF, oS, values, operandStep, evaluated⟩)
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · simp at resultEq
      · simp [valuesNil] at nilStep
    · rcases operandStep with
        ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
      · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
        subst rc
        cases abrupt
      · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
        subst rF rS
        simp only [readLocal?_rowFrame, List.getElem?_toArray,
          List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
          at readEq
        subst readEq
        injection valueEq with valueEq
        subst rv
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s f vs
        injection resultEq with h4 h5 h6
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [show (#[some (.address address), none, none] : Row) =
              (some (.address address) :: [none, none]).toArray from rfl]
            at evaluation <;>
          rw [globalBorrow_evaluate_rowFrame namespaceId typeId borrowType
            borrowMutable 0 address [none, none]
            { activeLoans := #[], loanLocations := #[], typeInstantiation := #[] }
            state _ (by simpa only [instantiatedTypeId_empty] using present)]
            at evaluation <;>
          simp only [instantiatedTypeId_empty] at evaluation <;>
          cases Option.some.inj evaluation
        refine ⟨?_, rfl, rfl⟩
        simp [rowFrame, Array.filter]
      · simp [valuesNil] at nilStep
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨2⟩⟩, fields := focusFields steps,
            referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
          valuesNil)
          (rowFrame #[some (.address address), none,
              some (.borrow state.nextLoan
                (focusValue steps (.integer fieldValue)))]
            { activeLoans := #[(⟨0⟩, state.nextLoan)]
              loanLocations := #[(state.nextLoan,
                { root := .global (globalKey namespaceId typeId (.address address)) })] })
          { state with
            globals := state.globals.insert
              (globalKey namespaceId typeId (.address address))
              (.loanHole state.nextLoan)
            globalLoans :=
              (state.nextLoan, globalKey namespaceId typeId (.address address))
                :: state.globalLoans
            nextLoan := state.nextLoan + 1 }
          rF rS c →
        rF = rowFrame #[some (.address address), none,
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            (bracketOneRegistries (globalKey namespaceId typeId (.address address))
              state.nextLoan steps) ∧
        rS = { state with
                globals := state.globals.insert
                  (globalKey namespaceId typeId (.address address))
                  (.loanHole state.nextLoan)
                globalLoans :=
                  (state.nextLoan, globalKey namespaceId typeId (.address address))
                    :: state.globalLoans
                nextLoan := state.nextLoan + 1 + 1 } ∧
        c = .value (.borrow (state.nextLoan + 1) (.integer fieldValue)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_nominalPath fieldType fieldMutable 1 ⟨2⟩
          steps state.nextLoan (.integer fieldValue) _ _ _ rfl]
          at evaluation <;>
        cases Option.some.inj evaluation
      refine ⟨?_, rfl, rfl⟩
      simp [rowFrame, Array.filter, Array.set!, bracketOneRegistries,
        show ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true from rfl]
  have letRun :
      ∀ {lF : RuntimeFrame} {lS : RuntimeState} {c : Control},
        (letNativeValue ⟨fuelOuter + 1, .variable ⟨1⟩⟩
          (letNativeValue ⟨fuelInner + 1, .variable ⟨2⟩⟩
            (nativeGlobalOperation
              (GlobalLocationOperation.borrow
                { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
                  kind := .mutable, lexicalLoan := 0 })
              (valuesCons (localVar ⟨0⟩) valuesNil))
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨2⟩⟩, fields := focusFields steps,
                referenceType := fieldType, kind := .mutable, lexicalLoan := 1 }
              valuesNil))
          inner)
          (rowFrame #[some (.address address), none, none]
            { activeLoans := #[], loanLocations := #[] })
          state lF lS c →
        inner
          (rowFrame #[some (.address address),
              some (.borrow (state.nextLoan + 1) (.integer fieldValue)),
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            (bracketOneRegistries (globalKey namespaceId typeId (.address address))
              state.nextLoan steps))
          { state with
            globals := state.globals.insert
              (globalKey namespaceId typeId (.address address))
              (.loanHole state.nextLoan)
            globalLoans :=
              (state.nextLoan, globalKey namespaceId typeId (.address address))
                :: state.globalLoans
            nextLoan := state.nextLoan + 1 + 1 }
          lF lS c := by
    rintro lF lS c
      (⟨initStep, abrupt⟩ |
        ⟨iF, iS, bound, boundFrame, initStep, bindEq, bodyStep⟩)
    · rcases initStep with ⟨borrowStep, abrupt'⟩ |
        ⟨bF, bS, borrowed, boundFrame', borrowStep, bindEq', reborrowStep⟩
      · obtain ⟨-, -, rfl⟩ := borrowRun borrowStep
        cases abrupt'
      · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
        injection valueEq with valueEq
        subst borrowed
        rw [bindVariable_rowFrame fuelInner ⟨2⟩ _ _ _ (by simp)] at bindEq'
        cases Option.some.inj bindEq'
        rw [show (#[some (.address address), none, none] : Row).set! 2
            (some (.borrow state.nextLoan (focusValue steps (.integer fieldValue)))) =
            #[some (.address address), none,
              some (.borrow state.nextLoan (focusValue steps (.integer fieldValue)))]
            from rfl] at reborrowStep
        obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
        cases abrupt
    · rcases initStep with ⟨borrowStep, abrupt'⟩ |
        ⟨bF, bS, borrowed, boundFrame', borrowStep, bindEq', reborrowStep⟩
      · cases abrupt'
      · obtain ⟨rfl, rfl, valueEq⟩ := borrowRun borrowStep
        injection valueEq with valueEq
        subst borrowed
        rw [bindVariable_rowFrame fuelInner ⟨2⟩ _ _ _ (by simp)] at bindEq'
        cases Option.some.inj bindEq'
        rw [show (#[some (.address address), none, none] : Row).set! 2
            (some (.borrow state.nextLoan (focusValue steps (.integer fieldValue)))) =
            #[some (.address address), none,
              some (.borrow state.nextLoan (focusValue steps (.integer fieldValue)))]
            from rfl] at reborrowStep
        obtain ⟨rfl, rfl, valueEq'⟩ := reborrowRun reborrowStep
        injection valueEq' with valueEq'
        subst bound
        rw [bindVariable_rowFrame fuelOuter ⟨1⟩ _ _ _ (by simp)] at bindEq
        cases Option.some.inj bindEq
        rw [show (#[some (.address address), none,
            some (.borrow state.nextLoan
              (focusValue steps (.loanHole (state.nextLoan + 1))))] : Row).set! 1
              (some (.borrow (state.nextLoan + 1) (.integer fieldValue))) =
            #[some (.address address),
              some (.borrow (state.nextLoan + 1) (.integer fieldValue)),
              some (.borrow state.nextLoan
                (focusValue steps (.loanHole (state.nextLoan + 1))))]
            from rfl] at bodyStep
        exact bodyStep
  rcases step with
    ⟨oF, oS, propagated, operandsStep, frameEq, stateEq, controlEq⟩ |
    ⟨oF, oS, values, operandsStep, evaluated⟩
  · subst finalFrame finalState control
    rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · have innerStep := letRun letStep
      injection resultEq with h1 h2 h3
      subst oS oF propagated
      have applied := innerWp lF lS lc innerStep
      cases lc with
      | throw_ kind thrown => exact applied
      | value produced => cases abrupt
      | return_ values => exact applied
      | break_ label => exact applied
      | continue_ label => exact applied
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandsStep with
      ⟨lF, lS, lc, letStep, abrupt, resultEq⟩ |
      ⟨lF, lS, lv, f, s, vs, letStep, nilStep, resultEq⟩ |
      ⟨lF, lS, lv, f, s, lc, letStep, nilStep, resultEq⟩
    · simp at resultEq
    · have innerStep := letRun letStep
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      obtain ⟨innerRow, innerRegistries, rfl, applied⟩ :=
        innerWp lF lS (.value lv) innerStep
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩
      · exact applied finalFrame finalState rv evaluation
      · exact absurd evaluation
          (liftPlaceEvaluator_ne_throw _ _ _ _ _ _ _ _)
    · simp [valuesNil] at nilStep

/-- The caller's loan discipline through a bracket around a call: the
bracket registered its resource loan, the callee kept every older lookup
and freshness, and the exit removed the resource loan's registration. -/
theorem LoanDiscipline.throughBracketCall (initial calleeFinal : RuntimeState)
    (key : GlobalKey) (bracketGlobals : GlobalMap)
    (bracketPending : Array (Nat × RuntimeValue))
    (exitGlobals : GlobalMap) (exitPending : Array (Nat × RuntimeValue))
    (fresh : FreshGlobalLoanIds initial)
    (discipline : LoanDiscipline
      { globals := bracketGlobals
        globalLoans := (initial.nextLoan, key) :: initial.globalLoans
        nextLoan := initial.nextLoan + 1 + 1 + 1
        pending := bracketPending }
      calleeFinal) :
    LoanDiscipline initial
      { globals := exitGlobals
        globalLoans := removeGlobalLoan calleeFinal.globalLoans initial.nextLoan
        nextLoan := calleeFinal.nextLoan
        pending := exitPending } := by
  obtain ⟨freshKept, stable, monotone⟩ := discipline
  have bracketFresh : FreshGlobalLoanIds
      { globals := bracketGlobals
        globalLoans := (initial.nextLoan, key) :: initial.globalLoans
        nextLoan := initial.nextLoan + 1 + 1 + 1
        pending := bracketPending } := by
    intro loan bound
    have advanced : initial.nextLoan + 1 + 1 + 1 ≤ loan := bound
    have headNe : (initial.nextLoan == loan) = false := by
      simp
      omega
    simp only [globalLoanKeyIn?, List.find?_cons, headNe]
    exact fresh loan (by omega)
  have monotone' : initial.nextLoan + 1 + 1 + 1 ≤ calleeFinal.nextLoan := monotone
  refine ⟨fun _ loan bound => ?_, fun loan bound => ?_,
    (by show initial.nextLoan ≤ calleeFinal.nextLoan; omega)⟩
  · have advanced : calleeFinal.nextLoan ≤ loan := bound
    rw [globalLoanKeyIn?_removeGlobalLoan_ne calleeFinal.globalLoans initial.nextLoan loan
      (by omega)]
    exact freshKept bracketFresh loan (by omega)
  · rw [globalLoanKeyIn?_removeGlobalLoan_ne calleeFinal.globalLoans initial.nextLoan loan
      (by omega)]
    rw [stable loan (by show loan < initial.nextLoan + 1 + 1 + 1; omega)]
    have headNe : (initial.nextLoan == loan) = false := by
      simp
      omega
    simp only [globalLoanKeyIn?, List.find?_cons, headNe]

/-! ## A returned reborrow of any parameter

`&mut *slot` as a returned value, at whichever local the parameter sits
in and whatever the registries hold: the loan is minted, its hole rests in
the parameter, and the borrow is the value.  This is the focused reborrow
at the empty path. -/

theorem wpRowThrow_returnedReborrowAt (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (localId : LocalId) (outer : Nat) (current : RuntimeValue)
    (row : Row) (registries : Registries) (state : RuntimeState)
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (slot : row[localId.index]? = some (some (.borrow outer current)))
    (exit : postValue
      (row.set! localId.index (some (.borrow outer (.loanHole state.nextLoan))))
      { activeLoans :=
          (registries.activeLoans.filter (·.1 != ⟨lex⟩)).push (⟨lex⟩, state.nextLoan)
        loanLocations := registries.loanLocations.push
          (state.nextLoan, ⟨.local localId, #[.deref], true⟩)
        typeInstantiation := registries.typeInstantiation }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan current))) :
    wpRowThrow
      (nativeDerefLocalBorrowOperation
        { location := ⟨localId⟩, fields := [], referenceType, kind := .mutable,
          lexicalLoan := lex }
        valuesNil)
      row registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with h1 h2 h3
    subst oS oF values
    have evaluate := derefLocalBorrow_evaluate_nominalPath referenceType mutableKind lex
      localId [] outer current row registries state (by simpa [focusValue] using slot)
    simp [focusFields, focusValue, focusProjections] at evaluate
    rcases evaluated with ⟨rv, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [evaluate] at evaluation <;>
      cases Option.some.inj evaluation
    exact ⟨_, _, rfl, exit⟩

/-- Finalizing a `Bool` selector beside two mutable parameters, the first
lent to the returned reborrow: the lender exports the returned loan's hole,
the other its current. -/
theorem exportFrameLoans_rowFrame_returnedChoiceLeft (state : RuntimeState)
    (flag : Bool) (leftLoan rightLoan returned : Nat) (right : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separateLeft : leftLoan ≠ returned) (separateRight : rightLoan ≠ returned)
    (noLeftGlobal : globalLoanKeyIn? state.globalLoans leftLoan = none)
    (noRightGlobal : globalLoanKeyIn? state.globalLoans rightLoan = none) :
    exportFrameLoans
        (rowFrame #[some (.bool flag), some (.borrow leftLoan (.loanHole returned)),
          some (.borrow rightLoan (.integer right))]
          { activeLoans, loanLocations })
        state
      = { state with
          pending := (state.pending.push (leftLoan, .loanHole returned)).push
            (rightLoan, .integer right) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have returnedLeft : returned ≠ leftLoan := Ne.symm separateLeft
  have returnedRight : returned ≠ rightLoan := Ne.symm separateRight
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    applyWriteBack_empty, globalLoanKey?, noLeftGlobal, noRightGlobal,
    returnedLeft, returnedRight, separateLeft, separateRight]

/-- The mirror: the second parameter lent to the returned reborrow. -/
theorem exportFrameLoans_rowFrame_returnedChoiceRight (state : RuntimeState)
    (flag : Bool) (leftLoan rightLoan returned : Nat) (left : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separateLeft : leftLoan ≠ returned) (separateRight : rightLoan ≠ returned)
    (noLeftGlobal : globalLoanKeyIn? state.globalLoans leftLoan = none)
    (noRightGlobal : globalLoanKeyIn? state.globalLoans rightLoan = none) :
    exportFrameLoans
        (rowFrame #[some (.bool flag), some (.borrow leftLoan (.integer left)),
          some (.borrow rightLoan (.loanHole returned))]
          { activeLoans, loanLocations })
        state
      = { state with
          pending := (state.pending.push (leftLoan, .integer left)).push
            (rightLoan, .loanHole returned) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have returnedLeft : returned ≠ leftLoan := Ne.symm separateLeft
  have returnedRight : returned ≠ rightLoan := Ne.symm separateRight
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    applyWriteBack_empty, globalLoanKey?, noLeftGlobal, noRightGlobal,
    returnedLeft, returnedRight, separateLeft, separateRight]

/-! ## A returned pair of reborrows

`(&mut *left, &mut *right)`: both parameters lent, the two borrows packed
into a tuple as the value, both holes exported. -/

/-- The tuple constructor packs its operand row. -/
theorem tuple_evaluate (values : Array RuntimeValue) (frame : RuntimeFrame)
    (state : RuntimeState) :
    PrimitiveLocationOperation.tuple.evaluate? values frame state =
      some (.value frame state (.tuple values)) := rfl

theorem wpRowThrow_returnedPair (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lexFirst lexSecond : Nat)
    (distinctLexical : lexFirst ≠ lexSecond)
    (leftOuter rightOuter : Nat) (leftCur rightCur : RuntimeValue) (state : RuntimeState)
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (exit : postValue
      #[some (.borrow leftOuter (.loanHole state.nextLoan)),
        some (.borrow rightOuter (.loanHole (state.nextLoan + 1)))]
      { activeLoans := #[(⟨lexFirst⟩, state.nextLoan), (⟨lexSecond⟩, state.nextLoan + 1)]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩),
          (state.nextLoan + 1, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
      { state with nextLoan := state.nextLoan + 1 + 1 }
      (.value (.tuple #[.borrow state.nextLoan leftCur,
        .borrow (state.nextLoan + 1) rightCur]))) :
    wpRowThrow
      (nativePrimitiveOperation PrimitiveLocationOperation.tuple
        (valuesCons
          (nativeDerefLocalBorrowOperation
            { location := ⟨⟨0⟩⟩, fields := [], referenceType, kind := .mutable,
              lexicalLoan := lexFirst }
            valuesNil)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨1⟩⟩, fields := [], referenceType, kind := .mutable,
                lexicalLoan := lexSecond }
              valuesNil)
            valuesNil)))
      #[some (.borrow leftOuter leftCur), some (.borrow rightOuter rightCur)]
      { activeLoans := #[]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
      state postValue postThrow := by
  intro finalFrame finalState control step
  let mintedOne : RuntimeFrame :=
    { locals := #[some (.borrow leftOuter (.loanHole state.nextLoan)),
        some (.borrow rightOuter rightCur)]
      activeLoans := #[(⟨lexFirst⟩, state.nextLoan)]
      loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
        (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
        (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
  have firstRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨0⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lexFirst }
          valuesNil)
          (rowFrame #[some (.borrow leftOuter leftCur), some (.borrow rightOuter rightCur)]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
          state rF rS c →
        rF = mintedOne ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan leftCur) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_firstOfTwo referenceType mutableKind lexFirst
          leftOuter rightOuter leftCur rightCur state] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  have secondRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨1⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lexSecond }
          valuesNil)
          mintedOne { state with nextLoan := state.nextLoan + 1 } rF rS c →
        rF = { locals := #[some (.borrow leftOuter (.loanHole state.nextLoan)),
                 some (.borrow rightOuter (.loanHole (state.nextLoan + 1)))]
               activeLoans := #[(⟨lexFirst⟩, state.nextLoan), (⟨lexSecond⟩, state.nextLoan + 1)]
               loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                 (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
                 (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩),
                 (state.nextLoan + 1, ⟨.local ⟨1⟩, #[.deref], true⟩)] } ∧
        rS = { state with nextLoan := state.nextLoan + 1 + 1 } ∧
        c = .value (.borrow (state.nextLoan + 1) rightCur) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [show mintedOne = _ from rfl,
          derefLocalBorrow_evaluate_secondOfTwo referenceType mutableKind lexFirst
            lexSecond leftOuter rightOuter state.nextLoan distinctLexical
            rightCur { state with nextLoan := state.nextLoan + 1 }] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  rcases step with ⟨oF, oS, propagated, operandStep, rfl, rfl, rfl⟩ |
    ⟨oF, oS, values, operandStep, evaluated⟩
  · rcases operandStep with
      ⟨rF, rS, rc, firstStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, firstStep, tailStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, firstStep, tailStep, resultEq⟩
    · obtain ⟨-, -, rfl⟩ := firstRun firstStep
      cases abrupt
    · simp at resultEq
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · simp at tailEq
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep
  · rcases operandStep with
      ⟨rF, rS, rc, firstStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, firstStep, tailStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, firstStep, tailStep, resultEq⟩
    · obtain ⟨-, -, rfl⟩ := firstRun firstStep
      cases abrupt
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · obtain ⟨rfl, rfl, veq'⟩ := secondRun secondStep
        injection veq' with veq'
        subst veq'
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s' f' vs'
        injection tailEq with h4 h5 h6
        subst s f vs
        injection resultEq with h7 h8 h9
        subst oS oF values
        rcases evaluated with ⟨rv, evaluation, rfl⟩ |
          ⟨kind, thrown, evaluation, rfl⟩ <;>
          rw [tuple_evaluate] at evaluation <;>
          cases Option.some.inj evaluation
        exact ⟨_, _, rfl, exit⟩
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · simp at tailEq
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep

/-- Finalizing two parameters both lent to returned reborrows: each exports
its returned loan's hole, in local order. -/
theorem exportFrameLoans_rowFrame_returnedPair (state : RuntimeState)
    (leftLoan rightLoan leftReturned rightReturned : Nat)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separateLeftLeft : leftLoan ≠ leftReturned)
    (separateLeftRight : leftLoan ≠ rightReturned)
    (separateRightLeft : rightLoan ≠ leftReturned)
    (separateRightRight : rightLoan ≠ rightReturned)
    (noLeftGlobal : globalLoanKeyIn? state.globalLoans leftLoan = none)
    (noRightGlobal : globalLoanKeyIn? state.globalLoans rightLoan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow leftLoan (.loanHole leftReturned)),
          some (.borrow rightLoan (.loanHole rightReturned))]
          { activeLoans, loanLocations })
        state
      = { state with
          pending := (state.pending.push (leftLoan, .loanHole leftReturned)).push
            (rightLoan, .loanHole rightReturned) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have h1 : leftReturned ≠ leftLoan := Ne.symm separateLeftLeft
  have h2 : rightReturned ≠ leftLoan := Ne.symm separateLeftRight
  have h3 : leftReturned ≠ rightLoan := Ne.symm separateRightLeft
  have h4 : rightReturned ≠ rightLoan := Ne.symm separateRightRight
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    applyWriteBack_empty, globalLoanKey?, noLeftGlobal, noRightGlobal,
    h1, h2, h3, h4, separateLeftLeft, separateLeftRight, separateRightLeft,
    separateRightRight]

/-! ## A caller consuming a returned pair of reborrows

`let (a, b) := f(&mut *left, &mut *right); *a := …; *b := …`: the callee
returned two borrows whose loans the caller's two parameters now hold as
holes; the caller writes through each returned borrow where it rests, and
one marker settles both. -/

/-- The four-local row of two lending parameters and the two returned
borrows resting beside them. -/
abbrev returnedPairRegistries (leftOuter rightOuter leftReturned rightReturned : Nat) :
    Registries :=
  { activeLoans := #[(⟨0⟩, leftReturned), (⟨1⟩, rightReturned)]
    loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
      (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
      (leftReturned, ⟨.local ⟨0⟩, #[.deref], true⟩),
      (rightReturned, ⟨.local ⟨1⟩, #[.deref], true⟩)] }

/-- A write through the first returned borrow, resting in local 2. -/
theorem mutate_evaluate_returnedPairLeft (state : RuntimeState)
    (leftOuter rightOuter leftReturned rightReturned : Nat)
    (leftCurrent rightCurrent : Int) (written : RuntimeValue)
    (separateLeft : leftOuter ≠ leftReturned)
    (separateRight : rightOuter ≠ leftReturned)
    (separateReturned : rightReturned ≠ leftReturned) :
    ReferenceLocationOperation.mutate.evaluate?
      #[.borrow leftReturned (.integer leftCurrent), written]
      (rowFrame #[some (.borrow leftOuter (.loanHole leftReturned)),
          some (.borrow rightOuter (.loanHole rightReturned)),
          some (.borrow leftReturned (.integer leftCurrent)),
          some (.borrow rightReturned (.integer rightCurrent))]
        (returnedPairRegistries leftOuter rightOuter leftReturned rightReturned))
      state =
    some (.value
      (rowFrame #[some (.borrow leftOuter (.loanHole leftReturned)),
          some (.borrow rightOuter (.loanHole rightReturned)),
          some (.borrow leftReturned written),
          some (.borrow rightReturned (.integer rightCurrent))]
        (returnedPairRegistries leftOuter rightOuter leftReturned rightReturned))
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, rewriteFirst, writeRuntimePlace?, writeRoot?,
    writeProjections?, returnedPairRegistries,
    separateLeft, separateRight, separateReturned]

/-- A write through the second returned borrow, resting in local 3. -/
theorem mutate_evaluate_returnedPairRight (state : RuntimeState)
    (leftOuter rightOuter leftReturned rightReturned : Nat)
    (leftCurrent rightCurrent : Int) (written : RuntimeValue)
    (separateLeft : leftOuter ≠ rightReturned)
    (separateRight : rightOuter ≠ rightReturned)
    (separateReturned : leftReturned ≠ rightReturned) :
    ReferenceLocationOperation.mutate.evaluate?
      #[.borrow rightReturned (.integer rightCurrent), written]
      (rowFrame #[some (.borrow leftOuter (.loanHole leftReturned)),
          some (.borrow rightOuter (.loanHole rightReturned)),
          some (.borrow leftReturned (.integer leftCurrent)),
          some (.borrow rightReturned (.integer rightCurrent))]
        (returnedPairRegistries leftOuter rightOuter leftReturned rightReturned))
      state =
    some (.value
      (rowFrame #[some (.borrow leftOuter (.loanHole leftReturned)),
          some (.borrow rightOuter (.loanHole rightReturned)),
          some (.borrow leftReturned (.integer leftCurrent)),
          some (.borrow rightReturned written)]
        (returnedPairRegistries leftOuter rightOuter leftReturned rightReturned))
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, rewriteFirst, writeRuntimePlace?, writeRoot?,
    writeProjections?, returnedPairRegistries,
    separateLeft, separateRight, separateReturned]

/-- The marker settling both returned loans: each resting current fills its
lender's hole, and both returned slots clear. -/
theorem endLoan_evaluate_returnedPair (state : RuntimeState)
    (leftOuter rightOuter leftReturned rightReturned : Nat)
    (leftCurrent rightCurrent : Int) (argument : RuntimeValue)
    (separateLeftLeft : leftOuter ≠ leftReturned)
    (separateLeftRight : leftOuter ≠ rightReturned)
    (separateRightLeft : rightOuter ≠ leftReturned)
    (separateRightRight : rightOuter ≠ rightReturned)
    (separateReturned : leftReturned ≠ rightReturned) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩, ⟨1⟩]).evaluate? #[argument]
      (rowFrame #[some (.borrow leftOuter (.loanHole leftReturned)),
          some (.borrow rightOuter (.loanHole rightReturned)),
          some (.borrow leftReturned (.integer leftCurrent)),
          some (.borrow rightReturned (.integer rightCurrent))]
        (returnedPairRegistries leftOuter rightOuter leftReturned rightReturned))
      state =
    some (.value
      (rowFrame #[some (.borrow leftOuter (.integer leftCurrent)),
          some (.borrow rightOuter (.integer rightCurrent)), some .unit, some .unit]
        { activeLoans := #[]
          loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
            (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
            (leftReturned, ⟨.local ⟨0⟩, #[.deref], true⟩),
            (rightReturned, ⟨.local ⟨1⟩, #[.deref], true⟩)] })
      state argument) := by
  have zeroSelf : (((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true) := by decide
  have oneSelf : (((⟨1⟩ : ExprId) == (⟨1⟩ : ExprId)) = true) := by decide
  have zeroOne : (((⟨0⟩ : ExprId) == (⟨1⟩ : ExprId)) = false) := by decide
  have zeroNe : (((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false) := by decide
  have oneNe : (((⟨1⟩ : ExprId) != (⟨1⟩ : ExprId)) = false) := by decide
  have zeroNeOne : (((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true) := by decide
  have returnedSymm : rightReturned ≠ leftReturned := Ne.symm separateReturned
  have leftSymm : leftReturned ≠ leftOuter := Ne.symm separateLeftLeft
  have rightSymm : rightReturned ≠ rightOuter := Ne.symm separateRightRight
  have leftRightSymm : rightReturned ≠ leftOuter := Ne.symm separateLeftRight
  have rightLeftSymm : leftReturned ≠ rightOuter := Ne.symm separateRightLeft
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, Array.find?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst, applyWriteBack, fillVisibleHole,
    holeInFrame, holeWithin, fillHole?, globalLoanKey?, globalLoanKeyIn?,
    transferGlobalLoan, transferredLoan?,
    returnedPairRegistries, Array.filter,
    zeroSelf, oneSelf, zeroOne, zeroNe, oneNe, zeroNeOne,
    separateLeftLeft, separateLeftRight, separateRightLeft, separateRightRight,
    separateReturned, returnedSymm, leftSymm, rightSymm, leftRightSymm,
    rightLeftSymm]

/-- Both callee write-backs reconciled into the caller's two parameters,
each carrying the hole of its returned loan. -/
theorem applyPendingFrom_twoReturnedPair
    (inherited : Array (Nat × RuntimeValue))
    (globals : GlobalMap) (globalLoans : List (Nat × GlobalKey))
    (runtimeNextLoan leftOuter rightOuter leftFresh rightFresh : Nat)
    (leftReturned rightReturned : Nat)
    (freshSeparate : leftFresh ≠ rightFresh)
    (leftFirst : leftOuter ≠ leftFresh) (leftSecond : leftOuter ≠ rightFresh)
    (rightFirst : rightOuter ≠ leftFresh) (rightSecond : rightOuter ≠ rightFresh)
    (returnedSeparate : leftReturned ≠ rightReturned)
    (leftReturnedFresh : leftReturned ≠ rightFresh) :
    applyPendingFrom inherited
        (rowFrame #[some (.borrow leftOuter (.loanHole leftFresh)),
            some (.borrow rightOuter (.loanHole rightFresh)), none, none]
          { activeLoans := #[(⟨0⟩, leftFresh), (⟨1⟩, rightFresh)]
            loanLocations :=
              #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
                (leftFresh, ⟨.local ⟨0⟩, #[.deref], true⟩),
                (rightFresh, ⟨.local ⟨1⟩, #[.deref], true⟩)] })
        { globals, globalLoans, nextLoan := runtimeNextLoan
          pending := (inherited.push (leftFresh, .loanHole leftReturned)).push
            (rightFresh, .loanHole rightReturned) } =
      (rowFrame #[some (.borrow leftOuter (.loanHole leftReturned)),
            some (.borrow rightOuter (.loanHole rightReturned)), none, none]
         { activeLoans := #[(⟨0⟩, leftReturned), (⟨1⟩, rightReturned)]
           loanLocations :=
             #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
               (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
               (leftReturned, ⟨.local ⟨0⟩, #[.deref], true⟩),
               (rightReturned, ⟨.local ⟨1⟩, #[.deref], true⟩)] },
       { globals, globalLoans, nextLoan := runtimeNextLoan
         pending := inherited }) := by
  have secondFirst : rightFresh ≠ leftFresh := Ne.symm freshSeparate
  have returnedSymm : rightReturned ≠ leftReturned := Ne.symm returnedSeparate
  rw [applyPendingFrom_two_push]
  simp [rowFrame, applyPendingWriteBack, fillLocalLoanHole?, localLoanPlace?,
    readRuntimePlace?, readRoot?, readLocal?, readProjections?, fillHole?,
    rewriteFirst, writeRuntimePlace?, writeRoot?, writeProjections?,
    transferActiveLoan, transferLoanLocation, transferredLoan?, findFirst,
    Array.filter, secondFirst, returnedSymm, freshSeparate, returnedSeparate,
    leftReturnedFresh, leftFirst, leftSecond, rightFirst, rightSecond]

/-- `core.call f(&mut *left, &mut *right)` returning a pair of reborrows:
both parameters lend, the callee's two exports are the returned loans'
holes, and the caller stands with both returned borrows resting in the two
`let`-bound locals. -/
theorem wpRowThrow_callReturnedPair
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lexFirst lexSecond : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    (distinctLexical : lexFirst ≠ lexSecond)
    (zeroFirst : lexFirst = 0) (oneSecond : lexSecond = 1)
    {leftOuter rightOuter : Nat} {leftCur rightCur : Int} {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (leftPrior : leftOuter < state.nextLoan)
    (rightPrior : rightOuter < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 + 1 }
      #[.borrow state.nextLoan (.integer leftCur),
        .borrow (state.nextLoan + 1) (.integer rightCur)]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ (leftReturned rightReturned : Nat) (leftValue rightValue : Int),
              results = #[.tuple #[.borrow leftReturned (.integer leftValue),
                .borrow rightReturned (.integer rightValue)]] ∧
              leftReturned ≠ rightReturned ∧
              leftReturned ≠ state.nextLoan + 1 ∧
              calleeFinal.pending =
                (state.pending.push (state.nextLoan, .loanHole leftReturned)).push
                  (state.nextLoan + 1, .loanHole rightReturned) ∧
              postValue
                #[some (.borrow leftOuter (.loanHole leftReturned)),
                  some (.borrow rightOuter (.loanHole rightReturned)), none, none]
                { activeLoans := #[(⟨0⟩, leftReturned), (⟨1⟩, rightReturned)]
                  loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                    (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
                    (leftReturned, ⟨.local ⟨0⟩, #[.deref], true⟩),
                    (rightReturned, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (.tuple #[.borrow leftReturned (.integer leftValue),
                  .borrow rightReturned (.integer rightValue)]))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeCall handle none
        (nativeFunctionRelation calleeUnit calleeShape calleeBody)
        (valuesCons
          (nativeDerefLocalBorrowOperation
            { location := ⟨⟨0⟩⟩, fields := []
              referenceType, kind := .mutable, lexicalLoan := lexFirst }
            valuesNil)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨1⟩⟩, fields := []
                referenceType, kind := .mutable, lexicalLoan := lexSecond }
              valuesNil)
            valuesNil)))
      #[some (.borrow leftOuter (.integer leftCur)),
        some (.borrow rightOuter (.integer rightCur)), none, none]
      { activeLoans := #[]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] }
      state postValue postThrow := by
  subst zeroFirst oneSecond
  have leftFirst : leftOuter ≠ state.nextLoan := by omega
  have leftSecond : leftOuter ≠ state.nextLoan + 1 := by omega
  have rightFirst : rightOuter ≠ state.nextLoan := by omega
  have rightSecond : rightOuter ≠ state.nextLoan + 1 := by omega
  have freshSeparate : state.nextLoan ≠ state.nextLoan + 1 := by omega
  unfold wpFunction at calleeWp
  intro finalFrame finalState control step
  /- The two minted frames, spelled through the row frame the evaluation
  lemmas produce. -/
  let mintedOne : RuntimeFrame :=
    rowFrame #[some (.borrow leftOuter (.loanHole state.nextLoan)),
        some (.borrow rightOuter (.integer rightCur)), none, none]
      { activeLoans := #[(⟨0⟩, state.nextLoan)]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
  let mintedTwo : RuntimeFrame :=
    rowFrame #[some (.borrow leftOuter (.loanHole state.nextLoan)),
        some (.borrow rightOuter (.loanHole (state.nextLoan + 1))), none, none]
      { activeLoans := #[(⟨0⟩, state.nextLoan), (⟨1⟩, state.nextLoan + 1)]
        loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
          (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
          (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩),
          (state.nextLoan + 1, ⟨.local ⟨1⟩, #[.deref], true⟩)] }
  have firstRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨0⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := 0 }
          valuesNil)
          (rowFrame #[some (.borrow leftOuter (.integer leftCur)),
              some (.borrow rightOuter (.integer rightCur)), none, none]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
          state rF rS c →
        rF = mintedOne ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan (.integer leftCur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      have evaluate := derefLocalBorrow_evaluate_nominalPath referenceType mutableKind 0
        ⟨0⟩ [] leftOuter (.integer leftCur)
        #[some (.borrow leftOuter (.integer leftCur)),
          some (.borrow rightOuter (.integer rightCur)), none, none]
        { activeLoans := #[]
          loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
            (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] } state (by simp [focusValue])
      simp [focusFields, focusValue, focusProjections] at evaluate
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [evaluate] at evaluation <;>
        cases Option.some.inj evaluation
      refine ⟨?_, rfl, rfl⟩
      simp [mintedOne, rowFrame, Array.filter, Array.set!]
  have secondRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨1⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := 1 }
          valuesNil)
          mintedOne { state with nextLoan := state.nextLoan + 1 } rF rS c →
        rF = mintedTwo ∧
        rS = { state with nextLoan := state.nextLoan + 1 + 1 } ∧
        c = .value (.borrow (state.nextLoan + 1) (.integer rightCur)) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      have evaluate := derefLocalBorrow_evaluate_nominalPath referenceType mutableKind 1
        ⟨1⟩ [] rightOuter (.integer rightCur)
        #[some (.borrow leftOuter (.loanHole state.nextLoan)),
          some (.borrow rightOuter (.integer rightCur)), none, none]
        { activeLoans := #[(⟨0⟩, state.nextLoan)]
          loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
            (rightOuter, ⟨.local ⟨1⟩, #[], true⟩),
            (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
        { state with nextLoan := state.nextLoan + 1 } (by simp [focusValue])
      simp [focusFields, focusValue, focusProjections] at evaluate
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [show mintedOne = _ from rfl, evaluate] at evaluation <;>
        cases Option.some.inj evaluation
      refine ⟨?_, rfl, rfl⟩
      simp [mintedTwo, rowFrame, Array.filter, Array.set!,
        show ((⟨0⟩ : ExprId) != (⟨1⟩ : ExprId)) = true from rfl]
  have operandsRun :
      ∀ {result : BigStep.ValuesResult},
        (valuesCons
          (nativeDerefLocalBorrowOperation
            { location := ⟨⟨0⟩⟩, fields := []
              referenceType, kind := .mutable, lexicalLoan := 0 }
            valuesNil)
          (valuesCons
            (nativeDerefLocalBorrowOperation
              { location := ⟨⟨1⟩⟩, fields := []
                referenceType, kind := .mutable, lexicalLoan := 1 }
              valuesNil)
            valuesNil))
          (rowFrame #[some (.borrow leftOuter (.integer leftCur)),
              some (.borrow rightOuter (.integer rightCur)), none, none]
            { activeLoans := #[]
              loanLocations := #[(leftOuter, ⟨.local ⟨0⟩, #[], true⟩),
                (rightOuter, ⟨.local ⟨1⟩, #[], true⟩)] })
          state result →
        result = .values { state with nextLoan := state.nextLoan + 1 + 1 } mintedTwo
          [.borrow state.nextLoan (.integer leftCur),
            .borrow (state.nextLoan + 1) (.integer rightCur)] := by
    rintro result
      (⟨rF, rS, rc, firstStep, abrupt, resultEq⟩ |
        ⟨rF, rS, rv, f, s, vs, firstStep, tailStep, resultEq⟩ |
        ⟨rF, rS, rv, f, s, rc, firstStep, tailStep, resultEq⟩)
    · obtain ⟨-, -, rfl⟩ := firstRun firstStep
      cases abrupt
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · obtain ⟨rfl, rfl, veq'⟩ := secondRun secondStep
        injection veq' with veq'
        subst veq'
        simp only [valuesNil] at nilStep
        injection nilStep with h1 h2 h3
        subst s' f' vs'
        injection tailEq with h4 h5 h6
        subst s f vs
        exact resultEq
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep
    · obtain ⟨rfl, rfl, veq⟩ := firstRun firstStep
      injection veq with veq
      subst veq
      rcases tailStep with
        ⟨tF, tS, tc, secondStep, abrupt, tailEq⟩ |
        ⟨tF, tS, tv, f', s', vs', secondStep, nilStep, tailEq⟩ |
        ⟨tF, tS, tv, f', s', tc, secondStep, nilStep, tailEq⟩
      · obtain ⟨-, -, rfl⟩ := secondRun secondStep
        cases abrupt
      · simp at tailEq
      · obtain ⟨rfl, rfl, -⟩ := secondRun secondStep
        simp [valuesNil] at nilStep
  rcases step with
    (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
      ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
        rfl, rfl, rfl⟩)
  · cases operandsRun operandControl
  · injection operandsRun operandValues with h1 h2 h3
    subst oS oF values
    have applied := calleeWp calleeState outcome (by simpa using calleeStep)
    cases outcome with
    | threw kind thrown => exact applied
    | returned results =>
        obtain ⟨leftReturned, rightReturned, leftValue, rightValue, resultsShape,
          returnedSeparate, leftReturnedFresh, pendingShape, continuation⟩ := applied
        subst resultsShape
        simp only [callControl, callFrame_returned, registerReturnedLoan]
        rw [packResults_singleton]
        rcases calleeState with ⟨globals, globalLoans, nextLoan, pending⟩
        simp only at pendingShape
        subst pendingShape
        rw [show mintedTwo = _ from rfl,
          applyPendingFrom_twoReturnedPair state.pending globals globalLoans nextLoan
            leftOuter rightOuter state.nextLoan (state.nextLoan + 1)
            leftReturned rightReturned freshSeparate leftFirst leftSecond
            rightFirst rightSecond returnedSeparate leftReturnedFresh]
        exact ⟨_, _, rfl, continuation⟩

/-- Binding a returned pair by a tuple pattern of locals 2 and 3 on a
four-local row. -/
theorem bindTuple_rowFrame (fuel : Nat) (row : Row) (registries : Registries)
    (first second : RuntimeValue) (inBounds : 3 < row.size) :
    NativePatternBinder.bind
      ⟨fuel + 2, .tuple [.variable ⟨2⟩, .variable ⟨3⟩]⟩
      (rowFrame row registries) (.tuple #[first, second]) =
    some (rowFrame ((row.set! 2 (some first)).set! 3 (some second)) registries) := by
  have twoInBounds : 2 < row.size := by omega
  simp [NativePatternBinder.bind, bindNativePatternFuel, bindNativePatternRow, rowFrame,
    twoInBounds, inBounds]

/-- Finalizing two scalar mutable parameters beside two cleared returned
slots: each parameter exports its current, in local order. -/
theorem exportFrameLoans_rowFrame_twoIntegersUnits (state : RuntimeState)
    (leftLoan rightLoan : Nat) (left right : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noLeftGlobal : globalLoanKeyIn? state.globalLoans leftLoan = none)
    (noRightGlobal : globalLoanKeyIn? state.globalLoans rightLoan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow leftLoan (.integer left)),
          some (.borrow rightLoan (.integer right)), some .unit, some .unit]
          { activeLoans, loanLocations })
        state
      = { state with
          pending :=
            (state.pending.push (leftLoan, .integer left)).push (rightLoan, .integer right) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst,
    applyWriteBack_empty, globalLoanKey?, noLeftGlobal, noRightGlobal]

/-! ## A returned reborrow projected from a struct parameter

`&mut pair.left`: the loan is minted at the focused field, its hole rests
inside the parameter's struct, and the borrow is the value.  The exit
exports the struct with the hole, and the callee's prophecy resolution
fills that hole from the returned borrow. -/

theorem wpRowThrow_returnedReborrowPath (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable) (lex : Nat)
    (localId : LocalId) (steps : List FocusStep) (outer : Nat) (leaf : RuntimeValue)
    (row : Row) (registries : Registries) (state : RuntimeState)
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (slot : row[localId.index]? = some (some (.borrow outer (focusValue steps leaf))))
    (exit : postValue
      (row.set! localId.index
        (some (.borrow outer (focusValue steps (.loanHole state.nextLoan)))))
      { activeLoans :=
          (registries.activeLoans.filter (·.1 != ⟨lex⟩)).push (⟨lex⟩, state.nextLoan)
        loanLocations := registries.loanLocations.push
          (state.nextLoan, ⟨.local localId, #[.deref] ++ focusProjections steps, true⟩)
        typeInstantiation := registries.typeInstantiation }
      { state with nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan leaf))) :
    wpRowThrow
      (nativeDerefLocalBorrowOperation
        { location := ⟨localId⟩, fields := focusFields steps, referenceType,
          kind := .mutable, lexicalLoan := lex }
        valuesNil)
      row registries state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
  · simp [valuesNil] at nilStep
  · simp only [valuesNil] at nilStep
    injection nilStep with h1 h2 h3
    subst oS oF values
    rcases evaluated with ⟨rv, evaluation, rfl⟩ |
      ⟨kind, thrown, evaluation, rfl⟩ <;>
      rw [derefLocalBorrow_evaluate_nominalPath referenceType mutableKind lex
        localId steps outer leaf row registries state slot] at evaluation <;>
      cases Option.some.inj evaluation
    exact ⟨_, _, rfl, exit⟩

/-- Finalizing a parameter whose focused field was lent to the returned
reborrow, at row level: the lender exports its struct with the hole. -/
theorem exportFrameLoans_rowFrame_returnedProjected (state : RuntimeState)
    (outer returned : Nat) (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (separate : outer ≠ returned)
    (noGlobal : globalLoanKeyIn? state.globalLoans outer = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow outer (focusValue steps (.loanHole returned)))]
          { activeLoans, loanLocations })
        state
      = { state with
          pending := state.pending.push
            (outer, focusValue steps (.loanHole returned)) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  have returnedSeparate : returned ≠ outer := Ne.symm separate
  obtain ⟨holeMark, -, -, -, -, -⟩ := focus_walks plainSteps
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows, outermostBorrows,
    borrowEntry?, collectPruned, holeInFrame, holeWithin, findFirst, holeMark,
    applyWriteBack_empty, globalLoanKey?, noGlobal, returnedSeparate]

/-! ## A projected returned reborrow, consumed

The callee's export is its struct with the returned loan's hole at the
focus.  The transfer is path-free — the hole names the loan — so the
caller's write-back installs the struct, retargets the lexical loan, and
keeps the returned loan's registered location; the write through the
returned borrow, its settling marker, and the export walk the focus. -/

theorem transferredLoan?_focusHole (returned : Nat) (steps : List FocusStep)
    (plainSteps : PlainSteps steps) :
    transferredLoan? (focusValue steps (.loanHole returned)) = some returned := by
  obtain ⟨-, anyHole, -, -, -, -⟩ := focus_walks plainSteps
  simp [transferredLoan?, anyHole, findFirst]

theorem transferActiveLoan_focusHole (lex loan returned : Nat) (steps : List FocusStep)
    (plainSteps : PlainSteps steps) :
    transferActiveLoan #[(⟨lex⟩, loan)] loan (focusValue steps (.loanHole returned)) =
      #[(⟨lex⟩, returned)] := by
  simp [transferActiveLoan, transferredLoan?_focusHole returned steps plainSteps,
    Array.filter]

theorem transferLoanLocation_focusHole (outer loan returned : Nat)
    (place projected : RuntimePlace) (steps : List FocusStep)
    (plainSteps : PlainSteps steps) (separate : outer ≠ loan) :
    transferLoanLocation #[(outer, place), (loan, projected)] loan
        (focusValue steps (.loanHole returned)) =
      #[(outer, place), (returned, projected)] := by
  simp [transferLoanLocation, transferredLoan?_focusHole returned steps plainSteps,
    Array.filter, separate, Array.findRev?]

/-- The value of a call whose callee returns a reborrow projected from the
argument reborrow: the callee's export is its struct with the returned
loan's hole at the focus, which the caller's write-back installs into its
parameter, retargeting the lexical loan to the returned one. -/
theorem wpRowThrow_callReturnedProjected
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat)
    (referenceType : ReferenceType)
    (mutableKind : referenceType.kind = .mutable)
    {outer : Nat} {cur : RuntimeValue} {state : RuntimeState}
    {rest : List (Option RuntimeValue)}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (priorLoan : outer < state.nextLoan)
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      { state with nextLoan := state.nextLoan + 1 }
      #[.borrow state.nextLoan cur]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            /- The focus is the callee's: its summary names the siblings
            around the hole. -/
            ∃ (returned : Nat) (value : Int) (steps : List FocusStep),
              PlainSteps steps ∧
              results = #[.borrow returned (.integer value)] ∧
              calleeFinal.pending =
                state.pending.push
                  (state.nextLoan, focusValue steps (.loanHole returned)) ∧
              postValue
                (some (.borrow outer (focusValue steps (.loanHole returned))) ::
                  rest).toArray
                { activeLoans := #[(⟨lex⟩, returned)]
                  loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
                    (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (.borrow returned (.integer value)))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeCall handle none
        (nativeFunctionRelation calleeUnit calleeShape calleeBody)
        (valuesCons
          (nativeDerefLocalBorrowOperation
            { location := ⟨⟨0⟩⟩, fields := []
              referenceType, kind := .mutable, lexicalLoan := lex }
            valuesNil)
          valuesNil))
      (some (.borrow outer cur) :: rest).toArray
      { activeLoans := #[]
        loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] }
      state postValue postThrow := by
  have separate : outer ≠ state.nextLoan := Nat.ne_of_lt priorLoan
  unfold wpFunction at calleeWp
  have reborrowRun :
      ∀ {rF : RuntimeFrame} {rS : RuntimeState} {c : Control},
        (nativeDerefLocalBorrowOperation
          { location := ⟨⟨0⟩⟩, fields := []
            referenceType, kind := .mutable, lexicalLoan := lex }
          valuesNil)
          (rowFrame (some (.borrow outer cur) :: rest).toArray
            { activeLoans := #[]
              loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩)] })
          state rF rS c →
        rF = { locals := (some (.borrow outer (.loanHole state.nextLoan)) :: rest).toArray
               activeLoans := #[(⟨lex⟩, state.nextLoan)]
               loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
                 (state.nextLoan, ⟨.local ⟨0⟩, #[.deref], true⟩)] } ∧
        rS = { state with nextLoan := state.nextLoan + 1 } ∧
        c = .value (.borrow state.nextLoan cur) := by
    rintro rF rS c
      (⟨oF, oS, propagated, nilStep, -⟩ | ⟨oF, oS, values, nilStep, evaluated⟩)
    · simp [valuesNil] at nilStep
    · simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [derefLocalBorrow_evaluate_singleBorrow referenceType
          mutableKind lex outer cur state rest] at evaluation <;>
        cases Option.some.inj evaluation
      exact ⟨rfl, rfl, rfl⟩
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
      ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
        rfl, rfl, rfl⟩)
  · rcases operandControl with
      ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
    · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
      cases abrupt
    · simp [valuesNil] at nilStep
      obtain ⟨rfl, rfl, rfl⟩ := nilStep
      simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandValues with
      ⟨rF, rS, rc, reborrowStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, reborrowStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, reborrowStep, nilStep, resultEq⟩
    · obtain ⟨-, -, rfl⟩ := reborrowRun reborrowStep
      cases abrupt
    · obtain ⟨rfl, rfl, veq⟩ := reborrowRun reborrowStep
      injection veq with veq
      subst veq
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      have applied := calleeWp calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown => exact applied
      | returned results =>
          obtain ⟨returned, value, steps, plainSteps, rfl, pendingShape, continuation⟩ :=
            applied
          have packOne : packResults #[.borrow returned (.integer value)] =
              .borrow returned (.integer value) := rfl
          simp only [callControl, packOne, callFrame_returned,
            registerReturnedLoan]
          rw [applyPendingFrom_single (inherited := state.pending)
            (loan := state.nextLoan)
            (current := focusValue steps (.loanHole returned)) pendingShape]
          rw [applyPendingWriteBack_derefLocalZero,
            transferActiveLoan_focusHole _ _ _ _ plainSteps,
            transferLoanLocation_focusHole _ _ _ _ _ _ plainSteps separate]
          exact ⟨_, ⟨#[(⟨lex⟩, returned)],
            #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
              (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)], #[]⟩, rfl, continuation⟩
    · obtain ⟨rfl, rfl, -⟩ := reborrowRun reborrowStep
      simp [valuesNil] at nilStep

/-- Writing through the returned reborrow in local 1 while its hole rests
at the focus of the lender in local 0. -/
theorem mutate_evaluate_returnedProjectedLocal1 (state : RuntimeState)
    (outer returned : Nat) (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (current written : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) (separate : outer ≠ returned) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow returned current, written]
      (rowFrame #[some (.borrow outer (focusValue steps (.loanHole returned))),
          some (.borrow returned current)]
        { activeLoans
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)] })
      state =
    some (.value
      (rowFrame #[some (.borrow outer (focusValue steps (.loanHole returned))),
          some (.borrow returned written)]
        { activeLoans
          loanLocations := #[(outer, ⟨.local ⟨0⟩, #[], true⟩),
            (returned, ⟨.local ⟨0⟩, #[.deref], true⟩)] })
      state .unit) := by
  obtain ⟨-, -, -, -, borrowRewrite, -⟩ := focus_walks plainSteps
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, readRuntimePlace?, readRoot?, readLocal?,
    readProjections?, borrowRewrite, rewriteFirst, writeRuntimePlace?, writeRoot?,
    separate]

/-- The caller's own death marker settling the returned loan into the
focus of its lender. -/
theorem endLoan_evaluate_returnedProjected_nil (state : RuntimeState)
    (outer returned : Nat) (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (current : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace)) (separate : outer ≠ returned) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[]
      (rowFrame #[some (.borrow outer (focusValue steps (.loanHole returned))),
          some (.borrow returned current)]
        { activeLoans := #[(⟨0⟩, returned)], loanLocations })
      state =
    some (.value
      (rowFrame #[some (.borrow outer (focusValue steps current)), some .unit]
        { activeLoans := #[], loanLocations })
      state .unit) := by
  obtain ⟨holeMark, -, borrowCurrent, holeFill, -, borrowClear⟩ :=
    focus_walks plainSteps
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by
    decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by
    decide
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    separate, siteSelf, siteNotDifferent, holeMark, borrowCurrent, holeFill, borrowClear]

/-- The same marker wrapping a value: the value passes through. -/
theorem endLoan_evaluate_returnedProjected_value (state : RuntimeState)
    (outer returned : Nat) (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (current argument : RuntimeValue)
    (loanLocations : Array (Nat × RuntimePlace)) (separate : outer ≠ returned) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[argument]
      (rowFrame #[some (.borrow outer (focusValue steps (.loanHole returned))),
          some (.borrow returned current)]
        { activeLoans := #[(⟨0⟩, returned)], loanLocations })
      state =
    some (.value
      (rowFrame #[some (.borrow outer (focusValue steps current)), some .unit]
        { activeLoans := #[], loanLocations })
      state argument) := by
  obtain ⟨holeMark, -, borrowCurrent, holeFill, -, borrowClear⟩ :=
    focus_walks plainSteps
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by
    decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by
    decide
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    separate, siteSelf, siteNotDifferent, holeMark, borrowCurrent, holeFill, borrowClear]

/-- Finalizing the lender once the write reached its focus, beside the
cleared returned slot. -/
theorem exportFrameLoans_rowFrame_focusedUnit (state : RuntimeState)
    (loan : Nat) (steps : List FocusStep) (plainSteps : PlainSteps steps) (value : Int)
    (activeLoans : Array (ExprId × Nat))
    (loanLocations : Array (Nat × RuntimePlace))
    (noGlobal : globalLoanKeyIn? state.globalLoans loan = none) :
    exportFrameLoans
        (rowFrame #[some (.borrow loan (focusValue steps (.integer value))), some .unit]
          { activeLoans, loanLocations })
        state
      = { state with pending := state.pending.push (loan, focusValue steps (.integer value)) } := by
  rcases state with ⟨globals, globalLoans, nextLoan, inherited⟩
  obtain ⟨holeMark, -, -, -, -, -⟩ := focus_walks plainSteps
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, holeMark, applyWriteBack_empty_export, globalLoanKey?, noGlobal]

/-! ## A returned reborrow projected from a global borrow

`&mut Pair[address].left`: the body borrows the resource into a local and
returns a reborrow of one of its fields.  The global borrow as a `let`
initializer is its own step, the reborrow is the path law at local 1, and
the exit writes the resource back with the returned loan's hole at the
focus, the storage key transferring to the returned loan. -/

/-- A mutable global borrow of a present resource, its key read from
local 0: the resource is lifted into the returned borrow, and storage
holds the outer loan's hole. -/
theorem wpRowThrow_globalBorrowLocal0 (namespaceId : NamespaceId) (typeId : TypeId)
    (borrowType : ReferenceType) (borrowMutable : borrowType.kind = .mutable)
    (lex : Nat) {address : String} {resource : RuntimeValue}
    {rest : List (Option RuntimeValue)}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (present : state.globals.lookup
      (globalKey namespaceId typeId (.address address)) = some resource)
    (exit : postValue (some (.address address) :: rest).toArray
      { activeLoans := #[(⟨lex⟩, state.nextLoan)]
        loanLocations :=
          #[(state.nextLoan, { root := .global (globalKey namespaceId typeId (.address address)) })]
        typeInstantiation := #[] }
      { state with
        globals := state.globals.insert
          (globalKey namespaceId typeId (.address address)) (.loanHole state.nextLoan)
        globalLoans :=
          (state.nextLoan, globalKey namespaceId typeId (.address address))
            :: state.globalLoans
        nextLoan := state.nextLoan + 1 }
      (.value (.borrow state.nextLoan resource))) :
    wpRowThrow
      (nativeGlobalOperation
        (GlobalLocationOperation.borrow
          { resource := ⟨namespaceId, typeId⟩, referenceType := borrowType,
            kind := .mutable, lexicalLoan := lex })
        (valuesCons (localVar ⟨0⟩) valuesNil))
      (some (.address address) :: rest).toArray
      { activeLoans := #[], loanLocations := #[], typeInstantiation := #[] }
      state postValue postThrow := by
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandStep, frameEq, stateEq, controlEq⟩ |
      ⟨oF, oS, values, operandStep, evaluated⟩)
  · rcases operandStep with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandStep with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
      subst rF rS
      simp only [readLocal?_rowFrame, List.getElem?_toArray,
        List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
        at readEq
      subst readEq
      injection valueEq with valueEq
      subst rv
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      rcases evaluated with ⟨rv, evaluation, rfl⟩ |
        ⟨kind, thrown, evaluation, rfl⟩ <;>
        rw [globalBorrow_evaluate_rowFrame namespaceId typeId borrowType
          borrowMutable lex address rest
          { activeLoans := #[], loanLocations := #[], typeInstantiation := #[] }
          state _ (by simpa only [instantiatedTypeId_empty] using present)]
          at evaluation <;>
        simp only [instantiatedTypeId_empty] at evaluation <;>
        cases Option.some.inj evaluation
      refine ⟨_, _, ?_, exit⟩
      simp [rowFrame, Array.filter]
    · simp [valuesNil] at nilStep

/-- Finalizing the local holding the resource, its focus lent to the
returned reborrow: storage takes the resource with the hole, and the key
transfers from the outer loan to the returned one. -/
theorem exportFrameLoans_rowFrame_returnedGlobal {globals : GlobalMap}
    {globalLoans : List (Nat × GlobalKey)} {nextLoan : Nat}
    {inherited : Array (Nat × RuntimeValue)}
    {key : GlobalKey} {outer returned : Nat} {address : String}
    {steps : List FocusStep} (plainSteps : PlainSteps steps)
    {activeLoans : Array (ExprId × Nat)}
    {loanLocations : Array (Nat × RuntimePlace)}
    (separate : outer ≠ returned)
    (fresh : globalLoanKeyIn? globalLoans outer = none) :
    exportFrameLoans
        (rowFrame #[some (.address address),
            some (.borrow outer (focusValue steps (.loanHole returned)))]
          { activeLoans, loanLocations })
        { globals := globals.insert key (.loanHole outer)
          globalLoans := (outer, key) :: globalLoans
          nextLoan
          pending := inherited }
      = { globals := (globals.insert key (.loanHole outer)).insert key
            (focusValue steps (.loanHole returned))
          globalLoans := (returned, key) :: globalLoans
          nextLoan
          pending := inherited } := by
  have returnedSeparate : returned ≠ outer := Ne.symm separate
  obtain ⟨holeMark, anyHole, -, -, -, -⟩ := focus_walks plainSteps
  have removed := removeGlobalLoan_of_free globalLoans outer fresh
  simp [rowFrame, exportFrameLoans, exportSettledLoans, frameBorrows,
    outermostBorrows, borrowEntry?, collectPruned, holeInFrame, holeWithin,
    findFirst, holeMark, anyHole, applyWriteBack, fillVisibleHole, globalLoanKey?,
    globalLoanKeyIn?, transferGlobalLoan, transferredLoan?, removeGlobalLoan,
    fillHole?, rewriteFirst, returnedSeparate, removed]

/-! ## A returned global reborrow, consumed

The callee's summary states its storage transfer: the resource rests at
its key with the returned loan's hole at the focus, and the key is
registered to that loan.  The call exports nothing and registers the
returned loan under the caller's lexical loan; the write through it finds
the borrow by scanning the locals, having no registered location; the
settling marker fills the hole in storage and retires the key. -/

/-- The value of a call whose callee returns a reborrow into storage. -/
theorem wpRowThrow_callReturnedGlobal
    {calleeUnit : Validation.ExecutableUnit} {calleeShape : FunctionShape}
    {calleeBody : ExprDenotation}
    (handle : FunctionHandle) (lex : Nat) {argument : RuntimeValue}
    {rest : List (Option RuntimeValue)}
    {state : RuntimeState}
    {postValue : Row → Registries → RuntimeState → Control → Prop}
    {postThrow : ThrowKind → Array RuntimeValue → Prop}
    (calleeWp : wpFunction
      (nativeFunctionRelation calleeUnit calleeShape calleeBody)
      state #[argument]
      (fun calleeFinal outcome =>
        match outcome with
        | .returned results =>
            ∃ (returned : Nat) (value : Int),
              results = #[.borrow returned (.integer value)] ∧
              calleeFinal.pending = state.pending ∧
              postValue (some argument :: rest).toArray
                { activeLoans := #[(⟨lex⟩, returned)], loanLocations := #[] }
                { globals := calleeFinal.globals
                  globalLoans := calleeFinal.globalLoans
                  nextLoan := calleeFinal.nextLoan
                  pending := state.pending }
                (.value (.borrow returned (.integer value)))
        | .threw kind thrown => postThrow kind thrown)) :
    wpRowThrow
      (nativeCall handle (some lex)
        (nativeFunctionRelation calleeUnit calleeShape calleeBody)
        (valuesCons (localVar ⟨0⟩) valuesNil))
      (some argument :: rest).toArray { activeLoans := #[], loanLocations := #[] }
      state postValue postThrow := by
  unfold wpFunction at calleeWp
  rintro finalFrame finalState control
    (⟨oF, oS, propagated, operandControl, rfl, rfl, rfl⟩ |
      ⟨oF, oS, values, calleeState, outcome, operandValues, calleeStep,
        rfl, rfl, rfl⟩)
  · rcases operandControl with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · simp at resultEq
    · simp [valuesNil] at nilStep
  · rcases operandValues with
      ⟨rF, rS, rc, readStep, abrupt, resultEq⟩ |
      ⟨rF, rS, rv, f, s, vs, readStep, nilStep, resultEq⟩ |
      ⟨rF, rS, rv, f, s, rc, readStep, nilStep, resultEq⟩
    · obtain ⟨readValue, readEq, frameEq', stateEq', controlEq'⟩ := readStep
      subst rc
      cases abrupt
    · obtain ⟨readValue, readEq, frameEq', stateEq', valueEq⟩ := readStep
      subst rF rS
      simp only [readLocal?_rowFrame, List.getElem?_toArray,
        List.getElem?_cons_zero, Option.join_some, Option.some.injEq]
        at readEq
      subst readEq
      injection valueEq with valueEq
      subst rv
      simp only [valuesNil] at nilStep
      injection nilStep with h1 h2 h3
      subst s f vs
      injection resultEq with h4 h5 h6
      subst oS oF values
      have applied := calleeWp calleeState outcome (by simpa using calleeStep)
      cases outcome with
      | threw kind thrown => exact applied
      | returned results =>
          obtain ⟨returned, value, rfl, pendingEq, continuation⟩ := applied
          have packOne : packResults #[.borrow returned (.integer value)] =
              .borrow returned (.integer value) := rfl
          simp only [callControl, packOne, callFrame_returned,
            registerReturnedLoan_some_singleBorrow]
          rw [applyPendingFrom_none pendingEq]
          refine ⟨_, _, ?_, continuation⟩
          simp [rowFrame, Array.filter]
    · simp [valuesNil] at nilStep

/-- Writing through the returned global reborrow in local 1: its loan has
no registered location, so the write finds the borrow by scanning. -/
theorem mutate_evaluate_returnedGlobalLocal1 (state : RuntimeState)
    (returned : Nat) (address : String) (current written : RuntimeValue)
    (activeLoans : Array (ExprId × Nat)) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow returned current, written]
      (rowFrame #[some (.address address), some (.borrow returned current)]
        { activeLoans, loanLocations := #[] })
      state =
    some (.value
      (rowFrame #[some (.address address), some (.borrow returned written)]
        { activeLoans, loanLocations := #[] })
      state .unit) := by
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator,
    mutateBorrow?, rowFrame, updateBorrowValue?, updateLocalBorrowValue?,
    localLoanPlace?, rewriteFirst]

/-- The caller's marker settling the returned global loan: the borrow's
current fills the hole at the focus of the resource in storage, and the
key retires from the registry. -/
theorem endLoan_evaluate_returnedGlobal (globals : GlobalMap)
    (rest : List (Nat × GlobalKey)) (nextLoan : Nat)
    (pending : Array (Nat × RuntimeValue)) (key : GlobalKey)
    (address : String) (returned : Nat) (value : Int)
    (steps : List FocusStep) (plainSteps : PlainSteps steps)
    (stored : globals.lookup key = some (focusValue steps (.loanHole returned)))
    (argument : RuntimeValue) :
    (ReferenceLocationOperation.endLoan #[⟨0⟩]).evaluate? #[argument]
      (rowFrame #[some (.address address), some (.borrow returned (.integer value))]
        { activeLoans := #[(⟨0⟩, returned)], loanLocations := #[] })
      { globals, globalLoans := (returned, key) :: rest, nextLoan, pending } =
    some (.value
      (rowFrame #[some (.address address), some .unit]
        { activeLoans := #[], loanLocations := #[] })
      { globals := globals.insert key (focusValue steps (.integer value))
        globalLoans := rest
        nextLoan
        pending }
      argument) := by
  have siteSelf : ((⟨0⟩ : ExprId) == (⟨0⟩ : ExprId)) = true := by decide
  have siteNotDifferent : ((⟨0⟩ : ExprId) != (⟨0⟩ : ExprId)) = false := by decide
  obtain ⟨holeMark, anyHole, -, holeFill, -, -⟩ := focus_walks plainSteps
  simp only [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, rowFrame]
  simp [endLoans?, findBorrowValue?, findFirst,
    clearBorrowValue, rewriteFirst,
    applyWriteBack, fillVisibleHole, holeInFrame, holeWithin, fillHole?, Array.filter,
    globalLoanKey?, globalLoanKeyIn?, stored, holeMark, anyHole, holeFill,
    transferGlobalLoan, transferredLoan?, removeGlobalLoan,
    siteSelf, siteNotDifferent]

/-! ## Taking a resource out of storage

`move_from<T>(addr)`: the resource leaves its key and is the value; an
absent resource aborts.  Destructuring it into its fields binds each
field's local. -/

theorem globalTake_evaluate_present (namespaceId : NamespaceId) (typeId : TypeId)
    (address : String) (frame : RuntimeFrame) (state : RuntimeState)
    (resource : RuntimeValue)
    (present : state.globals.lookup
      (globalKey namespaceId (instantiatedTypeId frame.typeInstantiation typeId)
        (.address address)) = some resource) :
    (GlobalLocationOperation.take ⟨namespaceId, typeId⟩).evaluate?
      #[.address address] frame state =
    some (.value frame
      { state with
        globals := state.globals.erase
          (globalKey namespaceId (instantiatedTypeId frame.typeInstantiation typeId)
            (.address address)) }
      resource) := by
  simp [GlobalLocationOperation.evaluate?, takeGlobalAt?, globalValue?, present,
    RuntimeValue.storageKey?]

theorem globalTake_evaluate_absent (namespaceId : NamespaceId) (typeId : TypeId)
    (address : String) (frame : RuntimeFrame) (state : RuntimeState)
    (absent : state.globals.lookup
      (globalKey namespaceId (instantiatedTypeId frame.typeInstantiation typeId)
        (.address address)) = none) :
    (GlobalLocationOperation.take ⟨namespaceId, typeId⟩).evaluate?
      #[.address address] frame state =
    some (.throw_ frame state .abort) := by
  simp [GlobalLocationOperation.evaluate?, takeGlobalAt?, globalValue?, absent,
    RuntimeValue.storageKey?]

/-- Destructuring a one-field struct into local 1. -/
theorem bindConstructorOne_rowFrame (fuel : Nat) (source : StructHandle)
    (row : Row) (registries : Registries) (value : RuntimeValue)
    (inBounds : 1 < row.size) :
    NativePatternBinder.bind
      ⟨fuel + 2, .constructor source none [.variable ⟨1⟩]⟩
      (rowFrame row registries) (.nominal source none #[value]) =
    some (rowFrame (row.set! 1 (some value)) registries) := by
  simp [NativePatternBinder.bind, bindNativePatternFuel, bindNativePatternRow, rowFrame,
    inBounds]

end LeanerIR.Proofs.Denotation
