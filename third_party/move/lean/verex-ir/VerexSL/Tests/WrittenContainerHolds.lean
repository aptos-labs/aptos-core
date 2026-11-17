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
reading directly. The same selection governs the two other readings of a
container's fact: the element a goal reads at a position, which the fact gives
where the read carries its bounds proof, and the whole-container operations
`eraseBounded` and `swapBounded`, whose result holds where their operand's
fact is in hand. -/

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

-- A removal reads its element as the one optional read of a container, so the
-- element's own fact is opened into its fields' and a clause about a field of
-- the removed element is decided from them. An insertion is an operation of
-- its own, whose result holds where the vector and the inserted element do,
-- and an append is the insertion at the length.
verex module 0x42::written_vector_elements where
  pragma verify = false

  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Rows {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<Entry<K, V> >

  struct Plain {K has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<K>

  -- A removal returning a field of the removed element.
  fun remove_rows {K has Copy, Drop, Store} {V has Copy, Drop, Store}(box : &mut Rows<K, V>,
      index : u64) -> V := do
    let items := &mut box.items
    let (removed, rest) := core.prim.removeVector(*items, index)
    *items := rest
    removed.value
  spec remove_rows where
    requires index < box.items.length
    ensures box.items.length + 1 == old(box).items.length

  -- The removed element itself, at the parameter's type; and a field of an
  -- element read without a removal.
  fun remove_plain {K has Copy, Drop, Store}(box : &mut Plain<K>, index : u64) -> K := do
    let items := &mut box.items
    let (removed, rest) := core.prim.removeVector(*items, index)
    *items := rest
    removed
  spec remove_plain where
    requires index < box.items.length
    ensures box.items.length + 1 == old(box).items.length

  fun remove_rows_whole {K has Copy, Drop, Store} {V has Copy, Drop, Store}(box : &mut Rows<K, V>,
      index : u64) -> Entry<K, V> := do
    let items := &mut box.items
    let (removed, rest) := core.prim.removeVector(*items, index)
    *items := rest
    removed
  spec remove_rows_whole where
    requires index < box.items.length
    ensures box.items.length + 1 == old(box).items.length

  fun read_rows {K has Copy, Drop, Store} {V has Copy, Drop, Store}(box : &Rows<K, V>,
      index : u64) -> V :=
    box.items[index].value
  spec read_rows where
    requires index < box.items.length

  struct LimitedRows {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<Entry<K, V> >
  spec LimitedRows where
    invariant items.length <= 100

  -- An insertion written back through a mutable borrow, at a type parameter
  -- and at a closed instance.
  fun insert_plain {K has Copy, Drop, Store}(box : &mut Plain<K>, index : u64, key : K) -> Unit := do
    let items := &mut box.items
    *items := core.prim.insertVector(*items, index, key)
  spec insert_plain where
    requires index <= box.items.length
    ensures box.items.length == old(box).items.length + 1

  fun insert_limited_rows_closed(box : &mut LimitedRows<u64, u64>, index : u64, key : u64,
      value : u64) -> Unit := do
    let items := &mut box.items
    *items := core.prim.insertVector(*items, index, new Entry<u64, u64> { key := key, value := value })
  spec insert_limited_rows_closed where
    requires index <= box.items.length && box.items.length < 100
    ensures box.items.length == old(box).items.length + 1

  struct Share has Copy, Drop, Store where
    value : u64
  spec Share where
    invariant value <= 100

  enum Payment has Copy, Drop, Store where
    | None
    | Direct (amount : u64)
  spec Payment where
    invariant match this with
      | Payment::None {} => true
      | Payment::Direct { amount := amount } => amount > 0

  -- An append of an element whose fact is in hand, to a vector of its type.
  fun push_struct_ok(v : Vector<Share>, p : Share) -> Vector<Share> :=
    core.prim.pushVector(v, p)
  spec push_struct_ok where
    aborts_if false
    ensures result.length == v.length + 1
    ensures result[v.length] == p

  fun push_enum_ok(v : Vector<Payment>, amount : u64) -> Vector<Payment> :=
    core.prim.pushVector(v, new Payment::Direct { amount })
  spec push_enum_ok where
    requires amount > 0
    aborts_if false
    ensures result.length == v.length + 1

  -- An append of a value its type's invariant refuses, and an append that
  -- does not leave the length it was given.
  fun push_struct_bad(v : Vector<Share>) -> Vector<Share> :=
    core.prim.pushVector(v, new Share { value := 101 })
  spec push_struct_bad where
    aborts_if false

  fun push_enum_bad(v : Vector<Payment>) -> Vector<Payment> :=
    core.prim.pushVector(v, new Payment::Direct { amount := 0 })
  spec push_enum_bad where
    aborts_if false

  fun push_struct_length_bad(v : Vector<Share>, p : Share) -> Vector<Share> :=
    core.prim.pushVector(v, p)
  spec push_struct_length_bad where
    aborts_if false
    ensures result.length == v.length

  -- The removal drops one element, so the length it leaves is not the one it
  -- was given.
  fun remove_rows_length_bad {K has Copy, Drop, Store} {V has Copy, Drop, Store}
      (box : &mut Rows<K, V>, index : u64) -> V := do
    let items := &mut box.items
    let (removed, rest) := core.prim.removeVector(*items, index)
    *items := rest
    removed.value
  spec remove_rows_length_bad where
    requires index < box.items.length
    ensures box.items.length == old(box).items.length

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
  expectCuts `written_container_holds.vec_write_invalid_bad [(percent, exit)]
    (← `(command| verify written_container_holds::vec_write_invalid_bad)) (some [])
  expectCuts `written_container_holds.array_two_writes_second_invalid_bad [(percent, exit)]
    (← `(command| verify written_container_holds::array_two_writes_second_invalid_bad)) (some [])
  expectCuts `written_container_holds.ordered_pair_cross_clause_bad [(ordered, mutationEnd)]
    (← `(command| verify written_container_holds::ordered_pair_cross_clause_bad))
  expectUnestablished `«0x42».written_vector_elements.remove_rows_length_bad
    ["ensures box.items.length == old(box).items.length"]
    (← `(command| verify 0x42::written_vector_elements::remove_rows_length_bad))
  let payment := "invariant match this with\n" ++
    "      | Payment::None {} => true\n" ++
    "      | Payment::Direct { amount := amount } => amount > 0"
  expectCuts `«0x42».written_vector_elements.push_struct_bad [(percent, construction)]
    (← `(command| verify 0x42::written_vector_elements::push_struct_bad)) (some [])
  expectCuts `«0x42».written_vector_elements.push_enum_bad [(payment, construction)]
    (← `(command| verify 0x42::written_vector_elements::push_enum_bad)) (some [])
  expectUnestablished `«0x42».written_vector_elements.push_struct_length_bad
    ["ensures result.length == v.length"]
    (← `(command| verify 0x42::written_vector_elements::push_struct_length_bad))

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

