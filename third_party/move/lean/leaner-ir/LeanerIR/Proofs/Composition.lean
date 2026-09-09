-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.Represent
import LeanerIR.Proofs.NativeCall
import LeanerIR.Proofs.RetirementWP

/-!
# Modular composition of normalized bodies

The normalizer stops at `wpFunction`. At that boundary we select the callee's
proved contract by its relation name, establish its precondition, and resume
normalization with its postcondition and frame. No body layout is recognized
and no callee implementation is unfolded.
-/

namespace LeanerIR.Proofs.Denotation.RowSpec

open Lean Meta Elab Tactic

initialize registerTraceClass `leaner.composition
initialize registerTraceClass `leaner.loopGoals
initialize registerTraceClass `leaner.loopResidue

/-- Decoding an invariant exposes encoded-array lengths and a natural
modulus. Reduce maps and closed absolute values before arithmetic sees
different atoms for the same length. Generated case facts must be included.
Only these facts are normalized, not the continuation or entire context. -/
elab "leaner_loop_size_facts" : tactic => do
  let mut remaining := []
  for goal in ← getGoals do
    setGoals [goal]
    let facts ← goal.withContext do
      let mut facts := #[]
      for declaration in ← getLCtx do
        let type ← instantiateMVars declaration.type
        if (type.find? fun term =>
            (term.isAppOfArity ``Int.natAbs 1 && !(term.getArg! 0).hasFVar) ||
              (term.isAppOfArity ``Array.size 2 &&
                (term.getArg! 1).consumeMData.isAppOfArity ``Array.map 4)).isSome then
          facts := facts.push (mkIdent declaration.userName)
      pure facts
    trace[leaner.composition] "loop size facts: {facts.size}"
    for fact in facts do
      if (← getGoals).isEmpty then break
      evalTactic (← `(tactic|
        simp (config := { failIfUnchanged := false }) only
          [Int.natAbs, Array.size_map] at $fact:ident))
    remaining := remaining ++ (← getGoals)
  setGoals remaining

attribute [lir_eval] SemanticOperations.transferActiveLoan
  SemanticOperations.transferLoanLocation SemanticOperations.transferredLoan?
  SemanticOperations.registerReturnedLoan_some_singleBorrow


/-- A call is a normalization boundary. Its continuation is simplified
only after the callee's summary has supplied the resulting state. A congruence
lemma alone cannot freeze it: simp falls back when its other arguments do
not change. This pre-simproc leaves the expression literally unchanged. -/
theorem wpFunction_arguments_congr (callee : FunctionDenotation)
    (state state' : RuntimeState) (arguments arguments' : Array RuntimeValue)
    (post : RuntimeState → Outcome → Prop)
    (sameState : state = state') (sameArguments : arguments = arguments') :
    wpFunction callee state arguments post = wpFunction callee state' arguments' post := by
  cases sameState
  cases sameArguments
  rfl

simproc ↓ [lir_wp_norm] stopAtCall (wpFunction _ _ _ _) := fun e => do
  unless e.isAppOfArity ``wpFunction 4 do return .continue
  let state ← Simp.simp (e.getArg! 1)
  let arguments ← Simp.simp (e.getArg! 2)
  let output := mkAppN e.getAppFn #[e.getArg! 0, state.expr, arguments.expr, e.getArg! 3]
  if output == e then return .done { expr := e }
  let proof ← mkAppM ``wpFunction_arguments_congr
    #[e.getArg! 0, e.getArg! 1, state.expr, e.getArg! 2, arguments.expr, e.getArg! 3,
      ← state.getProof, ← arguments.getProof]
  return .done { expr := output, proof? := some proof }

elab "composition_tick " label:str : tactic => do
  trace[leaner.composition] "{label.getString}: {← IO.getNumHeartbeats}"
  if label.getString == "freshness simplified" then
    trace[leaner.composition] "precondition:\n{← ppGoal (← getMainGoal)}"
  if label.getString == "results exposed" then
    trace[leaner.composition] "result context:\n{← ppGoal (← getMainGoal)}"
  if label.getString == "continuation normalized" then
    trace[leaner.loopResidue] "call continuation:\n{← ppGoal (← getMainGoal)}"

/-- Consume the typed transport once at the boundary. The continuation
uses the actual argument row, not a fresh existential argument record. -/
theorem runtimeTyped_post {σ ε : Type} (arguments : Codec A R) (results : Codec B S)
    (contract : Contract σ ε R S) {args : R} {initial final : σ} {out : S}
    (post : (Contract.runtime arguments results
      (Contract.typed arguments results contract)).ensures args initial out final) :
    ∃ value, results.decode? out = some value ∧
      contract.ensures args initial (results.encode value) final := by
  obtain ⟨native, value, encoded, decoded, post⟩ := post
  exact ⟨value, decoded, encoded ▸ post⟩

theorem runtimeTyped_frame {σ ε : Type} (arguments : Codec A R) (results : Codec B S)
    (contract : Contract σ ε R S) {args : R} {initial final : σ}
    (frame : (Contract.runtime arguments results
      (Contract.typed arguments results contract)).frame args initial final) :
    contract.frame args initial final := by
  obtain ⟨native, encoded, frame⟩ := frame
  exact encoded ▸ frame

theorem runtimeTyped_aborts {σ ε : Type} (arguments : Codec A R) (results : Codec B S)
    (contract : Contract σ ε R S) {args : R} {initial : σ} {error : ε}
    (aborts : (Contract.runtime arguments results
      (Contract.typed arguments results contract)).aborts args initial error) :
    contract.aborts args initial error := by
  obtain ⟨native, encoded, aborts⟩ := aborts
  exact encoded ▸ aborts

theorem runtimeTyped_notMust {σ ε : Type} (arguments : Codec A R) (results : Codec B S)
    (contract : Contract σ ε R S) {args : R} {initial : σ}
    (permitted : (Contract.runtime arguments results
      (Contract.typed arguments results contract)).requires args initial)
    (notMust : ¬(Contract.runtime arguments results
      (Contract.typed arguments results contract)).mustAbort args initial) :
    ¬contract.mustAbort args initial := by
  obtain ⟨native, encoded, _⟩ := permitted
  intro must
  apply notMust
  refine ⟨native, encoded, ?_⟩
  change contract.mustAbort (arguments.encode native) initial
  exact encoded.symm ▸ must

