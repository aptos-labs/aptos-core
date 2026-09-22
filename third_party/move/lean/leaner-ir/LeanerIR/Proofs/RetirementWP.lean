-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerIR.Proofs.OperationWP

/-! Stage loan retirement at its operation boundaries. Each certificate
preserves the evaluator's ordered search; no local layout is prescribed. -/

namespace LeanerIR.Proofs.Denotation.RowSpec

open Lean Meta LeanerIR.SemanticOperations

private def clearingBorrowIndex (frame : RuntimeFrame) (loan : Nat) : Option Nat :=
  frame.locals.toList.findIdx? fun slot =>
    (slot.bind (rewriteFirst (borrowClear? loan))).isSome

private def clearLocalCertificate? (frame state loan : Lean.Expr) : SimpM (Option Simp.Result) := do
  let found ← simplifyGround ``clearingBorrowIndex (← mkAppM ``clearingBorrowIndex #[frame, loan])
  unless found.expr.isAppOfArity ``Option.some 2 do return none
  let slot ← mkAppM ``LocalId.mk #[found.expr.getArg! 1]
  let read ← simplifyGround ``readLocal? (← mkAppM ``readLocal? #[frame, slot])
  unless read.expr.isAppOfArity ``Option.some 2 do return none
  let value := read.expr.getArg! 1
  unless value.isAppOfArity ``RuntimeValue.borrow 2 do return none
  unless ← isDefEq (value.getArg! 0) loan do return none
  let entry ← mkAppM ``readLocal_entry #[← read.getProof]
  let proof ← withTransparency .all <| mkAppM ``clearBorrowValue_local
    #[frame, state, slot, loan, value.getArg! 1, ← found.getProof, entry]
  let some (_, _, rhs) := (← inferType proof).eq? | return none
  let result : Simp.Result := { expr := rhs, proof? := some proof }
  some <$> result.mkEqTrans (← Simp.simp rhs)

private def retirementFrame (frame : RuntimeFrame) (lexical : LoanId) : RuntimeFrame :=
  { frame with activeLoans := frame.activeLoans.filter (·.1 != ⟨lexical.index⟩) }

private def retirementEntry (frame : RuntimeFrame) (lexical : LoanId) : Option (ExprId × Nat) :=
  frame.activeLoans.toList.find? (·.1 == ⟨lexical.index⟩)

private theorem retirementEntry_eq (frame : RuntimeFrame) (lexical : LoanId) :
    frame.activeLoans.find? (·.1 == ⟨lexical.index⟩) = retirementEntry frame lexical := by
  rw [← Array.find?_toList]
  rfl

def retireLoan (lexical : LoanId) (frame : RuntimeFrame) (state : RuntimeState) :
    RuntimeFrame × RuntimeState :=
  match frame.activeLoans.find? (·.1 == ⟨lexical.index⟩) with
  | none => (frame, state)
  | some (_, loan) =>
      let frame := retirementFrame frame lexical
      match findBorrowValue? frame state loan with
      | some current =>
          let (frame, state) := clearBorrowValue frame state loan
          applyWriteBack frame state loan current
      | none => (frame, state)

theorem endLoans_fold (loans : List LoanId) (arguments : Array RuntimeValue)
    (frame : RuntimeFrame) (state : RuntimeState) :
    endLoans? loans.toArray arguments frame state =
      let retired := loans.foldr (fun lexical state => retireLoan lexical state.1 state.2) (frame, state)
      match arguments.toList with
      | [value] => some (retired.1, retired.2, value)
      | [] => some (retired.1, retired.2, .unit)
      | _ => none := by
  simp only [endLoans?, List.foldr_toArray', retireLoan, retirementFrame]
  congr 4 <;> funext lexical pair <;> cases pair <;> rfl

private theorem retireLoan_inactive (lexical : LoanId) (frame : RuntimeFrame) (state : RuntimeState)
    (inactive : frame.activeLoans.find? (·.1 == ⟨lexical.index⟩) = none) :
    retireLoan lexical frame state = (frame, state) := by
  simp only [retireLoan, inactive]

private theorem retireLoan_certified (lexical : LoanId) (frame : RuntimeFrame) (state : RuntimeState)
    (entry : ExprId × Nat) (filtered : RuntimeFrame) (current : RuntimeValue)
    (cleared written : RuntimeFrame × RuntimeState)
    (active : frame.activeLoans.find? (·.1 == ⟨lexical.index⟩) = some entry)
    (filter : retirementFrame frame lexical = filtered)
    (found : findBorrowValue? filtered state entry.2 = some current)
    (clear : clearBorrowValue filtered state entry.2 = cleared)
    (write : applyWriteBack cleared.1 cleared.2 entry.2 current = written) :
    retireLoan lexical frame state = written := by
  simp only [retireLoan, active, filter, found, clear, write]

private def retirementCertificate? (lexical frame state : Lean.Expr) : SimpM (Option Lean.Expr) := do
  trace[leaner.normalize] "retirement certificate start: {← IO.getNumHeartbeats}"
  let entry ← simplifyGround ``retirementEntry (← mkAppM ``retirementEntry #[frame, lexical])
  let active ← mkEqTrans (← mkAppM ``retirementEntry_eq #[frame, lexical]) (← entry.getProof)
  if entry.expr.isAppOfArity ``Option.none 1 then
    return some (← mkAppM ``retireLoan_inactive #[lexical, frame, state, active])
  unless entry.expr.isAppOfArity ``Option.some 2 do return none
  let entryValue := entry.expr.getArg! 1
  let loan ← mkAppM ``Prod.snd #[entryValue]
  let filtered ← simplifyGround ``retirementFrame (← mkAppM ``retirementFrame #[frame, lexical])
  trace[leaner.normalize] "retirement certificate filtered: {← IO.getNumHeartbeats}"
  let found ← simplifyGround ``findBorrowValue?
    (← mkAppM ``findBorrowValue? #[filtered.expr, state, loan])
  unless found.expr.isAppOfArity ``Option.some 2 do return none
  let current := found.expr.getArg! 1
  trace[leaner.normalize] "retirement certificate found: {← IO.getNumHeartbeats}"
  let cleared : Simp.Result ← match ← clearLocalCertificate? filtered.expr state loan with
    | some cleared => pure cleared
    | none => do
      simplifyGround ``clearBorrowValue (← mkAppM ``clearBorrowValue #[filtered.expr, state, loan])
  unless cleared.expr.isAppOfArity ``Prod.mk 4 do return none
  trace[leaner.normalize] "retirement certificate cleared: {← IO.getNumHeartbeats}"
  let written ← simplifyGround ``applyWriteBack
    (← mkAppM ``applyWriteBack #[cleared.expr.getArg! 2, cleared.expr.getArg! 3, loan, current])
  unless written.expr.isAppOfArity ``Prod.mk 4 do return none
  trace[leaner.normalize] "retirement certificate written: {← IO.getNumHeartbeats}"
  -- Every parameter is explicit and already known. Assemble the application
  -- directly; elaborator unification would reduce the same symbolic state
  -- again for each premise. The kernel still checks the entire application.
  return some (mkAppN (mkConst ``retireLoan_certified)
    #[lexical, frame, state, entryValue, filtered.expr, current, cleared.expr, written.expr,
      active, ← filtered.getProof, ← found.getProof, ← cleared.getProof, ← written.getProof])

simproc [lir_eval] evalRetireLoan (retireLoan _ _ _) := fun e => do
  unless isLiteralFrame (e.getArg! 1) do return .continue
  let some proof ← retirementCertificate? (e.getArg! 0) (e.getArg! 1) (e.getArg! 2)
    | return .continue
  let some (_, _, result) := (← inferType proof).eq? | return .continue
  -- The final writeback has already normalized the resulting state.
  return .done { expr := result, proof? := some proof }

simproc ↓ [lir_eval, lir_call_eval] evalReferenceRetirement
    (ReferenceLocationOperation.evaluate? _ _ _ _) := fun e => do
  let operation ← withTransparency .default <| whnf (e.getArg! 0).consumeMData
  unless operation.isAppOfArity ``ReferenceLocationOperation.endLoan 1 do return .continue
  let loans := operation.getArg! 0
  unless loans.isAppOfArity ``Array.mk 2 || loans.isAppOfArity ``List.toArray 2 do return .continue
  let some entries := literalList? (loans.getArg! 1) | return .continue
  -- Even a single vector loan benefits from staging its search, clear, and
  -- writeback once; direct certificate assembly keeps that path inexpensive.
  unless !entries.isEmpty do return .continue
  try
    let started ← IO.getNumHeartbeats
    let equation ← mkAppM ``endLoans_fold
      #[loans.getArg! 1, e.getArg! 1, e.getArg! 2, e.getArg! 3]
    let some (_, _, rhs) := (← inferType equation).eq? | return .continue
    let result ← Simp.simp rhs
    unless result.expr.isAppOfArity ``Option.some 2 do return .continue
    if (result.expr.find? fun term =>
        term.isConstOf ``retireLoan || term.isConstOf ``forIn).isSome then return .continue
    let proof ← mkAppM ``endLoan_evaluate_of_endLoans
      #[← mkEqTrans equation (← result.getProof)]
    let some (_, lhs, rhs) := (← inferType proof).eq? | return .continue
    unless ← withTransparency .default <| isDefEq lhs e do return .continue
    trace[leaner.normalize] "staged retirement: {(← IO.getNumHeartbeats) - started}"
    return .visit { expr := rhs, proof? := some proof }
  catch error =>
    trace[leaner.normalize] "staged retirement fallback: {error.toMessageData}"
    return .continue

end LeanerIR.Proofs.Denotation.RowSpec
