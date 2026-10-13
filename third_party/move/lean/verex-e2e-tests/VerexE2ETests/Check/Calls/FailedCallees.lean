-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-!
# Callers of a callee that fails its specification

A callee seen through its contract whose own verification fails is reported
at the callee; its callers are verified assuming that contract, as the Move
Prover verifies a caller against its callees' specifications. A caller sees
the specification, not the body.
-/

namespace VerexSL.Tests.Check.Calls.FailedCallees

verex module 0x48::failed_callees where
  public fun bump(x : u64) -> u64 := x + 2

  spec bump where
    pragma opaque
    aborts_if x + 2 > MAX_U64
    ensures result == x + 1 -- error: the body adds two

  public fun twice(x : u64) -> u64 := bump(bump(x))

  spec twice where
    requires x < 1000
    aborts_if false
    ensures result == x + 2

  -- What the body computes is not what callers see.
  public fun twice_by_body(x : u64) -> u64 := bump(bump(x))

  spec twice_by_body where
    requires x < 1000
    ensures result == x + 4 -- error: callers see the specification

end VerexSL.Tests.Check.Calls.FailedCallees
