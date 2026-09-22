-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Arithmetic: checked unsigned arithmetic, comparison
spellings, Boolean results, and arithmetic through resource references. -/

leaner module 0x42::arithmetic where
  fun add_values(left : u64, right : u64) -> u64 := left + right
  spec add_values where
    ensures result == left + right
    aborts_if left + right > 18446744073709551615

  fun subtract_values(left : u64, right : u64) -> u64 := left - right
  spec subtract_values where
    ensures result == left - right
    aborts_if left < right

  fun multiply_values(left : u64, right : u64) -> u64 := left * right
  spec multiply_values where
    ensures result == left * right
    aborts_if left * right > 18446744073709551615

  fun divide_values(left : u64, right : u64) -> u64 := left / right
  spec divide_values where
    ensures result == left / right
    aborts_if right == 0

  fun modulo_values(left : u64, right : u64) -> u64 := left % right
  spec modulo_values where
    ensures result == left % right
    aborts_if right == 0

  fun at_most(left : u64, right : u64) -> u64 :=
    if left <= right then 1 else 0
  spec at_most where
    ensures result == if left <= right then 1 else 0
    aborts_if false

  fun exceeds(left : u64, right : u64) -> u64 :=
    if left > right then 1 else 0
  spec exceeds where
    ensures result == if left > right then 1 else 0
    aborts_if false

  fun at_least(left : u64, right : u64) -> u64 :=
    if left >= right then 1 else 0
  spec at_least where
    ensures result == if left >= right then 1 else 0
    aborts_if false

  fun differs(left : u64, right : u64) -> u64 :=
    if left != right then 1 else 0
  spec differs where
    ensures left == right ==> result == 0
    ensures !(left == right) ==> result == 1
    aborts_if false

  fun is_less(left : u64, right : u64) -> Bool := left < right
  spec is_less where
    ensures result == (left < right)

  struct Counter has Key where
    value : u64

  public entry fun multiply(addr : Address, factor : u64) -> Unit := do
    let value := &mut Counter[addr].value
    let old_value := *value
    *value := old_value * factor
  spec multiply where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) * factor
    aborts_if old(global<Counter>(addr).value) * factor > 18446744073709551615
  verify multiply

  public entry fun divide(addr : Address, divisor : u64) -> Unit := do
    let value := &mut Counter[addr].value
    let old_value := *value
    *value := old_value / divisor
  spec divide where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures global<Counter>(addr).value == old(global<Counter>(addr).value) / divisor
    aborts_if divisor == 0
  verify divide

set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::add_values
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::subtract_values
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::multiply_values

#leaner_require_native 0x42::arithmetic::add_values
#leaner_require_native 0x42::arithmetic::subtract_values
#leaner_require_native 0x42::arithmetic::multiply_values

set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::divide_values
#leaner_require_native 0x42::arithmetic::divide_values
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::modulo_values
#leaner_require_native 0x42::arithmetic::modulo_values

-- These proofs must use typed native branches, never a row fallback.
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::at_most
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::exceeds
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::at_least
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::differs
set_option leaner.route "native" in
#leaner_verify 0x42::arithmetic::is_less

