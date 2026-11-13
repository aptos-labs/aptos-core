-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexMove

/-! A Verex Move module calling another: the call is to the Move module
`0x42::Math`, which compiler-v2 loads from `Math.lean`. -/

verex module 0x42::Client where
  use 0x42::Math

  public fun imported_identity(value : u64) -> u64 :=
    (Math::identity::<u64>((Math::identity::<u64>(value) : u64)) : u64)