/-- Strip the typed transport in the reusable call rule, rather than
re-elaborating the existential argument adapters in every continuation. -/
theorem wpFunction_of_runtimeTyped
    {unit : Validation.ExecutableUnit} {shape : SemanticOperations.FunctionShape}
    {typeInstantiation : Array (TypeId × TypeId)}
    {body : ExprDenotation} {argumentsCodec : Codec A (Array RuntimeValue)}
    {resultsCodec : Codec B (Array RuntimeValue)} {contract : FunctionContract}
    (verified : Satisfies (nativeFunctionAt unit shape typeInstantiation body)
      (Contract.runtime argumentsCodec resultsCodec
        (Contract.typed argumentsCodec resultsCodec contract)))
    (exactAbort : contract.mayAbort = contract.mustAbort)
    {arguments : Array RuntimeValue} {state : RuntimeState}
    (permitted : (Contract.runtime argumentsCodec resultsCodec
      (Contract.typed argumentsCodec resultsCodec contract)).requires arguments state)
    {post : RuntimeState → Outcome → Prop}
    (onReturn : ∀ results final,
      (∃ value, resultsCodec.decode? results = some value ∧
        contract.ensures arguments state (resultsCodec.encode value) final) →
      contract.frame arguments state final →
      ¬contract.mustAbort arguments state → post final (.returned results))
    (onThrow : ∀ kind thrown final,
      contract.aborts arguments state (kind, thrown) → post final (.threw kind thrown)) :
    wpFunction (nativeFunctionRelationAt unit shape typeInstantiation body) state arguments post := by
  apply wpFunction_of_satisfiesAt verified permitted
  · intro results final ensures frame notMust
    have noAbort := runtimeTyped_notMust _ _ _ permitted notMust
    apply onReturn results final
    · apply runtimeTyped_post _ _ _
      apply ensures
      rintro ⟨native, encoded, aborts⟩
      apply noAbort
      rw [← exactAbort]
      exact encoded ▸ aborts
    · exact runtimeTyped_frame _ _ _ frame
    · exact noAbort
  · intro kind thrown final aborts
    exact onThrow kind thrown final (runtimeTyped_aborts _ _ _ aborts)

/-- Reconciliation changes globals and pending writes, neither of which
participates in the loan-registry discipline. Keep that fact folded. -/
theorem discipline_resumed (initial final : RuntimeState) (globals : GlobalMap)
    (pending : Array (Nat × RuntimeValue)) :
    SemanticOperations.LoanDiscipline initial
      { globals, globalLoans := final.globalLoans, nextLoan := final.nextLoan, pending } ↔
      SemanticOperations.LoanDiscipline initial final := Iff.rfl

theorem fresh_resumed (initial : RuntimeState) (globals : GlobalMap)
    (pending : Array (Nat × RuntimeValue)) (nextLoan : Nat)
    (fresh : SemanticOperations.FreshGlobalLoanIds initial)
    (bound : initial.nextLoan ≤ nextLoan) :
    SemanticOperations.FreshGlobalLoanIds
      { globals, globalLoans := initial.globalLoans, nextLoan, pending } := by
  intro loan h
  exact fresh loan (Nat.le_trans bound h)

theorem loan_key_after {initial final : RuntimeState} {loan : Nat}
    (discipline : SemanticOperations.LoanDiscipline initial final)
    (free : SemanticOperations.globalLoanKeyIn? initial.globalLoans loan = none)
    (bound : loan < initial.nextLoan) :
    SemanticOperations.globalLoanKeyIn? final.globalLoans loan = none := by
  rw [discipline.2.1 loan bound]
  exact free

/-- Preserve registrations as well as absence. In particular, a caller's
global lender must still route to its key after a parameter-only callee. -/
theorem loan_keys_after {initial final : RuntimeState}
    (discipline : SemanticOperations.LoanDiscipline initial final)
    (loan : Nat) (bound : loan < initial.nextLoan) :
    SemanticOperations.globalLoanKeyIn? final.globalLoans loan =
      SemanticOperations.globalLoanKeyIn? initial.globalLoans loan :=
  discipline.2.1 loan bound

theorem discipline_advance (initial : RuntimeState) {final : RuntimeState}
    {globals : GlobalMap} {nextLoan : Nat} {pending : Array (Nat × RuntimeValue)}
    (discipline : SemanticOperations.LoanDiscipline
      { globals, globalLoans := initial.globalLoans, nextLoan, pending } final)
    (bound : initial.nextLoan ≤ nextLoan) :
    SemanticOperations.LoanDiscipline initial final :=
  SemanticOperations.LoanDiscipline.trans
    (SemanticOperations.LoanDiscipline.of_eq
      (final := { globals, globalLoans := initial.globalLoans, nextLoan, pending })
      rfl bound) discipline

private theorem lookup_registered_prefix (registrations : List (Nat × GlobalKey))
    (rest : List (Nat × GlobalKey)) (loan : Nat)
    (different : ∀ entry ∈ registrations, entry.1 ≠ loan) :
    SemanticOperations.globalLoanKeyIn? (registrations ++ rest) loan =
      SemanticOperations.globalLoanKeyIn? rest loan := by
  induction registrations with
  | nil => rfl
  | cons head tail ih =>
      have headDifferent := different head (by simp)
      have tailDifferent : ∀ entry ∈ tail, entry.1 ≠ loan := by
        intro entry mem
        exact different entry (by simp [mem])
      simpa [SemanticOperations.globalLoanKeyIn?, headDifferent] using ih tailDifferent

/-- A caller may register global loans before invoking a parameter-only
callee. Only the finite new prefix matters to the allocation discipline. -/
theorem discipline_prefix_advance (initial : RuntimeState) {entry final : RuntimeState}
    (registrations : List (Nat × GlobalKey))
    (discipline : SemanticOperations.LoanDiscipline entry final)
    (registry : entry.globalLoans = registrations ++ initial.globalLoans)
    (bounds : ∀ item ∈ registrations, initial.nextLoan ≤ item.1 ∧ item.1 < entry.nextLoan)
    (frontier : initial.nextLoan ≤ entry.nextLoan) :
    SemanticOperations.LoanDiscipline initial final := by
  apply SemanticOperations.LoanDiscipline.trans (second := entry) ?_ discipline
  refine ⟨?_, ?_, frontier⟩
  · intro fresh loan bound
    rw [registry, lookup_registered_prefix registrations initial.globalLoans loan
      (fun item mem => by have := bounds item mem; omega)]
    exact fresh loan (by omega)
  · intro loan bound
    rw [registry, lookup_registered_prefix registrations initial.globalLoans loan
      (fun item mem => by have := bounds item mem; omega)]

