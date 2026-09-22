-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Normalize
import LeanerIR.Proofs.NativeStore

/-!
# Compositional operation rules

Apply an operation's certified state update before inspecting its continuation.
Focus paths describe the selected value, never the layout of a function body.
The semantic proof is shared with the row route; the normalizer only supplies
the current slot and path certificates.
-/

namespace LeanerIR.Proofs.Denotation.RowSpec

open LeanerIR.SemanticOperations Lean Meta

attribute [lir_eval] focusValue focusFields focusProjections FocusStep.fill FocusStep.index
  FocusStep.fieldStep
  List.reverse_cons List.reverse_nil List.find?_cons List.find?_nil
  List.findIdx?_cons List.findIdx?_nil

/-- Lookup a literal registry with list equations, without opening the
array iterator's control state. The payloads remain opaque. -/
@[lir_eval high] theorem findRev_literal {α : Type} (p : α → Bool) (xs : List α) :
    xs.toArray.findRev? p = xs.reverse.find? p := by
  rw [Array.findRev?_eq_find?_reverse, List.reverse_toArray, ← Array.find?_toList]

theorem wp_evaluate_value {evaluator : NativeEvaluator} {values : Array RuntimeValue}
    {s : RowState} {frame : RuntimeFrame} {state : RuntimeState} {value : RuntimeValue}
    (evaluated : evaluator values s.frame s.state = some (.value frame state value))
    (post : Control → RowState → Prop) (aborts : Failure → Prop) :
    wp (evaluate evaluator values) post aborts s =
      post (.value value) (RowState.ofFrame frame state) := by
  apply propext
  rw [wp_evaluate, evaluated]

