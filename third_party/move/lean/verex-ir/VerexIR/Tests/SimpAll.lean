-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.SimpAll

namespace VerexIR.Tests.SimpAll

open Lean Meta Elab Tactic VerexIR.Proofs.Denote

private def toPrimitiveProjections (e : Expr) : CoreM Expr :=
  Core.transform e (post := fun sub => do
    if sub.isAppOfArity ``Prod.fst 3 then return .done (mkProj ``Prod 0 sub.appArg!)
    if sub.isAppOfArity ``Prod.snd 3 then return .done (mkProj ``Prod 1 sub.appArg!)
    if sub.isAppOfArity ``Subtype.val 3 then return .done (mkProj ``Subtype 0 sub.appArg!)
    return .continue)

private def replaceHypLhsWithPrimitive (hypName : Name) : TacticM Expr := withMainContext do
  let fvarId ← getFVarId (mkIdent hypName)
  let decl ← fvarId.getDecl
  let some (_, lhs, rhs) := decl.type.eq? | throwError "expected equality"
  let lhs ← toPrimitiveProjections lhs
  unless lhs.isProj do throwError "expected primitive projection on lhs"
  unless (rhs.find? (·.isProj)).isNone do throwError "expected projection functions on rhs"
  let type ← mkEq lhs rhs
  let goal ← (← getMainGoal).replaceLocalDeclDefEq fvarId type
  setGoals [goal]
  return type

private def replaceHypRhsWithPrimitive (hypName : Name) : TacticM Expr := withMainContext do
  let fvarId ← getFVarId (mkIdent hypName)
  let decl ← fvarId.getDecl
  let some (_, lhs, rhs) := decl.type.eq? | throwError "expected equality"
  let rhs ← toPrimitiveProjections rhs
  unless (lhs.find? (·.isProj)).isNone do throwError "expected projection functions on lhs"
  unless (rhs.find? (·.isProj)).isSome do throwError "expected primitive projections on rhs"
  let type ← mkEq lhs rhs
  let goal ← (← getMainGoal).replaceLocalDeclDefEq fvarId type
  setGoals [goal]
  return type

-- Primitive `.proj` on LHS and `Subtype.val`/`Prod.fst`/`Prod.snd` on RHS.
set_option linter.unusedVariables false in
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (z : Nat)
    (h1 : x.1 = (x.val.fst, x.val.snd))
    (hb : x.val.fst ≤ z) (hz : z = 7) :
    x.val.fst ≤ 7 := by
  run_tac do
    let h1Type ← replaceHypLhsWithPrimitive `h1
    withMainContext do
      unless ← selfReferential h1Type do
        throwError "expected primitive-LHS projection cycle to be self-referential"
  verex_simp_all

-- `Subtype.val` on LHS and primitive `.proj` on RHS.
set_option linter.unusedVariables false in
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (z : Nat)
    (h2 : x.val = (x.1.1, x.1.2))
    (hb : x.val.fst ≤ z) (hz : z = 7) :
    x.val.fst ≤ 7 := by
  run_tac do
    let h2Type ← replaceHypRhsWithPrimitive `h2
    withMainContext do
      unless ← selfReferential h2Type do
        throwError "expected primitive-RHS projection cycle to be self-referential"
  verex_simp_all

-- Both spellings in the same context.
set_option linter.unusedVariables false in
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (z : Nat)
    (h1 : x.1 = (x.val.fst, x.val.snd))
    (h2 : x.val = (x.1.1, x.1.2))
    (hb : x.val.fst ≤ z) (hz : z = 7) :
    x.val.fst ≤ 7 := by
  run_tac do
    let h1Type ← replaceHypLhsWithPrimitive `h1
    let h2Type ← replaceHypRhsWithPrimitive `h2
    withMainContext do
      unless ← selfReferential h1Type do
        throwError "expected h1 to be self-referential"
      unless ← selfReferential h2Type do
        throwError "expected h2 to be self-referential"
  verex_simp_all

