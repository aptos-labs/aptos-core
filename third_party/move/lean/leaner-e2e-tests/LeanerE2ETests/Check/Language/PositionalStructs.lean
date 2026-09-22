-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

-- Port of v0 Language/PositionalStructs. Positional fields use their lowered
-- `_0`/`_1` names in LeanerLang; both source destructuring spellings map to
-- the same explicit constructor pattern.
leaner module 0x42::positional_structs where
  struct Pair has Copy, Drop, Store where
    _0 : u64
    _1 : Bool

  fun make() -> Pair := new Pair { _0 := 7, _1 := true }

  fun first(pair : Pair) -> u64 := pair._0
  spec first where
    ensures result == pair._0

  fun destructure(pair : Pair) -> u64 := do
    let Pair { _0 := left, _1 := _ } := pair
    return left

  fun destructure_move_spelling(pair : Pair) -> u64 := do
    let Pair { _0 := left, _1 := right } := pair
    return if right then left else 0
  spec destructure_move_spelling where
    ensures result == if pair._1 then pair._0 else 0
    aborts_if false

set_option leaner.route "native" in
#leaner_verify 0x42::positional_structs::first
#leaner_require_native 0x42::positional_structs::first
set_option leaner.route "native" in
#leaner_verify 0x42::positional_structs::destructure_move_spelling
#leaner_require_native 0x42::positional_structs::destructure_move_spelling

open LeanerIR LeanerE2ETests.CheckSupport in
run_cmd do
  let pair (left : Int) (right : Bool) : RuntimeValue :=
    .nominal ⟨⟨0⟩, 0⟩ none #[.integer left, .bool right]
  assertRuns `«0x42».positional_structs #[
    ⟨"make", #[], .returned #[pair 7 true], {}⟩,
    ⟨"first", #[pair 9 false], .returned #[.integer 9], {}⟩,
    ⟨"destructure", #[pair 11 true], .returned #[.integer 11], {}⟩,
    ⟨"destructure_move_spelling", #[pair 12 true], .returned #[.integer 12], {}⟩]

#leaner_require_native_all
