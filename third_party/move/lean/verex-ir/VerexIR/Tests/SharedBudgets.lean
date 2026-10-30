-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.SharedBudgets

open VerexIR.Proofs.Denote.SharedBudgets

private def initial : Ledger :=
  ((({} : Ledger).allocate 20000000).1.allocate 20000000).1

-- A shared operation costs the target once and every benefiting clause once.
private def prepared := initial.charge #[0, 1, 0] .preparation 800000
#guard prepared.physical == 800000
#guard prepared.allowance #[0, 1] .leaf == some 19200000
#guard prepared.allowance #[0, 1] .preparation == some 200000

-- A clause's cases, retries and fallback retain that clause's owner.
private def firstCase := prepared.charge #[0] .leaf 4000000
private def failedCase := firstCase.charge #[0] .leaf 300000
#guard failedCase.allowance #[0] .leaf == some 14900000
#guard failedCase.allowance #[1] .leaf == some 19200000
#guard failedCase.physical == 5100000

-- Slice exhaustion does not exhaust the leaf; owner exhaustion blocks only
-- that owner, and a group cannot use a sibling's remaining allowance for it.
private def sliceSpent := prepared.charge #[0, 1] .preparation 200000
#guard sliceSpent.allowance #[0, 1] .preparation == some 0
#guard sliceSpent.allowance #[0, 1] .leaf == some 19000000
private def exhausted := sliceSpent.charge #[0] .leaf 19000000
#guard exhausted.allowance #[0, 1] .leaf == some 0
#guard exhausted.allowance #[1] .leaf == some 19000000

-- Unlimited leaves retain the internal slice; tiny leaves omit preparation.
#guard (({} : Ledger).allocate 0).1.allowance #[0] .leaf == none
#guard (({} : Ledger).allocate 0).1.allowance #[0] .preparation == some 1000000
#guard (({} : Ledger).allocate 19).1.allowance #[0] .preparation == some 0
#guard initial.allowance #[] .leaf == some 0
#guard initial.allowance #[10] .leaf == some 0

-- Earlier feasibility proofs debit every downstream clause. A shared
-- ancestor reaching the join twice is charged once per clause, and copying
-- the debit does not count that physical work a second time.
private def deferredFirst := ({} : Ledger).defer .preparation 300000
private def deferredSecond := deferredFirst.1.defer .preparation 200000
private def inherited := deferredSecond.1.allocateAfter 20000000 #[0, 1, 0]
#guard inherited.isSome
#guard inherited.get!.1.allowance #[0] .leaf == some 19500000
#guard inherited.get!.1.allowance #[0] .preparation == some 500000
private def inheritedSibling := inherited.get!.1.allocateAfter 20000000 #[1, 0]
#guard inheritedSibling.get!.1.allowance #[0, 1] .leaf == some 19500000
#guard inheritedSibling.get!.1.physical == 500000
private def afterLeaf := inherited.get!.1.charge #[0] .leaf 100000
#guard (afterLeaf.allocateAfter 20000000 #[0, 1]).get!.1.allowance #[1] .leaf == some 19500000
#guard (afterLeaf.allocateAfter 20000000 #[0, 1]).get!.1.physical == 600000
#guard (deferredSecond.1.allocateAfter 20000000 #[2]).isNone
#guard ((deferredSecond.1.afterDebits 400000 #[0, 1]).get!).remaining == some 0
#guard ((deferredSecond.1.afterDebits 0 #[0, 1]).get!).preparationRemaining == 500000

-- Failure rolls back goal assignments, but the measured debit remains.
open Lean Meta Elab Tactic in
example : True := by
  run_tac
    let ledger ← IO.mkRef initial
    let original ← getMainGoal
    let (result, cost) ← VerexIR.Proofs.Denote.SharedBudgets.run (α := Unit) ledger #[0, 1] .preparation do
      original.assign (mkConst ``True.intro)
      throwError "deliberate failed attempt"
    match result with
    | .failed _ => pure ()
    | _ => throwError "the failed attempt was accepted"
    if ← original.isAssigned then throwError "a failed assignment escaped rollback"
    unless cost > 0 && (← ledger.get).owners[0]!.spent == cost &&
        (← ledger.get).owners[1]!.spent == cost && (← ledger.get).physical == cost do
      throwError "failed preparation was refunded or counted twice"
  trivial

