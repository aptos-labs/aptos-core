-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x99::field_update where
  struct Pair {T has Copy, Drop} has Copy, Drop where
    first : T
    second : u64

  fun replace {T has Copy, Drop}(pair : Pair<T>, value : T) -> Pair<T> := do
    let mut updated := pair
    updated.first := value
    updated
  spec replace where
    ensures result == core.data.updateField(pair, first, value)

  enum Choice has Copy, Drop where
    | A (count : u64, flag : Bool)
    | B (flag : Bool, count : u64)

  fun replace_count(choice : Choice, value : u64) -> Choice := do
    let mut updated := choice
    updated.count := value
    updated
  spec replace_count where
    ensures result == core.data.updateField(choice, count, value)

  fun preserve_other(pair : Pair<u64>, value : u64) -> u64 := pair.second
  spec preserve_other where
    ensures result == core.data.updateField(pair, first, value).second

  fun replace_both(pair : Pair<u64>, first : u64, second : u64) -> Pair<u64> :=
    new Pair<u64> { first := first, second := second }
  spec replace_both where
    ensures result == core.data.updateField(core.data.updateField(pair, first, first), second, second)
