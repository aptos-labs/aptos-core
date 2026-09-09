-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Vectors: six original contracts and executions.
Insertion returns the updated vector; removal returns both the removed
element and updated vector before writing back through the reference. -/

set_option leaner.route "native"

leaner module 0x42::vectors where
  fun make() -> Vector<u64> := vector<u64>[10, 20, 30]
  spec make where
    ensures result == vector<u64>[10, 20, 30]
    aborts_if false

  fun length() -> u64 := make().length
  spec length where
    ensures result == 3

  fun middle() -> u64 := do
    let values := make()
    let value := &values[1]
    return *value
  spec middle where
    ensures result == 20
    aborts_if false

  fun replace() -> u64 := do
    let mut values := make()
    let value := &mut values[1]
    *value := 42
    return *value
  spec replace where
    ensures result == 42
    aborts_if false

  fun insert_middle() -> u64 := do
    let mut values : Vector<u64> := vector<u64>[10, 30]
    let values_ref := &mut values
    *values_ref := core.prim.insertVector(*values_ref, 1, 20)
    let updated := *values_ref
    let middle := &updated[1]
    return *middle
  spec insert_middle where
    ensures result == 20
    aborts_if false

  fun remove_middle() -> u64 := do
    let mut values : Vector<u64> := vector<u64>[10, 20, 30]
    let values_ref := &mut values
    let (removed, updated) := core.prim.removeVector(*values_ref, 1)
    *values_ref := updated
    let after := *values_ref
    let shifted := &after[1]
    return removed + *shifted
  spec remove_middle where
    ensures result == 50
    aborts_if false

-- Publish native callees before checking their consumers. Shared indexing
-- is native; the three mutation consumers still fail closed.
set_option leaner.route "native" in
#leaner_verify 0x42::vectors::make
#leaner_require_native 0x42::vectors::make

set_option leaner.route "native" in
#leaner_verify 0x42::vectors::length
#leaner_require_native 0x42::vectors::length

#leaner_verify 0x42::vectors::middle
#leaner_require_native 0x42::vectors::middle
#leaner_verify 0x42::vectors::replace
#leaner_verify 0x42::vectors::insert_middle
#leaner_verify 0x42::vectors::remove_middle

open Lean Elab Command in
run_cmd do
  for function in ["make", "length", "middle", "replace", "insert_middle", "remove_middle"] do
    let name := ((`«0x42».vectors).str function).str "verified"
    unless (← getEnv).contains name do throwError "missing vector verification: {name}"
    if (← collectAxioms name).contains ``sorryAx then
      throwError "generated vector proof contains an admission: {name}"

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».vectors #[
    ⟨"make", #[], .returned #[.vector #[.integer 10, .integer 20, .integer 30]], {}⟩,
    ⟨"length", #[], .returned #[.integer 3], {}⟩,
    ⟨"middle", #[], .returned #[.integer 20], {}⟩,
    ⟨"replace", #[], .returned #[.integer 42], {}⟩,
    ⟨"insert_middle", #[], .returned #[.integer 20], {}⟩,
    ⟨"remove_middle", #[], .returned #[.integer 50], {}⟩]