/-- A same-owner variant test observes only the tag; the payload and state
remain opaque. This is the evaluator's own equation, shared by all bodies. -/
@[lir_eval high] theorem evaluate_variant_test_nominal
    (source : StructHandle) (variants : Array String) (variant : Option String)
    (fields : Array RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    NominalVariantTest.evaluate? ⟨source, variants⟩
      #[.nominal source variant fields] frame state =
      some (.value frame state (.bool (variant.any variants.contains))) := by
  simp [NominalVariantTest.evaluate?, liftConstructorEvaluator, testNominalVariants?]

@[lir_eval high] theorem evaluate_variant_field_nominal
    (source : StructHandle) (choices : Array (String × Nat)) (variant : String)
    (fields : Array RuntimeValue) (frame : RuntimeFrame) (state : RuntimeState) :
    NominalVariantFieldLocation.evaluateSelect? ⟨source, choices⟩
      #[.nominal source (some variant) fields] frame state =
      match choices.find? (fun choice => choice.1 == variant) with
      | some (_, index) => (fields[index]?).map (GlobalOperationResult.value frame state)
      | none => none := by
  simp only [NominalVariantFieldLocation.evaluateSelect?, liftConstructorEvaluator,
    selectNominalVariantFieldAt?, bne_self_eq_false, Bool.false_eq_true, ite_false]
  cases choices.find? (fun choice => choice.1 == variant) with
  | none => simp
  | some choice =>
    rcases choice with ⟨name, index⟩
    cases selected : fields[index]? <;> simp [selected]

@[lir_wp_norm high] theorem wp_evaluate_variant_test
    (source : StructHandle) (variants : Array String) (variant : Option String)
    (fields : Array RuntimeValue) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) (state : RowState) :
    wp (evaluate (NominalVariantTest.evaluate? ⟨source, variants⟩)
      #[.nominal source variant fields]) post aborts state ↔
      post (.value (.bool (variant.any variants.contains))) state := by
  rw [wp_evaluate]
  simp [NominalVariantTest.evaluate?, liftConstructorEvaluator, testNominalVariants?]

/-- Payload selection preserves the row and state. Reduce the choice and
index before visiting the continuation, including the partial failure cases. -/
@[lir_wp_norm high] theorem wp_evaluate_variant_field
    (source : StructHandle) (choices : Array (String × Nat)) (variant : String)
    (fields : Array RuntimeValue) (post : Control → RowState → Prop)
    (aborts : Failure → Prop) (state : RowState) :
    wp (evaluate (NominalVariantFieldLocation.evaluateSelect? ⟨source, choices⟩)
      #[.nominal source (some variant) fields]) post aborts state ↔
      match choices.find? (fun choice => choice.1 == variant) with
      | some (_, index) => match fields[index]? with
        | some value => post (.value value) state
        | none => True
      | none => True := by
  rw [wp_evaluate]
  simp only [NominalVariantFieldLocation.evaluateSelect?, liftConstructorEvaluator,
    selectNominalVariantFieldAt?, bne_self_eq_false, Bool.false_eq_true,
    ite_false]
  cases found : choices.find? (fun choice => choice.1 == variant) with
  | none => simp
  | some choice =>
    rcases choice with ⟨name, index⟩
    cases selected : fields[index]? <;> simp [selected, RowState.ofFrame, RowState.frame, rowFrame]

/-- A successful cached read certifies the slot bound as well as the value. -/
theorem readLocal_inBounds {frame : RuntimeFrame} {slot : LocalId} {value : RuntimeValue}
    (read : readLocal? frame slot = some value) :
    slot.index < frame.locals.size := by
  by_cases bound : slot.index < frame.locals.size
  · exact bound
  · simp [readLocal?, getElem?_neg frame.locals slot.index bound] at read

/-- A successful joined read identifies the occupied array entry. -/
theorem readLocal_entry {frame : RuntimeFrame} {slot : LocalId} {value : RuntimeValue}
    (read : readLocal? frame slot = some value) :
    frame.locals[slot.index]? = some (some value) := by
  change frame.locals[slot.index]?.join = _ at read
  cases h : frame.locals[slot.index]? with
  | none => simp [h] at read
  | some stored =>
      simp only [h, Option.join_some] at read
      exact congrArg some read

/-- Mutate a registered resting borrow without searching unrelated locals.
Both the cached address and the loan at that address are checked. -/
theorem evaluate_mutate_local (s : RowState) (slot : LocalId) (loan : Nat)
    (current argument replacement : RuntimeValue)
    (location : localLoanPlace? s.frame loan = some ⟨.local slot, #[], true⟩)
    (read : readLocal? s.frame slot = some (.borrow loan current)) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow loan argument, replacement]
      s.frame s.state =
      some (.value { s.frame with
        locals := s.frame.locals.set! slot.index (some (.borrow loan replacement)) }
        s.state .unit) := by
  have bound := readLocal_inBounds read
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, mutateBorrow?,
    updateBorrowValue?, updateLocalBorrowValue?, location, readRuntimePlace?, readRoot?,
    read, readProjections?, rewriteFirst_borrow, borrowRewrite?,
    writeRuntimePlace?, writeRoot?, Nat.not_le.mpr bound]

/-- Mutate a resting local even when its cached address still points to the
lender's hole. The operation law is independent of the function's layout. -/
theorem evaluate_mutate_found (s : RowState) (slot : LocalId) (loan : Nat)
    (current argument replacement : RuntimeValue)
    (cached : updateLocalBorrowValue? s.frame s.state loan replacement = none)
    (found : s.frame.locals.toList.findIdx? (fun value =>
      (value.bind (rewriteFirst (borrowRewrite? loan replacement))).isSome) = some slot.index)
    (read : readLocal? s.frame slot = some (.borrow loan current)) :
    ReferenceLocationOperation.mutate.evaluate? #[.borrow loan argument, replacement]
      s.frame s.state =
      some (.value { s.frame with
        locals := s.frame.locals.set! slot.index (some (.borrow loan replacement)) }
        s.state .unit) := by
  have slotValue : s.frame.locals[slot.index]? = some (some (.borrow loan current)) := by
    change s.frame.locals[slot.index]?.join = _ at read
    cases h : s.frame.locals[slot.index]? with
    | none => simp [h] at read
    | some value =>
      simp only [h, Option.join_some] at read
      exact congrArg some read
  simp [ReferenceLocationOperation.evaluate?, liftPlaceEvaluator, mutateBorrow?,
    updateBorrowValue_uncached_local s.frame s.state slot loan current replacement
      cached found slotValue]

/-- A mutable reborrow with any nominal focus and any surrounding row. -/
theorem evaluate_borrow_focus (referenceType : ReferenceType)
    (kind : referenceType.kind = .mutable) (lexical : Nat) (slot : LocalId)
    (steps : List FocusStep) (outer : Nat) (leaf : RuntimeValue)
    (s : RowState)
    (read : readLocal? s.frame slot = some (.borrow outer (focusValue steps leaf))) :
    (DerefLocalBorrowOperation.evaluate?
      { location := ⟨slot⟩, fields := focusFields steps, referenceType,
        kind := .mutable, lexicalLoan := lexical }) #[] s.frame s.state =
      some (.value
        (rowFrame (s.row.set! slot.index
          (some (.borrow outer (focusValue steps (.loanHole s.state.nextLoan)))))
          { s.registries with
            activeLoans := (s.registries.activeLoans.filter (·.1 != ⟨lexical⟩)).push
              (⟨lexical⟩, s.state.nextLoan)
            loanLocations := s.registries.loanLocations.push
              (s.state.nextLoan, ⟨.local slot, #[.deref] ++ focusProjections steps, true⟩) })
        { s.state with nextLoan := s.state.nextLoan + 1 }
        (.borrow s.state.nextLoan leaf)) := by
  have slotValue : s.row[slot.index]? = some (some (.borrow outer (focusValue steps leaf))) := by
    change s.row[slot.index]?.join = _ at read
    cases h : s.row[slot.index]? with
    | none => simp [h] at read
    | some value =>
      simp only [h, Option.join_some] at read
      exact congrArg some read
  exact derefLocalBorrow_evaluate_nominalPath referenceType kind lexical slot steps outer leaf
    s.row s.registries s.state slotValue

/-- Shared field borrows are observations: no loan is minted and no
registry or payload changes. The same nominal focus serves all row shapes. -/
theorem evaluate_shared_borrow_focus (referenceType : ReferenceType)
    (kind : referenceType.kind = .shared) (lexical : Nat) (slot : LocalId)
    (steps : List FocusStep) (outer : Nat) (leaf : RuntimeValue) (s : RowState)
    (read : readLocal? s.frame slot = some (.borrow outer (focusValue steps leaf))) :
    (DerefLocalBorrowOperation.evaluate?
      { location := ⟨slot⟩, fields := focusFields steps, referenceType,
        kind := .immutable, lexicalLoan := lexical }) #[] s.frame s.state =
      some (.value s.frame s.state leaf) := by
  have rootRead : readRuntimePlace? s.frame s.state ⟨.local slot, #[.deref], true⟩ =
      some (focusValue steps leaf) := by
    simp [readRuntimePlace?, readRoot?, read, readProjections?]
  have resolved := resolveNominalFieldSteps?_focus s.frame s.state steps leaf
    ⟨.local slot, #[.deref], true⟩ rootRead
  have slotBound := readLocal_inBounds read
  simp [DerefLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    DerefLocalBorrowOperation.resolve?, resolveDerefLocalFieldPath?,
    slotBound, read, resolved, borrowRuntimePlaceAt?, kind,
    readRuntimePlace?, readRoot?, readProjections?, readProjections?_focusValue,
    show (ReferenceKind.shared != ReferenceKind.shared) = false from rfl]

private def focusCertificate? (operation state : Lean.Expr) : SimpM (Option Lean.Expr) := do
  let operation ← whnf operation
  unless operation.isAppOfArity ``DerefLocalBorrowOperation.mk 5 do return none
  let shared := (operation.getArg! 3).isConstOf ``BorrowKind.immutable
  unless shared || (operation.getArg! 3).isConstOf ``BorrowKind.mutable do return none
  let some fields := literalList? (operation.getArg! 1) | return none
  let slot ← whnf (← mkAppM ``LocalLocation.localId #[operation.getArg! 0])
  let frame ← mkAppM ``RowState.frame #[state]
  let read ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, slot])
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let borrowed := read.expr.getArg! 1
  unless borrowed.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let mut leaf := borrowed.getArg! 1
  let mut steps := #[]
  for field in fields do
    let field ← whnf field
    unless field.isAppOfArity ``NominalFieldStep.mk 3 do return none
    unless leaf.isAppOfArity ``RuntimeValue.nominal 3 do return none
    unless ← isDefEq (field.getArg! 0) (leaf.getArg! 0) do return none
    unless ← isDefEq (field.getArg! 1) (leaf.getArg! 1) do return none
    let array := leaf.getArg! 2
    unless array.isAppOfArity ``Array.mk 2 || array.isAppOfArity ``List.toArray 2 do return none
    let some elements := literalList? (array.getArg! 1) | return none
    let some index ← (evalNat (field.getArg! 2)).run | return none
    unless index < elements.size do return none
    let before ← mkArrayLit (mkConst ``RuntimeValue) (elements.extract 0 index).toList
    let after ← mkArrayLit (mkConst ``RuntimeValue) (elements.extract (index + 1) elements.size).toList
    steps := steps.push (← mkAppM ``FocusStep.mk
      #[leaf.getArg! 0, before, after, leaf.getArg! 1])
    leaf := elements[index]!
  let path ← mkListLit (mkConst ``FocusStep) steps.toList
  let kind ← mkEqRefl (mkConst (if shared then ``ReferenceKind.shared else ``ReferenceKind.mutable))
  return some (← withTransparency .default <| mkAppM
    (if shared then ``evaluate_shared_borrow_focus else ``evaluate_borrow_focus)
    #[operation.getArg! 2, kind, operation.getArg! 4, slot, path,
      borrowed.getArg! 0, leaf, state, ← read.getProof])

/-- An indexed reborrow observes one element and updates one local slot.
The row and the vector contents are arbitrary; the resolver and bounds are
checked separately before this shared semantic equation is instantiated. -/
theorem evaluate_indexed_borrow_vector
    (operation : IndexedLocalBorrowOperation)
    (kind : operation.kind = .mutable)
    (referenceKind : operation.referenceType.kind = .mutable)
    (s : RowState) (slot : LocalId) (outer index : Nat) (elements : Array RuntimeValue)
    (resolved : operation.resolve? s.frame s.state =
      some ⟨.local slot, #[.deref, .index index], true⟩)
    (read : readLocal? s.frame slot = some (.borrow outer (.vector elements)))
    (bound : index < elements.size) :
    operation.evaluate? #[] s.frame s.state =
      some (.value
        (rowFrame (s.row.set! slot.index
          (some (.borrow outer (.vector (elements.set! index (.loanHole s.state.nextLoan))))))
          { s.registries with
            activeLoans := (s.registries.activeLoans.filter (·.1 != ⟨operation.lexicalLoan⟩)).push
              (⟨operation.lexicalLoan⟩, s.state.nextLoan)
            loanLocations := s.registries.loanLocations.push
              (s.state.nextLoan, ⟨.local slot, #[.deref, .index index], true⟩) })
        { s.state with nextLoan := s.state.nextLoan + 1 }
        (.borrow s.state.nextLoan elements[index])) := by
  have slotBound := readLocal_inBounds read
  simp only [IndexedLocalBorrowOperation.evaluate?, liftPlaceEvaluator,
    Array.isEmpty_empty, Bool.not_true, Bool.false_eq_true, ite_false, resolved]
  simp only [RowState.frame, rowFrame] at read slotBound ⊢
  simp [borrowRuntimePlaceAt?, kind, referenceKind, readRuntimePlace?, readRoot?, read,
    readProjections?, writeRuntimePlace?, writeRoot?, writeProjections?,
    bound, Nat.not_le.mpr slotBound,
    show (ReferenceKind.mutable != ReferenceKind.mutable) = false from rfl]

/-- Dynamic resolution needs only two local reads and the index range, not
the recursive place evaluator's entire definition inventory. -/
theorem resolve_indexed_borrow_vector
    (operation : IndexedLocalBorrowOperation) (deref : operation.dereference = true)
    (indexLocal : LocalId) (dynamic : operation.indexLocal = some indexLocal)
    (frame : RuntimeFrame) (state : RuntimeState) (outer : Nat) (index : Int)
    (elements : Array RuntimeValue)
    (read : readLocal? frame operation.location.localId = some (.borrow outer (.vector elements)))
    (readIndex : readLocal? frame indexLocal = some (.integer index))
    (nonnegative : ¬index < 0) (bound : index.toNat < elements.size) :
    operation.resolve? frame state =
      some ⟨.local operation.location.localId, #[.deref, .index index.toNat], true⟩ := by
  have slotBound := readLocal_inBounds read
  simp [IndexedLocalBorrowOperation.resolve?, dynamic, deref,
    resolveLocalDynamicIndex?, resolveLocalLiteralIndex?,
    Nat.not_le.mpr slotBound, read, readIndex, nonnegative,
    readRuntimePlace?, readRoot?, readProjections?, bound]

private def indexedBorrowCertificate? (operation state : Lean.Expr) :
    SimpM (Option Lean.Expr) := do
  let operation ← whnf operation
  unless operation.isAppOfArity ``IndexedLocalBorrowOperation.mk 7 do return none
  unless (operation.getArg! 4).isConstOf ``BorrowKind.mutable do return none
  unless (operation.getArg! 1).isConstOf ``Bool.true do return none
  let referenceType ← mkAppM ``IndexedLocalBorrowOperation.referenceType #[operation]
  let referenceKind ← whnf (← mkAppM ``ReferenceType.kind #[referenceType])
  unless referenceKind.isConstOf ``ReferenceKind.mutable do return none
  let dynamic := operation.getArg! 6
  unless dynamic.isAppOfArity ``Option.some 2 do return none
  let indexLocal := dynamic.getArg! 1
  trace[leaner.normalize] "indexed certificate start: {← IO.getNumHeartbeats}"
  let frame ← mkAppM ``RowState.frame #[state]
  let runtimeState ← mkAppM ``RowState.state #[state]
  let slot ← whnf (← mkAppM ``LocalLocation.localId #[operation.getArg! 0])
  let read ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, slot])
  trace[leaner.normalize] "indexed certificate read: {← IO.getNumHeartbeats}"
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let borrowed := read.expr.getArg! 1
  unless borrowed.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let value := borrowed.getArg! 1
  unless value.isAppOfArity ``RuntimeValue.vector 1 do return none
  let elements := value.getArg! 0
  let readIndex ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, indexLocal])
  unless readIndex.expr.isAppOfArity ``Option.some 2 do return none
  let indexValue := readIndex.expr.getArg! 1
  unless indexValue.isAppOfArity ``RuntimeValue.integer 1 do return none
  let position := indexValue.getArg! 0
  let index ← mkAppM ``Int.toNat #[position]
  let negative ← Simp.simp (← mkAppM ``LT.lt #[position, toExpr (0 : Int)])
  unless negative.expr.isConstOf ``False do return none
  let nonnegative ← mkAppM ``of_eq_false #[← negative.getProof]
  let bound ← Simp.simp (← mkAppM ``LT.lt #[index, ← mkAppM ``Array.size #[elements]])
  trace[leaner.normalize] "indexed certificate bounded: {← IO.getNumHeartbeats}"
  unless bound.expr.isConstOf ``True do return none
  let boundProof ← mkAppM ``of_eq_true #[← bound.getProof]
  -- These equations have only explicit parameters. Avoid repeating state
  -- reduction during elaborator unification; the kernel checks the result.
  let resolved := mkAppN (mkConst ``resolve_indexed_borrow_vector)
    #[operation, ← mkEqRefl (mkConst ``Bool.true), indexLocal, ← mkEqRefl dynamic,
      frame, runtimeState, borrowed.getArg! 0, position, elements,
      ← read.getProof, ← readIndex.getProof, nonnegative, boundProof]
  return some (mkAppN (mkConst ``evaluate_indexed_borrow_vector)
    #[operation, ← mkEqRefl (mkConst ``BorrowKind.mutable),
      ← mkEqRefl (mkConst ``ReferenceKind.mutable), state, slot, borrowed.getArg! 0,
      index, elements, resolved, ← read.getProof, boundProof])

/-- Select the borrow law before the generic evaluation rule traverses the
continuation. Failure to find a focus leaves the ordinary evaluator intact. -/
simproc_decl wpBorrowFocus
    (wp (evaluate _ _) _ _ _) := fun e => do
  let arguments := e.getAppArgs
  let action := arguments[arguments.size - 4]!
  unless action.isAppOfArity ``evaluate 2 do return .continue
  let evaluator := action.getArg! 0
  let indexed := evaluator.isAppOfArity ``IndexedLocalBorrowOperation.evaluate? 1
  unless indexed || evaluator.isAppOfArity ``DerefLocalBorrowOperation.evaluate? 1 do
    return .continue
  try
    let s := arguments.back!
    let certificate? ← if indexed then indexedBorrowCertificate? (evaluator.getArg! 0) s
      else focusCertificate? (evaluator.getArg! 0) s
    let some certificate := certificate? | return .continue
    let proof ← withTransparency .default <| mkAppOptM ``wp_evaluate_value
      #[none, none, some s, none, none, none, some certificate,
        some arguments[arguments.size - 3]!, some arguments[arguments.size - 2]!]
    let some (_, lhs, rhs) := (← instantiateMVars (← inferType proof)).eq? | return .continue
    unless ← withTransparency .default <| isDefEq lhs e do return .continue
    trace[leaner.normalize] "compositional borrow focus"
    return .visit { expr := rhs, proof? := some proof }
  catch error =>
    trace[leaner.normalize] "borrow focus fallback: {error.toMessageData}"
    return .continue

/-- An explicitly selected lookup equation avoids unfolding the array's
iterator before literal-registry simp rules have a chance to match. -/
private def cachedLocalPlace (frame : RuntimeFrame) (loan : Nat) : Option RuntimePlace := do
  let (_, place) ← frame.loanLocations.toList.reverse.find? (·.1 == loan)
  let .local _ := place.root | none
  some place

private theorem localLoanPlace_eq_cached (frame : RuntimeFrame) (loan : Nat) :
    localLoanPlace? frame loan = cachedLocalPlace frame loan := by
  unfold localLoanPlace? cachedLocalPlace
  rw [Array.findRev?_eq_find?_reverse, ← Array.find?_toList, Array.toList_reverse]
  rfl

/-- Certify the shared cached loan lookup without opening array iterators. -/
def localLoanCertificate (frame loan : Lean.Expr) : SimpM Simp.Result := do
  let result ← simplifyGround ``cachedLocalPlace (← mkAppM ``cachedLocalPlace #[frame, loan])
  let bridge ← mkAppM ``localLoanPlace_eq_cached #[frame, loan]
  let proof ← mkEqTrans bridge (← result.getProof)
  return { result with proof? := some proof }

private def cachedMutationCertificate? (values state : Lean.Expr) (location : Simp.Result) :
    SimpM (Option Lean.Expr) := do
  unless values.isAppOfArity ``Array.mk 2 || values.isAppOfArity ``List.toArray 2 do return none
  let some elements := literalList? (values.getArg! 1) | return none
  unless elements.size == 2 do return none
  let borrowed := elements[0]!
  unless borrowed.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let loan := borrowed.getArg! 0
  let frame ← mkAppM ``RowState.frame #[state]
  unless location.expr.isAppOfArity ``Option.some 2 do return none
  let place := location.expr.getArg! 1
  unless place.isAppOfArity ``RuntimePlace.mk 3 do return none
  let root := place.getArg! 0
  unless root.isAppOfArity ``RuntimePlaceRoot.local 1 do return none
  let empty ← mkArrayLit (mkConst ``RuntimeProjection) []
  unless ← isDefEq (place.getArg! 1) empty do return none
  unless (place.getArg! 2).isConstOf ``Bool.true do return none
  let slot := root.getArg! 0
  let read ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, slot])
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let current := read.expr.getArg! 1
  unless current.isAppOfArity ``RuntimeValue.borrow 2 do return none
  unless ← isDefEq (current.getArg! 0) loan do return none
  return some (← withTransparency .default <| mkAppM ``evaluate_mutate_local
    #[state, slot, loan, current.getArg! 1, borrowed.getArg! 1, elements[1]!,
      ← location.getProof, ← read.getProof])

/-- Keep the public list equations visible to the evaluator simplifier,
instead of opening the library's well-founded search implementation. -/
private def restingBorrowIndex? (loan : Nat) (replacement : RuntimeValue)
    (locals : List (Option RuntimeValue)) : Option Nat :=
  locals.findIdx? fun value =>
    (value.bind (rewriteFirst (borrowRewrite? loan replacement))).isSome

/-- A reborrow's cached lender address can still contain its hole. Certify
that common case directly, without evaluating the whole runtime place. -/
theorem updateLocalBorrowValue_deref_hole
    (frame : RuntimeFrame) (state : RuntimeState) (slot : LocalId)
    (loan outer hole : Nat) (replacement : RuntimeValue) (mutable : Bool)
    (location : localLoanPlace? frame loan = some ⟨.local slot, #[.deref], mutable⟩)
    (read : readLocal? frame slot = some (.borrow outer (.loanHole hole))) :
    updateLocalBorrowValue? frame state loan replacement = none := by
  simp [updateLocalBorrowValue?, location, readRuntimePlace?, readRoot?, read,
    readProjections?, rewriteFirst_loanHole, borrowRewrite?]

private def absentMutationCache? (frame runtime loan replacement : Lean.Expr)
    (location : Simp.Result) :
    SimpM (Option Lean.Expr) := do
  if location.expr.isAppOfArity ``Option.none 1 then
    return some (← withTransparency .default <| mkAppM ``updateLocalBorrowValue_no_location
      #[frame, runtime, loan, replacement, ← location.getProof])
  unless location.expr.isAppOfArity ``Option.some 2 do return none
  let place := location.expr.getArg! 1
  if place.isAppOfArity ``RuntimePlace.mk 3 then
    let root := place.getArg! 0
    let deref ← mkArrayLit (mkConst ``RuntimeProjection) [mkConst ``RuntimeProjection.deref]
    if root.isAppOfArity ``RuntimePlaceRoot.local 1 &&
        (← isDefEq (place.getArg! 1) deref) then
      let slot := root.getArg! 0
      let read ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, slot])
      if read.expr.isAppOfArity ``Option.some 2 then
        let value := read.expr.getArg! 1
        if value.isAppOfArity ``RuntimeValue.borrow 2 &&
            (value.getArg! 1).isAppOfArity ``RuntimeValue.loanHole 1 then
          return some (← withTransparency .default <| mkAppM ``updateLocalBorrowValue_deref_hole
            #[frame, runtime, slot, loan, value.getArg! 0, (value.getArg! 1).getArg! 0,
              replacement, place.getArg! 2, ← location.getProof, ← read.getProof])
  let read ← simplifyGround ``readRuntimePlace?
    (← mkAppM ``readRuntimePlace? #[frame, runtime, place])
  if read.expr.isAppOfArity ``Option.none 1 then
    return some (← withTransparency .default <| mkAppM ``updateLocalBorrowValue_no_read
      #[frame, runtime, loan, replacement, place, ← location.getProof, ← read.getProof])
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let value := read.expr.getArg! 1
  let rewrite ← mkAppM ``borrowRewrite? #[loan, replacement]
  let absent ← simplifyGround ``rewriteFirst (← mkAppM ``rewriteFirst #[rewrite, value])
  unless absent.expr.isAppOfArity ``Option.none 1 do return none
  -- `rewriteFirst` is a well-founded mutual definition. Its generated
  -- equation is kernel-convertible to the public wrapper at full transparency.
  return some (← withTransparency .all <| mkAppM ``updateLocalBorrowValue_no_rewrite
    #[frame, runtime, loan, replacement, value, place,
      ← location.getProof, ← read.getProof, ← absent.getProof])

private def searchedMutationCertificate? (values state : Lean.Expr) (location : Simp.Result) :
    SimpM (Option Lean.Expr) := do
  unless values.isAppOfArity ``Array.mk 2 || values.isAppOfArity ``List.toArray 2 do return none
  let some elements := literalList? (values.getArg! 1) | return none
  unless elements.size == 2 do return none
  let borrowed := elements[0]!
  unless borrowed.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let loan := borrowed.getArg! 0
  let replacement := elements[1]!
  let frame ← mkAppM ``RowState.frame #[state]
  let runtime ← mkAppM ``RowState.state #[state]
  -- Only attempt the search certificate when a literal row has a candidate
  -- resting holder. Opaque rows and consumed temporaries keep their existing
  -- fallback instead of paying for an unsuccessful proof-time search twice.
  let locals ← withTransparency .default <| whnf (← mkAppM ``RuntimeFrame.locals #[frame])
  let locals := (← Simp.simp locals).expr
  unless locals.isAppOfArity ``Array.mk 2 || locals.isAppOfArity ``List.toArray 2 do return none
  let some slots := literalList? (locals.getArg! 1) | return none
  let mut candidate := false
  for slot in slots do
    if slot.isAppOfArity ``Option.some 2 then
      let value := slot.getArg! 1
      if value.isAppOfArity ``RuntimeValue.borrow 2 then
        if ← isDefEq (value.getArg! 0) loan then
          candidate := true
          break
  unless candidate do return none
  let some cached ← absentMutationCache? frame runtime loan replacement location | return none
  let locals ← mkAppM ``Array.toList #[locals]
  let found ← simplifyGround ``restingBorrowIndex?
    (← mkAppM ``restingBorrowIndex? #[loan, replacement, locals])
  unless found.expr.isAppOfArity ``Option.some 2 do return none
  let slot ← mkAppM ``LocalId.mk #[found.expr.getArg! 1]
  let read ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, slot])
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let current := read.expr.getArg! 1
  unless current.isAppOfArity ``RuntimeValue.borrow 2 do return none
  unless ← isDefEq (current.getArg! 0) loan do return none
  let proof ← withTransparency .default <| mkAppM ``evaluate_mutate_found
    #[state, slot, loan, current.getArg! 1, borrowed.getArg! 1, replacement,
      cached, ← found.getProof, ← read.getProof]
  return some proof

private def mutationCertificate? (values state : Lean.Expr) : SimpM (Option Lean.Expr) := do
  let values := (← Simp.simp values).expr
  unless values.isAppOfArity ``Array.mk 2 || values.isAppOfArity ``List.toArray 2 do return none
  let some elements := literalList? (values.getArg! 1) | return none
  unless elements.size == 2 && elements[0]!.isAppOfArity ``RuntimeValue.borrow 2 do return none
  let frame ← mkAppM ``RowState.frame #[state]
  -- The validated cache miss is shared by the direct and searched rules.
  -- A stale lender address must not trigger a second registry traversal.
  let location ← localLoanCertificate frame (elements[0]!.getArg! 0)
  if let some proof ← cachedMutationCertificate? values state location then return some proof
  searchedMutationCertificate? values state location

simproc_decl wpMutateFocus
    (wp (evaluate (ReferenceLocationOperation.evaluate? .mutate) _) _ _ _) := fun e => do
  let arguments := e.getAppArgs
  let action := arguments[arguments.size - 4]!
  unless action.isAppOfArity ``evaluate 2 do return .continue
  try
    let certificate ← mutationCertificate? (action.getArg! 1) arguments.back!
    let some certificate := certificate |
      return .continue
    let proof ← withTransparency .default <| mkAppOptM ``wp_evaluate_value
      #[none, none, some arguments.back!, none, none, none, some certificate,
        some arguments[arguments.size - 3]!, some arguments[arguments.size - 2]!]
    let some (_, lhs, rhs) := (← instantiateMVars (← inferType proof)).eq? | return .continue
    unless ← withTransparency .default <| isDefEq lhs e do return .continue
    trace[leaner.normalize] "compositional mutation focus"
    return .visit { expr := rhs, proof? := some proof }
  catch error =>
    trace[leaner.normalize] "mutation focus fallback: {error.toMessageData}"
    return .continue

/-- Operand evaluation can expose the resting holder only after the WP
rule has run. Apply the same operation certificate at that boundary too;
do not fall back to unfolding the mutation evaluator for those operands. -/
simproc_decl evalMutateFocus
    (ReferenceLocationOperation.evaluate? .mutate _ _ _) := fun e => do
  let arguments := e.getAppArgs
  unless arguments.size == 4 do return .continue
  try
    let state ← mkAppM ``RowState.ofFrame #[arguments[2]!, arguments[3]!]
    let certificate ← mutationCertificate? arguments[1]! state
    let some proof := certificate | return .continue
    let some (_, lhs, rhs) := (← instantiateMVars (← inferType proof)).eq? | return .continue
    unless ← withTransparency .default <| isDefEq lhs e do return .continue
    trace[leaner.normalize] "compositional mutation evaluator"
    return .visit { expr := rhs, proof? := some proof }
  catch error =>
    trace[leaner.normalize] "mutation evaluator focus fallback: {error.toMessageData}"
    return .continue

-- A pre-simproc runs before the existing post-evaluator. Attribute erasure
-- is module-local and cannot enforce this ordering for downstream imports.
simproc ↓ [lir_eval] evalReferenceFocused
    (ReferenceLocationOperation.evaluate? _ _ _ _) := fun e => do
  let operation ← withTransparency .default <| whnf (e.getArg! 0).consumeMData
  if operation.consumeMData.isConstOf ``ReferenceLocationOperation.mutate then
    evalMutateFocus e
  else return .continue

/-- Focus first, then consume the generic evaluation law if no certificate
applies. In either case, do not simplify a symbolic continuation first. -/
simproc ↓ [lir_wp_norm] wpFocusedOperation (wp (evaluate _ _) _ _ _) := fun e => do
  let arguments := e.getAppArgs
  let action := arguments[arguments.size - 4]!
  unless action.isAppOfArity ``evaluate 2 do return .continue
  let evaluator := action.getArg! 0
  let step ← if evaluator.isAppOfArity ``DerefLocalBorrowOperation.evaluate? 1 ||
      evaluator.isAppOfArity ``IndexedLocalBorrowOperation.evaluate? 1 then
      wpBorrowFocus e
    else if evaluator.isAppOfArity ``ReferenceLocationOperation.evaluate? 1 &&
        (evaluator.getArg! 0).isConstOf ``ReferenceLocationOperation.mutate then
      wpMutateFocus e
    else return .continue
  match step with
  | .continue => Lean.Meta.Simp.rewritePost (rflOnly := false) e
  | step => return step

end LeanerIR.Proofs.Denotation.RowSpec
