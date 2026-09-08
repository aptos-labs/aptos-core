-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Integers: every Move integer width, checked casts,
bit operations, shifts, constants, and primitive-pattern behavior. Source
range patterns are expressed as their ordered decision tree in LeanerLang. -/

leaner module 0x42::integers where
  const COMPLEX : u64 := 1 + 2 * 3
  const SIGNED_COMPLEX : i64 := -5i64 + 2
  const SHIFTED_COMPLEX : u64 := 1u64 << 4u8

  fun typed_u8() -> u8 := 112u8
  fun grouped_u64() -> u64 := 1_234_567u64

  fun small_sum(left : u8, right : u8) -> u8 := left + right
  spec small_sum where
    ensures true
    aborts_if left + right > 255

  fun wide_product(left : u128, right : u128) -> u128 := left * right
  spec wide_product where
    ensures true
    aborts_if left * right > 340282366920938463463374607431768211455

  fun narrow(value : u64) -> u8 := value as u8
  spec narrow where
    ensures result == value
    aborts_if value > 255

  fun widen(value : u8) -> u256 := value as u256
  spec widen where
    ensures result == value
    aborts_if false

  fun masked(value : u64, mask : u64) -> u64 := value & mask
  spec masked where
    ensures result == value & mask
    aborts_if false

  fun combined(left : u32, right : u32) -> u32 := (left | right) ^ right

  fun shifted(value : u64, amount : u8) -> u64 := value << amount
  spec shifted where
    ensures result == (value << amount) % 18446744073709551616
    aborts_if amount >= 64

  fun halved(value : u16) -> u16 := value >> 1u8
  spec halved where
    ensures result == value >> 1u8
    aborts_if false

  fun complex_constant() -> u64 := COMPLEX
  spec complex_constant where
    ensures result == 7
    aborts_if false

  fun signed_complex_constant() -> i64 := SIGNED_COMPLEX
  fun shifted_complex_constant() -> u64 := SHIFTED_COMPLEX

  fun classify_primitive(value : u64) -> u64 :=
    if value == 0 then 10
    else if 1 <= value && value < 4 && value != 2 then 20
    else if 4 <= value && value <= 6 then 30
    else 40
  spec classify_primitive where
    ensures result == if value == 0 then 10
      else if 1 <= value && value < 4 && value != 2 then 20
      else if 4 <= value && value <= 6 then 30
      else 40
    aborts_if false

  fun primitive_match_effect(selector : u64, value : u64) -> u64 :=
    if selector == 0 then value + 1 else 0
  spec primitive_match_effect where
    ensures result == if selector == 0 then value + 1 else 0
    aborts_if selector == 0 && value == 18446744073709551615

  fun classify_open_range(value : u64) -> u64 :=
    if value < 3 then 1 else 2
  spec classify_open_range where
    ensures result == if value < 3 then 1 else 2
    aborts_if false

  fun primitive_match_reference(value : u64) -> u64 := do
    let reference := &value
    if *reference == 0 then 1
    else if 1 <= *reference && *reference <= 9 then 2
    else 3
  spec primitive_match_reference where
    ensures result == if value == 0 then 1
      else if 1 <= value && value <= 9 then 2
      else 3
    aborts_if false
  verify primitive_match_reference

set_option leaner.route "native" in
#leaner_verify 0x42::integers::shifted
#leaner_require_native 0x42::integers::shifted

set_option leaner.route "native" in
#leaner_verify 0x42::integers::halved
#leaner_require_native 0x42::integers::halved

set_option leaner.route "native" in
#leaner_verify 0x42::integers::masked
#leaner_require_native 0x42::integers::masked

set_option leaner.route "native" in
#leaner_verify 0x42::integers::small_sum
#leaner_require_native 0x42::integers::small_sum

set_option leaner.route "native" in
#leaner_verify 0x42::integers::wide_product
#leaner_require_native 0x42::integers::wide_product

set_option leaner.route "native" in
#leaner_verify 0x42::integers::narrow
#leaner_require_native 0x42::integers::narrow

