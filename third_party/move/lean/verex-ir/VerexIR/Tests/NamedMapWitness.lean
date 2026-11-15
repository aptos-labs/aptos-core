-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve
open Lean Meta VerexIR VerexIR.Proofs.Denote

namespace VerexIR.Tests.NamedMapWitness

-- A map already named by its read keeps the facts attached to that name.

example (raw : SpecEntries (Int × Int)) (m : SpecMap Int Int)
    (read : SpecMap.ofEntries id id raw = m)
    (valid : SpecMap.Valid .sequence m) :
    SpecMap.Valid .sequence (SpecMap.ofEntries id id raw) := by
  run_tac nameTypedMaps
  assumption

-- A different interpretation of the same raw carrier is not the known map.
example (raw : SpecEntries (Int × Int)) (m : SpecMap Int Int)
    (_read : SpecMap.ofEntries id id raw = m)
    (_valid : SpecMap.Valid .sequence m) : True := by
  fail_if_success
    have : SpecMap.Valid .sequence (SpecMap.ofEntries (fun k => k % 2) id raw) := by
      run_tac nameTypedMaps
      assumption
  trivial

-- Equal carrier substitution can coalesce maps named on separate paths.
example (raw other : SpecEntries (Int × Int)) (m n : SpecMap Int Int)
    (read_m : SpecMap.ofEntries id id raw = m)
    (read_n : SpecMap.ofEntries id id other = n) (same : other = raw)
    (valid : SpecMap.Valid .sequence m) : SpecMap.Valid .sequence n := by
  subst other
  run_tac nameTypedMaps
  assumption

private def NamedInt : Type := Int

-- Definitionally equal view types do not require another map name.
example (raw : SpecEntries (Int × Int)) (m : SpecMap NamedInt NamedInt)
    (read : SpecMap.ofEntries (fun x : Int => (x : NamedInt))
      (fun x : Int => (x : NamedInt)) raw = m)
    (valid : SpecMap.Valid .sequence m) :
    SpecMap.Valid .sequence (SpecMap.ofEntries id id raw) := by
  run_tac nameTypedMaps
  assumption

-- Position facts use validity of the same keys when specifications read
-- values through a different view.
example (raw : SpecEntries (Int × SpecInt (.bits 64) false)) (i : Int)
    (valid : SpecMap.Valid .sequence (SpecMap.ofEntries id id raw))
    (low : 0 ≤ i) (high : i < (SpecMap.ofEntries id SpecInt.val raw).size) :
    (SpecMap.ofEntries id SpecInt.val raw).rank
      ((SpecMap.ofEntries id SpecInt.val raw).keyAt i) = i := by
  verex_map_positions
  verex_assumption

-- A named native map reaches the same observed view through its equation.
example (raw : SpecEntries (Int × SpecInt (.bits 64) false))
    (m : SpecMap Int (SpecInt (.bits 64) false)) (i : Int)
    (read : SpecMap.ofEntries id id raw = m) (valid : SpecMap.Valid .sequence m)
    (low : 0 ≤ i) (high : i < (SpecMap.ofEntries id SpecInt.val raw).size) :
    (SpecMap.ofEntries id SpecInt.val raw).rank
      ((SpecMap.ofEntries id SpecInt.val raw).keyAt i) = i := by
  verex_map_positions
  verex_assumption

-- Naming preserves native validity at an already named specification view.
example (raw : SpecEntries (Int × SpecInt (.bits 64) false)) (m : SpecMap Int Int)
    (read : SpecMap.ofEntries id SpecInt.val raw = m)
    (valid : SpecMap.Valid .sequence (SpecMap.ofEntries id id raw)) :
    SpecMap.Valid .sequence m := by
  run_tac nameTypedMaps
  verex_assumption

private def NativeValue : Type := SpecInt (.bits 64) false

-- The observation emits the requested map's type spelling, even when a
-- scalar reader's inferred domain would unfold its native type alias.
example (raw : SpecEntries (Int × SpecInt (.bits 64) false)) (named : SpecMap Int Int)
    (read : @SpecMap.ofEntries Int NativeValue Int Int id SpecInt.val raw = named)
    (valid : SpecMap.Valid .sequence (SpecMap.ofEntries id id raw)) :
    SpecMap.Valid .sequence named := by
  run_tac do
    let some read := (← getLCtx).findFromUserName? `read | throwError "missing read"
    let some valid := (← getLCtx).findFromUserName? `valid | throwError "missing validity"
    let map := (← inferType read.toExpr).getArg! 1
    let source := (← inferType valid.toExpr).getArg! 3
    let viewed ← observedMapValidities #[map] #[(source, valid.toExpr)]
    unless viewed.size == 1 do throwError "missing requested validity view"
    unless (← inferType viewed[0]!.2).getArg! 3 == map do
      throwError "validity observation changed the requested map's native type spelling"
  run_tac nameTypedMaps
  verex_assumption

-- A certified write-back's plain view keeps the same named read bridge.
example [Carriers] (native : SpecMap String (SpecInt (.bits 64) false))
    (named : SpecMap String Int)
    (read : SpecMap.ofEntries id SpecInt.val
      (NTy.forget (.table .address (.int 64 false)) (SpecMap.writeBack id native)) = named)
    (valid : SpecMap.Valid .sequence (SpecMap.ofEntries id id
      (NTy.forget (.table .address (.int 64 false)) (SpecMap.writeBack id native)))) :
    SpecMap.Valid .sequence named := by
  run_tac nameTypedMaps
  verex_assumption

