-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
An authored proof addresses an obligation by its key
(`doc/plan/proof-language.md`, "Obligation addressing"): a callee's
precondition by the callee's clause and the call, numbered among the calls
of the callee in source order, so that one call's precondition is routed
and the other's is not; without the call, every call's.
-/

namespace VerexSL.Tests.Check.Specifications.Addressing

verex module 0x42::addressing where
  fun positive(x : u64) -> u64 := x
  spec positive where
    pragma opaque
    requires x > 0
    aborts_if false
    ensures result == x

  fun twice(x : u64) -> u64 := do
    let a := positive(x + 1)
    positive(a)
  spec twice where
    requires x < 100
    aborts_if false
    ensures result == x + 1

  fun twice_again(x : u64) -> u64 := do
    let a := positive(x + 1)
    positive(a)
  spec twice_again where
    requires x < 100
    aborts_if false
    ensures result == x + 1

  -- Only the second call's precondition takes the first alternative.
  verify twice by
    verex_obligations
    verex_on
    | requires 1 of positive at call 2 => trace "routed: the second call's precondition"; verex_solve
    | _ => verex_solve

  -- Both calls' preconditions take it.
  verify twice_again by
    verex_obligations
    verex_on
    | requires 1 of positive => trace "routed: a call's precondition"; verex_solve
    | _ => verex_solve

end VerexSL.Tests.Check.Specifications.Addressing