set_option leaner.route "native" in
#leaner_verify 0x42::integers::widen
#leaner_require_native 0x42::integers::widen

set_option leaner.route "native" in
#leaner_verify 0x42::integers::complex_constant
#leaner_require_native 0x42::integers::complex_constant

set_option leaner.route "native" in
#leaner_verify 0x42::integers::classify_primitive
#leaner_require_native 0x42::integers::classify_primitive

set_option leaner.route "native" in
#leaner_verify 0x42::integers::primitive_match_effect
#leaner_require_native 0x42::integers::primitive_match_effect

set_option leaner.route "native" in
#leaner_verify 0x42::integers::classify_open_range
#leaner_require_native 0x42::integers::classify_open_range

#leaner_require_native 0x42::integers::primitive_match_reference
#leaner_require_native_all

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».integers #[
    ⟨"typed_u8", #[], .returned #[.integer 112], {}⟩,
    ⟨"grouped_u64", #[], .returned #[.integer 1_234_567], {}⟩,
    ⟨"small_sum", #[.integer 100, .integer 55], .returned #[.integer 155], {}⟩,
    ⟨"small_sum", #[.integer 200, .integer 56], .threw .abort #[.integer 256], {}⟩,
    ⟨"wide_product", #[.integer (2 ^ 100), .integer 4],
      .returned #[.integer (2 ^ 102)], {}⟩,
    ⟨"narrow", #[.integer 255], .returned #[.integer 255], {}⟩,
    ⟨"narrow", #[.integer 256], .threw .abort #[.integer 256], {}⟩,
    ⟨"widen", #[.integer 255], .returned #[.integer 255], {}⟩,
    ⟨"masked", #[.integer 0b1100, .integer 0b1010], .returned #[.integer 0b1000], {}⟩,
    ⟨"combined", #[.integer 0b1100, .integer 0b1010], .returned #[.integer 0b0100], {}⟩,
    ⟨"shifted", #[.integer 1, .integer 63], .returned #[.integer (2 ^ 63)], {}⟩,
    ⟨"shifted", #[.integer (2 ^ 63), .integer 1], .returned #[.integer 0], {}⟩,
    ⟨"shifted", #[.integer 1, .integer 64], .threw .abort #[.integer 64], {}⟩,
    ⟨"halved", #[.integer 9], .returned #[.integer 4], {}⟩,
    ⟨"complex_constant", #[], .returned #[.integer 7], {}⟩,
    ⟨"signed_complex_constant", #[], .returned #[.integer (-3)], {}⟩,
    ⟨"shifted_complex_constant", #[], .returned #[.integer 16], {}⟩,
    ⟨"classify_primitive", #[.integer 0], .returned #[.integer 10], {}⟩,
    ⟨"classify_primitive", #[.integer 1], .returned #[.integer 20], {}⟩,
    ⟨"classify_primitive", #[.integer 2], .returned #[.integer 40], {}⟩,
    ⟨"classify_primitive", #[.integer 4], .returned #[.integer 30], {}⟩,
    ⟨"classify_primitive", #[.integer 6], .returned #[.integer 30], {}⟩,
    ⟨"classify_primitive", #[.integer 7], .returned #[.integer 40], {}⟩,
    ⟨"primitive_match_effect", #[.integer 1, .integer 18446744073709551615],
      .returned #[.integer 0], {}⟩,
    ⟨"primitive_match_effect", #[.integer 0, .integer 5], .returned #[.integer 6], {}⟩,
    ⟨"primitive_match_effect", #[.integer 0, .integer 18446744073709551615],
      .threw .abort #[.integer 18446744073709551616], {}⟩,
    ⟨"classify_open_range", #[.integer 2], .returned #[.integer 1], {}⟩,
    ⟨"classify_open_range", #[.integer 3], .returned #[.integer 2], {}⟩,
    ⟨"primitive_match_reference", #[.integer 0], .returned #[.integer 1], {}⟩,
    ⟨"primitive_match_reference", #[.integer 5], .returned #[.integer 2], {}⟩,
    ⟨"primitive_match_reference", #[.integer 10], .returned #[.integer 3], {}⟩]