#leaner_require_native 0x42::arithmetic::at_most
#leaner_require_native 0x42::arithmetic::exceeds
#leaner_require_native 0x42::arithmetic::at_least
#leaner_require_native 0x42::arithmetic::differs
#leaner_require_native 0x42::arithmetic::is_less

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».arithmetic #[
    ⟨"add_values", #[.integer 6, .integer 7], .returned #[.integer 13], {}⟩,
    ⟨"add_values", #[.integer 0, .integer 0], .returned #[.integer 0], {}⟩,
    ⟨"add_values", #[.integer 18446744073709551615, .integer 0],
      .returned #[.integer 18446744073709551615], {}⟩,
    ⟨"add_values", #[.integer 18446744073709551615, .integer 1],
      .threw .abort #[.integer 18446744073709551616], {}⟩,
    ⟨"subtract_values", #[.integer 17, .integer 5], .returned #[.integer 12], {}⟩,
    ⟨"subtract_values", #[.integer 0, .integer 0], .returned #[.integer 0], {}⟩,
    ⟨"subtract_values", #[.integer 5, .integer 17], .threw .abort #[.integer (-12)], {}⟩,
    ⟨"multiply_values", #[.integer 6, .integer 7], .returned #[.integer 42], {}⟩,
    ⟨"multiply_values", #[.integer 18446744073709551615, .integer 1],
      .returned #[.integer 18446744073709551615], {}⟩,
    ⟨"multiply_values", #[.integer 0, .integer 18446744073709551615],
      .returned #[.integer 0], {}⟩,
    ⟨"multiply_values", #[.integer 18446744073709551615, .integer 2],
      .threw .abort #[.integer 36893488147419103230], {}⟩,
    ⟨"divide_values", #[.integer 17, .integer 5], .returned #[.integer 3], {}⟩,
    ⟨"divide_values", #[.integer 0, .integer 7], .returned #[.integer 0], {}⟩,
    ⟨"divide_values", #[.integer 18446744073709551615, .integer 1],
      .returned #[.integer 18446744073709551615], {}⟩,
    ⟨"divide_values", #[.integer 17, .integer 0], .threw .abort #[], {}⟩,
    ⟨"modulo_values", #[.integer 17, .integer 5], .returned #[.integer 2], {}⟩,
    ⟨"modulo_values", #[.integer 17, .integer 20], .returned #[.integer 17], {}⟩,
    ⟨"modulo_values", #[.integer 18446744073709551615, .integer 2],
      .returned #[.integer 1], {}⟩,
    ⟨"modulo_values", #[.integer 17, .integer 0], .threw .abort #[], {}⟩,
    ⟨"at_most", #[.integer 2, .integer 2], .returned #[.integer 1], {}⟩,
    ⟨"at_most", #[.integer 3, .integer 2], .returned #[.integer 0], {}⟩,
    ⟨"exceeds", #[.integer 3, .integer 2], .returned #[.integer 1], {}⟩,
    ⟨"exceeds", #[.integer 2, .integer 2], .returned #[.integer 0], {}⟩,
    ⟨"at_least", #[.integer 2, .integer 2], .returned #[.integer 1], {}⟩,
    ⟨"at_least", #[.integer 1, .integer 2], .returned #[.integer 0], {}⟩,
    ⟨"differs", #[.integer 1, .integer 2], .returned #[.integer 1], {}⟩,
    ⟨"differs", #[.integer 2, .integer 2], .returned #[.integer 0], {}⟩,
    ⟨"is_less", #[.integer 1, .integer 2], .returned #[.bool true], {}⟩,
    ⟨"is_less", #[.integer 2, .integer 2], .returned #[.bool false], {}⟩,
    ⟨"divide", #[.address "0x2", .integer 1], .threw .abort #[], {}⟩]

  let initial6 ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 6]
  let final42 ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 42] 2
  let initialMax ← singleResourceState `«0x42».arithmetic "Counter" "0x2"
    #[.integer 18446744073709551615]
  let unchangedMax ← singleResourceState `«0x42».arithmetic "Counter" "0x2"
    #[.integer 18446744073709551615]
  let settledMax ← singleResourceState `«0x42».arithmetic "Counter" "0x2"
    #[.integer 18446744073709551615] 2
  let finalZero ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 0] 2
  let initial17 ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 17]
  let final3 ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 3] 2
  let initial0 ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 0]
  let settled0 ← singleResourceState `«0x42».arithmetic "Counter" "0x2" #[.integer 0] 2
  assertRunsState `«0x42».arithmetic #[
    ⟨"multiply", #[.address "0x2", .integer 7], .returned #[], initial6, final42⟩,
    ⟨"multiply", #[.address "0x2", .integer 1], .returned #[], initialMax, settledMax⟩,
    ⟨"multiply", #[.address "0x2", .integer 0], .returned #[], initialMax, finalZero⟩,
    ⟨"multiply", #[.address "0x2", .integer 2],
      .threw .abort #[.integer 36893488147419103230], initialMax, unchangedMax⟩,
    ⟨"divide", #[.address "0x2", .integer 5], .returned #[], initial17, final3⟩,
    ⟨"divide", #[.address "0x2", .integer 7], .returned #[], initial0, settled0⟩,
    ⟨"divide", #[.address "0x2", .integer 1], .returned #[], initialMax, settledMax⟩,
    ⟨"divide", #[.address "0x2", .integer 0], .threw .abort #[], initial17, initial17⟩]