/-- Advance over a syntactically known registration prefix. Do not ask
unification to reconstruct a symbolic registry or leave arithmetic metavariables. -/
private def advanceDiscipline? (initial discipline : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let type ← instantiateMVars (← inferType discipline)
  unless type.isAppOfArity ``SemanticOperations.LoanDiscipline 2 do return none
  let entry := type.getArg! 0
  let original ← whnf (← mkAppM ``RuntimeState.globalLoans #[initial])
  let registered ← whnf (← mkAppM ``RuntimeState.globalLoans #[entry])
  let mut rest := registered
  let mut registrations := #[]
  while rest != original && rest.isAppOfArity ``List.cons 3 do
    registrations := registrations.push (rest.getArg! 1)
    rest ← whnf (rest.getArg! 2)
  unless rest == original do return none
  if registrations.isEmpty then
    let step ← mkAppM ``discipline_advance #[initial, discipline]
    let .forallE _ bound _ _ ← inferType step | return none
    let some proof ← proveFits bound | return none
    return some (mkApp step proof)
  let list ← mkListLit (← inferType registrations[0]!) registrations.toList
  let equality ← mkEqRefl registered
  let step ← mkAppM ``discipline_prefix_advance #[initial, list, discipline, equality]
  let .forallE _ bounds _ _ ← inferType step | return none
  let goal ← mkFreshExprMVar bounds
  let remaining ← Term.TermElabM.run' do
    Tactic.run goal.mvarId! do
      withoutRecover <| evalTactic (← `(tactic|
        simp only [List.forall_mem_cons, List.mem_nil_iff, false_implies, implies_true, and_true]
        <;> omega))
  unless remaining.isEmpty do return none
  let step := mkApp step (← instantiateMVars goal)
  let .forallE _ bound _ _ ← inferType step | return none
  let some proof ← proveFits (← LeanerIR.Proofs.Certify.normalizeProjections bound) | return none
  trace[leaner.composition] "advanced over {registrations.size} global registrations"
  return some (mkApp step proof)

/-- Reconcile only once the caller's row is concrete. Do not expose a
symbolic fallback scan to the surrounding weakest-precondition simplifier. -/
theorem writeBack_cached {frame : RuntimeFrame} {state : RuntimeState}
    {loan : Nat} {current : RuntimeValue} {resolved : RuntimeFrame × RuntimeState}
    (cached : SemanticOperations.fillLocalLoanHole? frame state loan current = some resolved) :
    SemanticOperations.applyPendingWriteBack frame state loan current = resolved := by
  simp only [SemanticOperations.applyPendingWriteBack, cached]


/-- A cached root/dereference is an operation-level focus, independent of
the surrounding row. Consume its read certificate without interpreting the
read/write pipeline again for each projection of the resulting frame. -/
theorem fillLocalLoanHole_deref {frame : RuntimeFrame} {state : RuntimeState}
    {slot : LocalId} {loan outer : Nat} {replacement : RuntimeValue}
    (location : SemanticOperations.localLoanPlace? frame loan =
      some { root := .local slot, projections := #[.deref] })
    (read : SemanticOperations.readLocal? frame slot = some (.borrow outer (.loanHole loan)))
    (bound : slot.index < frame.locals.size) :
    SemanticOperations.fillLocalLoanHole? frame state loan replacement =
      some ({ frame with
        locals := frame.locals.set! slot.index (some (.borrow outer replacement))
        activeLoans := SemanticOperations.transferActiveLoan frame.activeLoans loan replacement
        loanLocations := SemanticOperations.transferLoanLocation frame.loanLocations loan replacement },
        state) := by
  simp [SemanticOperations.fillLocalLoanHole?, location,
    SemanticOperations.readRuntimePlace?, SemanticOperations.readRoot?, read,
    SemanticOperations.readProjections?, SemanticOperations.fillHole?,
    SemanticOperations.rewriteFirst_loanHole, SemanticOperations.holeFill?,
    SemanticOperations.writeRuntimePlace?, SemanticOperations.writeRoot?,
    SemanticOperations.writeProjections?, Nat.not_le.mpr bound]

theorem fillLocalLoanHole_root {frame : RuntimeFrame} {state : RuntimeState}
    {slot : LocalId} {loan : Nat} {replacement : RuntimeValue}
    (location : SemanticOperations.localLoanPlace? frame loan =
      some { root := .local slot })
    (read : SemanticOperations.readLocal? frame slot = some (.loanHole loan))
    (bound : slot.index < frame.locals.size) :
    SemanticOperations.fillLocalLoanHole? frame state loan replacement =
      some ({ frame with
        locals := frame.locals.set! slot.index (some replacement)
        activeLoans := SemanticOperations.transferActiveLoan frame.activeLoans loan replacement
        loanLocations := SemanticOperations.transferLoanLocation frame.loanLocations loan replacement },
        state) := by
  simp [SemanticOperations.fillLocalLoanHole?, location,
    SemanticOperations.readRuntimePlace?, SemanticOperations.readRoot?, read,
    SemanticOperations.readProjections?, SemanticOperations.fillHole?,
    SemanticOperations.rewriteFirst_loanHole, SemanticOperations.holeFill?,
    SemanticOperations.writeRuntimePlace?, SemanticOperations.writeRoot?, Nat.not_le.mpr bound]

/-- Reconcile any finite export suffix in order. The inherited prefix is
retained but never replayed, independently of the caller's local layout. -/
theorem applyPendingFrom_suffix (inherited : Array (Nat × RuntimeValue))
    (suffix : List (Nat × RuntimeValue)) (frame : RuntimeFrame) (state : RuntimeState)
    (pending : state.pending = inherited ++ suffix.toArray) :
    SemanticOperations.applyPendingFrom inherited frame state =
      suffix.foldl (fun (frame, state) (loan, value) =>
        SemanticOperations.applyPendingWriteBack frame state loan value)
        (frame, { state with pending := inherited }) := by
  unfold SemanticOperations.applyPendingFrom
  rw [pending]
  simp only [Array.size_append, Array.extract_append_right, Array.extract_size]
  simp

private def pendingSuffixCertificate? (inherited frame state rhs equation : Lean.Expr) :
    MetaM (Option Lean.Expr) := do
  let mut inheritedPart := rhs
  let mut entries := #[]
  while inheritedPart.isAppOfArity ``Array.push 3 do
    entries := entries.push (inheritedPart.getArg! 2)
    inheritedPart := inheritedPart.getArg! 1
  unless inheritedPart == inherited && !entries.isEmpty do return none
  let entryType ← mkAppM ``Prod #[mkConst ``Nat, mkConst ``RuntimeValue]
  let suffix ← mkListLit entryType entries.reverse.toList
  let mut rules : SimpTheorems := {}
  rules ← rules.addConst ``Array.push_eq_append
  rules ← rules.addConst ``Array.append_assoc
  let context ← Simp.mkContext (simpTheorems := #[rules])
    (congrTheorems := ← getSimpCongrTheorems)
  let (appended, _) ← Meta.simp rhs context
  let pending ← mkEqTrans equation (← appended.getProof)
  some <$> withTransparency .default (mkAppM ``applyPendingFrom_suffix
    #[inherited, suffix, frame, state, pending])

/-- Read the exported suffix from the summary equation. Its loan and value
occur only in the premise of the reconciliation law; a general simp
discharger cannot infer those output-only parameters reliably. -/
simproc [lir_eval] evalCallPending (SemanticOperations.applyPendingFrom _ _ _) := fun e => do
  unless e.isAppOfArity ``SemanticOperations.applyPendingFrom 3 do return .continue
  let inherited := e.getArg! 0
  let frame := e.getArg! 1
  let state := e.getArg! 2
  let pending ← mkAppM ``RuntimeState.pending #[state]
  for declaration in ← getLCtx do
    let type ← instantiateMVars declaration.type
    unless type.isAppOfArity ``Eq 3 && type.getArg! 1 == pending do continue
    let rhs := type.getArg! 2
    let proof? ← if rhs == inherited then
        some <$> mkAppOptM ``SemanticOperations.applyPendingFrom_none
          #[some inherited, some frame, some state, some (mkFVar declaration.fvarId)]
      else if rhs.isAppOfArity ``Array.push 3 && rhs.getArg! 1 == inherited then do
        let entry := rhs.getArg! 2
        if entry.isAppOfArity ``Prod.mk 4 then
          some <$> mkAppOptM ``SemanticOperations.applyPendingFrom_single
            #[some inherited, some frame, some state, some (entry.getArg! 2),
              some (entry.getArg! 3), some (mkFVar declaration.fvarId)]
        else pure none
      else pendingSuffixCertificate? inherited frame state rhs (mkFVar declaration.fvarId)
    if let some proof := proof? then
      let some (_, _, result) := (← inferType proof).eq? | return .continue
      return .visit { expr := result, proof? := some proof }
  return .continue

private def focusedCallWriteBack? (e : Lean.Expr) : Simp.SimpM (Option Simp.Result) := do
  let frame := e.getArg! 0
  let state := e.getArg! 1
  let loan := e.getArg! 2
  let replacement := e.getArg! 3
  let location ← localLoanCertificate frame loan
  unless location.expr.isAppOfArity ``Option.some 2 do return none
  let place := location.expr.getArg! 1
  unless place.isAppOfArity ``RuntimePlace.mk 3 do return none
  let root := place.getArg! 0
  unless root.isAppOfArity ``RuntimePlaceRoot.local 1 do return none
  let projections := place.getArg! 1
  let dereference := (literalSingleton? projections).any (·.isConstOf ``RuntimeProjection.deref)
  let empty := (projections.isAppOfArity ``List.toArray 2 ||
    projections.isAppOfArity ``Array.mk 2) &&
    (projections.getArg! 1).isAppOfArity ``List.nil 1
  unless dereference || empty do return none
  let slot := root.getArg! 0
  let read ← simplifyGround ``SemanticOperations.readLocal?
    (← mkAppM ``SemanticOperations.readLocal? #[frame, slot])
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let value := read.expr.getArg! 1
  let mut inputs := #[some frame, some state, some slot, some loan]
  if dereference then
    unless value.isAppOfArity ``RuntimeValue.borrow 2 &&
        (value.getArg! 1).isAppOfArity ``RuntimeValue.loanHole 1 &&
        (value.getArg! 1).getArg! 0 == loan do return none
    inputs := inputs.push (some (value.getArg! 0))
  else
    unless value.isAppOfArity ``RuntimeValue.loanHole 1 && value.getArg! 0 == loan do return none
  inputs := inputs ++ #[some replacement, some (← location.getProof), some (← read.getProof)]
  let step ← withTransparency .default <| mkAppOptM
    (if dereference then ``fillLocalLoanHole_deref else ``fillLocalLoanHole_root) inputs
  let bound ← withTransparency .default <| mkAppM ``readLocal_inBounds #[← read.getProof]
  let proof := mkApp step bound
  let some (_, _, result) := (← instantiateMVars (← inferType proof)).eq? | return none
  -- This is already an evaluator simp pass. Retain its cache: a fresh
  -- Meta.simp repeats reductions of the caller's unchanged state fields.
  let finished ← Simp.simp result
  let result : Simp.Result := { expr := result, proof? := some proof }
  some <$> result.mkEqTrans finished

simproc [lir_eval] evalCallWriteBack
    (SemanticOperations.applyPendingWriteBack _ _ _ _) := fun e => do
  unless e.isAppOfArity ``SemanticOperations.applyPendingWriteBack 4 do return .continue
  unless isLiteralFrame (e.getArg! 0) do return .continue
  let start ← IO.getNumHeartbeats
  let cached := mkAppN (mkConst ``SemanticOperations.fillLocalLoanHole?) e.getAppArgs
  let focused? ← try focusedCallWriteBack? e catch error => do
    trace[leaner.composition] "focused fallback: {error.toMessageData}"
    pure none
  let reduced ← match focused? with
    | some result => pure result
    | none => simplifyGround ``SemanticOperations.fillLocalLoanHole? cached
  if reduced.expr.isAppOfArity ``Option.some 2 then
    let proof ← mkAppM ``writeBack_cached #[← reduced.getProof]
    trace[leaner.composition] "cached writeback cost: {(← IO.getNumHeartbeats) - start}"
    return .done { expr := reduced.expr.getArg! 1, proof? := some proof }
  let result ← simplifyGround ``SemanticOperations.applyPendingWriteBack e
  unless result.expr.consumeMData.isAppOfArity ``Prod.mk 4 do return .continue
  trace[leaner.composition] "writeback cost: {(← IO.getNumHeartbeats) - start}"
  return .done result

/-- Eliminate a generic contract's logical input witnesses before erasures
expand. The decoded native input record already determines those witnesses. -/
elab "leaner_call_native_requires" : tactic => do
  let target ← instantiateMVars (← (← getMainGoal).getType)
  let constants := target.getUsedConstants
  unless constants.contains ``Codec.identity do return
  let env ← getEnv
  let mut names := #[]
  for name in constants do
    if ["typedContract", "rawContract", "argumentsCodec", "resultsCodec"].contains name.getString! then
      names := names.push name
      if name.getString! == "typedContract" then
        for suffix in [`rawContract, `argumentsCodec, `resultsCodec] do
          let related := name.getPrefix ++ suffix
          if env.contains related then names := names.push related
  let lemmas ← names.mapM fun name => `(Lean.Parser.Tactic.simpLemma| $(mkIdent name):ident)
  evalTactic (← `(tactic|
    simp (config := { failIfUnchanged := false }) only
      [Contract.runtime, Contract.typed, $lemmas,*, Codec.encode_eq_encode,
       Array.mk.injEq, List.cons.injEq, and_assoc, and_true, true_and,
       exists_eq_left, exists_eq_left', exists_eq_right]))

/-- Recover the native argument record from the runtime row at a call boundary.
The generated decoder supplies range certificates using the caller's context. -/
elab "leaner_call_arguments" : tactic => do
  let goal ← getMainGoal
  let next ← goal.withContext do
    let target ← whnf (← instantiateMVars (← goal.getType))
    unless target.isAppOfArity ``Exists 2 do throwError "expected native call arguments"
    let predicate := target.getArg! 1
    let .lam _ _ body _ := predicate.consumeMData | throwError "expected argument predicate"
    let equation := body.getArg! 0
    let encoder := equation.getArg! 1
    let runtime := equation.getArg! 2
    unless encoder.isAppOfArity ``Codec.encode 4 do throwError "expected argument encoder: {encoder}"
    let codec := encoder.getArg! 2
    let application ← mkAppM ``Codec.decode? #[codec, runtime]
    let roots := codec.getUsedConstants
    let some (arguments, _) ← evaluateDecoding roots application
      | trace[leaner.composition] "undecoded argument context:\n{← ppGoal goal}"
        throwError "cannot decode call arguments: {application}"
    let rest ← mkFreshExprMVar (predicate.beta #[arguments])
    goal.assign (← mkAppOptM ``Exists.intro #[none, some predicate, some arguments, some rest])
    pure rest.mvarId!
  replaceMainGoal [next]

syntax compositionEntry := ident " => " term
syntax compositionLoop := num " => " term
syntax "leaner_compose" " [" compositionEntry,* "]"
  " [" Lean.Parser.Tactic.simpLemma,* "]" : tactic
syntax "leaner_compose" " [" compositionEntry,* "]"
  " [" Lean.Parser.Tactic.simpLemma,* "]" " with_loops" " [" compositionLoop,* "]" : tactic

macro_rules
  | `(tactic| leaner_compose [$entries,*] [$facts,*]) =>
      `(tactic| leaner_compose [$entries,*] [$facts,*] with_loops [])

syntax "leaner_call_normalize" " [" Lean.Parser.Tactic.simpLemma,* "]" : tactic

elab_rules : tactic
  | `(tactic| leaner_call_normalize [$facts,*]) => do
    trace[leaner.composition] "normalization context start: {← IO.getNumHeartbeats}"
    /- Native summaries can retain projections of literal nominal payloads
    or identity-mapped vectors. Expose these before decoding or range
    reasoning; do not run the evaluator over the surrounding context. -/
    let mut callGoal ← getMainGoal
    let valueFacts ← callGoal.withContext do
      return (← getLCtx).foldl (init := #[]) fun found declaration =>
        if declaration.type.isAppOfArity ``Eq 3 &&
            ([``Nat, ``Int, ``RuntimeValue].any (declaration.type.getArg! 0).isConstOf ||
              ((declaration.type.getArg! 0).isAppOfArity ``Array 1 &&
                (declaration.type.getArg! 0).appArg!.isConstOf ``RuntimeValue)) then
          found.push declaration.fvarId else found
    for fact in valueFacts do
      let normalized ← callGoal.withContext do
        LeanerIR.Proofs.Certify.normalizeProjections (← fact.getType)
      callGoal ← callGoal.replaceLocalDeclDefEq fact normalized
    replaceMainGoal [callGoal]
    let valueNames ← callGoal.withContext do
      valueFacts.mapM fun fact => do pure (mkIdent (← fact.getDecl).userName)
    for hypothesis in valueNames do
      if (← getGoals).isEmpty then return
      evalTactic (← `(tactic| simp (config := { failIfUnchanged := false }) only
        [lir_spec_norm, Array.map_id, String.reduceBEq, ite_true, ite_false,
          Bool.false_eq_true, eq_self_iff_true] at $hypothesis:ident))
    if (← getGoals).isEmpty then return
    let disciplines ← (← getMainGoal).withContext do
      let mut found : Array FVarId := #[]
      for declaration in ← getLCtx do
        if declaration.type.isAppOfArity ``SemanticOperations.LoanDiscipline 2 then
          found := found.push declaration.fvarId
      pure found
    let mut registryFacts : Array (TSyntax `Lean.Parser.Tactic.simpLemma) := #[]
    if let some latest := disciplines.back? then
      let goal ← getMainGoal
      let preserveRegistrations ← goal.withContext do
        let type ← inferType (mkFVar latest)
        let registered ← whnf (← mkAppM ``RuntimeState.globalLoans #[type.getArg! 0])
        pure (registered.isAppOfArity ``List.cons 3)
      if preserveRegistrations then
        let priorKeys ← goal.withContext do
          Term.exprToSyntax (← mkAppM ``loan_keys_after #[mkFVar latest])
        registryFacts := registryFacts ++ #[
          ← `(Lean.Parser.Tactic.simpLemma| $priorKeys:term),
          ← `(Lean.Parser.Tactic.simpLemma| Nat.add_assoc),
          ← `(Lean.Parser.Tactic.simpLemma| SemanticOperations.globalLoanKeyIn?_remove_other)]
      let h ← goal.withContext do Term.exprToSyntax (mkFVar latest)
      withoutRecover <| evalTactic (← `(tactic|
        (have callMonotone := ($h:term).2.2
         dsimp (config := { failIfUnchanged := false }) only at callMonotone
         have callFresh := ($h:term).1 (by
          first
          | assumption
          | (intro loan bound; dsimp only at bound; leaner_fresh_loan)
          | exact fresh_resumed _ _ _ _ (by assumption) (by omega)))))
      let lookups ← (← getMainGoal).withContext do
        let mut found : Array FVarId := #[]
        for declaration in ← getLCtx do
          let type := declaration.type
          if type.isAppOfArity ``Eq 3 &&
              (type.getArg! 1).isAppOfArity ``SemanticOperations.globalLoanKeyIn? 2 &&
              (type.getArg! 2).isAppOfArity ``Option.none 1 then
            found := found.push declaration.fvarId
        pure found
      for lookup in lookups do
        let goal ← getMainGoal
        let proof? ← goal.withContext do
          try
            let step ← mkAppM ``loan_key_after #[mkFVar latest, mkFVar lookup]
            let .forallE _ bound _ _ ← inferType step | return none
            let some proof ← proveFits bound | return none
            return some (mkApp step proof)
          catch _ => return none
        if let some proof := proof? then
          let next ← goal.withContext do
            goal.assert (← mkFreshUserName `callKeyFree) (← inferType proof) proof
          let (_, next) ← next.intro1P
          replaceMainGoal [next]
      /- Extend only the latest accumulated discipline. Composing every
      historical pair duplicates the history exponentially across calls. -/
      for prior in disciplines.pop.back?.toArray do
        let goal ← getMainGoal
        let proof? ← goal.withContext do
          try
            let earlier ← inferType (mkFVar prior)
            let some advanced ← advanceDiscipline? (earlier.getArg! 1) (mkFVar latest)
              | return none
            return some (← mkAppM ``SemanticOperations.LoanDiscipline.trans
              #[mkFVar prior, advanced])
          catch _ => return none
        if let some proof := proof? then
          let next ← goal.withContext do
            goal.assert (← mkFreshUserName `callDiscipline) (← inferType proof) proof
          let (_, next) ← next.intro1P
          replaceMainGoal [next]
      let goal ← getMainGoal
      let advanced? ← goal.withContext do
        let mut initial? := none
        for declaration in ← getLCtx do
          let type ← instantiateMVars declaration.type
          if type.isAppOfArity ``SemanticOperations.FreshGlobalLoanIds 1 then
            initial? := some (type.getArg! 0)
            break
        let some initial := initial? | return none
        /- The newest call starts after earlier calls, whose registry need
        not equal the caller's. First compose their disciplines, then
        discharge the original allocation prefix from the composed fact. -/
        let candidates := (← getLCtx).foldl (init := #[]) fun found declaration =>
          if declaration.type.isAppOfArity ``SemanticOperations.LoanDiscipline 2 then
            found.push declaration.fvarId else found
        for candidate in candidates.reverse do
          let saved ← saveState
          let result ← try
            advanceDiscipline? initial (mkFVar candidate)
            catch _ => pure none
          if result.isSome then return result
          saved.restore
        return none
      if let some proof := advanced? then
        let next ← goal.withContext do
          goal.assert (← mkFreshUserName `callAllocationDiscipline) (← inferType proof) proof
        let (id, next) ← next.intro1P
        replaceMainGoal [next]
        if preserveRegistrations then
          let originalKeys ← next.withContext do
            Term.exprToSyntax (← mkAppM ``loan_keys_after #[mkFVar id])
          registryFacts := registryFacts.push
            (← `(Lean.Parser.Tactic.simpLemma| $originalKeys:term))
    let summaries ← (← getMainGoal).withContext do
      let mut summaries : Array (TSyntax `Lean.Parser.Tactic.simpLemma) := #[]
      for declaration in ← getLCtx do
        if declaration.isImplementationDetail then continue
        let type ← instantiateMVars declaration.type
        let stateEquation := type.isAppOfArity ``Eq 3 &&
          ((type.getArg! 1).getUsedConstants.any (fun n =>
            n == ``RuntimeState.globals || n == ``RuntimeState.globalLoans ||
              n == ``RuntimeState.pending) ||
           [``Nat, ``Int, ``Bool, ``RuntimeValue].any (type.getArg! 0).isConstOf ||
           ((type.getArg! 0).isAppOfArity ``Array 1 &&
             (type.getArg! 0).appArg!.isConstOf ``RuntimeValue) ||
           (type.getArg! 1).isAppOfArity ``SemanticOperations.globalLoanKeyIn? 2)
        let reverse ← if stateEquation && (type.getArg! 0).isConstOf ``RuntimeValue then
            pure ((← isConstructorApp (type.getArg! 1)) &&
              !(← isConstructorApp (type.getArg! 2)))
          else pure false
        /- A summary can relate a value to its returned-borrow resolution,
        which contains that same value. Installing this direction as a
        rewrite expands the value into itself indefinitely. Keep the fact
        available to the closer, but not in the normalization inventory. -/
        if stateEquation &&
            ((type.getArg! (if reverse then 1 else 2)).find?
              (· == type.getArg! (if reverse then 2 else 1))).isSome then
          continue
        /- Boolean results can characterize data equality as well as another
        Boolean's truth test. Consume that proposition directly, provided it
        does not refer back to the same returned Boolean. -/
        let booleanSummary := type.isAppOfArity ``Iff 2 &&
          let side := type.getArg! 0
          side.isAppOfArity ``Eq 3 && (side.getArg! 0).isConstOf ``Bool &&
            (side.getArg! 1).isFVar && (side.getArg! 2).isConstOf ``Bool.true &&
            ((type.getArg! 1).find? (· == side.getArg! 1)).isNone
        if stateEquation || booleanSummary ||
            type.isAppOfArity ``SemanticOperations.LoanDiscipline 2 then
          let fact ← Term.exprToSyntax (mkFVar declaration.fvarId)
          -- A decoded input may be stated as `integer n = input.field`.
          -- Keep the known constructor as the normal form; rewriting it
          -- into a symbolic field would hide the next decoder's shape.
          summaries := summaries.push (← if reverse then
            `(Lean.Parser.Tactic.simpLemma| ← $fact:term)
            else `(Lean.Parser.Tactic.simpLemma| $fact:term))
      pure summaries
    trace[leaner.composition] "normalization context ready: {← IO.getNumHeartbeats}"
    let normalizationFacts := facts.getElems ++ summaries ++ registryFacts
    try
      evalTactic (← `(tactic| leaner_normalize [discipline_resumed,
        lir_call_eval,
        resumeCall, RowState.ofFrame, SemanticOperations.applyPendingFrom_none,
        SemanticOperations.applyPendingFrom_single, callControl, callFrame,
        packResults_nil, packResults_singleton, $normalizationFacts,*]))
    catch error =>
      unless (← error.toMessageData.toString) == "`simp` made no progress" do throw error

/-- Resolve the lender's exported hole from the returned current without
unfolding the recursive value traversal. Payload representation is arbitrary. -/
@[lir_eval] theorem resolveReturnedBorrow_hole (loan : Nat) (value : RuntimeValue) :
    SemanticOperations.resolveReturnedBorrows #[.borrow loan value] (.loanHole loan) =
      value := by
  rw [SemanticOperations.resolveReturnedBorrows_singleBorrow]
  simp [SemanticOperations.fillHole?, SemanticOperations.rewriteFirst_loanHole,
    SemanticOperations.holeFill?]

/-- Invert a scalar mutable result in one step. Keep the native argument
record intact instead of introducing and substituting another decoded
existential through the caller's continuation. -/
theorem mutableIntResult_shape {width : IntWidth} {signed : Bool}
    {current : RuntimeValue} {loan : Nat}
    {argument : MutableArgument (SpecInt width signed)}
    (decoded : (LeanerIR.decodeInt? width signed current).map
      (fun value => MutableArgument.mk loan value) = some argument) :
    current = .integer argument.value.val ∧ loan = argument.loan := by
  obtain ⟨value, currentDecoded, rfl⟩ := Option.map_eq_some_iff.mp decoded
  exact ⟨Codec.specInt_decode?_eq_some currentDecoded, rfl⟩

elab "leaner_call_results" : tactic => do
  let decodedMap ← IO.mkRef false
  -- Injection may retain an equation used by dependent certificates.
  -- Like Meta.injections, visit that equation only once.
  let injected ← IO.mkRef ({} : FVarIdSet)
  let rec expose (goal : MVarId) : Nat → TacticM Unit
    | 0 => throwError "call result decoding exceeded its depth limit"
    | fuel + 1 => do
      setGoals [goal]
      let action ← goal.withContext do
        let mut action : Option (TSyntax `tactic) := none
        for localDecl in ← getLCtx do
          if localDecl.isImplementationDetail then continue
          let type ← instantiateMVars localDecl.type
          unless type.isAppOfArity ``Eq 3 do continue
          let lhs := type.getArg! 1
          let rhs := type.getArg! 2
          let h := mkIdent localDecl.userName
          if (type.getArg! 0).isConstOf ``String && lhs.isLit && rhs.isLit then
            action := some (← `(tactic| simp only [String.reduceEq] at $h:ident))
            break
          /- Decoding an enum exposes the constructors in its postcondition.
          Consume those equations before another call: impossible variants
          must not each generate a new callee proof obligation. -/
          if (type.getArg! 0).isConstOf ``RuntimeValue then
            unless (← injected.get).contains localDecl.fvarId do
              if ← isConstructorApp (← whnf lhs) then
                if ← isConstructorApp (← whnf rhs) then
                  injected.modify (·.insert localDecl.fvarId)
                  action := some (← `(tactic| leaner_cases $h:ident))
                  break
          if lhs.isAppOfArity ``BEq.beq 4 && (lhs.getArg! 0).isConstOf ``String &&
              rhs.isConstOf ``Bool.true then
            action := some (← `(tactic|
              (have encoded := beq_iff_eq.mp $h:ident
               clear $h:ident
               leaner_cases encoded)))
            break
          if lhs.isAppOfArity ``Array.toList 2 && lhs.appArg!.isFVar then
            action := some (← `(tactic|
              (have row := congrArg List.toArray $h:ident
               simp (config := { failIfUnchanged := false }) only
                 [Array.toArray_toList] at row
               clear $h:ident
               leaner_cases row)))
            break
          if rhs.isAppOfArity ``Option.some 2 then
            let twinDecoder := (rhs.getArg! 0).getAppFn.constName?.any fun twin =>
              lhs.getAppFn.isConstOf (twin ++ `decode?)
            let matcher ← match lhs.getAppFn.constName? with
              | some name => Option.isSome <$> getMatcherInfo? name
              | none => pure false
            if lhs.isAppOfArity ``Codec.decode? 4 &&
                (rhs.getArg! 0).isAppOfArity ``SpecVector 1 &&
                (rhs.getArg! 0).appArg!.isConstOf ``RuntimeValue then
              action := some (← `(tactic|
                (have encoded := Codec.boundedVector_decode?_eq_some
                   (Codec.identity RuntimeValue) (fun h => Option.some.inj h) $h:ident
                 simp only [Codec.identity, Array.map_id] at encoded
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``Codec.decode? 4 &&
                (rhs.getArg! 0).isAppOfArity ``SpecVector 1 &&
                (rhs.getArg! 0).appArg!.isAppOfArity ``SpecInt 2 then
              action := some (← `(tactic|
                (have encoded := Codec.boundedVector_decode?_eq_some
                   (Codec.specInt _ _) (fun h => Codec.specInt_decode?_eq_some h) $h:ident
                 dsimp only [Codec.specInt] at encoded
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeInt? 3 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeInt?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeBool? 1 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeBool?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeString? 1 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeString?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeAddress? 1 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeAddress?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeSigner? 1 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeSigner?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeBytes? 1 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeBytes?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.isAppOfArity ``LeanerIR.decodeUnit? 1 then
              action := some (← `(tactic|
                (have encoded := LeanerIR.decodeUnit?_shape $h:ident
                 clear $h:ident
                 leaner_cases encoded)))
            else if lhs.getAppFn.isConstOf ``Option.map then
              decodedMap.set true
              action := some (← `(tactic|
                first
                | (have encoded := mutableIntResult_shape $h:ident
                   clear $h:ident
                   leaner_cases encoded)
                | (simp only [Option.map_eq_some_iff] at $h:ident
                   leaner_cases $h:ident)))
            else if lhs.getAppFn.isConstOf ``Option.bind then
              action := some (← `(tactic|
                (simp only [Option.bind_eq_some_iff, Option.some.injEq] at $h:ident
                 leaner_cases $h:ident)))
            else if twinDecoder then
              let decoder := mkIdent lhs.getAppFn.constName!
              action := some (← `(tactic| unfold $decoder:ident at $h:ident))
            else if matcher || lhs.isAppOfArity ``ite 5 || lhs.isAppOfArity ``dite 5 then
              action := some (← `(tactic|
                (split at $h:ident <;>
                 simp (config := { failIfUnchanged := false }) only
                   [Option.bind_eq_some_iff, Option.some.injEq, reduceCtorEq] at $h:ident
                 all_goals leaner_cases $h:ident)))
            if action.isSome then break
        pure action
      if let some tactic := action then
        evalTactic tactic
        let mut remaining := []
        for next in ← getGoals do
          expose next fuel
          remaining := remaining ++ (← getGoals)
        setGoals remaining
  let goals ← getGoals
  let mut remaining := []
  for goal in goals do
    expose goal 64
    remaining := remaining ++ (← getGoals)
  setGoals remaining
  /- Substituting the decoded record leaves projections in loan bounds.
  Omega otherwise treats those projections and the loan itself as different
  atoms, so fresh returned loans appear to alias their lenders. -/
  if ← decodedMap.get then
    evalTactic (← `(tactic| all_goals
      (dsimp (config := { failIfUnchanged := false }) only at *
       simp (config := { failIfUnchanged := false }) only
         [resolveReturnedBorrow_hole] at *)))
    /- A projected return's clauses constrain siblings through a resolved
    lender, not directly through its fresh existential fields. Normalize
    only those summary facts; traversing the whole continuation here would
    repeat the work of the subsequent WP pass. -/
    let mut normalized := []
    for goal in ← getGoals do
      setGoals [goal]
      let summaries ← goal.withContext do
        let mut found := #[]
        for declaration in ← getLCtx do
          if declaration.isImplementationDetail then continue
          if (declaration.type.find? (·.isConstOf
              ``SemanticOperations.resolveReturnedBorrows)).isSome then
            found := found.push (mkIdent declaration.userName)
        pure found
      for summary in summaries do
        if (← getGoals).isEmpty then break
        evalTactic (← `(tactic|
          (simp (config := { failIfUnchanged := false }) (disch := leaner_plain) only
             [SemanticOperations.resolveReturnedBorrows_returnedBorrow_focus]
             at $summary:ident
           all_goals simp (config := { failIfUnchanged := false })
             [SemanticOperations.focusValue, SemanticOperations.FocusStep.fill,
              RuntimeValue.field, RuntimeValue.asInt, RuntimeValue.asBool]
             at $summary:ident)))
        unless (← getGoals).isEmpty do
          let current ← getMainGoal
          let hypothesis? ← current.withContext do
            pure (((← getLCtx).findFromUserName? summary.getId).map (·.fvarId))
          if let some hypothesis := hypothesis? then
            let decodedGoal ← Certify.decodeLiteralTwins current (some hypothesis)
            setGoals decodedGoal.toList
      normalized := normalized ++ (← getGoals)
    setGoals normalized

elab_rules : tactic
  | `(tactic| leaner_compose [$entries,*] [$facts,*] with_loops [$loops,*]) => do
    let entries ← entries.getElems.mapM fun entry => do
      let `(compositionEntry| $relation:ident => $verified:term) := entry
        | throwUnsupportedSyntax
      let relation ← realizeGlobalConstNoOverloadWithInfo relation
      pure (relation, verified)
    let loops ← loops.getElems.mapM fun entry => do
      let `(compositionLoop| $site:num => $invariant:term) := entry
        | throwUnsupportedSyntax
      pure (site.getNat, invariant)
    let invariants ← loops.mapM fun (_, invariant) =>
      `(Lean.Parser.Tactic.simpLemma| $invariant:term)
    let rec close (goal : MVarId) : Nat → TacticM Unit
      | 0 => throwError "composition exceeded its structural depth limit"
      | fuel + 1 => do
      if ← goal.isAssigned then return
      setGoals [goal]
      let target ← goal.withContext do
        let target ← instantiateMVars (← goal.getType)
        if target.isAppOfArity ``wp 7 &&
            ((target.getArg! 3).isAppOfArity ``loop 2 ||
             (target.getArg! 3).isAppOfArity ``ite 5) then
          pure target
        else whnf target
      if target.isAppOfArity ``wp 7 && (target.getArg! 3).isAppOfArity ``loop 2 then
        let action := target.getArg! 3
        let predicate? ← goal.withContext do
          loops.findSomeM? fun (site, predicate) => do
            if ← isDefEq (action.getArg! 0) (mkApp (mkConst ``ExprId.mk) (toExpr site)) then
              pure (some predicate)
            else pure none
        let some predicate := predicate?
          | throwError "no authored invariant supplied for loop {action.getArg! 0}"
        let initial ← goal.withContext do Term.exprToSyntax (target.getArg! 6)
        trace[leaner.composition] "loop entry start: {← IO.getNumHeartbeats}"
        evalTactic (← `(tactic|
          apply wp_loop_of_invariant (invariant := fun state =>
            $predicate ($initial).frame ($initial).state state.frame state.state)))
        let [entry, preserved] ← getGoals
          | throwError "unexpected loop invariant obligations"
        setGoals [entry]
        evalTactic (← `(tactic|
          (simp only [$predicate:term, RowState.frame, RowState.ofFrame, rowFrame,
             lir_data_norm, List.getElem?_toArray, List.getElem?_cons_zero,
             List.getElem?_cons_succ, Option.join_some, Bool.false_eq_true,
             ite_true, ite_false]
           leaner_certified_close!)))
        trace[leaner.composition] "loop entry done: {← IO.getNumHeartbeats}"
        setGoals [preserved]
        evalTactic (← `(tactic|
          (intro iteration invariantHolds
           rcases iteration with ⟨row, ⟨activeLoans, loanLocations, typeInstantiation⟩, state⟩
           simp only [$predicate:term, RowState.frame, RowState.ofFrame, rowFrame,
             Array.size_map, List.size_toArray, List.length_map, Array.length_toList]
             at invariantHolds
           leaner_cases invariantHolds
           all_goals composition_tick "loop context exposed"
           all_goals
             (dsimp (config := { failIfUnchanged := false }) only at *
              leaner_loop_size_facts
              set_option leaner.branchBoundaries true in
                leaner_normalize [loopPost, $facts,*]))))
        trace[leaner.composition] "loop body normalized: {← IO.getNumHeartbeats}"
        for continuation in ← getGoals do
          trace[leaner.loopGoals] "loop continuation:\n{← continuation.withContext do ppGoal continuation}"
        for continuation in ← getGoals do close continuation fuel
      else if target.isAppOfArity ``wp 7 && (target.getArg! 3).isAppOfArity ``ite 5 then
        trace[leaner.composition] "branch start: {← IO.getNumHeartbeats}"
        evalTactic (← `(tactic|
          (apply (wp_ite_prop _ _ _ _ _ _).mpr
           constructor
           all_goals
             (intro branchGuard
              simp (config := { failIfUnchanged := false })
                [-not_and, Classical.not_and_iff_not_or_not] at branchGuard
              all_goals try leaner_cases branchGuard
              all_goals try (exfalso; omega)
              all_goals
                (set_option leaner.branchBoundaries true in
                  leaner_normalize [loopPost, $facts,*])))))
        trace[leaner.composition] "branch normalized: {← IO.getNumHeartbeats}"
        for continuation in ← getGoals do
          trace[leaner.loopGoals] "branch continuation:\n{← continuation.withContext do ppGoal continuation}"
        for continuation in ← getGoals do close continuation fuel
      else if target.isAppOfArity ``wpFunction 4 then
        let relation := target.getArg! 0
        let some (_, verified) := entries.find? fun (name, _) =>
            relation.getAppFn.isConstOf name
          | throwError "no proved contract supplied for call relation {relation.getAppFn}"
        -- A shared generic summary is quantified over the invocation map.
        -- Specialize it at this call, without opening the callee body.
        let verified ← goal.withContext do
          let proof ← Term.elabTerm verified none
          -- Do not unfold Satisfies: a monomorphic summary itself reduces
          -- to a forall, but only an explicit outer binder is a type map.
          let type := (← instantiateMVars (← inferType proof)).consumeMData
          if type.isForall then
            unless relation.getAppNumArgs == 2 do
              throwError "generic call relation has no invocation map"
            Term.exprToSyntax (mkApp proof (relation.getArg! 1))
          else pure verified
        evalTactic (← `(tactic| apply wpFunction_of_runtimeTyped $verified rfl))
        let [permitted, returned, thrown] ← getGoals
          | throwError "unexpected modular call obligations"
        setGoals [permitted]
        evalTactic (← `(tactic|
          (composition_tick "precondition start"
           leaner_call_arguments
           composition_tick "arguments decoded"
           leaner_call_native_requires
           simp only [Contract.runtime, Contract.typed, Obligation_iff, $facts,*]
           dsimp (config := { failIfUnchanged := false }) only
           composition_tick "requires simplified"
           simp (config := { failIfUnchanged := false })
             (disch := first | assumption | omega) only [fresh_resumed]
           composition_tick "freshness simplified"
           first | leaner_certified_close! | leaner_close_normalized [$facts,*]
           composition_tick "precondition done")))
        unless (← getGoals).isEmpty do throwError "callee precondition remains unproved"
        setGoals [returned]
        let results := mkIdent (← mkFreshUserName `callResults)
        let final := mkIdent (← mkFreshUserName `callFinal)
        let frame := mkIdent (← mkFreshUserName `callFrame)
        let notMust := mkIdent (← mkFreshUserName `callNotMust)
        let established := mkIdent (← mkFreshUserName `callPost)
        evalTactic (← `(tactic|
          (intro $results:ident $final:ident $established:ident $frame:ident $notMust:ident
           simp only [Contract.runtime, Contract.typed, Obligation_iff,
             SemanticOperations.resolveReturnedBorrows_empty, $facts,*]
             at $established:ident $frame:ident $notMust:ident
           simp (config := { failIfUnchanged := false }) at $notMust:ident
           leaner_cases $established:ident
           all_goals leaner_cases $frame:ident
           all_goals leaner_call_results
           all_goals composition_tick "results exposed"
           all_goals
             (leaner_call_normalize []
              composition_tick "continuation normalized"
              ))))
        let continuations ← getGoals
        for continuation in continuations do close continuation fuel
        setGoals [thrown]
        let kind := mkIdent (← mkFreshUserName `callKind)
        let payload := mkIdent (← mkFreshUserName `callThrown)
        let aborted := mkIdent (← mkFreshUserName `callAborts)
        evalTactic (← `(tactic|
          (intro $kind:ident $payload:ident $final:ident $aborted:ident
           simp only [Contract.runtime, Contract.typed, Obligation_iff, $facts,*]
             at $aborted:ident
           simp (config := { failIfUnchanged := false }) only
             [and_false, false_and, exists_false] at $aborted:ident
           all_goals
            (
           leaner_cases $aborted:ident
           all_goals
             (leaner_call_normalize [])))))
        for continuation in ← getGoals do close continuation fuel
      else if (target.find? (fun e => e.isConstOf ``wpFunction || e.isConstOf ``loop ||
          (e.isAppOfArity ``wp 7 && (e.getArg! 3).isAppOfArity ``ite 5))).isSome then
        if target.isForall then
          let (_, next) ← goal.intro1P
          close next fuel
        else if target.isAppOfArity ``And 2 then
          for next in ← goal.apply (mkConst ``And.intro) do close next fuel
        else
          goal.withContext do
            throwError "composition cannot expose call:\n{← ppGoal goal}"
      else
        if !loops.isEmpty then
          trace[leaner.composition] "loop closing start: {← IO.getNumHeartbeats}"
          evalTactic (← `(tactic|
            (simp (config := { failIfUnchanged := false }) only
               [$invariants,*, RowState.frame, RowState.ofFrame, rowFrame,
                lir_data_norm, List.getElem?_toArray, List.getElem?_cons_zero,
                List.getElem?_cons_succ, Option.join_some, Bool.false_eq_true,
                ite_true, ite_false, $facts,*])))
          trace[leaner.composition] "loop closing simplified: {← IO.getNumHeartbeats}"
          for residual in ← getGoals do
            trace[leaner.loopResidue] "loop closing residue:\n{← residual.withContext do ppExpr (← instantiateMVars (← residual.getType))}"
          evalTactic (← `(tactic| leaner_certified_close!))
          trace[leaner.composition] "loop closing done: {← IO.getNumHeartbeats}"
        else evalTactic (← `(tactic|
          (simp (config := { failIfUnchanged := false }) only [$facts,*]
           first
           | leaner_certified_close!
           | leaner_close_normalized [$facts,*])))
        unless (← getGoals).isEmpty do throwError "composition left an unproved continuation"
    let goals ← getGoals
    for goal in goals do close goal 1024
    setGoals []

end LeanerIR.Proofs.Denotation.RowSpec
