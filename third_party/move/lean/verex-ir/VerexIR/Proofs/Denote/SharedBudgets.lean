-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-! Invocation-local automatic allowances. The ledger lives outside tactic
snapshots: restoring a failed attempt cannot refund its work. -/

namespace VerexIR.Proofs.Denote.SharedBudgets

open Lean Meta Elab Tactic

abbrev OwnerId := Nat

/-- Work done before a completed region exposes its clause owners. -/
abbrev DebitId := Nat

inductive Work where
  | preparation
  | leaf
  deriving BEq, Inhabited

structure Owner where
  /-- Zero preserves the unlimited-leaf setting. -/
  limit : Nat
  spent : Nat := 0
  preparationSpent : Nat := 0
  deriving Inhabited

def Owner.remaining (owner : Owner) : Option Nat :=
  if owner.limit == 0 then none else some (owner.limit - owner.spent)

def Owner.preparationRemaining (owner : Owner) : Nat :=
  (if owner.limit == 0 then 1000000 else min 1000000 (owner.limit / 20)) -
    owner.preparationSpent

structure Ledger where
  owners : Array Owner := #[]
  /-- Immutable costs; descendants inherit identities, so joins do not
  debit a shared ancestor twice. They are not additional leaf allowances. -/
  deferred : Array (Nat × Nat) := #[]
  /-- Physical work is counted once, independently of overlapping debits. -/
  physical : Nat := 0
  deriving Inhabited

def Ledger.allocate (ledger : Ledger) (limit : Nat) : Ledger × OwnerId :=
  ({ ledger with owners := ledger.owners.push { limit } }, ledger.owners.size)

/-- Construct an allowance after all distinct earlier debits. Missing
identities fail closed rather than silently granting an uncharged attempt. -/
def Ledger.afterDebits (ledger : Ledger) (limit : Nat) (debits : Array DebitId) : Option Owner := do
  let mut owner : Owner := { limit }
  for id in debits.toList.eraseDups do
    let (spent, preparationSpent) ← ledger.deferred[id]?
    owner := { owner with
      spent := owner.spent + spent
      preparationSpent := owner.preparationSpent + preparationSpent }
  return owner

def Ledger.allocateAfter (ledger : Ledger) (limit : Nat) (debits : Array DebitId) :
    Option (Ledger × OwnerId) := do
  let owner ← ledger.afterDebits limit debits
  return ({ ledger with owners := ledger.owners.push owner }, ledger.owners.size)

/-- Record physical work once. Allocating each downstream clause later
inherits its debit without adding the work again to the physical total. -/
def Ledger.defer (ledger : Ledger) (work : Work) (cost : Nat) : Ledger × DebitId :=
  ({ ledger with
    deferred := ledger.deferred.push (cost, if work == .preparation then cost else 0)
    physical := ledger.physical + cost }, ledger.deferred.size)

private def intersect (left right : Option Nat) : Option Nat :=
  match left, right with
  | none, limit | limit, none => limit
  | some left, some right => some (min left right)

/-- Every participating owner must have headroom. Unknown or empty owner
sets fail closed; duplicated identities never create another allowance. -/
def Ledger.allowance (ledger : Ledger) (owners : Array OwnerId) (work : Work) : Option Nat :=
  if owners.isEmpty then some 0 else owners.foldl (init := none) fun limit id =>
    match ledger.owners[id]? with
    | none => some 0
    | some owner => intersect limit <| match work with
      | .leaf => owner.remaining
      | .preparation => intersect owner.remaining (some owner.preparationRemaining)

/-- Charge each distinct owner once, including after an unsuccessful attempt. -/
def Ledger.charge (ledger : Ledger) (owners : Array OwnerId) (work : Work) (cost : Nat) : Ledger :=
  let charged := owners.toList.eraseDups.foldl (init := ledger.owners) fun all id =>
    all.modify id fun owner => { owner with
      spent := owner.spent + cost
      preparationSpent := owner.preparationSpent + (if work == .preparation then cost else 0) }
  { ledger with owners := charged, physical := ledger.physical + cost }

inductive Outcome (α : Type) where
  | done (value : α)
  | exhausted
  | failed (exception : Exception)

private def checkTarget (context : Core.Context) : TacticM Unit := do
  if context.maxHeartbeats != 0 &&
      (← IO.getNumHeartbeats) - context.initHeartbeats >= context.maxHeartbeats then
    Core.throwMaxHeartbeat `verex `maxHeartbeats context.maxHeartbeats

/-- Run one physical operation, bounded by all participating owners and the
original target deadline. A shared-slice failure leaves the remaining leaf
allowance available. Only tactic state is rolled back on failure. -/
def run (ledger : IO.Ref Ledger) (owners : Array OwnerId) (work : Work)
    (action : TacticM α) : TacticM (Outcome α × Nat) := do
  let context ← readThe Core.Context
  checkTarget context
  let start ← IO.getNumHeartbeats
  let allowance := (← ledger.get).allowance owners work
  if allowance == some 0 then return (.exhausted, 0)
  let saved ← saveState
  let now ← IO.getNumHeartbeats
  let available := allowance.map (· - (now - start))
  let target := if context.maxHeartbeats == 0 then none
    else some (context.maxHeartbeats - (now - context.initHeartbeats))
  let limit := intersect available target
  let outcome ← if limit == some 0 then pure .exhausted else
    tryCatchRuntimeEx
      (do
        let value ← match limit with
          | none => action
          | some limit =>
            withTheReader Core.Context
              (fun context => { context with initHeartbeats := now, maxHeartbeats := limit }) action
        pure (.done value))
      (fun exception => pure <| if exception.isMaxHeartbeat then .exhausted else .failed exception)
  let cost := (← IO.getNumHeartbeats) - start
  ledger.modify (·.charge owners work cost)
  -- Tactics may catch a nested timeout and return normally. Their physical
  -- work still exhausts this allowance; do not commit such an attempt.
  let outcome := if allowance.any (cost >= ·) then .exhausted else outcome
  match outcome with
  | .done _ => pure ()
  | _ => saved.restore
  -- The enclosing deadline is a target failure, never an ordinary leaf or
  -- shared-slice exhaustion, even when the nested cap caught its exception.
  checkTarget context
  return (outcome, cost)

end VerexIR.Proofs.Denote.SharedBudgets