-- The same observation boundary retains ordered-map comparisons.
example (raw : SpecEntries (Int × SpecInt (.bits 64) false)) (i j : Int)
    (_sequence : SpecMap.Valid .sequence (SpecMap.ofEntries id id raw))
    (valid : SpecMap.Valid (.ordered SpecMap.intOrder) (SpecMap.ofEntries id id raw))
    (low : 0 ≤ i) (later : i < j)
    (high : j < (SpecMap.ofEntries id SpecInt.val raw).size) :
    (SpecMap.ofEntries id SpecInt.val raw).keyAt i <
      (SpecMap.ofEntries id SpecInt.val raw).keyAt j := by
  verex_map_positions
  verex_instance

-- A different key reader is a different validity observation, even when
-- it reads the same entries and values.
example (raw : SpecEntries (Int × SpecInt (.bits 64) false)) (i : Int)
    (_valid : SpecMap.Valid .sequence (SpecMap.ofEntries id id raw))
    (_low : 0 ≤ i) (_high : i < (SpecMap.ofEntries (fun key => key % 2) SpecInt.val raw).size) :
    True := by
  fail_if_success
    have : (SpecMap.ofEntries (fun key => key % 2) SpecInt.val raw).rank
        ((SpecMap.ofEntries (fun key => key % 2) SpecInt.val raw).keyAt i) = i := by
      verex_map_positions
      verex_assumption
  trivial

-- A projected table is named as an observed view; its source owner stays
-- in scope and no inverse from the model map to that owner is introduced.
example (owner : String × (SpecEntries (Int × SpecInt (.bits 64) false) × PUnit)) (i : Int)
    (valid : SpecMap.Valid .sequence (SpecMap.ofEntries id id owner.2.1))
    (low : 0 ≤ i) (high : i < (SpecMap.ofEntries id SpecInt.val owner.2.1).size) :
    (SpecMap.ofEntries id SpecInt.val owner.2.1).rank
      ((SpecMap.ofEntries id SpecInt.val owner.2.1).keyAt i) = i := by
  run_tac nameTypedMaps
  run_tac Lean.Elab.Tactic.withMainContext do
    let some owner := (← getLCtx).findFromUserName? `owner | throwError "lost projected owner"
    unless (← inferType owner.toExpr).isAppOfArity ``Prod 2 do
      throwError "projected map naming replaced its owner"
    let some read := (← getLCtx).findFromUserName? `owner_map_read
      | throwError "projected map naming lost its read bridge"
    let type ← inferType read.toExpr
    unless type.isAppOfArity ``Eq 3 && (type.getArg! 1).isAppOfArity ``SpecMap.ofEntries 7 do
      throwError "projected map naming changed its read bridge"
  verex_map_positions
  verex_assumption

-- Distinct projected fields of one owner retain distinct named views.
example (owner : SpecEntries (Int × SpecInt (.bits 64) false) ×
      SpecEntries (Int × SpecInt (.bits 64) false)) (i j : Int)
    (leftValid : SpecMap.Valid .sequence (SpecMap.ofEntries id id owner.1))
    (rightValid : SpecMap.Valid .sequence (SpecMap.ofEntries id id owner.2))
    (leftLow : 0 ≤ i) (leftHigh : i < (SpecMap.ofEntries id SpecInt.val owner.1).size)
    (rightLow : 0 ≤ j) (rightHigh : j < (SpecMap.ofEntries id SpecInt.val owner.2).size) :
    (SpecMap.ofEntries id SpecInt.val owner.1).rank
        ((SpecMap.ofEntries id SpecInt.val owner.1).keyAt i) = i ∧
      (SpecMap.ofEntries id SpecInt.val owner.2).rank
        ((SpecMap.ofEntries id SpecInt.val owner.2).keyAt j) = j := by
  run_tac nameTypedMaps
  constructor <;> verex_map_positions <;> verex_assumption

end VerexIR.Tests.NamedMapWitness

-- Automatic preparation reads a projected table's independent definition
-- forward, without reconstructing its source owner from the model map.
example (owner : String × (SpecEntries (Int × Int) × PUnit))
    (defined : SpecMap.ofEntries id id owner.2.1 = [(1, 7)]) :
    (SpecMap.ofEntries id id owner.2.1).valueAt 1 = 7 := by
  verex_subst_maps
  run_tac Lean.Elab.Tactic.withMainContext do
    let some kept := (← Lean.getLCtx).findFromUserName? `owner
      | throwError "the projected definition replaced its owner"
    unless (← Lean.Meta.inferType kept.toExpr).isAppOfArity ``Prod 2 do
      throwError "the projected definition changed its owner's type"
  simp only [vir_denote_norm]

-- Another field of the same owner is not an independent definition. Keep
-- the equation as a fact rather than using it to rewrite the owner in place.
example (owner : SpecEntries (Int × Int) × SpecEntries (Int × Int))
    (related : SpecMap.ofEntries id id owner.1 = SpecMap.ofEntries id id owner.2) : True := by
  fail_if_success
    have : SpecMap.ofEntries id id owner.1 = SpecMap.ofEntries id id owner.2 := by
      verex_subst_maps
      rfl
  trivial
