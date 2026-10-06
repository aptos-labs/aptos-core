-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Data invariant violations

A function without a specification is checked against the data invariants
of the values it takes and returns, a multi-value result component by
component. A structure invariant reads the whole value as `this`.
-/

namespace LeanerLang.Tests.Check.Structs.InvariantErrors

leaner module 0x42::invariant_errors where
  struct Percent has Copy, Drop where
    value : u64

  spec Percent where
    invariant this.value <= 100

  fun full() -> Percent := new Percent { value := 100 }

  fun too_much() -> Percent := new Percent { value := 101 }

  fun pair() -> (u64, Percent) := (1, new Percent { value := 200 })

end LeanerLang.Tests.Check.Structs.InvariantErrors
