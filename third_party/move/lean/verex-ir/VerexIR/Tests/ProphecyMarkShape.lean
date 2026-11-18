-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexIR.Proofs.Denote.Solve.Base

/-!
# Prophecy mark consumption by recorded projection and producer shape

Checks that `consumeProphecy` distinguishes single whole-local place borrows
(`Term.borrowPlace` at `Proj.nil`) from paired `(referent, owner)` marks
(`Proj.deref Proj.nil`, or whole-local slice borrows/reborrows
`borrowArraySliceAt`, `borrowVectorSliceAt`, `reborrowSliceAt` at `Proj.nil`):
1. Whole-local struct borrow at `Proj.nil` binds `map'` to the whole struct
   value `chosen` (not `Prod.fst chosen`) and omits `map_owner`, even when
   `chosen`'s carrier type is already unfolded to `Prod`.
2. Whole-referent borrow at `Proj.deref Proj.nil` binds `r'` to
   `Prod.fst chosenPair` and `r_owner` to `Prod.snd chosenPair`.
3. Non-whole component borrow at `Proj.field` clears the mark without binding
   `map'` or `map_owner`.
4. Whole-local slice borrow (`borrowVectorSliceAt`) at `Proj.nil` binds `v'` to
   `Prod.fst chosenSlice` (`SpecSliceRef`) and `v_owner` to
   `Prod.snd chosenSlice` (`SpecVector`).
5. Whole-local slice reborrow (`reborrowSliceAt`, as in
   `PlainValueBoundaries.lean:721–734`) at `Proj.nil` binds `s'` to
   `Prod.fst chosenReborrow` and `s_owner` to `Prod.snd chosenReborrow`.
6. Whole-local vector place borrow (`Term.borrowPlace` on `.vector` at
   `Proj.nil`) binds `v'` to the single `SpecVector` value and omits `v_owner`.
-/

namespace VerexIR.Tests.ProphecyMarkShape

open Lean Meta Elab Tactic
open VerexIR.Proofs.Denote

private def testStructHandle : StructHandle := ⟨⟨0⟩, 0⟩

private abbrev testStructTy : NTy :=
  .struct testStructHandle .nil (.cons .num (.cons .bool .nil))

-- 1. Whole-local borrow (`Proj.nil`) of a struct whose carrier unfolds to `Prod`:
-- `map'` must bind the whole struct value `chosen : testStructTy.carrier`,
-- NOT `Prod.fst chosen : Int`, and `map_owner` must not be bound.
private theorem whole_local_struct_borrow_binds_whole_struct [Carriers]
    (chosen : NTy.num.carrier × NTy.bool.carrier × Unit)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := testStructTy))
      (Proj.nil (τ := testStructTy)) chosen) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["map", "map'", "map_owner"], []))
    state.scopeLocals.set #["map"]
    let chosenFVar := (← getLocalDeclFromUserName `chosen).toExpr
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Bool := false
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "map'" then foundWritten := some val
          if entryName == "map_owner" then foundOwner := true
      let some writtenVal := foundWritten
        | throwError "whole-local struct borrow did not bind `map'`"
      if writtenVal != chosenFVar then
        throwError m!"whole-local struct borrow corrupted `map'` to `{writtenVal}` instead of whole struct `{chosenFVar}`"
      if foundOwner then
        throwError "whole-local struct borrow spuriously bound `map_owner`"
  trivial

-- 2. Whole-referent reborrow through a reference (`Proj.deref Proj.nil`):
-- `r'` binds `Prod.fst chosenPair` (of type `testStructTy.carrier`), and
-- `r_owner` binds `Prod.snd chosenPair`.
private theorem whole_referent_borrow_binds_component_and_owner [Carriers]
    (chosenPair : testStructTy.carrier × (NTy.ref testStructTy).carrier)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := .ref testStructTy))
      (Proj.deref (Proj.nil (τ := testStructTy))) chosenPair) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["r", "r'", "r_owner"], ["r"]))
    state.scopeLocals.set #["r"]
    let pairFVar := (← getLocalDeclFromUserName `chosenPair).toExpr
    let expectedFst ← mkAppM ``Prod.fst #[pairFVar]
    let expectedSnd ← mkAppM ``Prod.snd #[pairFVar]
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Option Lean.Expr := none
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "r'" then foundWritten := some val
          if entryName == "r_owner" then foundOwner := some val
      unless foundWritten == some expectedFst do
        throwError m!"whole-referent borrow bound `r'` to `{foundWritten}` instead of `{expectedFst}`"
      unless foundOwner == some expectedSnd do
        throwError m!"whole-referent borrow bound `r_owner` to `{foundOwner}` instead of `{expectedSnd}`"
  trivial

-- 3. Genuine non-whole component borrow (`Proj.field`):
-- Clears the mark without binding `map'` or `map_owner`.
private theorem component_field_borrow_clears_mark_without_binding [Carriers]
    (chosenComp : NTy.num.carrier × testStructTy.carrier)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := testStructTy))
      (Proj.field (source := testStructHandle) (arguments := .nil)
        (fields := .cons .num (.cons .bool .nil)) Var.here (Proj.nil (τ := .num))) chosenComp) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["map", "map'", "map_owner"], []))
    state.scopeLocals.set #["map"]
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      for decl in ← getLCtx do
        if decl.fvarId == markId then
          throwError "component borrow prophecy mark was not cleared"
        if let some (entryName, _, _) := scopeEntry? (← instantiateMVars decl.type) then
          throwError m!"component borrow spuriously bound scope entry `{entryName}`"
  trivial

