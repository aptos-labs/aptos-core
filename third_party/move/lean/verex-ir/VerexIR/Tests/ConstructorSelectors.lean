-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Normal

open Lean Meta Elab Tactic VerexIR.Proofs.Denote

-- Select only the existing constructor observer. The check must see its
-- constructor result, rather than closing the equation by definitional equality.
example [Carriers] (value : NTy.num.plain) :
    (Which.there Which.here : Which ["End", "Position"]
      (.cons .nil (.cons (.cons .num .nil) .nil)) (.cons .num .nil)).project?
      (.inr (.inl (value, PUnit.unit))) = some ((value, PUnit.unit) : HListPlain (.cons .num .nil)) := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    let procs ← Simp.SimprocsArray.add #[] ``whichProjectConstructor false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    let (result, _) ← simp lhs context procs
    unless result.expr.isAppOf ``Option.some do
      throwError "constructed projection did not reduce: {result.expr}"
  rfl

-- The mapped observer is separate: only its own handler is enabled here.
example [Carriers] (value : NTy.num.plain) :
    Option.map Var.here.selectPlain
      ((Which.there Which.here : Which ["End", "Position"]
        (.cons .nil (.cons (.cons .num .nil) .nil)) (.cons .num .nil)).project?
        (.inr (.inl (value, PUnit.unit)))) = some value := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    let procs ← Simp.SimprocsArray.add #[] ``optionMapProjectConstructor true
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    let (result, _) ← simp lhs context procs
    unless result.expr.isAppOf ``Option.some do
      throwError "mapped constructed projection did not reduce: {result.expr}"
  rfl

-- An unknown variant supplies no constructor observation.
example [Carriers]
    (value : variantPlain ["End", "Position"]
      (.cons .nil (.cons (.cons .num .nil) .nil))) :
    (Which.there Which.here : Which ["End", "Position"]
      (.cons .nil (.cons (.cons .num .nil) .nil)) (.cons .num .nil)).project? value =
    (Which.there Which.here : Which ["End", "Position"]
      (.cons .nil (.cons (.cons .num .nil) .nil)) (.cons .num .nil)).project? value := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    let procs ← Simp.SimprocsArray.add #[] ``whichProjectConstructor false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    let (result, _) ← simp lhs context procs
    unless result.expr == lhs do
      throwError "unknown variant projection was changed: {result.expr}"
  rfl

-- Unknown row components use the same inferred Prod projections as clauses
-- and post-write equations. Primitive projections are convertible but distinct
-- atoms to the leaf deciders; inspect the emitted expression, not just equality.
example [Carriers] (values : HList (.cons .address (.cons .bool .nil))) :
    (Var.there Var.here : Var (.cons .address (.cons .bool .nil)) .bool).select values =
      values.2.1 := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    let procs ← Simp.SimprocsArray.add #[] ``varSelectConstructor false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    let (result, _) ← simp lhs context procs
    let values := (← getLocalDeclFromUserName `values).toExpr
    let expected ← mkAppM ``Prod.fst #[← mkAppM ``Prod.snd #[values]]
    unless result.expr == expected do
      throwError "row selection did not emit inferred Prod.fst/Prod.snd applications"
    check result.expr
  rfl