-- Two-step cycle mixing primitive `.proj` (`h3`) and `Subtype.val` (`h4`).
set_option linter.unusedVariables false in
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (y : Nat × Nat) (z : Nat)
    (h3 : x.1 = y)
    (h4 : y = (x.val.fst, x.val.snd))
    (hb : x.val.fst ≤ z) (hz : z = 7) :
    x.val.fst ≤ 7 := by
  run_tac do
    let h3Type ← replaceHypLhsWithPrimitive `h3
    withMainContext do
      let h4Decl ← (← getFVarId (mkIdent `h4)).getDecl
      if ← selfReferential h3Type then
        throwError "h3 alone should not be self-referential"
      if ← selfReferential h4Decl.type then
        throwError "h4 alone should not be self-referential"
      let some (_, h3Lhs, h3Rhs) := h3Type.eq? | throwError "expected equality"
      let some (_, h4Lhs, h4Rhs) := h4Decl.type.eq? | throwError "expected equality"
      unless ← closesCycle #[(.other `h3, h3Lhs, h3Rhs)] h4Lhs h4Rhs do
        throwError "expected 2-step projection cycle to be detected"
  verex_simp_all

-- Two-step cycle in the converse projection spelling (`Subtype.val` in `h3`, primitive `.proj` in `h4`).
set_option linter.unusedVariables false in
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (y : Nat × Nat) (z : Nat)
    (h3 : x.val = y)
    (h4 : y = (x.1.1, x.1.2))
    (hb : x.val.fst ≤ z) (hz : z = 7) :
    x.val.fst ≤ 7 := by
  run_tac do
    let h4Type ← replaceHypRhsWithPrimitive `h4
    withMainContext do
      let h3Decl ← (← getFVarId (mkIdent `h3)).getDecl
      let some (_, h3Lhs, h3Rhs) := h3Decl.type.eq? | throwError "expected equality"
      let some (_, h4Lhs, h4Rhs) := h4Type.eq? | throwError "expected equality"
      unless ← closesCycle #[(.other `h3, h3Lhs, h3Rhs)] h4Lhs h4Rhs do
        throwError "expected converse 2-step projection cycle to be detected"
  verex_simp_all

-- Conjunction split by `mkSimpTheoremFromExpr` with a cyclic primitive-projection conjunct.
set_option linter.unusedVariables false in
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (z : Nat)
    (hConj : z = 7 ∧ x.1 = (x.val.fst, x.val.snd))
    (hb : x.val.fst ≤ z) (hz : z = 7) :
    x.val.fst ≤ 7 := by
  run_tac do
    let conjType ← withMainContext do
      let fvarId ← getFVarId (mkIdent `hConj)
      let decl ← fvarId.getDecl
      let some (leftConj, rightConj) := decl.type.and? | throwError "expected conjunction"
      let some (_, lhs, rhs) := rightConj.eq? | throwError "expected equality"
      let lhs ← toPrimitiveProjections lhs
      let conjType := mkAnd leftConj (← mkEq lhs rhs)
      let goal ← (← getMainGoal).replaceLocalDeclDefEq fvarId conjType
      setGoals [goal]
      return conjType
    withMainContext do
      unless ← selfReferential conjType do
        throwError "expected conjunction with cyclic conjunct to be self-referential"
      if ← selfReferential (mkConst ``Nat) then
        throwError "non-proposition should not be self-referential"
  verex_simp_all

-- Non-cyclic primitive-projection equation remains admitted as a rewrite rule.
example (x : { p : Nat × Nat // p.1 ≤ p.2 }) (y : Nat × Nat)
    (hEq : x.1 = y) :
    x.val = y := by
  run_tac do
    let hEqType ← replaceHypLhsWithPrimitive `hEq
    withMainContext do
      if ← selfReferential hEqType then
        throwError "non-cyclic projection equation should not be self-referential"
  verex_simp_all

end VerexIR.Tests.SimpAll
