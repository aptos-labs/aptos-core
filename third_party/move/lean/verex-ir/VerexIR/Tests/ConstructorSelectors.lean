-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Normal

open Lean Meta Elab Tactic VerexIR.Proofs.Denote

-- Negative first: an unknown variant supplies no constructor observation.
example [Carriers]
    (value : variantCarrier ["End", "Position"]
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

-- Select only the existing constructor observer. The check must see its
-- constructor result, rather than closing the equation by definitional equality.
example [Carriers] (value : NTy.num.carrier) :
    (Which.there Which.here : Which ["End", "Position"]
      (.cons .nil (.cons (.cons .num .nil) .nil)) (.cons .num .nil)).project?
      (.inr (.inl (value, PUnit.unit))) = some ((value, PUnit.unit) : HList (.cons .num .nil)) := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    let procs ← Simp.SimprocsArray.add #[] ``whichProjectConstructor false
    let context ← Simp.mkContext { zeta := false, beta := false, iota := false, proj := false }
    let (result, _) ← simp lhs context procs
    unless result.expr.isAppOf ``Option.some do
      throwError "constructed projection did not reduce: {result.expr}"
  rfl

-- The mapped observer is separate: only its own handler is enabled here.
example [Carriers] (value : NTy.num.carrier) :
    Option.map Var.here.select
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

-- A reading of an option (`Choices.select?`) follows the option under the
-- normal forms: `withSome_congr` is simp's congruence for it, so a read
-- equation reaches the option a reading takes and the reading then reduces.

namespace OptionReadings

/-- A branch that consumes the equation its reading states. -/
private def readBack (option : Option Nat) : (a : Nat) → option = some a → Nat :=
  fun _ reads => option.get (by simp [reads])

/-- The normal forms of a reading of an option, applied to a term. -/
private def normalized (e : Lean.Expr) : MetaM Lean.Expr := do
  let some normal ← getSimpExtension? `vir_denote_norm | throwError "no normal forms"
  let context ← Simp.mkContext {} #[← normal.getTheorems] (← getSimpCongrTheorems)
  return (← simp e context).1.expr

-- Negative first: an undecided option leaves its reading exactly as it is;
-- the congruence takes no case of the option a path has yet to split.
private theorem undecided_reading_unchanged (option : Option Nat) (missing : Nat) :
    withSome option (fun a _ => a + 1) missing =
      withSome option (fun a _ => a + 1) missing := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    unless (← normalized lhs) == lhs do
      throwError "a reading of an undecided option was changed"
  rfl

-- Negative: the same of a branch that consumes the equation.
private theorem undecided_dependent_reading_unchanged (option : Option Nat) (missing : Nat) :
    withSome option (readBack option) missing = withSome option (readBack option) missing := by
  run_tac withMainContext do
    let some (_, lhs, _) := (← getMainTarget).eq? | throwError "expected equality"
    unless (← normalized lhs) == lhs do
      throwError "a dependent reading of an undecided option was changed"
  rfl

-- An option a context equation decides is reached through the reading, which
-- then reduces to the branch at the value.
private theorem decided_reading_reduces (option : Option Nat) (value missing : Nat)
    (reads : option = some value) :
    withSome option (fun a _ => a + 1) missing = value + 1 := by
  simp only [reads, withSome_some]

-- The branch that consumes the equation reduces at the transported proof.
private theorem decided_dependent_reading_reduces (option : Option Nat) (value missing : Nat)
    (reads : option = some value) :
    withSome option (readBack option) missing = value := by
  simp only [reads, withSome_some, readBack, Option.get_some]

-- An option a context equation empties reduces to the reading's empty value.
private theorem emptied_reading_reduces (option : Option Nat) (missing : Nat)
    (reads : option = none) :
    withSome option (fun a _ => a + 1) missing = missing := by
  simp only [reads, withSome_none]

-- Reject any unexpected axiom in the congruence and in its controls.
open Lean Elab Command in
run_cmd do
  for name in [``VerexIR.Proofs.Denote.withSome_congr,
      ``undecided_reading_unchanged, ``undecided_dependent_reading_unchanged,
      ``decided_reading_reduces, ``decided_dependent_reading_reduces,
      ``emptied_reading_reduces] do
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"

end OptionReadings
