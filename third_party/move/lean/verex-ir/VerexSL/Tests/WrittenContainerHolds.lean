-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

set_option Elab.async false

/-! ## Written-container invariant preservation and whole-container bridges

`openHoldsGoal` (`verex_holds`) discharges whole-container `NTy.Holds` goals
through local `.vector`, `.slice`, `.array`, and `.sliceRef` hypotheses before
peeling `SpecVector.set` writes, and selects the written-container preservation
rule only on non-nominal containers whose base container fact and in-bounds
write provenance are in hand. Once selected, a failing element clause stays as
the written element's named obligation rather than falling back to elementwise
quantification over the updated backing array; when unselected (no old-container
fact in hand, an out-of-bounds spec write, or a declared nominal container with
its own cross-element invariant), the opener takes the structural or elementwise
reading directly. -/

open Lean Meta Elab Tactic VerexIR VerexIR.Proofs VerexIR.Proofs.Denote VerexSL

verex namespace written_container_holds where
  pragma verify = false

  struct Percent where
    value : u32
  spec Percent where
    invariant value <= 100

  struct OrderedPair where
    items : Vector<Percent, const 2>
  spec OrderedPair where
    invariant items[0].value <= items[1].value

  fun vec_write_invalid_bad(v : &mut Vector<Percent>, idx : usize) -> Unit := do
    (*v)[idx].value := 101u32
  spec vec_write_invalid_bad where
    requires idx < v.length

  fun array_two_writes_second_invalid_bad(r : &mut Vector<Percent, const 2>, a : u32) -> Unit := do
    (*r)[0usize].value := a
    (*r)[1usize].value := 101u32
  spec array_two_writes_second_invalid_bad where
    requires a <= 100

  fun ordered_pair_cross_clause_bad() -> OrderedPair := do
    let mut p := new OrderedPair {
      items := #[new Percent { value := 0u32 }, new Percent { value := 50u32 }] }
    let items := &mut p.items
    (*items)[0usize].value := 80u32
    p
  spec ordered_pair_cross_clause_bad where
    panics_if false

  fun vec_write_symbolic_ok(v : &mut Vector<Percent>, idx : usize, amount : u32) -> Unit := do
    (*v)[idx].value := amount
  spec vec_write_symbolic_ok where
    requires idx < v.length && amount <= 100
    panics_if false

  fun array_chained_two_writes_ok(r : &mut Vector<Percent, const 2>, a : u32, b : u32) -> Unit := do
    (*r)[0usize].value := a
    (*r)[1usize].value := b
  spec array_chained_two_writes_ok where
    requires a <= 100 && b <= 100
    panics_if false

  fun vec_repaired_before_death_ok(v : &mut Vector<Percent>, idx : usize, amount : u32) -> Unit := do
    let elem := &mut (*v)[idx]
    (*elem).value := 101u32
    (*elem).value := amount
  spec vec_repaired_before_death_ok where
    requires idx < v.length && amount <= 100
    panics_if false

  fun ordered_pair_write_symbolic_ok(amount : u32) -> OrderedPair := do
    let mut p := new OrderedPair {
      items := #[new Percent { value := 0u32 }, new Percent { value := 100u32 }] }
    let items := &mut p.items
    (*items)[0usize].value := amount
    p
  spec ordered_pair_write_symbolic_ok where
    requires amount <= 100
    panics_if false

-- Elaborate the unit's carrier family before running negative and tactic controls.
run_cmd do
  let some unit := VerexSL.registeredUnit? (← getEnv) `written_container_holds
    | throwError "missing `written_container_holds` unit"
  discard <| VerexSL.Verify.ensureFunctionCarriers #["written_container_holds"] unit

-- ============================================================================
-- 1. Source-level negative controls
-- ============================================================================

open LoanDeathControls in
run_cmd do
  let percent := "invariant value <= 100"
  let ordered := "invariant items[0].value <= items[1].value"
  expectUnestablished `written_container_holds.vec_write_invalid_bad [percent]
    (← `(command| verify written_container_holds::vec_write_invalid_bad))
  expectUnestablished `written_container_holds.array_two_writes_second_invalid_bad [percent]
    (← `(command| verify written_container_holds::array_two_writes_second_invalid_bad))
  expectCuts `written_container_holds.ordered_pair_cross_clause_bad [(ordered, mutationEnd)]
    (← `(command| verify written_container_holds::ordered_pair_cross_clause_bad))

open LoanDeathControls in
run_cmd do
  for (base, command) in [
      (`written_container_holds.vec_write_invalid_bad,
        ← `(command| verify written_container_holds::vec_write_invalid_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`written_container_holds.array_two_writes_second_invalid_bad,
        ← `(command| verify written_container_holds::array_two_writes_second_invalid_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega)))),
      (`written_container_holds.ordered_pair_cross_clause_bad,
        ← `(command| verify written_container_holds::ordered_pair_cross_clause_bad by lean =>
          verex_obligations; all_goals (verex_prepare only <;> (exfalso; omega))))] do
    expectScriptFails base command

-- ============================================================================
-- 2. Tactic-level negative, selection/boundary, and positive controls
-- ============================================================================

namespace VerexSL.Tests.WrittenContainerHolds

private def percentNTyName : Name :=
  ((((`written_container_holds.unit).str "nominal" |>.str "namespace").num 0).str "declaration").num 0

private def orderedPairNTyName : Name :=
  ((((`written_container_holds.unit).str "nominal" |>.str "namespace").num 0).str "declaration").num 1

elab "wh_percent_nty" : term => do
  return mkConst percentNTyName

elab "wh_ordered_pair_nty" : term => do
  return mkConst orderedPairNTyName

/-- Assert that no residual goal contains array lookup, write, or membership terms. -/
elab "assert_no_lookup_in_goals" : tactic => do
  for g in ← getGoals do
    g.withContext do
      let t ← instantiateMVars (← g.getType)
      if (t.find? fun sub =>
          sub.isAppOf ``GetElem?.getElem? ||
          sub.isAppOf ``Array.setIfInBounds ||
          sub.isAppOf ``Array.set! ||
          sub.isAppOf ``Membership.mem).isSome then
        throwError "unexpected container lookup or membership in goal:\n{← ppExpr t}"

/-- Assert that every residual goal contains `OwnAtArguments` and has exact count `n`. -/
elab "assert_named_clause_goals " n:num : tactic => do
  let goals ← getGoals
  unless goals.length == n.getNat do
    throwError "expected {n.getNat} residual goals, got {goals.length}"
  for g in goals do
    g.withContext do
      let t ← instantiateMVars (← g.getType)
      unless (t.find? (·.isAppOf ``OwnAtArguments)).isSome do
        throwError "expected named OwnAtArguments clause in goal, got:\n{← ppExpr t}"

/-- Assert that every residual goal reduced to a scalar `≤` clause with count `n`. -/
elab "assert_scalar_clause_goals " n:num : tactic => do
  let goals ← getGoals
  unless goals.length == n.getNat do
    throwError "expected {n.getNat} residual goals, got {goals.length}"
  for g in goals do
    g.withContext do
      let t := (← instantiateMVars (← g.getType)).consumeMData
      unless (t.find? (·.isAppOf ``LE.le)).isSome && (t.find? (·.isAppOf ``NTy.Holds)).isNone do
        throwError "expected scalar `≤` clause with no residual NTy.Holds, got:\n{← ppExpr t}"

/-- Assert that the single residual goal took the elementwise membership reading. -/
elab "assert_elementwise_reading" : tactic => do
  let goals ← getGoals
  unless goals.length == 1 do
    throwError "expected 1 elementwise goal, got {goals.length}"
  let g := goals[0]!
  g.withContext do
    let hasMem ← (← getLCtx).anyM fun d => do
      if d.isAuxDecl then return false
      let ty ← instantiateMVars d.type
      return (ty.find? (·.isAppOf ``Membership.mem)).isSome
    unless hasMem do
      throwError "expected elementwise membership hypothesis in local context, got:\n{← Meta.ppGoal g}"

noncomputable section

local instance : Carriers :=
  (@Skolems.runtime _root_.written_container_holds.unit _root_.written_container_holds.functionCarriers).toCarriers

local notation "PercentTy" => wh_percent_nty
local notation "OrderedPairTy" => wh_ordered_pair_nty

private def percent50 : (PercentTy : NTy).carrier :=
  (⟨50, by decide⟩, ())

private def percent80 : (PercentTy : NTy).carrier :=
  (⟨80, by decide⟩, ())

private def percent101 : (PercentTy : NTy).carrier :=
  (⟨101, by decide⟩, ())

macro "unseal_own_goal" : tactic =>
  `(tactic| (verex_own_encoded; all_goals (try verex_normalize); all_goals (try simp only [VerexIR.Proofs.Obligation_eq, percent50, percent80, percent101])))

-- 2.1. Tactic-level negative controls: once write preservation is selected, an
-- invalid written element leaves the exact named clause without falling back
-- to elementwise quantification.

theorem neg_vector_invalid_element_leaves_exact_clause
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (oldElem : (PercentTy : NTy).carrier)
    (_ : v.values[0]? = some oldElem)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds (.vector PercentTy) (v.set 0 percent101) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 1
  exact hImpossible

theorem neg_vector_invalid_element_refuted
    (v : (NTy.vector PercentTy).carrier)
    (oldElem : (PercentTy : NTy).carrier)
    (hInBounds : v.values[0]? = some oldElem) :
    ¬ NTy.Holds (.vector PercentTy) (v.set 0 percent101) := by
  intro hBad
  have hLt : 0 < v.values.size := (Array.getElem?_eq_some_iff.mp hInBounds).1
  have hRead : (v.set 0 percent101).values[0]? = some percent101 := by
    simp [SpecVector.values_set, Array.set!_eq_setIfInBounds, hLt]
  have hElem : (PercentTy : NTy).Holds percent101 := hBad.of_getElem? hRead
  dsimp only [percent101] at hElem
  verex_prepare

theorem neg_sliceRef_withCurrent_invalid_element_leaves_exact_clause
    (r : (NTy.sliceRef PercentTy).carrier)
    (hr : NTy.Holds (.sliceRef PercentTy) r)
    (oldElem : (PercentTy : NTy).carrier)
    (_ : r.currentVector.values[0]? = some oldElem)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds (.sliceRef PercentTy)
      (r.withCurrent ⟨r.currentVector.set 0 percent101,
        r.current.size_set 0 percent101⟩) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 1
  exact hImpossible

-- 2.2. Tactic-level selection and boundary controls: no old-container fact,
-- out-of-bounds write, same-index dead-write collapse, nominal cross-element
-- invariant preservation, and whole-container bridge priority.

theorem boundary_no_old_container_fact_uses_elementwise_and_proves_overwrite
    (v : (NTy.vector PercentTy).carrier)
    (hSize : v.values.size = 1)
    (oldElem : (PercentTy : NTy).carrier)
    (_ : v.values[0]? = some oldElem) :
    NTy.Holds (.vector PercentTy) (v.set 0 percent50) := by
  verex_holds
  assert_elementwise_reading
  rename_i elem hMem
  verex_normalize
  unseal_own_goal
  obtain ⟨k, hk, rfl⟩ := Array.getElem_of_mem hMem
  have hk0 : k = 0 := by simp [SpecVector.values_set] at hk; omega
  subst hk0
  simp [SpecVector.values_set, Array.set!_eq_setIfInBounds, percent50]

theorem boundary_oob_set_not_selected_and_provable
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (hSize : v.values.size = 1) :
    NTy.Holds (.vector PercentTy) (v.set 99 percent101) := by
  verex_holds
  assert_elementwise_reading
  rename_i elem hMem
  have hNotLt : ¬ (99 < v.values.size) := by omega
  simp only [SpecVector.values_set, Array.set!_eq_setIfInBounds,
    Array.setIfInBounds, dif_neg hNotLt] at hMem
  have hElem : (PercentTy : NTy).Holds elem := hv.of_mem hMem
  exact hElem.deepHolds.1

theorem boundary_same_index_overwrite_collapses_dead_write
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (oldElem : (PercentTy : NTy).carrier)
    (_ : v.values[0]? = some oldElem) :
    NTy.Holds (.vector PercentTy)
      ((v.set 0 percent101).set 0 percent50) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 1
  omega

theorem boundary_nominal_struct_preserves_own_cross_element_clause
    (items : (NTy.array PercentTy 2).carrier)
    (hItems : NTy.Holds (.array PercentTy 2) items)
    (hOrder : (OrderedPairTy : NTy).Holds
      (⟨items.vector.set 0 percent80, items.size_set 0 percent80⟩, ())) :
    NTy.Holds OrderedPairTy
      (⟨items.vector.set 0 percent80, items.size_set 0 percent80⟩, ()) := by
  have hOuterOwn := ((NTy.holds_struct _ _ _ _).mp hOrder).1
  clear hOrder
  verex_holds
  all_goals verex_normalize
  assert_named_clause_goals 2
  · exact hOuterOwn
  · unseal_own_goal
    omega

theorem boundary_whole_container_bridge_before_write_preservation
    (r : (NTy.array PercentTy 2).carrier)
    (_ : NTy.Holds (.array PercentTy 2) r)
    (invariantHolds : NTy.Holds (.array PercentTy 2)
      ⟨r.vector.set 0 percent101, r.size_set 0 percent101⟩) :
    NTy.Holds (.slice PercentTy) (r.vector.set 0 percent101) := by
  verex_holds

-- 2.3. Tactic-level positive controls: `.vector`, `.array`, `.slice` projections,
-- nested `SpecSliceRef.withCurrent`, and chained element writes.

theorem pos_vector_set_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (idx : Nat) (oldElem : (PercentTy : NTy).carrier)
    (_ : v.values[idx]? = some oldElem)
    (amount : (NTy.int 32 false).carrier) (hValid : amount.val ≤ 100) :
    NTy.Holds (.vector PercentTy) (v.set idx (amount, ())) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  all_goals verex_decide

theorem pos_array_set_closes
    (r : (NTy.array PercentTy 2).carrier)
    (hr : NTy.Holds (.array PercentTy 2) r)
    (amount : (NTy.int 32 false).carrier) (hValid : amount.val ≤ 100) :
    NTy.Holds (.array PercentTy 2)
      ⟨r.vector.set 0 (amount, ()), r.size_set 0 (amount, ())⟩ := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  all_goals verex_decide

theorem pos_slice_currentVector_set_closes
    (r : (NTy.sliceRef PercentTy).carrier)
    (hr : NTy.Holds (.sliceRef PercentTy) r)
    (oldElem : (PercentTy : NTy).carrier)
    (_ : r.currentVector.values[0]? = some oldElem)
    (amount : (NTy.int 32 false).carrier) (hValid : amount.val ≤ 100) :
    NTy.Holds (.slice PercentTy) (r.currentVector.set 0 (amount, ())) ∧
    NTy.Holds (.slice PercentTy)
      (SpecArray.mk (length := r.extent) (r.currentVector.set 0 (amount, ()))
        (r.current.size_set 0 (amount, ()))).vector := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 2
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  all_goals verex_decide

theorem pos_sliceRef_withCurrent_single_and_nested_closes
    (r : (NTy.sliceRef PercentTy).carrier)
    (hr : NTy.Holds (.sliceRef PercentTy) r)
    (w : SpecArray ((PercentTy : NTy).carrier) r.extent)
    (oldElem : (PercentTy : NTy).carrier)
    (_ : r.currentVector.values[0]? = some oldElem)
    (amount : (NTy.int 32 false).carrier) (hValid : amount.val ≤ 100) :
    NTy.Holds (.sliceRef PercentTy)
      ((r.withCurrent w).withCurrent
        ⟨r.currentVector.set 0 (amount, ()),
          r.current.size_set 0 (amount, ())⟩) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  all_goals verex_decide

theorem pos_chained_two_element_write_closes
    (r : (NTy.array PercentTy 2).carrier)
    (hr : NTy.Holds (.array PercentTy 2) r)
    (a0 a1 : (NTy.int 32 false).carrier)
    (h0 : a0.val ≤ 100) (h1 : a1.val ≤ 100) :
    NTy.Holds (.array PercentTy 2)
      ⟨(r.vector.set 0 (a0, ())).set 1 (a1, ()),
        by simp [SpecVector.values_set, r.size_eq]⟩ := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 2
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  all_goals verex_decide

theorem pos_chained_symbolic_two_element_write_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (i j : Nat) (old0 old1 : (PercentTy : NTy).carrier)
    (_ : v.values[i]? = some old0)
    (_ : (v.values.setIfInBounds i percent50)[j]? = some old1)
    (a0 a1 : (NTy.int 32 false).carrier)
    (h0 : a0.val ≤ 100) (h1 : a1.val ≤ 100) :
    NTy.Holds (.vector PercentTy) ((v.set i (a0, ())).set j (a1, ())) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 2
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  all_goals verex_decide

end
end VerexSL.Tests.WrittenContainerHolds

-- ============================================================================
-- 3. Source-level positive controls and axiom audit
-- ============================================================================

verify written_container_holds::vec_write_symbolic_ok
verify written_container_holds::array_chained_two_writes_ok
verify written_container_holds::vec_repaired_before_death_ok
verify written_container_holds::ordered_pair_write_symbolic_ok

run_cmd do
  let env ← getEnv
  for name in [
      `VerexSL.Tests.WrittenContainerHolds.neg_vector_invalid_element_leaves_exact_clause,
      `VerexSL.Tests.WrittenContainerHolds.neg_vector_invalid_element_refuted,
      `VerexSL.Tests.WrittenContainerHolds.neg_sliceRef_withCurrent_invalid_element_leaves_exact_clause,
      `VerexSL.Tests.WrittenContainerHolds.boundary_no_old_container_fact_uses_elementwise_and_proves_overwrite,
      `VerexSL.Tests.WrittenContainerHolds.boundary_oob_set_not_selected_and_provable,
      `VerexSL.Tests.WrittenContainerHolds.boundary_same_index_overwrite_collapses_dead_write,
      `VerexSL.Tests.WrittenContainerHolds.boundary_nominal_struct_preserves_own_cross_element_clause,
      `VerexSL.Tests.WrittenContainerHolds.boundary_whole_container_bridge_before_write_preservation,
      `VerexSL.Tests.WrittenContainerHolds.pos_vector_set_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_array_set_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_slice_currentVector_set_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_sliceRef_withCurrent_single_and_nested_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_chained_two_element_write_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_chained_symbolic_two_element_write_closes,
      `written_container_holds.vec_write_symbolic_ok.typedVerified,
      `written_container_holds.array_chained_two_writes_ok.typedVerified,
      `written_container_holds.vec_repaired_before_death_ok.typedVerified,
      `written_container_holds.ordered_pair_write_symbolic_ok.typedVerified] do
    unless env.contains name do throwError "written-container control {name} did not verify"
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"
