-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL
import VerexSL.Tests.LoanDeathSupport

set_option Elab.async false

/-! ## A program's reads of a vector's elements

An index read and a removal read their element through `withVectorElem`: the
walk splits the read while it still names its vector and states the element's
fact from the vector's there. The controls run the read at a symbolic and at
a literal vector, on its failure path, after a join, inside a loop, out of an
operation's result and out of a vector a struct holds, and check that no read
is left in a prepared leaf. -/

open Lean Meta Elab Tactic VerexIR VerexIR.Proofs VerexIR.Proofs.Denote VerexSL

verex module 0x42::executable_vector_reads where
  pragma verify = false

  struct Percent has Copy, Drop, Store where
    value : u64
  spec Percent where
    invariant value <= 100

  struct Entry {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    key : K
    value : V

  struct Rows {K has Copy, Drop, Store} {V has Copy, Drop, Store} has Copy, Drop, Store where
    items : Vector<Entry<K, V> >

  -- The element's invariant is not more than it states.
  fun read_bad(v : Vector<Percent>, i : u64) -> u64 :=
    v[i].value
  spec read_bad where
    requires i < v.length
    ensures result <= 99

  -- A position the vector does not hold aborts.
  fun read_out_of_range_bad(v : Vector<Percent>, i : u64) -> u64 :=
    v[i].value
  spec read_out_of_range_bad where
    aborts_if false

  -- The removal out of a swap's result drops one element.
  fun take_rows_length_bad {K has Copy, Drop, Store} {V has Copy, Drop, Store}
      (box : &mut Rows<K, V>, index : u64) -> V := do
    let items := &mut box.items
    let last := (*items).length - 1
    *items := core.prim.swapVector(*items, index, last)
    let (removed, rest) := core.prim.removeVector(*items, last)
    *items := rest
    removed.value
  spec take_rows_length_bad where
    requires index < box.items.length
    ensures box.items.length == old(box).items.length

  fun read_ok(v : Vector<Percent>, i : u64) -> u64 :=
    v[i].value
  spec read_ok where
    requires i < v.length
    aborts_if false
    ensures result <= 100

  fun read_out_of_range_ok(v : Vector<Percent>, i : u64) -> u64 :=
    v[i].value
  spec read_out_of_range_ok where
    aborts_if i >= v.length
    ensures result <= 100

  fun literal_read() -> u64 := do
    let v := vector<u64>[10, 20, 30]
    v[1]
  spec literal_read where
    aborts_if false
    ensures result == 20

  fun literal_out_of_range() -> u64 := do
    let v := vector<u64>[10]
    v[1]
  spec literal_out_of_range where
    aborts_if true
    ensures false

  -- A read after a join of two paths.
  fun read_after_join(v : Vector<Percent>, i : u64, first : Bool) -> u64 := do
    let position := if first then 0 else i
    v[position].value
  spec read_after_join where
    requires i < v.length
    aborts_if false
    ensures result <= 100

  -- A read in a loop's body.
  fun largest(v : Vector<Percent>) -> u64 := do
    let i := 0u64
    let best := 0u64
    while i < v.length do
      let here := v[i].value
      if best < here then best := here else ()
      i := i + 1
      continue
    best
  spec largest where
    aborts_if false
    ensures result <= 100
    loop 1 where
      invariant i <= v.length
      invariant best <= 100

  -- The element read out of a swap's result, of a vector a struct holds
  -- behind a mutable borrow and behind a shared one.
  fun take_rows {K has Copy, Drop, Store} {V has Copy, Drop, Store}(box : &mut Rows<K, V>,
      index : u64) -> V := do
    let items := &mut box.items
    let last := (*items).length - 1
    *items := core.prim.swapVector(*items, index, last)
    let (removed, rest) := core.prim.removeVector(*items, last)
    *items := rest
    removed.value
  spec take_rows where
    requires index < box.items.length
    ensures box.items.length + 1 == old(box).items.length

  fun swapped_read {K has Copy, Drop, Store} {V has Copy, Drop, Store}(box : &Rows<K, V>,
      index : u64) -> V := do
    let swapped := core.prim.swapVector(box.items, index, 0)
    swapped[index].value
  spec swapped_read where
    requires index < box.items.length

open LoanDeathControls in
run_cmd do
  expectUnestablished `«0x42».executable_vector_reads.read_bad ["ensures result <= 99"]
    (← `(command| verify 0x42::executable_vector_reads::read_bad))
  expectUnestablished `«0x42».executable_vector_reads.read_out_of_range_bad ["aborts_if false"]
    (← `(command| verify 0x42::executable_vector_reads::read_out_of_range_bad))
  expectUnestablished `«0x42».executable_vector_reads.take_rows_length_bad
    ["ensures box.items.length == old(box).items.length"]
    (← `(command| verify 0x42::executable_vector_reads::take_rows_length_bad))

verify 0x42::executable_vector_reads::read_ok
verify 0x42::executable_vector_reads::read_out_of_range_ok
verify 0x42::executable_vector_reads::literal_read
verify 0x42::executable_vector_reads::literal_out_of_range
verify 0x42::executable_vector_reads::read_after_join
verify 0x42::executable_vector_reads::largest
verify 0x42::executable_vector_reads::take_rows
verify 0x42::executable_vector_reads::swapped_read

/-- No read of a vector's element remains in a prepared leaf. -/
elab "assert_no_read_residue" : tactic => do
  for goal in ← getGoals do
    goal.withContext do
      let residue (e : Lean.Expr) := (e.find? (·.isConstOf ``withVectorElem)).isSome
      if residue (← instantiateMVars (← goal.getType)) then
        throwError "a prepared leaf still runs a read of a vector's element"
      for decl in ← getLCtx do
        if decl.isImplementationDetail then continue
        if residue (← instantiateMVars decl.type) then
          throwError "{decl.userName} still states a read of a vector's element"

verify 0x42::executable_vector_reads::read_out_of_range_ok by lean =>
  verex_obligations
  all_goals (verex_prepare <;> assert_no_read_residue)
  all_goals verex_decide
