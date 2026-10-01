-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Abstract and concrete clauses

A `[concrete]` clause is proved of the function's body; an `[abstract]` one
is what its callers see of it through its contract, which they assume.
An unmarked clause is both.
-/

namespace LeanerLang.Tests.Check.Calls.AbstractClauses

leaner module 0x47::codes where
  const INVALID_ARGUMENT : u64 := 1

  -- The body computes a code; callers see only its category.
  public fun canonical(category : u64, reason : u64) -> u64 :=
    (category << 16u8) + reason

  spec canonical where
    pragma opaque
    let_pre shl_res := (category << 16) % 18446744073709551616
    ensures [concrete] result == shl_res + reason
    aborts_if [abstract] false
    ensures [abstract] result == category

  public fun invalid_argument(reason : u64) -> u64 := canonical(INVALID_ARGUMENT, reason)

  spec invalid_argument where
    pragma opaque
    aborts_if false
    ensures result == INVALID_ARGUMENT

leaner module 0x47::client where
  use 0x47::codes

  public fun category() -> u64 := codes::invalid_argument(7)

  spec category where
    aborts_if false
    ensures result == 1

  -- Through the abstract view the concrete code is not seen.
  public fun code() -> u64 := codes::canonical(1, 7)

  spec code where
    ensures result == 65543 -- error: callers see the abstract view only

end LeanerLang.Tests.Check.Calls.AbstractClauses
