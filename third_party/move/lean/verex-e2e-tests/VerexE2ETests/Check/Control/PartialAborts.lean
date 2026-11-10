-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

-- Strictness supplies an implicit `aborts_if false` for an empty list.
-- Partial mode makes that condition sufficient only, so it does not forbid
-- other aborts, including those of an opaque callee.
verex module 0x99::partial_aborts where
  pragma aborts_if_is_strict

  public fun increment(value : u64) -> u64 := value + 1
  spec increment where
    pragma aborts_if_is_partial
    pragma opaque
    ensures result == value + 1

  public fun twice(value : u64) -> u64 := increment(increment(value))
  spec twice where
    pragma aborts_if_is_partial
    ensures result == value + 2

  -- The implicit empty condition agrees with an explicit false condition.
  public fun explicit_false(value : u64) -> u64 := value + 1
  spec explicit_false where
    pragma aborts_if_is_partial
    aborts_if false
    ensures result == value + 1
