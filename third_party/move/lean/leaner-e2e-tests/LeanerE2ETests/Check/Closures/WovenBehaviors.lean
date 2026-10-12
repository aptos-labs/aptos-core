-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Behaviors of a closure that captures

A closure capturing some of its target's arguments behaves as the function
value naming the target at the woven arguments: their `aborts_of` and
`ensures_of` agree, a leading or a trailing capture alike.
-/

namespace LeanerLang.Tests.Check.Closures.WovenBehaviors

leaner module 0x42::woven_behaviors where
  fun sub(x : u64, y : u64) -> u64 := x - y
  spec sub where
    aborts_if x < y
    ensures result == x - y

  fun leading(x : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](sub, x)
  spec leading where
    aborts_if false
    ensures ∀ (y : u64),
      aborts_of<result>(y) == aborts_of<function[Fn(u64, u64) -> u64](sub)>(x, y)
    ensures ∀ (y : u64), ∀ (r : u64),
      ensures_of<result>(y, r) == ensures_of<function[Fn(u64, u64) -> u64](sub)>(x, y, r)

  fun trailing(y : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](sub, _, y)
  spec trailing where
    aborts_if false
    ensures ∀ (x : u64),
      aborts_of<result>(x) == aborts_of<function[Fn(u64, u64) -> u64](sub)>(x, y)

  -- The captured value is the first argument, not the second.
  fun swapped(x : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](sub, x)
  spec swapped where
    aborts_if false
    ensures ∀ (y : u64),
      aborts_of<result>(y) == aborts_of<function[Fn(u64, u64) -> u64](sub)>(y, x) -- error: the arguments are exchanged

end LeanerLang.Tests.Check.Closures.WovenBehaviors
