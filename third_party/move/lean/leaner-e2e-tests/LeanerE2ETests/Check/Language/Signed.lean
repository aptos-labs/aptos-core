-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Signed. The remainder abort clause intentionally
matches the production VM: MIN_INT % -1 overflows, just like division.
The deprecated reference is retained unchanged. -/

leaner module 0x42::signed where
  fun add_values(left : i64, right : i64) -> i64 := left + right
  spec add_values where
    ensures result == left + right
    aborts_if left + right < -9223372036854775808 || left + right > 9223372036854775807

  fun sub_values(left : i32, right : i32) -> i32 := left - right
  spec sub_values where
    ensures result == left - right
    aborts_if left - right < -2147483648 || left - right > 2147483647

  fun mul_values(left : i16, right : i16) -> i16 := left * right
  spec mul_values where
    ensures result == left * right
    aborts_if left * right < -32768 || left * right > 32767

  fun div_values(left : i32, right : i32) -> i32 := left / right
  spec div_values where
    ensures true
    aborts_if right == 0 || left / right < -2147483648 || left / right > 2147483647

  fun mod_values(left : i32, right : i32) -> i32 := left % right
  spec mod_values where
    ensures result == left % right
    aborts_if right == 0 || (left == -2147483648 && right == -1)

  -- These four functions were execution-only in v0.
  fun positive_literal() -> i8 := 100
  fun negative_literal() -> i32 := -5
  fun negate_value(value : i64) -> i64 := -value
  fun below(left : i32, right : i32) -> Bool := left < right

  struct Balance has Key where
    amount : i64

  public entry fun credit(addr : Address, delta : i64) -> Unit := do
    let value := &mut Balance[addr].amount
    let current := *value
    *value := current + delta
  spec credit where
    requires exists<Balance>(addr)
    modifies global<Balance>(addr)
    ensures global<Balance>(addr).amount == old(global<Balance>(addr).amount) + delta
    aborts_if old(global<Balance>(addr).amount) + delta < -9223372036854775808 ||
      old(global<Balance>(addr).amount) + delta > 9223372036854775807
  verify credit

-- Pure checked arithmetic uses typed native computations, with no row/frame
-- in its VC. Storage effects remain to migrate.
set_option leaner.route "native" in
#leaner_verify 0x42::signed::add_values
set_option leaner.route "native" in
#leaner_verify 0x42::signed::sub_values
set_option leaner.route "native" in
#leaner_verify 0x42::signed::mul_values

set_option leaner.route "native" in
#leaner_verify 0x42::signed::div_values
set_option leaner.route "native" in
#leaner_verify 0x42::signed::mod_values

#leaner_require_native 0x42::signed::add_values
#leaner_require_native 0x42::signed::sub_values
#leaner_require_native 0x42::signed::mul_values
#leaner_require_native 0x42::signed::div_values
#leaner_require_native 0x42::signed::mod_values

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».signed #[
    ⟨"mod_values", #[.integer (-2147483648), .integer (-1)],
      .threw .abort #[.integer 2147483648], {}⟩,
    ⟨"div_values", #[.integer (-2147483648), .integer (-1)],
      .threw .abort #[.integer 2147483648], {}⟩,
    ⟨"mod_values", #[.integer (-2147483648), .integer 1], .returned #[.integer 0], {}⟩,
    ⟨"mod_values", #[.integer (-7), .integer 3], .returned #[.integer (-1)], {}⟩,
    ⟨"mod_values", #[.integer 7, .integer (-3)], .returned #[.integer 1], {}⟩,
    ⟨"div_values", #[.integer (-7), .integer 3], .returned #[.integer (-2)], {}⟩,
    ⟨"mod_values", #[.integer 7, .integer 0], .threw .abort #[], {}⟩,
    ⟨"div_values", #[.integer 7, .integer 0], .threw .abort #[], {}⟩,
    ⟨"positive_literal", #[], .returned #[.integer 100], {}⟩,
    ⟨"negative_literal", #[], .returned #[.integer (-5)], {}⟩,
    ⟨"negate_value", #[.integer (-5)], .returned #[.integer 5], {}⟩,
    ⟨"below", #[.integer (-7), .integer 3], .returned #[.bool true], {}⟩]
