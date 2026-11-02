-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Contract
import Lean

/-! Cheap closing from the logical content of existing contract facts.
Keep diagnostic markers and the goal's context intact. In particular, a
contract relating a known Boolean result to a condition can refute a later
case before that case prepares the rest of its verification context. -/

namespace VerexIR.Proofs.Denote.StatedFacts

open Lean Meta

private partial def unmarked (type : Lean.Expr) : Lean.Expr :=
  let type := type.consumeMData
  if type.isAppOfArity ``Obligation 4 then unmarked (type.getArg! 3) else type

private structure Facts where
  proofs : Std.HashMap Lean.Expr Lean.Expr := {}
  ordered : Array (Lean.Expr × Lean.Expr) := #[]
  negatives : Array (Lean.Expr × Lean.Expr) := #[]

private def literalProof? (type : Lean.Expr) : MetaM (Option Lean.Expr) := do
  let type := unmarked type
  if type.isConstOf ``True then return some (mkConst ``True.intro)
  if let some (_, left, right) := type.eq? then
    if left == right then return some (← mkEqRefl left)
  return none

private partial def add (proof type : Lean.Expr) (fuel : Nat := 32) :
    StateT Facts MetaM Unit := do
  let type := unmarked type
  if (← get).proofs.contains type then return
  modify fun facts => { facts with
    proofs := facts.proofs.insert type proof
    ordered := facts.ordered.push (type, proof) }
  if let some proposition := type.not? then
    modify fun facts => { facts with negatives := facts.negatives.push (unmarked proposition, proof) }
  if fuel == 0 then return
  if type.isAppOfArity ``And 2 then
    let left := type.getArg! 0
    let right := type.getArg! 1
    add (mkApp3 (mkConst ``And.left) left right proof) left (fuel - 1)
    add (mkApp3 (mkConst ``And.right) left right proof) right (fuel - 1)
  else if let some proposition := type.not? then
    let proposition := unmarked proposition
    if proposition.isAppOfArity ``Or 2 then
      let left := proposition.getArg! 0
      let right := proposition.getArg! 1
      let parts := mkApp2 (mkConst ``And) (mkNot left) (mkNot right)
      let equivalence := mkApp2 (mkConst ``not_or) left right
      add (mkApp4 (mkConst ``Iff.mp) type parts equivalence proof) parts (fuel - 1)
  else if type.isAppOfArity ``Iff 2 then
    let left := type.getArg! 0
    let right := type.getArg! 1
    if let some trivial ← literalProof? left then
      add (mkApp4 (mkConst ``Iff.mp) left right proof trivial) right (fuel - 1)
    if let some trivial ← literalProof? right then
      add (mkApp4 (mkConst ``Iff.mpr) left right proof trivial) left (fuel - 1)

private partial def prove (facts : Facts) (target : Lean.Expr) (fuel : Nat := 32) :
    MetaM (Option Lean.Expr) := do
  let target := unmarked target
  if let some proof := facts.proofs[target]? then return some proof
  for (type, proof) in facts.ordered do
    if ← withReducible (isDefEq target type) then return some proof
  if fuel > 0 && target.isAppOfArity ``And 2 then
    let left := target.getArg! 0
    let right := target.getArg! 1
    if let some leftProof ← prove facts left (fuel - 1) then
      if let some rightProof ← prove facts right (fuel - 1) then
        return some (mkApp4 (mkConst ``And.intro) left right leftProof rightProof)
  return none

/-- Prove a stated consequence or close a contradicted case, without
normalizing source expressions, introducing binders, or changing the
context. A failed attempt restores meta state. The caller supplies its
existing budget; this helper grants no preparation or search allowance. -/
def close (goal : MVarId) : MetaM Bool := goal.withContext do
  let saved ← saveState
  try
    let collect : StateT Facts MetaM Unit := do
      for decl in ← getLCtx do
        unless decl.isImplementationDetail do
          add decl.toExpr (← instantiateMVars decl.type)
    let (_, facts) ← collect.run {}
    let target ← instantiateMVars (← goal.getType)
    if let some contradiction := facts.proofs[mkConst ``False]? then
      goal.assign (← mkFalseElim target contradiction)
      return true
    for (proposition, negative) in facts.negatives do
      if let some positive := facts.proofs[proposition]? then
        goal.assign (← mkFalseElim target (mkApp negative positive))
        return true
    if let some proof ← prove facts target then
      goal.assign proof
      return true
    saved.restore
    return false
  catch error =>
    saved.restore
    throw error

end VerexIR.Proofs.Denote.StatedFacts
