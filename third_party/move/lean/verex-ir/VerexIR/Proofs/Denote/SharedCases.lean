-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Lean

/-! Select a shared proof case without enumerating a clause's local
binders or changing its source markers. Program branching is handled by the
continuation graph; these candidates belong only to completed proof groups. -/

namespace VerexIR.Proofs.Denote.SharedCases

open Lean Meta

inductive Discriminator where
  | condition (proposition decision : Lean.Expr)
  | variant (value : Lean.Expr)
  deriving BEq

private partial def clauses (target : Lean.Expr) : Array Lean.Expr :=
  let target := target.consumeMData
  if target.isAppOfArity ``And 2 then
    clauses (target.getArg! 0) ++ clauses (target.getArg! 1)
  else #[target]

private def discriminators (target : Lean.Expr) : MetaM (Array Discriminator) := do
  let visit : StateT (Array Discriminator) MetaM Unit := target.forEach fun term => do
    let mut candidates := #[]
    if term.isAppOfArity ``ite 5 || term.isAppOfArity ``dite 5 then
      let proposition := term.getArg! 1
      let decision := term.getArg! 2
      if !proposition.hasLooseBVars && !proposition.hasMVar &&
          !decision.hasLooseBVars && !decision.hasMVar then
        candidates := candidates.push (.condition proposition decision)
    else if term.isAppOfArity ``Option.elim 5 then
      if (term.getArg! 2).isFVar then candidates := candidates.push (.variant (term.getArg! 2))
    else if let some matcher ← matchMatcherApp? term then
      for value in matcher.discrs do
        if value.isFVar then candidates := candidates.push (.variant value)
    for candidate in candidates do
      unless (← get).contains candidate do modify (·.push candidate)
  return (← visit.run #[]).2

private def eligible (candidate : Discriminator) : MetaM Bool := do
  match candidate with
  | .condition proposition decision =>
      let variables := collectFVars (collectFVars {} proposition) decision
      unless variables.fvarIds.all (← getLCtx).contains do return false
      if proposition.isTrue || proposition.isFalse then return false
      let known ← (← getLCtx).anyM fun decl => do
        let type ← instantiateMVars decl.type
        return type == proposition || type == mkNot proposition
      return !known
  | .variant value =>
      unless value.isFVar && (← getLCtx).contains value.fvarId! do return false
      let type ← whnfD (← inferType value)
      unless type.isAppOf ``Option || type.isAppOf ``Sum || type.isConstOf ``Bool do return false
      if ← isConstructorApp (← whnfR value) then return false
      let known ← (← getLCtx).anyM fun decl => do
        let some (_, left, right) := (← instantiateMVars decl.type).eq? | return false
        if left == value then isConstructorApp (← whnfR right)
        else if right == value then isConstructorApp (← whnfR left)
        else pure false
      return !known

/-- The first scoped discriminator requested by multiple independent
clauses. Clause-local binders stay below their quantifiers. Conditions and
option/sum/Boolean values already fixed by assumptions need no new case. -/
def find? (goal : MVarId) : MetaM (Option Discriminator) := goal.withContext do
  let requests ← (clauses (← instantiateMVars (← goal.getType))).mapM discriminators
  if requests.size < 2 then return none
  let mut considered : Array Discriminator := #[]
  for requested in requests do
    for candidate in requested do
      if considered.contains candidate then continue
      considered := considered.push candidate
      unless (requests.filter (·.contains candidate)).size > 1 do continue
      unless ← eligible candidate do continue
      return some candidate
  return none

end VerexIR.Proofs.Denote.SharedCases
