-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.SharedContinuations

namespace VerexIR.Tests.StageFamilyRecovery

open Lean Meta Elab Tactic
open VerexIR VerexIR.Proofs VerexIR.Proofs.Denote
open VerexIR.Proofs.Denote.SharedContinuations

private abbrev stage (θ : TypeArgs) (outer : Carriers) : Carriers :=
  { Carriers.instantiate θ outer with declared := fun _ _ _ _ => False }

private abbrev installed (θ : TypeArgs) (outer : Carriers) : Carriers :=
  { stage θ outer with declared := fun _ _ _ raw => raw = .bool true }

/-- Check the index of the actual summary premise before closing and
certifying the join. Definitional equality of the Holds propositions
cannot distinguish stages while K2 still uses K1's validity reading. -/
elab "check_join_family " head:ident family:term " capturing " capture:ident
    " pending " pending:num " : " incoming:tacticSeq " with " suffix:tacticSeq : tactic => do
  let saved ← Lean.Elab.Tactic.saveState
  let erasedOrigin := head.getId == `erasedCapture
  let expectedHead ← if erasedOrigin then pure .anonymous else realizeGlobalConstNoOverloadWithInfo head
  let expected ← withMainContext do
    instantiateMVars (← elabTerm family (some (mkConst ``Carriers)))
  let opaqueDefinitions ← withMainContext do
    if capture.getId == `noCapture then pure {}
    else pure (({} : Std.HashSet FVarId).insert (← getFVarId capture))
  let some (folded, node) ← fold? (← getMainGoal) opaqueDefinitions
    | throwError "expected a Flow continuation"
  setGoals [folded]
  evalTactic incoming
  let mut node := node
  for goal in ← getGoals do
    let some edge ← arrival? node goal | throwError "expected a normal arrival"
    node := { node with arrivals := node.arrivals.push edge }
  let sealedNode ← sealNode (← closeArrivals (← prepareArrivals node {}))
  if erasedOrigin then
    unless sealedNode.sealed && sealedNode.finished && !sealedNode.certified &&
        sealedNode.summary?.isNone && sealedNode.relation?.isNone &&
        sealedNode.holdsProofs.isEmpty && sealedNode.pendingHolds.isEmpty &&
        sealedNode.holdsCount == 0 && sealedNode.arrivals.size == 2 do
      throwError "an erased capture should resume both arrivals without a shared certificate"
    for arrival in sealedNode.arrivals do
      let goal ← unfoldArrival sealedNode arrival
      setGoals [goal]
      evalTactic suffix
      unless (← getUnsolvedGoals).isEmpty do throwError "unshared suffix incomplete"
    return
  unless sealedNode.holdsCount == 1 do
    throwError "expected one mandatory Holds premise, got {sealedNode.holdsCount}"
  let some summary := sealedNode.summary? | throwError "expected a summary"
  let (_, goal) ← summary.introNP sealedNode.summaryArguments.size
  goal.withContext do
    let .forallE _ premise _ _ := (← instantiateMVars (← goal.getType)).consumeMData
      | throwError "expected a mandatory Holds premise in the summary"
    let premise := premise.consumeMData
    unless premise.isAppOfArity expectedHead 3 do
      throwError "expected {expectedHead} in the summary, got {premise}"
    let actual := (premise.getArg! 0).consumeMData
    unless actual == expected.consumeMData do
      throwError "join selected the wrong semantic family: {actual}, expected {expected}"
  if pending.getNat > 0 then
    unless sealedNode.pendingHolds.size == pending.getNat do
      throwError "expected {pending.getNat} pending Holds obligations, got {sealedNode.pendingHolds.size}"
    if (← finish? sealedNode).isSome then
      throwError "a join with pending Holds obligations was certified"
    saved.restore
    return
  unless sealedNode.pendingHolds.isEmpty do
    throwError "the join still owes {sealedNode.pendingHolds.size} Holds obligations"
  let (_, goal) ← goal.introNP (sealedNode.hypotheses + 1)
  let goal ← VerexIR.Proofs.Denote.SharedContinuations.unfold sealedNode goal
  setGoals [goal]
  evalTactic suffix
  unless (← getUnsolvedGoals).isEmpty do throwError "shared summary incomplete"
  let some finished ← finish? sealedNode | throwError "summary has unresolved dependencies"
  unless finished.finished && finished.certified do throwError "summary not certified"
  setGoals []

section Controls

variable {unit : Validation.ValidatedUnit} [FunctionCarriers unit] {ρ : ResultShape} {Γ : NRow}

-- The returned value is spelled at the pre-install parent's expression.
-- Its producer is the installed Flow.
example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @NTy.carrier (stage θ outer).toSorts (.param 0))
    (hLeft : @NTy.Holds (installed θ outer) (.param 0) left)
    (hRight : @NTy.Holds (installed θ outer) (.param 0) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (@NTy.carrier (stage θ outer).toSorts (.param 0)))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family NTy.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

-- The same producer remains authoritative when the parent's projection
-- has reduced to the standalone operational instantiation.
example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @NTy.carrier (Sorts.instantiate θ outer.toSorts) (.param 0))
    (hLeft : @NTy.Holds (installed θ outer) (.param 0) left)
    (hRight : @NTy.Holds (installed θ outer) (.param 0) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (@NTy.carrier (Sorts.instantiate θ outer.toSorts) (.param 0)))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family NTy.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

