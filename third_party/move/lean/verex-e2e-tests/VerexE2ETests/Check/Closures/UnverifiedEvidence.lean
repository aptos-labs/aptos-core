-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Behaviors of a callee whose body is not verified

`pragma verify = false` takes a body out of the automatic verification. The
typing a run of a function value the proof cannot see needs is owed by the
callee's signature and contract either way, so a caller establishes it at the
call and obtains the behavior the contract states — and nothing beyond it.
-/

namespace VerexSL.Tests.Check.Closures.UnverifiedEvidence

verex module 0x4e::unverified_evidence where
  fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    aborts_if x + y > 18446744073709551615
    ensures result == x + y

  fun apply(f : Fn(u64) -> u64 has Copy, Drop, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    pragma verify = false
    aborts_if aborts_of<f>(x)
    ensures ensures_of<f>(x, result)

  -- The closure the caller hands over is typed by evaluation over the unit,
  -- which is what the callee's contract assumes.
  fun use_apply(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](add, 1, _), x)
  spec use_apply where
    aborts_if x + 1 > 18446744073709551615
    ensures result == x + 1

  -- The contract states the target's own failures, so the capture overflows.
  fun overclaim(x : u64) -> u64 := apply(function[Fn(u64) -> u64 has Copy, Drop](add, 1, _), x)
  spec overclaim where
    aborts_if false -- error: the captured addition overflows at the greatest `u64`
    ensures result == x + 1

end VerexSL.Tests.Check.Closures.UnverifiedEvidence
