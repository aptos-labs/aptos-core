-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

open VerexIR VerexSL

section SliceLeafSupport
open Lean Meta Elab Tactic VerexIR VerexIR.Proofs VerexIR.Proofs.Denote

/-! Gap at the pre-switch representation: an ordinary mutable slice borrow of a
struct's certified array (and slice mutation through an open reference) is not
verified automatically yet. Specifically:
1. Slice birth/reborrow leaves carry an unprojected equality
   `h : NTy.forget (.sliceRef Percent) value_0 = SpecSliceRef.ofArrays current final`
   in `LCtx`, and `NTy.forget_array_values` lacks `no_index` on its outer
   `@SpecVector.values` / `@SpecArray.vector` type arguments at concrete struct
   types, leaving `↑value_0.extent ≤ 1 ⊢ False` (`no panic`) and enclosing
   `Holder` obligations `arr[0].value <= arr[1].value` (`data invariant`) open.
2. Open-reference slice/array mutation leaves additionally require array size
   facts (`SpecArray.size_eq`) and element membership instantiation across
   `Array.setIfInBounds`.
Half of these missing projections concern `NTy.forget` and `NTy.ofDeepHolds`,
which the Phase 2 switch deletes. The 8 positive controls below therefore use a
test-local script (`verex_slice_close`), while every negative control is checked
to reject both `(exfalso; omega)` and `verex_slice_close`. -/

theorem s_forget_array_vals [Carriers] (σ : NTy) {n : Nat} (a : (NTy.array σ n).carrier) :
    (@SpecVector.values (no_index σ.plain)
      (@SpecArray.vector (no_index σ.plain) n (NTy.forget (.array σ n) a))) =
      a.vector.values.map (NTy.forget σ) := rfl

theorem s_forget_slice_vals [Carriers] (σ : NTy) (s : (NTy.slice σ).carrier) :
    (@SpecVector.values (no_index σ.plain) (NTy.forget (.slice σ) s)) =
      s.values.map (NTy.forget σ) := rfl

theorem s_ofDeepHolds_array_size [Carriers] (σ : NTy) {n : Nat}
    (a : (NTy.array σ n).plain) (h : NTy.deepHolds (.array σ n) a) :
    (@Array.size (no_index σ.carrier)
      (@SpecVector.values (no_index σ.carrier)
        (@SpecArray.vector (no_index σ.carrier) n (NTy.ofDeepHolds (.array σ n) a h)))) = n :=
  by simp [NTy.ofDeepHolds, a.size_eq]

theorem s_ofDeepHolds_struct_val [Carriers] (src : StructHandle) (args fields : NRow)
    (row : HListPlain fields) (h : NTy.deepHolds (.struct src args fields) row) :
    (@Subtype.val (no_index (HListPlain fields)) (no_index _)
      (NTy.ofDeepHolds (.struct src args fields) row h)) = row := rfl

theorem s_ofArrays_ext {α : Type} {n : Nat} (c f : SpecArray α n) :
    (SpecSliceRef.ofArrays c f).extent = n := rfl

theorem s_ofArrays_cur_vals {α : Type} {n : Nat} (c f : SpecArray α n) :
    (SpecSliceRef.ofArrays c f).currentVector.values = c.vector.values := rfl

theorem s_ofArrays_fin_vals {α : Type} {n : Nat} (c f : SpecArray α n) :
    (SpecSliceRef.ofArrays c f).finalVector.values = f.vector.values := rfl

theorem s_withCurrent_ext {α : Type} (s : SpecSliceRef α) (c : SpecArray α s.extent) :
    (s.withCurrent c).extent = s.extent := rfl

theorem s_withCurrent_cur_vals {α : Type} (s : SpecSliceRef α) (c : SpecArray α s.extent) :
    (s.withCurrent c).currentVector.values = c.vector.values := rfl

theorem s_withCurrent_fin_vals {α : Type} (s : SpecSliceRef α) (c : SpecArray α s.extent) :
    (s.withCurrent c).finalVector.values = s.finalVector.values := rfl

theorem s_sliceRef_mk_ext {α : Type} (n : Nat) (c f : SpecArray α n) :
    ({ extent := n, current := c, final := f } : SpecSliceRef α).extent = n := rfl

