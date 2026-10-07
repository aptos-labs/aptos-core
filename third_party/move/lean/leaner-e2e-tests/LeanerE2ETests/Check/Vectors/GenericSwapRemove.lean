-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

-- The result crosses generic caller/callee carriers after both operations.
leaner module 0x42::generic_swap_remove where
  fun take {T has Copy, Drop, Store}(values : &mut Vector<T>, index : u64) -> T := do
    let last := (*values).length - 1
    *values := core.prim.swapVector(*values, index, last)
    let (removed, remaining) := core.prim.removeVector(*values, last)
    *values := remaining
    removed
  spec take where
    requires index < values.length
    ensures values.length + 1 == old(values).length
    aborts_if false

  fun take_u64() -> u64 := do
    let mut values := vector<u64>[10, 20, 30]
    core.call take::<u64>(&mut values, 1)
  spec take_u64 where
    ensures result == 20
    aborts_if false

  fun take_bool() -> Bool := do
    let mut values := vector<Bool>[true, false]
    core.call take::<Bool>(&mut values, 0)
  spec take_bool where
    ensures result
    aborts_if false

  fun empty_swap() -> Unit := do
    let values := core.prim.swapVector(vector<u64>[], 0, 0)
    ()
  spec empty_swap where
    ensures false
    aborts_if true

  fun remove_at_end() -> u64 := do
    let (removed, remaining) := core.prim.removeVector(vector<u64>[10], 1)
    removed
  spec remove_at_end where
    ensures false
    aborts_if true

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».generic_swap_remove #[
    ⟨"take_u64", #[], .returned #[.integer 20], {}⟩,
    ⟨"take_bool", #[], .returned #[.bool true], {}⟩,
    ⟨"empty_swap", #[], .threw .abort #[.integer 0, .integer 0], {}⟩,
    ⟨"remove_at_end", #[], .threw .abort #[.integer 1], {}⟩]