-- Opaque results exercise each whole-value Holds collector.
example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @HEnv (stage θ outer).toSorts (.cons (.param 0) .nil))
    (hLeft : @HEnv.Holds (installed θ outer) (.cons (.param 0) .nil) left)
    (hRight : @HEnv.Holds (installed θ outer) (.cons (.param 0) .nil) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (@HEnv (stage θ outer).toSorts (.cons (.param 0) .nil)))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family HEnv.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @HList (stage θ outer).toSorts (.cons (.param 0) .nil))
    (hLeft : @HList.Holds (installed θ outer) (.cons (.param 0) .nil) left)
    (hRight : @HList.Holds (installed θ outer) (.cons (.param 0) .nil) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (@HList (stage θ outer).toSorts (.cons (.param 0) .nil)))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family HList.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : Option (@NTy.carrier (stage θ outer).toSorts (.param 0)))
    (hLeft : @Slot.Holds (installed θ outer) (.param 0) left)
    (hRight : @Slot.Holds (installed θ outer) (.param 0) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (Option (@NTy.carrier (stage θ outer).toSorts (.param 0))))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family Slot.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

example (θ : TypeArgs) (outer : Carriers) (b : Bool) (result : ResultShape)
    (left right : @ResultShape.carrier (stage θ outer).toSorts result)
    (hLeft : @ResultShape.Holds (installed θ outer) result left)
    (hRight : @ResultShape.Holds (installed θ outer) result right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (@ResultShape.carrier (stage θ outer).toSorts result))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family ResultShape.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

-- A captured value in the suffix keeps the family that produced it, even
-- when the unrelated Flow uses an installed family with that same parent.
example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @NTy.carrier (stage θ outer).toSorts (.param 0))
    (hLeft : @NTy.Holds (stage θ outer) (.param 0) left)
    (hRight : @NTy.Holds (stage θ outer) (.param 0) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ Unit)
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ Unit () env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ Unit () env))
      (fun _ _ => @NTy.Holds (stage θ outer) (.param 0) (if b then left else right))
      (fun (_ : Failure) => False) state := by
  let picked : @NTy.carrier (stage θ outer).toSorts (.param 0) := if b then left else right
  have pickedHolds : @NTy.Holds (stage θ outer) (.param 0) picked := by
    cases b <;> simp only [picked, Bool.false_eq_true, if_false, if_true]
    all_goals assumption
  change wp _ (fun _ _ => @NTy.Holds (stage θ outer) (.param 0) picked) _ _
  check_join_family NTy.Holds (stage θ outer) capturing picked pending 0 :
    cases b <;> simp only [picked, Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

-- Even a bare parent equal to the installed parent cannot identify the
-- origin of a captured value. We can still prove this example without joining.
example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @NTy.carrier (Sorts.instantiate θ outer.toSorts) (.param 0))
    (hLeft : @NTy.Holds (installed θ outer) (.param 0) left)
    (hRight : @NTy.Holds (installed θ outer) (.param 0) right)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ Unit)
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ Unit () env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ Unit () env))
      (fun _ _ => @NTy.Holds (installed θ outer) (.param 0) (if b then left else right))
      (fun (_ : Failure) => False) state := by
  let picked : @NTy.carrier (Sorts.instantiate θ outer.toSorts) (.param 0) :=
    if b then left else right
  change wp _ (fun _ _ => @NTy.Holds (installed θ outer) (.param 0) picked) _ _
  check_join_family erasedCapture (installed θ outer) capturing picked pending 0 :
    cases b <;> simp only [picked, Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    assumption

private abbrev foreign (θ : TypeArgs) (outer : Carriers) : Carriers :=
  { stage θ outer with holds := fun _ _ => True }

-- The same representation permits the value to cross, but the join owes
-- validity at the receiving stage. An explicit entry premise pays it.
example (θ : TypeArgs) (outer : Carriers) (b : Bool)
    (left right : @NTy.carrier (foreign θ outer).toSorts (.param 0))
    (hLeft : @NTy.Holds (foreign θ outer) (.param 0) left)
    (hRight : @NTy.Holds (foreign θ outer) (.param 0) right)
    (transport : ∀ value : @NTy.carrier (foreign θ outer).toSorts (.param 0),
      @NTy.Holds (foreign θ outer) (.param 0) value →
        @NTy.Holds (installed θ outer) (.param 0) value)
    (env : @HEnv (installed θ outer).toSorts Γ) (state : Memory unit) :
    wp (Result := @Flow (installed θ outer) ρ Γ
        (@NTy.carrier (foreign θ outer).toSorts (.param 0)))
      (if b then Spec.pure (@Flow.value (installed θ outer) ρ Γ _ left env)
        else Spec.pure (@Flow.value (installed θ outer) ρ Γ _ right env))
      (fun _ _ => True) (fun (_ : Failure) => False) state := by
  check_join_family NTy.Holds (installed θ outer) capturing noCapture pending 2 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial
  have installedLeft := transport left hLeft
  have installedRight := transport right hRight
  check_join_family NTy.Holds (installed θ outer) capturing noCapture pending 0 :
    cases b <;> simp only [Bool.false_eq_true, if_false, if_true, wp_pure]
  with
    trivial

end Controls

end VerexIR.Tests.StageFamilyRecovery