-- Successful primitives commit before the next primitive starts. Its
-- failure restores only its own changes, retaining both the earlier fact
-- and the cumulative charges against every participating clause.
open Lean Meta Elab Tactic in
example : True := by
  run_tac
    let ledger ← IO.mkRef initial
    let original ← getMainGoal
    let (first, firstCost) ← VerexIR.Proofs.Denote.SharedBudgets.run ledger #[0, 1] .preparation do
      let next ← original.assert `preparedFact (mkConst ``True) (mkConst ``True.intro)
      let (fact, next) ← next.intro1P
      setGoals [next]
      return (fact, next)
    let .done (fact, prepared) := first | throwError "the first primitive did not commit"
    let (second, secondCost) ← VerexIR.Proofs.Denote.SharedBudgets.run (α := Unit) ledger #[0, 1] .preparation do
      let cleared ← prepared.clear fact
      setGoals [cleared]
      cleared.assign (mkConst ``True.intro)
      throwError "deliberate failure of a later primitive"
    match second with
    | .failed _ => pure ()
    | _ => throwError "the failed second primitive was accepted"
    unless (← getGoals) == [prepared] && !(← prepared.isAssigned) do
      throwError "the second primitive escaped its checkpoint"
    prepared.withContext do
      unless (← getLCtx).contains fact do throwError "an earlier committed fact was discarded"
    let paid := firstCost + secondCost
    unless firstCost > 0 && secondCost > 0 &&
        (← ledger.get).owners[0]!.spent == paid &&
        (← ledger.get).owners[1]!.spent == paid && (← ledger.get).physical == paid do
      throwError "separate primitive checkpoints renewed or refunded the allowance"
  trivial

-- A failed pre-owner check transfers its actual debit after rollback. The
-- inherited preparation slice is cumulative across nested region attempts.
open Lean Meta Elab Tactic in
example : True := by
  run_tac
    let (earlier, first) := ({} : Ledger).defer .preparation 400000
    let allowance := (earlier.afterDebits 20000000 #[first]).get!
    let pending ← IO.mkRef ({ owners := #[allowance] } : Ledger)
    let original ← getMainGoal
    let (outcome, cost) ← VerexIR.Proofs.Denote.SharedBudgets.run (α := Unit) pending #[0] .preparation do
      original.assign (mkConst ``True.intro)
      throwError "deliberate failed feasibility check"
    match outcome with
    | .failed _ => pure ()
    | _ => throwError "a failed feasibility check was accepted"
    if ← original.isAssigned then throwError "a feasibility assignment escaped rollback"
    let (paid, second) := earlier.defer .preparation cost
    let inherited := (paid.allocateAfter 20000000 #[first, second, first]).get!.1
    unless cost > 0 && inherited.owners[0]!.spent == 400000 + cost &&
        inherited.physical == 400000 + cost &&
        inherited.allowance #[0] .preparation == some (600000 - cost) do
      throwError "failed pre-owner work was refunded or counted more than once"
  trivial

-- Exhausted owners do not start work; a shared-slice timeout leaves the
-- clause's remaining allowance available to its ordinary leaf pipeline.
open Lean Meta Elab Tactic in
example : True := by
  run_tac
    let ledger ← IO.mkRef exhausted
    let ran ← IO.mkRef false
    let (result, cost) ← VerexIR.Proofs.Denote.SharedBudgets.run ledger #[0] .leaf do
      ran.set true
    match result with
    | .exhausted => pure ()
    | _ => throwError "an exhausted owner ran work"
    unless cost == 0 && !(← ran.get) do throwError "zero allowance started an operation"
    let ledger ← IO.mkRef (({} : Ledger).allocate 20000).1
    let (result, _) ← VerexIR.Proofs.Denote.SharedBudgets.run (α := Unit) ledger #[0] .preparation do
      repeat Core.checkMaxHeartbeats "shared budget test"
    match result with
    | .exhausted => pure ()
    | _ => throwError "the shared slice did not stop its operation"
    unless ((← ledger.get).allowance #[0] .leaf).any (· > 0) do
      throwError "shared-slice exhaustion consumed the whole leaf"
  trivial

-- A nested allowance must never turn the enclosing target's timeout into
-- an ordinary leaf failure or grant another heartbeat after that deadline.
open Lean Meta Elab Tactic in
example : True := by
  run_tac
    let ledger ← IO.mkRef initial
    let now ← IO.getNumHeartbeats
    let timedOut ← tryCatchRuntimeEx
      (withTheReader Core.Context
        (fun context => { context with initHeartbeats := now, maxHeartbeats := 1000 }) do
          discard <| VerexIR.Proofs.Denote.SharedBudgets.run (α := Unit) ledger #[0] .leaf do
            repeat Core.checkMaxHeartbeats "shared target test"
          pure false)
      (fun failure => pure failure.isMaxHeartbeat)
    unless timedOut do throwError "a target timeout became an ordinary attempt failure"
  trivial

-- Catching the nested timeout does not authorize a successful result. The
-- transaction still restores its proof assignment and retains the debit.
open Lean Meta Elab Tactic in
example : True := by
  run_tac
    let original ← getMainGoal
    let ledger ← IO.mkRef (({} : Ledger).allocate 20000).1
    let (result, _) ← VerexIR.Proofs.Denote.SharedBudgets.run ledger #[0] .preparation do
      tryCatchRuntimeEx
        (do repeat Core.checkMaxHeartbeats "caught shared timeout")
        (fun _ => pure ())
      original.assign (mkConst ``True.intro)
    match result with
    | .exhausted => pure ()
    | _ => throwError "catching a timeout committed an over-budget attempt"
    if ← original.isAssigned then throwError "the over-budget assignment escaped rollback"
    unless (← ledger.get).owners[0]!.preparationSpent > 0 do
      throwError "the caught timeout refunded the work"
  trivial