-- 4. Whole-local slice borrow (`borrowVectorSliceAt` at `Proj.nil`):
-- `v'` binds `Prod.fst chosenSlice` (`SpecSliceRef`) and `v_owner` binds
-- `Prod.snd chosenSlice` (`SpecVector`).
private theorem whole_local_vector_slice_borrow_binds_slice_and_owner [Carriers]
    (chosenSlice : (NTy.sliceRef .num).carrier × (NTy.vector .num).carrier)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := .vector .num))
      (Proj.nil (τ := .vector .num)) chosenSlice) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["v", "v'", "v_owner"], []))
    state.scopeLocals.set #["v"]
    let pairFVar := (← getLocalDeclFromUserName `chosenSlice).toExpr
    let expectedFst ← mkAppM ``Prod.fst #[pairFVar]
    let expectedSnd ← mkAppM ``Prod.snd #[pairFVar]
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Option Lean.Expr := none
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "v'" then foundWritten := some val
          if entryName == "v_owner" then foundOwner := some val
      unless foundWritten == some expectedFst do
        throwError m!"whole-local vector slice borrow bound `v'` to `{foundWritten}` instead of `{expectedFst}`"
      unless foundOwner == some expectedSnd do
        throwError m!"whole-local vector slice borrow bound `v_owner` to `{foundOwner}` instead of `{expectedSnd}`"
  trivial

-- 5. Whole-local slice reborrow (`reborrowSliceAt` at `Proj.nil`, matching
-- `PlainValueBoundaries.lean:721–734`):
-- `s'` binds `Prod.fst chosenReborrow` and `s_owner` binds
-- `Prod.snd chosenReborrow`.
private theorem whole_local_slice_reborrow_binds_slice_and_owner [Carriers]
    (chosenReborrow : (NTy.sliceRef .num).carrier × (NTy.sliceRef .num).carrier)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := .sliceRef .num))
      (Proj.nil (τ := .sliceRef .num)) chosenReborrow) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["s", "s'", "s_owner"], ["s"]))
    state.scopeLocals.set #["s"]
    let pairFVar := (← getLocalDeclFromUserName `chosenReborrow).toExpr
    let expectedFst ← mkAppM ``Prod.fst #[pairFVar]
    let expectedSnd ← mkAppM ``Prod.snd #[pairFVar]
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Option Lean.Expr := none
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "s'" then foundWritten := some val
          if entryName == "s_owner" then foundOwner := some val
      unless foundWritten == some expectedFst do
        throwError m!"whole-local slice reborrow bound `s'` to `{foundWritten}` instead of `{expectedFst}`"
      unless foundOwner == some expectedSnd do
        throwError m!"whole-local slice reborrow bound `s_owner` to `{foundOwner}` instead of `{expectedSnd}`"
  trivial

-- 6. Whole-local place borrow (`Term.borrowPlace` on `.vector` at `Proj.nil`):
-- `v'` binds the single `SpecVector` value `chosenVec` and omits `v_owner`.
private theorem whole_local_vector_place_borrow_binds_whole_vector [Carriers]
    (chosenVec : (NTy.vector .num).carrier)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := .vector .num))
      (Proj.nil (τ := .vector .num)) chosenVec) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["v", "v'", "v_owner"], []))
    state.scopeLocals.set #["v"]
    let chosenFVar := (← getLocalDeclFromUserName `chosenVec).toExpr
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Bool := false
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "v'" then foundWritten := some val
          if entryName == "v_owner" then foundOwner := true
      unless foundWritten == some chosenFVar && !foundOwner do
        throwError m!"whole-local vector place borrow bound `v'`=`{foundWritten}`, `v_owner`={foundOwner}"
  trivial

-- 7. Expanded whole-local borrow pair (`Term.denote_borrowPlace_nil_expanded`,
-- `Term.lean:4916`, where `α = τ.carrier × τ.carrier` at `Proj.nil`):
-- `map'` binds `Prod.fst chosenPair` and `map_owner` binds `Prod.snd chosenPair`.
private theorem expanded_whole_local_borrow_pair_binds_component_and_owner [Carriers]
    (chosenPair : testStructTy.carrier × testStructTy.carrier)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := testStructTy))
      (Proj.nil (τ := testStructTy)) chosenPair) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["map", "map'", "map_owner"], []))
    state.scopeLocals.set #["map"]
    let pairFVar := (← getLocalDeclFromUserName `chosenPair).toExpr
    let expectedFst ← mkAppM ``Prod.fst #[pairFVar]
    let expectedSnd ← mkAppM ``Prod.snd #[pairFVar]
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Option Lean.Expr := none
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "map'" then foundWritten := some val
          if entryName == "map_owner" then foundOwner := some val
      unless foundWritten == some expectedFst do
        throwError m!"expanded whole-local borrow pair bound `map'` to `{foundWritten}` instead of `{expectedFst}`"
      unless foundOwner == some expectedSnd do
        throwError m!"expanded whole-local borrow pair bound `map_owner` to `{foundOwner}` instead of `{expectedSnd}`"
  trivial

-- 8. Whole-local ordinary borrow (`Proj.nil`) of a reference `r : NTy.ref .bool`
-- whose single carrier `(NTy.ref .bool).carrier` is already reduced to `Bool × Bool`:
-- `fstTy` (`Bool`) is not definitionally equal to `(NTy.ref .bool).carrier`
-- (`Bool × Bool`), so `r'` must bind the whole reference value `chosenRef`
-- (not `Prod.fst chosenRef`) and `r_owner` must not be bound.
private theorem whole_local_reduced_ref_borrow_binds_whole_ref [Carriers]
    (chosenRef : Bool × Bool)
    (_mark : Prophecy (Var.here (Γ := .nil) (τ := .ref .bool))
      (Proj.nil (τ := .ref .bool)) chosenRef) :
    True := by
  run_tac withMainContext do
    let state ← targetState
    state.scopeNames.set (some (["r", "r'", "r_owner"], ["r"]))
    state.scopeLocals.set #["r"]
    let chosenFVar := (← getLocalDeclFromUserName `chosenRef).toExpr
    let markId := (← getLocalDeclFromUserName `_mark).fvarId
    let goal ← consumeProphecy (← getMainGoal) markId
    replaceMainGoal [goal]
    withMainContext do
      let mut foundWritten : Option Lean.Expr := none
      let mut foundOwner : Bool := false
      for decl in ← getLCtx do
        if let some (entryName, _, val) := scopeEntry? (← instantiateMVars decl.type) then
          if entryName == "r'" then foundWritten := some val
          if entryName == "r_owner" then foundOwner := true
      unless foundWritten == some chosenFVar && !foundOwner do
        throwError m!"whole-local reduced ref borrow bound `r'`=`{foundWritten}` (expected `{chosenFVar}`), `r_owner`={foundOwner}"
  trivial

end VerexIR.Tests.ProphecyMarkShape
