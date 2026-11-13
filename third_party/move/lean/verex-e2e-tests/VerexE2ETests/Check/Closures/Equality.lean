-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Equality of known function values

Two closures of one site are equal where their captures are, closures of
different sites are not: equality is read off the generated structure, the
target, mask and instantiation and the captures as native values.
-/

namespace VerexSL.Tests.Check.Closures.Equality

verex module 0x42::closure_equality where
  fun add(x : u64, y : u64) -> u64 := x + y
  fun sub(x : u64, y : u64) -> u64 := x - y

  fun same(a : u64) -> Bool :=
    function[Fn(u64) -> u64 has Copy, Drop](add, a, _) ==
      function[Fn(u64) -> u64 has Copy, Drop](add, a, _)
  spec same where
    ensures result

  fun by_capture(a : u64, b : u64) -> Bool :=
    function[Fn(u64) -> u64 has Copy, Drop](add, a, _) ==
      function[Fn(u64) -> u64 has Copy, Drop](add, b, _)
  spec by_capture where
    ensures result == (a == b)

  fun by_site(a : u64) -> Bool :=
    function[Fn(u64) -> u64 has Copy, Drop](add, a, _) ==
      function[Fn(u64) -> u64 has Copy, Drop](sub, a, _)
  spec by_site where
    ensures !result

  fun by_site_incorrect(a : u64) -> Bool :=
    function[Fn(u64) -> u64 has Copy, Drop](add, a, _) ==
      function[Fn(u64) -> u64 has Copy, Drop](sub, a, _)
  spec by_site_incorrect where
    ensures result -- error: closures of different targets differ

end VerexSL.Tests.Check.Closures.Equality
