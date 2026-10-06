-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

namespace LeanerLang.Tests.Check.Specifications.CapturedFunctionValues

leaner module 0x42::captured_function_values where
  fun sub(x : u64, y : u64) -> u64 := x - y
  spec sub where
    aborts_if x < y
    ensures result == x - y

  fun leading(x : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](sub, x)
  spec leading where
    aborts_if false
    ensures result == function[Fn(u64) -> u64 has Copy, Drop](sub, x)

  fun trailing(y : u64) -> Fn(u64) -> u64 has Copy, Drop :=
    function[Fn(u64) -> u64 has Copy, Drop](sub, _, y)
  spec trailing where
    aborts_if false
    ensures result == function[Fn(u64) -> u64 has Copy, Drop](sub, _, y)

end LeanerLang.Tests.Check.Specifications.CapturedFunctionValues
