-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-! Port of v0 Language/Tuples: flattened Move multiple returns, tuple
destructuring, transient local tuples, and nominal destructuring spellings. -/

leaner module 0x42::tuples where
  struct Pair has Copy, Drop, Store where
    first : u64
    second : Bool

  fun pure_pair(value : u64) -> (u64, Bool) := (value, true)
  spec pure_pair where
    ensures spec.result[0] == value && spec.result[1] == true
    aborts_if false
  verify pure_pair

  fun effect_pair(value : u64) -> (u64, Bool) := do
    return (value, true)
  spec effect_pair where
    ensures spec.result[0] == value && spec.result[1] == true
    aborts_if false
  verify effect_pair

  fun destructure_pure(value : u64) -> u64 := do
    let (first, _) := pure_pair(value)
    return first
  spec destructure_pure where
    ensures result == value
    aborts_if false

  fun destructure_effect(value : u64) -> u64 := do
    let (first, _) := effect_pair(value)
    return first
  spec destructure_effect where
    ensures result == value
    aborts_if false

  fun triple(value : u64) -> (u64, Bool, u8) := (value, true, 3)
  spec triple where
    ensures spec.result[0] == value && spec.result[1] == true && spec.result[2] == 3
    aborts_if false
  verify triple

  fun destructure_triple(value : u64) -> u8 := do
    let (_, _, third) := triple(value)
    return third
  spec destructure_triple where
    ensures result == 3
    aborts_if false

  fun destructure_local(value : u64) -> u64 := do
    let pair := (value, false)
    let (first, _) := pair
    return first
  spec destructure_local where
    ensures result == value
    aborts_if false
  verify destructure_local

  fun destructure_struct(value : u64) -> u64 := do
    let pair := new Pair { first := value, second := true }
    let Pair { first := first, second := _ } := pair
    return first
  spec destructure_struct where
    ensures result == value
    aborts_if false
  verify destructure_struct

  fun destructure_struct_partial(value : u64) -> u64 := do
    let pair := new Pair { first := value, second := true }
    let Pair { first := first, second := _ } := pair
    return first
  spec destructure_struct_partial where
    ensures result == value
    aborts_if false
  verify destructure_struct_partial

  fun destructure_struct_move_spelling(value : u64) -> u64 := do
    let pair := new Pair { first := value, second := true }
    let Pair { first := first, second := second } := pair
    return if second then first else 0
  spec destructure_struct_move_spelling where
    ensures result == value
    aborts_if false
  verify destructure_struct_move_spelling

set_option leaner.route "native" in
#leaner_verify 0x42::tuples::destructure_pure
set_option leaner.route "native" in
#leaner_verify 0x42::tuples::destructure_effect
set_option leaner.route "native" in
#leaner_verify 0x42::tuples::destructure_triple

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  assertRuns `«0x42».tuples #[
    ⟨"pure_pair", #[.integer 7], .returned #[.tuple #[.integer 7, .bool true]], {}⟩,
    ⟨"effect_pair", #[.integer 7], .returned #[.tuple #[.integer 7, .bool true]], {}⟩,
    ⟨"destructure_pure", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"destructure_effect", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"triple", #[.integer 7],
      .returned #[.tuple #[.integer 7, .bool true, .integer 3]], {}⟩,
    ⟨"destructure_triple", #[.integer 7], .returned #[.integer 3], {}⟩,
    ⟨"destructure_local", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"destructure_struct", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"destructure_struct_partial", #[.integer 7], .returned #[.integer 7], {}⟩,
    ⟨"destructure_struct_move_spelling", #[.integer 7], .returned #[.integer 7], {}⟩]
