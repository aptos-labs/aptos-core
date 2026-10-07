-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Literal bounds from opaque callees still expose finite positions, including
large literals, strict bounds, generic instantiation and mutable results. -/

set_option maxHeartbeats 25000
set_option leaner.verifyHeartbeats 25000

leaner module 0x42::call_range_results where
  fun choose(flag : Bool) -> u64 := if flag then 0 else 1
  spec choose where
    pragma opaque
    aborts_if false
    ensures 0 <= result
    ensures result <= 1

  fun choose_large() -> u64 := 1000000000000
  spec choose_large where
    pragma opaque
    aborts_if false
    ensures 999999999999 < result
    ensures result < 1000000000001

  fun write_choice(slot : &mut u64, flag : Bool) -> Unit :=
    *slot := if flag then 0 else 1
  spec write_choice where
    pragma opaque
    aborts_if false
    ensures 0 <= *slot
    ensures *slot <= 1

  fun read_choice(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_choice where
    aborts_if false
    ensures result == 7

  fun read_large() -> u64 := do
    let index := choose_large()
    let values := vector<u64>[7]
    values[index - 1000000000000]
  spec read_large where
    aborts_if false
    ensures result == 7

  fun read_written(flag : Bool) -> u64 := do
    let mut index := 0u64
    write_choice(&mut index, flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_written where
    aborts_if false
    ensures result == 7

  fun choose_generic {T}(flag : Bool) -> u64 := if flag then 0 else 1
  spec choose_generic where
    pragma opaque
    aborts_if false
    ensures 0 <= result
    ensures result <= 1

  fun read_generic(flag : Bool) -> u64 := do
    let index := choose_generic::<Bool>(flag)
    let values := vector<u64>[7, 7]
    values[index]
  spec read_generic where
    aborts_if false
    ensures result == 7

-- Computed positions retain the finite range through arithmetic temporaries.
leaner module 0x42::computed_call_range_results where
  fun choose(flag : Bool) -> u64 := if flag then 10 else 11
  spec choose where
    pragma opaque
    aborts_if false
    ensures 10 <= result
    ensures result <= 11

  fun choose_large() -> u64 := 1000000000000
  spec choose_large where
    pragma opaque
    aborts_if false
    ensures 999999999999 < result
    ensures result < 1000000000001

  fun write_choice(slot : &mut u64, flag : Bool) -> Unit :=
    *slot := if flag then 10 else 11
  spec write_choice where
    pragma opaque
    aborts_if false
    ensures 10 <= *slot
    ensures *slot <= 11

  fun read_choice(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_choice where
    aborts_if false
    ensures result == 7

  fun read_large() -> u64 := do
    let index := choose_large()
    let values := vector<u64>[7]
    values[index - 1000000000000]
  spec read_large where
    aborts_if false
    ensures result == 7

  fun read_written(flag : Bool) -> u64 := do
    let mut index := 0u64
    write_choice(&mut index, flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_written where
    aborts_if false
    ensures result == 7

  fun choose_generic {T}(flag : Bool) -> u64 := if flag then 10 else 11
  spec choose_generic where
    pragma opaque
    aborts_if false
    ensures 10 <= result
    ensures result <= 11

  fun read_generic(flag : Bool) -> u64 := do
    let index := choose_generic::<Bool>(flag)
    let values := vector<u64>[7, 7]
    values[index - 10]
  spec read_generic where
    aborts_if false
    ensures result == 7

  fun wrong_value(flag : Bool) -> u64 := read_choice(flag)
  spec wrong_value where
    pragma verify = false
    aborts_if false
    ensures result == 8

  fun wrong_index(flag : Bool) -> u64 := do
    let index := choose(flag)
    let values := vector<u64>[7, 7]
    values[index - 9]
  spec wrong_index where
    pragma verify = false
    aborts_if false

/--
error: the specification clause `ensures result == 8` is not established
---
error: leaner verification failed: the automatic verification of `wrong_value` failed; provide a proof: `verify wrong_value by …` in the module (`verify wrong_value by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::computed_call_range_results::wrong_value

/--
error: the specification clause `aborts_if false` is not established
---
error: leaner verification failed: the automatic verification of `wrong_index` failed; provide a proof: `verify wrong_index by …` in the module (`verify wrong_index by skip` shows the obligations it leaves)
-/
#guard_msgs in
verify 0x42::computed_call_range_results::wrong_index