theorem s_sliceRef_mk_cur {α : Type} (n : Nat) (c f : SpecArray α n) :
    ({ extent := n, current := c, final := f } : SpecSliceRef α).currentVector = c.vector := rfl

theorem s_sliceRef_mk_fin {α : Type} (n : Nat) (c f : SpecArray α n) :
    ({ extent := n, current := c, final := f } : SpecSliceRef α).finalVector = f.vector := rfl

theorem s_specArray_size {α : Type} {n : Nat} (a : SpecArray α n) :
    a.vector.values.size = n := a.size_eq

theorem s_specVector_set_vals {α : Type} (v : SpecVector α) (i : Nat) (x : α) :
    (v.set i x).values = v.values.setIfInBounds i x := rfl

theorem s_eq_prod_iff {α β : Type} (p : α × β) (a : α) (b : β) :
    (@Eq (no_index _) p (a, b)) = (p.1 = a ∧ p.2 = b) :=
  propext ⟨fun h => ⟨by rw [h], by rw [h]⟩, fun ⟨h1, h2⟩ => Prod.ext h1 h2⟩

theorem s_mem_setIfInBounds {α : Type} {xs : Array α} {i : Nat} {v x : α}
    (h : x ∈ xs.setIfInBounds i v) : x = v ∨ x ∈ xs := by
  obtain ⟨k, hk, rfl⟩ := Array.getElem_of_mem h
  have hk' : k < xs.size := by rwa [Array.size_setIfInBounds] at hk
  rw [Array.getElem_setIfInBounds hk']
  split
  · exact Or.inl rfl
  · exact Or.inr (Array.getElem_mem hk')

elab "verex_slice_project" : tactic => do
  let mut goal ← getMainGoal
  let (seenForget, arrayPfs) ← goal.withContext do
    let mut acc : Array Lean.Expr := #[← instantiateMVars (← goal.getType)]
    let mut arrPfs : Array Lean.Expr := #[]
    for decl in ← getLCtx do
      if !decl.isImplementationDetail then
        let dty ← instantiateMVars decl.type
        acc := acc.push dty
        acc := acc.push decl.toExpr
        unless ← isProp dty do
          let wty := (← whnf dty).consumeMData
          if wty.isAppOfArity ``SpecArray 2 then
            arrPfs := arrPfs.push (← mkAppM ``SpecArray.size_eq #[decl.toExpr])
          else if wty.isAppOfArity ``SpecSliceRef 1 then
            let cur ← mkAppM ``SpecSliceRef.current #[decl.toExpr]
            let fin ← mkAppM ``SpecSliceRef.final #[decl.toExpr]
            arrPfs := arrPfs.push (← mkAppM ``SpecArray.size_eq #[cur])
            arrPfs := arrPfs.push (← mkAppM ``SpecArray.size_eq #[fin])
    let fRef ← IO.mkRef (#[] : Array Lean.Expr)
    for root in acc do
      Meta.forEachExpr' root fun sub => do
        if !sub.hasLooseBVars && sub.isAppOfArity ``NTy.forget 3 then
          let fs ← fRef.get
          unless fs.any (· == sub) do fRef.set (fs.push sub)
        return true
    return (← fRef.get, arrPfs)
  for f in seenForget do
    let args := f.getAppArgs
    let pf := mkApp3 (mkConst ``NTy.holds_forget) args[0]! args[1]! args[2]!
    let ty ← goal.withContext (inferType pf)
    let (_, g') ← (← goal.assert `hForget ty pf).intro1P
    goal := g'
  for pf in arrayPfs do
    let ty ← goal.withContext (inferType pf)
    let (_, g') ← (← goal.assert `hArrSz ty pf).intro1P
    goal := g'
  let eqProofs ← goal.withContext do
    let mut pfs : Array Lean.Expr := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let_expr Eq eqTy _ _ := ty | continue
      let eqTy := (← whnf eqTy).consumeMData
      if eqTy.isAppOfArity ``SpecSliceRef 1 then
        let extFn ← withLocalDeclD `s eqTy fun s => do
          mkLambdaFVars #[s] (← mkAppM ``SpecSliceRef.extent #[s])
        let curFn ← withLocalDeclD `s eqTy fun s => do
          let cv ← mkAppM ``SpecSliceRef.currentVector #[s]
          mkLambdaFVars #[s] (← mkAppM ``SpecVector.values #[cv])
        let finFn ← withLocalDeclD `s eqTy fun s => do
          let fv ← mkAppM ``SpecSliceRef.finalVector #[s]
          mkLambdaFVars #[s] (← mkAppM ``SpecVector.values #[fv])
        pfs := pfs.push (← mkAppM ``congrArg #[extFn, decl.toExpr])
        pfs := pfs.push (← mkAppM ``congrArg #[curFn, decl.toExpr])
        pfs := pfs.push (← mkAppM ``congrArg #[finFn, decl.toExpr])
      else if eqTy.isAppOfArity ``SpecVector 1 then
        let valFn ← withLocalDeclD `v eqTy fun v => do
          mkLambdaFVars #[v] (← mkAppM ``SpecVector.values #[v])
        pfs := pfs.push (← mkAppM ``congrArg #[valFn, decl.toExpr])
    pure pfs
  for pf in eqProofs do
    let ty ← goal.withContext (inferType pf)
    let (_, g') ← (← goal.assert `hSliceEq ty pf).intro1P
    goal := g'
  replaceMainGoal [goal]

elab "verex_slice_mem_split" : tactic => do
  for _ in [:4] do
    let goals ← getGoals
    let mut nextGoals : List MVarId := []
    for g in goals do
      let action? ← g.withContext do
        let mut hit : Option (Sum FVarId (FVarId × Lean.Expr × Bool)) := none
        for decl in ← getLCtx do
          if decl.isImplementationDetail then continue
          let ty := (← instantiateMVars decl.type).consumeMData
          if ty.isConstOf ``False then
            hit := some (.inl decl.fvarId)
            break
          if ty.isAppOfArity ``Or 2 then
            hit := some (.inl decl.fvarId)
            break
          if let some (_, lhs, rhs) := ty.eq? then
            if lhs.isAppOf ``GetElem?.getElem? && rhs.isAppOfArity ``Option.some 2 then
              if let some pf ← try some <$> mkAppM ``List.mem_of_getElem? #[decl.toExpr]
                  catch _ => pure none then
                hit := some (.inr (decl.fvarId, pf, false))
                break
          if ty.isAppOf ``Membership.mem then
            let args := ty.getAppArgs
            if args.size >= 5 &&
                ((args[3]!.consumeMData).isAppOfArity ``Array.setIfInBounds 4 ||
                 (args[4]!.consumeMData).isAppOfArity ``Array.setIfInBounds 4) then
              let pf ← mkAppM ``s_mem_setIfInBounds #[decl.toExpr]
              hit := some (.inr (decl.fvarId, pf, true))
              break
        pure hit
      match action? with
      | none => nextGoals := nextGoals ++ [g]
      | some (.inl fv) =>
          let sub ← g.cases fv
          nextGoals := nextGoals ++ (sub.map (·.mvarId)).toList
      | some (.inr (oldFv, pf, splitOr)) =>
          let ty ← g.withContext (inferType pf)
          let (fv, g1) ← (← g.assert `hSetMem ty pf).intro1P
          let g2 ← g1.tryClear oldFv
          if splitOr then
            let sub ← g2.cases fv
            nextGoals := nextGoals ++ (sub.map (·.mvarId)).toList
          else
            nextGoals := nextGoals ++ [g2]
    setGoals nextGoals

elab "verex_slice_mem" : tactic => do
  let mut goal ← getMainGoal
  let newPfs ← goal.withContext do
    let mut memPfs : Array Lean.Expr := #[]
    let mut memElems : Array (Lean.Expr × Lean.Expr) := #[]
    let mut alls : Array Lean.Expr := #[]
    let mut holdsDecls : Array Lean.Expr := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      if (ty.consumeMData).isAppOf ``NTy.Holds then
        holdsDecls := holdsDecls.push decl.toExpr
      if let some (_, lhs, rhs) := ty.eq? then
        if lhs.isAppOf ``GetElem?.getElem? && rhs.isAppOfArity ``Option.some 2 then
          if let some memPf ← try some <$> mkAppM ``Array.mem_of_getElem? #[decl.toExpr]
              catch _ => pure none then
            memPfs := memPfs.push memPf
            memElems := memElems.push (rhs.appArg!, memPf)
      if ty.consumeMData.isAppOf ``Membership.mem then
        memPfs := memPfs.push decl.toExpr
        let mArgs := ty.consumeMData.getAppArgs
        if mArgs.size >= 5 then
          memElems := memElems.push (mArgs[3]!, decl.toExpr)
          memElems := memElems.push (mArgs[4]!, decl.toExpr)
      let checkForall (fProof fTy : Lean.Expr) : MetaM (Option Lean.Expr) := do
        if !fTy.isForall then return none
        forallBoundedTelescope fTy (some 1) fun _ body => do
          if body.isForall && (body.bindingDomain!.consumeMData).isAppOf ``Membership.mem then
            return some fProof
          return none
      if let some p ← checkForall decl.toExpr ty then
        alls := alls.push p
      if ty.isAppOfArity ``And 2 then
        let p1 ← mkAppM ``And.left #[decl.toExpr]
        let p2 ← mkAppM ``And.right #[decl.toExpr]
        if let some p ← checkForall p1 (ty.getArg! 0) then alls := alls.push p
        if let some p ← checkForall p2 (ty.getArg! 1) then alls := alls.push p
    let mut pfs : Array Lean.Expr := #[]
    for (elem, memPf) in memElems do
      for hAll in alls do
        if let some pf ← try some <$> mkAppM' hAll #[elem, memPf] catch _ => pure none then
          pfs := pfs.push pf
    for hHolds in holdsDecls do
      for memPf in memPfs do
        for lem in #[``NTy.Holds.of_mem_array, ``NTy.Holds.of_mem_slice,
            ``NTy.Holds.of_current, ``NTy.Holds.of_final] do
          if let some pf ← try some <$> mkAppM lem #[hHolds, memPf] catch _ => pure none then
            pfs := pfs.push pf
    pure pfs
  for pf in newPfs do
    let ty ← goal.withContext (inferType pf)
    let (_, g') ← (← goal.assert `hElemHolds ty pf).intro1P
    goal := g'
  replaceMainGoal [goal]

elab "verex_slice_rw" : tactic => do
  let mut usedLhs : Array Lean.Expr := #[]
  for _ in [:4] do
    if (← getGoals).isEmpty then return
    let goal ← getMainGoal
    let candidates ← goal.withContext do
      let isTargetShape (e : Lean.Expr) : Bool :=
        let e := e.consumeMData
        e.isAppOf ``Array.map || e.isAppOf ``SpecVector.values ||
          e.isAppOf ``SpecSliceRef.extent
      let mut out : Array (FVarId × Lean.Expr × Lean.Expr) := #[]
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        let ty ← instantiateMVars decl.type
        let some (_, rawLhs, rawRhs) := ty.eq? | continue
        let lhs := rawLhs.consumeMData
        let rhs := rawRhs.consumeMData
        if isTargetShape lhs && !usedLhs.any (· == lhs) &&
            lhs != rhs && (rhs.find? (· == lhs)).isNone then
          out := out.push (decl.fvarId, lhs, decl.toExpr)
        else if isTargetShape rhs && !usedLhs.any (· == rhs) &&
            lhs != rhs && (lhs.find? (· == rhs)).isNone then
          out := out.push (decl.fvarId, rhs, ← mkEqSymm decl.toExpr)
      pure out
    let mut progressed := false
    for (eqFv, patLhs, eqExpr) in candidates do
      let headFn := patLhs.getAppFn
      let hasHead (e : Lean.Expr) : Bool :=
        (e.find? fun sub => sub.consumeMData.getAppFn == headFn).isSome
      let mut g ← getMainGoal
      let mut changed := false
      let tgt ← g.withContext (instantiateMVars (← g.getType))
      if hasHead tgt then
        if let some g' ← try
            let r ← g.rewrite tgt eqExpr
            if r.eNew == tgt then pure none
            else some <$> g.replaceTargetEq r.eNew r.eqProof
          catch _ => pure none then
          g := g'
          changed := true
      let otherFvs ← g.withContext do
        let mut ids : Array FVarId := #[]
        for decl in ← getLCtx do
          if !decl.isImplementationDetail && decl.fvarId != eqFv then
            let dty ← instantiateMVars decl.type
            if hasHead dty then
              ids := ids.push decl.fvarId
        pure ids
      for fv in otherFvs do
        if let some g' ← try
            let decl ← g.withContext fv.getDecl
            let r ← g.rewrite decl.type eqExpr
            if r.eNew == decl.type then pure none
            else some <$> (·.mvarId) <$> g.replaceLocalDecl fv r.eNew r.eqProof
          catch _ => pure none then
          g := g'
          changed := true
      if changed then
        usedLhs := usedLhs.push patLhs
        replaceMainGoal [g]
        progressed := true
        break
    unless progressed do break

macro "verex_slice_simp" : tactic => `(tactic|
  simp (config := { failIfUnchanged := false }) only [
    VerexIR.Proofs.Obligation_eq,
    nominalCertifiedValue, nominalHolds, nominalForget,
    forget_sliceRef_extent, ofDeepHolds_sliceRef_extent,
    forget_sliceRef_currentVector, forget_sliceRef_finalVector,
    s_forget_array_vals, s_forget_slice_vals, s_ofDeepHolds_array_size,
    s_ofDeepHolds_struct_val, NTy.forget_struct, NTy.forget_ofDeepHolds,
    NTy.holds_array, NTy.holds_slice, NTy.holds_sliceRef,
    NTy.holds_forget_eq, NTy.holds_struct_val,
    s_ofArrays_ext, s_ofArrays_cur_vals, s_ofArrays_fin_vals,
    s_withCurrent_ext, s_withCurrent_cur_vals, s_withCurrent_fin_vals,
    s_sliceRef_mk_ext, s_sliceRef_mk_cur, s_sliceRef_mk_fin,
    s_specArray_size, s_specVector_set_vals, SpecVector.values_mk,
    Array.set!_eq_setIfInBounds, Array.size_setIfInBounds,
    Array.map_setIfInBounds, Array.mem_map,
    List.map_toArray, List.setIfInBounds_toArray, List.getElem?_toArray,
    List.size_toArray, List.mem_toArray, List.mem_cons, List.not_mem_nil,
    List.map_cons, List.map_nil, List.set_cons_zero, List.set_cons_succ, List.set_nil,
    List.length_cons, List.length_nil, List.getElem?_cons_zero, List.getElem?_cons_succ,
    List.getElem?_nil, Option.getD_some, Option.some.injEq, Prod.mk.injEq, s_eq_prod_iff,
    reduceCtorEq, ite_true, ite_false,
    Int.toNat_natCast, Int.toNat_zero, Int.toNat_one] at *)

macro "verex_slice_simp_late" : tactic => `(tactic|
  simp (config := { failIfUnchanged := false }) only [
    VerexIR.Proofs.Obligation_eq,
    nominalCertifiedValue, nominalHolds, nominalForget,
    s_forget_array_vals, s_forget_slice_vals, s_ofDeepHolds_array_size,
    s_ofDeepHolds_struct_val, NTy.forget_struct, NTy.forget_ofDeepHolds,
    s_specArray_size, s_specVector_set_vals, SpecVector.values_mk,
    Array.set!_eq_setIfInBounds, Array.size_setIfInBounds, Array.size_map,
    Array.map_setIfInBounds, Array.getElem?_setIfInBounds, Array.getElem?_map,
    Array.mem_map, Option.map_eq_some_iff,
    List.map_toArray, List.setIfInBounds_toArray, List.getElem?_toArray,
    List.size_toArray, List.mem_toArray, List.mem_cons, List.not_mem_nil,
    List.map_cons, List.map_nil, List.set_cons_zero, List.set_cons_succ, List.set_nil,
    List.length_cons, List.length_nil, List.getElem?_cons_zero, List.getElem?_cons_succ,
    List.getElem?_nil, Option.getD_some, Option.some.injEq, Prod.mk.injEq, s_eq_prod_iff,
    reduceCtorEq, ite_true, ite_false,
    Int.toNat_natCast, Int.toNat_zero, Int.toNat_one] at *)

elab "verex_slice_dedup" : tactic => do
  let mut goal ← getMainGoal
  let toClear ← goal.withContext do
    let mut seen : Array Lean.Expr := #[]
    let mut drop : Array FVarId := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail || decl.isLet then continue
      let ty := (← instantiateMVars decl.type).consumeMData
      if ← isProp ty then
        if ty.isConstOf ``True || seen.any (· == ty) then
          drop := drop.push decl.fvarId
        else
          seen := seen.push ty
    pure drop
  for fv in toClear.reverse do
    goal ← goal.tryClear fv
  replaceMainGoal [goal]

elab "verex_slice_prod_eq" : tactic => do
  let mut goal ← getMainGoal
  let pfs ← goal.withContext do
    let mut out : Array (FVarId × Lean.Expr × Lean.Expr) := #[]
    for decl in ← getLCtx do
      if decl.isImplementationDetail then continue
      let ty ← instantiateMVars decl.type
      let some (eqTy, rawLhs, rawRhs) := ty.eq? | continue
      let lhs := rawLhs.consumeMData
      let rhs := rawRhs.consumeMData
      if rhs.isAppOfArity ``Prod.mk 4 || lhs.isAppOfArity ``Prod.mk 4 then
        if let some (p1, p2) ← try
            let f1 ← withLocalDeclD `z eqTy fun z => do
              mkLambdaFVars #[z] (← mkAppM ``Prod.fst #[z])
            let f2 ← withLocalDeclD `z eqTy fun z => do
              mkLambdaFVars #[z] (← mkAppM ``Prod.snd #[z])
            pure (some (← mkAppM ``congrArg #[f1, decl.toExpr],
                        ← mkAppM ``congrArg #[f2, decl.toExpr]))
          catch _ => pure none then
          out := out.push (decl.fvarId, p1, p2)
    pure out
  for (oldFv, p1, p2) in pfs do
    let t1 ← goal.withContext (inferType p1)
    let (_, g1) ← (← goal.assert `hProd1 t1 p1).intro1P
    let t2 ← g1.withContext (inferType p2)
    let (_, g2) ← (← g1.assert `hProd2 t2 p2).intro1P
    goal ← g2.tryClear oldFv
  replaceMainGoal [goal]

elab "verex_slice_close" : tactic => do
  evalTactic (← `(tactic| verex_paths))
  let mut rem : List MVarId := []
  for g in ← getGoals do
    setGoals [g]
    evalTactic (← `(tactic| verex_canonical_families))
    evalTactic (← `(tactic| all_goals verex_clear_computations))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals (try verex_boolean_decisions)))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals simp (config := { failIfUnchanged := false }) only [
      VerexIR.Proofs.Obligation_eq] at *))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_reduce_projections))
    evalTactic (← `(tactic| all_goals verex_open_holds))
    evalTactic (← `(tactic| all_goals verex_intro_any))
    evalTactic (← `(tactic| all_goals verex_open_holds))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals verex_finish_prepare))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_slice_dedup))
    evalTactic (← `(tactic| all_goals verex_slice_project))
    evalTactic (← `(tactic| all_goals verex_slice_dedup))
    evalTactic (← `(tactic| all_goals verex_slice_simp))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_slice_rw))
    evalTactic (← `(tactic| all_goals verex_slice_simp))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_slice_rw))
    evalTactic (← `(tactic| all_goals verex_slice_simp))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_slice_dedup))
    evalTactic (← `(tactic| all_goals verex_slice_mem_split))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_slice_simp_late))
    evalTactic (← `(tactic| all_goals verex_slice_mem_split))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals ((try split at *); (try split at *))))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if (← getUnsolvedGoals).isEmpty then continue
    evalTactic (← `(tactic| all_goals verex_slice_simp_late))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals verex_slice_mem))
    evalTactic (← `(tactic| all_goals verex_certified_clauses))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals verex_type_bounds))
    evalTactic (← `(tactic| all_goals verex_slice_simp_late))
    evalTactic (← `(tactic| all_goals verex_split_hypotheses))
    evalTactic (← `(tactic| all_goals verex_slice_prod_eq))
    evalTactic (← `(tactic| all_goals verex_subst_vars))
    evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    if !(← getUnsolvedGoals).isEmpty then
      evalTactic (← `(tactic| all_goals simp (config := { failIfUnchanged := false }) only [
        List.getElem_toArray, List.getElem_cons_zero, List.getElem_cons_succ] at *))
      evalTactic (← `(tactic| all_goals (try (first | assumption | contradiction | omega))))
    rem := rem ++ (← getUnsolvedGoals)
  setGoals rem

end SliceLeafSupport