-- 2.4. Tactic-level controls for the element a goal reads out of a container:
-- selected only when that container's own fact is in hand, otherwise the
-- element's own type is read structurally.

theorem neg_read_element_without_container_fact_reads_structurally
    (v : (NTy.vector PercentTy).carrier) (inBounds : 0 < v.values.size)
    (hBound : (v.values[0]'inBounds).1.val ≤ 100) :
    NTy.Holds PercentTy (v.values[0]'inBounds) := by
  verex_holds
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_scalar_clause_goals 1
  omega

theorem neg_read_element_of_other_container_reads_structurally
    (v w : (NTy.vector PercentTy).carrier)
    (_hw : NTy.Holds (.vector PercentTy) w) (inBounds : 0 < v.values.size)
    (hBound : (v.values[0]'inBounds).1.val ≤ 100) :
    NTy.Holds PercentTy (v.values[0]'inBounds) := by
  verex_holds
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_scalar_clause_goals 1
  omega

theorem pos_read_element_of_vector_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) (idx : Nat)
    (inBounds : idx < v.values.size) :
    NTy.Holds PercentTy (v.values[idx]'inBounds) := by
  verex_holds

theorem pos_read_element_of_array_closes
    (r : (NTy.array PercentTy 2).carrier)
    (hr : NTy.Holds (.array PercentTy 2) r)
    (inBounds : 0 < r.vector.values.size) :
    NTy.Holds PercentTy (r.vector.values[0]'inBounds) := by
  verex_holds

theorem pos_read_element_of_slice_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.slice PercentTy) v) (idx : Nat)
    (inBounds : idx < v.values.size) :
    NTy.Holds PercentTy (v.values[idx]'inBounds) := by
  verex_holds

theorem pos_read_element_of_sliceRef_current_closes
    (r : (NTy.sliceRef PercentTy).carrier)
    (hr : NTy.Holds (.sliceRef PercentTy) r)
    (inBounds : 0 < r.currentVector.values.size) :
    NTy.Holds PercentTy (r.currentVector.values[0]'inBounds) := by
  verex_holds

-- 2.5. Tactic-level controls for the whole-container operations that preserve
-- a container's fact: selected only when the operand's own fact is in hand,
-- otherwise the opener takes the elementwise reading.

private def emptyPercents : (NTy.vector PercentTy).carrier := ⟨#[], by decide⟩

theorem neg_eraseBounded_of_other_container_reads_elementwise
    (w : (NTy.vector PercentTy).carrier)
    (_hw : NTy.Holds (.vector PercentTy) w) :
    NTy.Holds (.vector PercentTy) (eraseBounded emptyPercents 0) := by
  verex_holds
  assert_elementwise_reading
  rename_i elem hMem
  simp [emptyPercents, eraseBounded_values] at hMem

theorem pos_eraseBounded_preserves_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) (i : Nat) :
    NTy.Holds (.vector PercentTy) (eraseBounded v i) := by
  verex_holds

theorem pos_swapBounded_preserves_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) (i j : Nat) :
    NTy.Holds (.vector PercentTy) (swapBounded v i j) := by
  verex_holds

theorem pos_eraseBounded_of_swapBounded_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) (i j : Nat) :
    NTy.Holds (.vector PercentTy) (eraseBounded (swapBounded v i j) j) := by
  verex_holds

theorem pos_eraseBounded_of_array_fact_closes
    (r : (NTy.array PercentTy 2).carrier)
    (hr : NTy.Holds (.array PercentTy 2) r) (i : Nat) :
    NTy.Holds (.slice PercentTy) (eraseBounded r.vector i) := by
  verex_holds

-- 2.5b. Tactic-level controls for an insertion: it keeps the elements its
-- vector holds and adds one of its own, so each inserted element owes its
-- clause, no earlier insertion is dead, and the position may be the length.

-- Negative (first): two insertions at the same position, the first element
-- invalid. Both inserted elements are in the result, so both clauses stay and
-- the invalid one is the obligation that is not met.
theorem neg_two_insertions_same_index_keep_the_first_element_clause
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (firstBounded : (v.values.insertIdxIfInBounds 0 percent101).size < 2 ^ 64)
    (secondBounded :
      ((insertBounded v 0 percent101 firstBounded).values.insertIdxIfInBounds 0 percent50).size <
        2 ^ 64)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds (.vector PercentTy)
      (insertBounded (insertBounded v 0 percent101 firstBounded) 0 percent50 secondBounded) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 2
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 2
  · exact hImpossible
  · omega

-- Positive: the same two insertions with both elements valid.
theorem pos_two_insertions_same_index_both_valid_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (firstBounded : (v.values.insertIdxIfInBounds 0 percent80).size < 2 ^ 64)
    (secondBounded :
      ((insertBounded v 0 percent80 firstBounded).values.insertIdxIfInBounds 0 percent50).size <
        2 ^ 64) :
    NTy.Holds (.vector PercentTy)
      (insertBounded (insertBounded v 0 percent80 firstBounded) 0 percent50 secondBounded) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 2
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 2
  all_goals omega

-- Positive: at the length, where no element is written, and into a vector
-- that has none.
theorem pos_insert_at_size_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (bounded : (v.values.insertIdxIfInBounds v.values.size percent50).size < 2 ^ 64) :
    NTy.Holds (.vector PercentTy) (insertBounded v v.values.size percent50 bounded) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_scalar_clause_goals 1
  omega

theorem pos_insert_into_empty_closes
    (hEmpty : NTy.Holds (.vector PercentTy) emptyPercents)
    (bounded : (emptyPercents.values.insertIdxIfInBounds 0 percent50).size < 2 ^ 64) :
    NTy.Holds (.vector PercentTy) (insertBounded emptyPercents 0 percent50 bounded) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_scalar_clause_goals 1
  omega

-- Negative: without the vector's own fact the operation is not selected and
-- the opener takes the elementwise reading, as it does for a removal.
theorem neg_insertBounded_of_other_container_reads_elementwise
    (w : (NTy.vector PercentTy).carrier)
    (_hw : NTy.Holds (.vector PercentTy) w)
    (hFifty : NTy.Holds PercentTy percent50)
    (bounded : (emptyPercents.values.insertIdxIfInBounds 0 percent50).size < 2 ^ 64) :
    NTy.Holds (.vector PercentTy) (insertBounded emptyPercents 0 percent50 bounded) := by
  verex_holds
  assert_elementwise_reading
  rename_i elem hMem
  simp only [insertBounded_values, emptyPercents, Array.insertIdxIfInBounds] at hMem
  have inserted : elem = percent50 := by simpa using hMem
  subst inserted
  exact hFifty.deepHolds.1

-- 2.5c. Tactic-level controls for an append, the insertion at the length:
-- its result holds where the vector and the appended element do, the element
-- owes its clause, and so does every element inserted before it.

-- Negative (first): an appended element its type refuses leaves exactly its
-- clause, which is not met.
theorem neg_push_invalid_element_leaves_its_clause
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds (.vector PercentTy) (v.push percent101) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 1
  exact hImpossible

-- Negative: an append after the insertion of a refused element keeps that
-- element's clause beside its own.
theorem neg_push_after_invalid_insertion_keeps_the_inserted_clause
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (bounded : (v.values.insertIdxIfInBounds 0 percent101).size < 2 ^ 64)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds (.vector PercentTy) ((insertBounded v 0 percent101 bounded).push percent50) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 2
  all_goals unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 2
  · exact hImpossible
  · omega

-- Negative: without the vector's own fact the append is not selected and the
-- opener takes the elementwise reading, as it does for an insertion.
theorem neg_push_of_other_container_reads_elementwise
    (w : (NTy.vector PercentTy).carrier)
    (_hw : NTy.Holds (.vector PercentTy) w)
    (hFifty : NTy.Holds PercentTy percent50) :
    NTy.Holds (.vector PercentTy) (emptyPercents.push percent50) := by
  verex_holds
  assert_elementwise_reading
  rename_i elem hMem
  have appended : elem = percent50 := by
    simpa [emptyPercents, VerexIR.SpecVector.push] using hMem
  subst appended
  exact hFifty.deepHolds.1

-- Positive: an appended element its type accepts.
theorem pos_push_valid_element_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) :
    NTy.Holds (.vector PercentTy) (v.push percent50) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_scalar_clause_goals 1
  omega

-- 2.5d. Tactic-level controls for a concatenation: its result holds exactly
-- the elements of both of its vectors, so its fact is the facts of both. It is
-- selected where the fact of one of them is in hand, and the other one is then
-- asked for its fact whole; where neither is, the opener takes the elementwise
-- reading of the result. The element type is a parameter, so an element's
-- fact that is owed stays the goal it is.

-- Negative (first): the fact of the second vector is missing. What stays owed
-- is the fact of the elements of that vector alone.
theorem neg_concatBounded_second_fact_missing_stays_owed
    (l r : (NTy.vector (NTy.param 0)).carrier)
    (hl : NTy.Holds (.vector (NTy.param 0)) l)
    (bounded : (l.values ++ r.values).size < 2 ^ 64)
    (owed : ∀ value ∈ r.values, NTy.Holds (.param 0) value) :
    NTy.Holds (.vector (NTy.param 0)) (concatBounded l r bounded) := by
  fail_if_success (verex_holds; done)
  verex_holds
  assert_elementwise_reading
  rename_i element member
  exact owed element member

-- Negative: the fact of the first vector is missing.
theorem neg_concatBounded_first_fact_missing_stays_owed
    (l r : (NTy.vector (NTy.param 0)).carrier)
    (hr : NTy.Holds (.vector (NTy.param 0)) r)
    (bounded : (l.values ++ r.values).size < 2 ^ 64)
    (owed : ∀ value ∈ l.values, NTy.Holds (.param 0) value) :
    NTy.Holds (.vector (NTy.param 0)) (concatBounded l r bounded) := by
  fail_if_success (verex_holds; done)
  verex_holds
  assert_elementwise_reading
  rename_i element member
  exact owed element member

-- Negative: the fact of neither vector is in hand, although another vector's
-- is. Nothing is selected, and the reading is elementwise over the result.
theorem neg_concatBounded_without_facts_reads_elementwise
    (l r w : (NTy.vector (NTy.param 0)).carrier)
    (_hw : NTy.Holds (.vector (NTy.param 0)) w)
    (bounded : (l.values ++ r.values).size < 2 ^ 64)
    (owed : ∀ value ∈ (concatBounded l r bounded).values, NTy.Holds (.param 0) value) :
    NTy.Holds (.vector (NTy.param 0)) (concatBounded l r bounded) := by
  fail_if_success (verex_holds; done)
  verex_holds
  assert_elementwise_reading
  rename_i element member
  exact owed element member

-- Negative: a literal of one element whose fact is missing. The vector's fact
-- is taken, and the element's own fact is what stays owed.
theorem neg_concatBounded_literal_element_fact_missing_stays_owed
    (l : (NTy.vector (NTy.param 0)).carrier) (e : (NTy.param 0).carrier)
    (hl : NTy.Holds (.vector (NTy.param 0)) l) (single : #[e].size < 2 ^ 64)
    (bounded : (l.values ++ (⟨#[e], single⟩ : (NTy.vector (NTy.param 0)).carrier).values).size <
      2 ^ 64)
    (owed : ∀ value ∈ #[e], NTy.Holds (.param 0) value) :
    NTy.Holds (.vector (NTy.param 0)) (concatBounded l ⟨#[e], single⟩ bounded) := by
  fail_if_success (verex_holds; done)
  verex_holds
  assert_elementwise_reading
  rename_i element member
  exact owed element member

-- Positive: both facts in hand.
theorem pos_concatBounded_both_facts_closes
    (l r : (NTy.vector (NTy.param 0)).carrier)
    (hl : NTy.Holds (.vector (NTy.param 0)) l) (hr : NTy.Holds (.vector (NTy.param 0)) r)
    (bounded : (l.values ++ r.values).size < 2 ^ 64) :
    NTy.Holds (.vector (NTy.param 0)) (concatBounded l r bounded) := by
  verex_holds

-- Positive: the same at an element type that declares an invariant.
theorem pos_concatBounded_declared_elements_closes
    (l r : (NTy.vector PercentTy).carrier)
    (hl : NTy.Holds (.vector PercentTy) l) (hr : NTy.Holds (.vector PercentTy) r)
    (bounded : (l.values ++ r.values).size < 2 ^ 64) :
    NTy.Holds (.vector PercentTy) (concatBounded l r bounded) := by
  verex_holds

-- Positive: a concatenation of the results of other operations, each reached
-- through its own vector.
theorem pos_concatBounded_of_operations_closes
    (l r : (NTy.vector PercentTy).carrier)
    (hl : NTy.Holds (.vector PercentTy) l) (hr : NTy.Holds (.vector PercentTy) r) (i j : Nat)
    (inner : ((swapBounded l i j).values ++ (eraseBounded r i).values).size < 2 ^ 64)
    (outer : ((concatBounded (swapBounded l i j) (eraseBounded r i) inner).values ++ l.values).size <
      2 ^ 64) :
    NTy.Holds (.vector PercentTy)
      (concatBounded (concatBounded (swapBounded l i j) (eraseBounded r i) inner) l outer) := by
  verex_holds

-- 2.5e. Tactic-level controls for a slice and a reversed range: each keeps
-- only elements of its vector, so its fact is that vector's where it is in
-- hand. Where it is not, nothing is selected and the reading is elementwise
-- over the result: the fact of the whole vector is not asked for.

-- Negative (first): no fact of the sliced vector in hand, although another
-- vector's is. The elements of the slice alone stay owed.
theorem neg_extract_without_fact_reads_elementwise
    (v w : (NTy.vector (NTy.param 0)).carrier) (lower upper : Nat)
    (_hw : NTy.Holds (.vector (NTy.param 0)) w)
    (owed : ∀ value ∈ (VerexIR.SpecVector.extract v lower upper).values, NTy.Holds (.param 0) value) :
    NTy.Holds (.vector (NTy.param 0)) (VerexIR.SpecVector.extract v lower upper) := by
  fail_if_success (verex_holds; done)
  verex_holds
  assert_elementwise_reading
  rename_i element member
  exact owed element member

-- Negative: the same for a reversed range.
theorem neg_reverseSliceBounded_without_fact_reads_elementwise
    (v w : (NTy.vector (NTy.param 0)).carrier) (left right : Nat)
    (_hw : NTy.Holds (.vector (NTy.param 0)) w)
    (owed : ∀ value ∈ (reverseSliceBounded v left right).values, NTy.Holds (.param 0) value) :
    NTy.Holds (.vector (NTy.param 0)) (reverseSliceBounded v left right) := by
  fail_if_success (verex_holds; done)
  verex_holds
  assert_elementwise_reading
  rename_i element member
  exact owed element member

-- Positive: the vector's fact in hand, at an opaque and at a declared
-- element type, and through another operation.
theorem pos_extract_preserves_closes
    (v : (NTy.vector (NTy.param 0)).carrier)
    (hv : NTy.Holds (.vector (NTy.param 0)) v) (lower upper : Nat) :
    NTy.Holds (.vector (NTy.param 0)) (VerexIR.SpecVector.extract v lower upper) := by
  verex_holds

theorem pos_extract_declared_elements_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) (lower upper : Nat) :
    NTy.Holds (.vector PercentTy) (VerexIR.SpecVector.extract v lower upper) := by
  verex_holds

theorem pos_reverseSliceBounded_preserves_closes
    (v : (NTy.vector (NTy.param 0)).carrier)
    (hv : NTy.Holds (.vector (NTy.param 0)) v) (left right : Nat) :
    NTy.Holds (.vector (NTy.param 0)) (reverseSliceBounded v left right) := by
  verex_holds

theorem pos_reverseSliceBounded_of_extract_declared_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v) (lower upper left right : Nat) :
    NTy.Holds (.vector PercentTy)
      (reverseSliceBounded (VerexIR.SpecVector.extract v lower upper) left right) := by
  verex_holds

-- 2.6. Tactic-level controls for a fact at a type parameter, whose family is
-- not known: it is the fact of a value as any other, and the opener leaves the
-- goal as it stands where no fact of it or of its container is in hand.

theorem neg_param_element_without_container_fact_is_not_read
    (v : (NTy.vector (NTy.param 0)).carrier) (idx : Nat)
    (inBounds : idx < v.values.size)
    (hElement : NTy.Holds (.param 0) (v.values[idx]'inBounds)) :
    NTy.Holds (.param 0) (v.values[idx]'inBounds) := by
  fail_if_success (clear hElement; verex_holds)
  exact hElement

theorem pos_param_element_in_hand_closes
    (v : (NTy.vector (NTy.param 0)).carrier) (idx : Nat)
    (inBounds : idx < v.values.size)
    (hElement : NTy.Holds (.param 0) (v.values[idx]'inBounds)) :
    NTy.Holds (.param 0) (v.values[idx]'inBounds) := by
  verex_holds

theorem pos_param_read_element_closes
    (v : (NTy.vector (NTy.param 0)).carrier)
    (hv : NTy.Holds (.vector (NTy.param 0)) v) (idx : Nat)
    (inBounds : idx < v.values.size) :
    NTy.Holds (.param 0) (v.values[idx]'inBounds) := by
  verex_holds

-- 2.6b. Tactic-level controls for a check at the carriers a call's type
-- arguments induce, as the body of a generic callee inlined on the generic
-- route emits it. The normal form states the check at the outer carriers
-- (`checkingAtOuterFamily`), of the same value, so the fact it owes is the
-- one the caller's context holds. The opener itself reconciles nothing: it
-- reads a fact at the carriers it is stated at.

-- Negative (first): the opener does not take a fact at the outer carriers
-- for the goal at the induced ones.
theorem neg_opener_does_not_reconcile_families
    (v : (NTy.vector (NTy.param 0)).carrier)
    (_hv : NTy.Holds (.vector (NTy.param 0)) v)
    (owed : @NTy.Holds (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
      (.vector (NTy.param 0)) v) :
    @NTy.Holds (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
      (.vector (NTy.param 0)) v := by
  fail_if_success (clear owed; verex_holds; done)
  exact owed

-- Negative: after the check is stated at the outer carriers, the fact of
-- another vector gives nothing; the fact of this one stays owed.
theorem neg_induced_family_check_of_other_value_stays_owed {State : Type} (state : State)
    (v w : (NTy.vector (NTy.param 0)).carrier)
    (_hw : NTy.Holds (.vector (NTy.param 0)) w)
    (owed : ∀ value ∈ v.values, NTy.Holds (.param 0) value) :
    VerexIR.Proofs.wp
      (@checkingAt (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
        State 7 .exit (.vector (NTy.param 0)) v)
      (fun _ _ => True) (fun _ => True) state := by
  simp only [checkingAtOuterFamily]
  apply wp_checkingAt_intro_holds
  · fail_if_success (verex_holds; done)
    verex_holds
    assert_elementwise_reading
    rename_i element member
    exact owed element member
  · intro _; trivial

-- Positive: the fact at the induced carriers alone, of the goal as it is
-- stated: the opener closes it whole.
theorem pos_induced_family_fact_at_its_own_family_closes
    (v : @NTy.carrier (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
      (.vector (NTy.param 0)))
    (hv : @NTy.Holds (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
      (.vector (NTy.param 0)) v) :
    @NTy.Holds (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
      (.vector (NTy.param 0)) v := by
  verex_holds

-- Positive: the check at the induced carriers is the check at the outer
-- ones, of the same value.
theorem pos_induced_family_check_is_the_outer_check {State : Type}
    (v : (NTy.vector (NTy.param 0)).carrier) :
    @checkingAt (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
        State 7 .exit (.vector (NTy.param 0)) v =
      @checkingAt inferInstance State 7 .exit (.vector (NTy.param 0)) v := by
  simp only [checkingAtOuterFamily]

-- Positive: the check of a swap and a removal of a vector whose fact the
-- context holds at the outer carriers, through the operations' own laws.
theorem pos_induced_family_check_of_operations_closes {State : Type} (state : State)
    (v : (NTy.vector (NTy.param 0)).carrier)
    (hv : NTy.Holds (.vector (NTy.param 0)) v) (i j : Nat) :
    VerexIR.Proofs.wp
      (@checkingAt (Carriers.instantiate (⟨.cons (.param 0) .nil, rfl⟩ : TypeArgs) inferInstance)
        State 7 .exit (.vector (NTy.param 0)) (eraseBounded (swapBounded v i j) j))
      (fun _ _ => True) (fun _ => True) state := by
  simp only [checkingAtOuterFamily]
  apply wp_checkingAt_intro_holds
  · verex_holds
  · intro _; trivial

-- 2.7. Tactic-level controls for an element read out of an inserting
-- expression: the read at the written position is the inserted element, so
-- the opener leaves exactly that element's clause and no elementwise lookup,
-- and it closes where that element's fact is in hand.  A moving operation over
-- the written base leaves the same clause.

theorem neg_read_element_of_written_vector_leaves_inserted_clause
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (_ : 0 < v.values.size)
    (inBounds : 0 < (v.set 0 percent101).values.size)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds PercentTy ((v.set 0 percent101).values[0]'inBounds) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 1
  exact hImpossible

theorem pos_read_element_of_written_vector_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (element : (PercentTy : NTy).carrier)
    (hElement : NTy.Holds PercentTy element)
    (_ : 0 < v.values.size)
    (inBounds : 0 < (v.set 0 element).values.size) :
    NTy.Holds PercentTy ((v.set 0 element).values[0]'inBounds) := by
  verex_holds

theorem pos_swapped_written_vector_holds_closes
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (element : (PercentTy : NTy).carrier)
    (hElement : NTy.Holds PercentTy element)
    (_ : 0 < v.values.size)
    (left right : Nat) :
    NTy.Holds (.vector PercentTy) (swapBounded (v.set 0 element) left right) := by
  verex_holds

theorem neg_read_element_of_swapped_written_vector_leaves_inserted_clause
    (v : (NTy.vector PercentTy).carrier)
    (hv : NTy.Holds (.vector PercentTy) v)
    (_ : 0 < v.values.size)
    (left right idx : Nat)
    (inBounds : idx < (swapBounded (v.set 0 percent101) left right).values.size)
    (hImpossible : (101 : Int) ≤ 100) :
    NTy.Holds PercentTy
      ((swapBounded (v.set 0 percent101) left right).values[idx]'inBounds) := by
  verex_holds
  assert_no_lookup_in_goals
  all_goals verex_normalize
  assert_named_clause_goals 1
  unseal_own_goal
  assert_no_lookup_in_goals
  assert_scalar_clause_goals 1
  exact hImpossible

end
end VerexSL.Tests.WrittenContainerHolds

-- ============================================================================
-- 3. Source-level positive controls and axiom audit
-- ============================================================================

verify written_container_holds::vec_write_symbolic_ok
verify written_container_holds::array_chained_two_writes_ok
verify written_container_holds::vec_repaired_before_death_ok
verify written_container_holds::ordered_pair_write_symbolic_ok

-- The removed element's own fact, and its fields', at the one read form.
verify 0x42::written_vector_elements::remove_rows
verify 0x42::written_vector_elements::remove_plain
verify 0x42::written_vector_elements::remove_rows_whole
verify 0x42::written_vector_elements::read_rows
verify 0x42::written_vector_elements::insert_plain
verify 0x42::written_vector_elements::insert_limited_rows_closed
verify 0x42::written_vector_elements::push_struct_ok
verify 0x42::written_vector_elements::push_enum_ok

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
      `VerexSL.Tests.WrittenContainerHolds.neg_read_element_without_container_fact_reads_structurally,
      `VerexSL.Tests.WrittenContainerHolds.neg_read_element_of_other_container_reads_structurally,
      `VerexSL.Tests.WrittenContainerHolds.pos_read_element_of_vector_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_read_element_of_array_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_read_element_of_slice_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_read_element_of_sliceRef_current_closes,
      `VerexSL.Tests.WrittenContainerHolds.neg_eraseBounded_of_other_container_reads_elementwise,
      `VerexSL.Tests.WrittenContainerHolds.pos_eraseBounded_preserves_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_swapBounded_preserves_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_eraseBounded_of_swapBounded_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_eraseBounded_of_array_fact_closes,
      `VerexSL.Tests.WrittenContainerHolds.neg_two_insertions_same_index_keep_the_first_element_clause,
      `VerexSL.Tests.WrittenContainerHolds.pos_two_insertions_same_index_both_valid_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_insert_at_size_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_insert_into_empty_closes,
      `VerexSL.Tests.WrittenContainerHolds.neg_insertBounded_of_other_container_reads_elementwise,
      `VerexSL.Tests.WrittenContainerHolds.neg_push_invalid_element_leaves_its_clause,
      `VerexSL.Tests.WrittenContainerHolds.neg_push_after_invalid_insertion_keeps_the_inserted_clause,
      `VerexSL.Tests.WrittenContainerHolds.neg_push_of_other_container_reads_elementwise,
      `VerexSL.Tests.WrittenContainerHolds.pos_push_valid_element_closes,
      `VerexSL.Tests.WrittenContainerHolds.neg_param_element_without_container_fact_is_not_read,
      `VerexSL.Tests.WrittenContainerHolds.pos_param_element_in_hand_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_param_read_element_closes,
      `VerexSL.Tests.WrittenContainerHolds.neg_read_element_of_written_vector_leaves_inserted_clause,
      `VerexSL.Tests.WrittenContainerHolds.pos_read_element_of_written_vector_closes,
      `VerexSL.Tests.WrittenContainerHolds.pos_swapped_written_vector_holds_closes,
      `VerexSL.Tests.WrittenContainerHolds.neg_read_element_of_swapped_written_vector_leaves_inserted_clause,
      `written_container_holds.vec_write_symbolic_ok.typedVerified,
      `written_container_holds.array_chained_two_writes_ok.typedVerified,
      `written_container_holds.vec_repaired_before_death_ok.typedVerified,
      `written_container_holds.ordered_pair_write_symbolic_ok.typedVerified,
      `«0x42».written_vector_elements.remove_rows.typedVerified,
      `«0x42».written_vector_elements.remove_plain.typedVerified,
      `«0x42».written_vector_elements.remove_rows_whole.typedVerified,
      `«0x42».written_vector_elements.read_rows.typedVerified,
      `«0x42».written_vector_elements.insert_plain.typedVerified,
      `«0x42».written_vector_elements.insert_limited_rows_closed.typedVerified,
      `«0x42».written_vector_elements.push_struct_ok.typedVerified,
      `«0x42».written_vector_elements.push_enum_ok.typedVerified] do
    unless env.contains name do throwError "written-container control {name} did not verify"
    let axioms ← collectAxioms name
    unless axioms.all ([``propext, ``Classical.choice, ``Quot.sound].contains ·) do
      throwError "unexpected axioms in {name}: {axioms}"
