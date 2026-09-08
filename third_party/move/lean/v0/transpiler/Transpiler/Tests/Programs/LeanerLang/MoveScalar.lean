-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A compact Move fixture for the canonical LeanerLang round trip. -/
leaner module 0x42::move_scalar where
  const ZERO : u64 := 0

  struct Pair has Copy, Drop where
    left : u64
    right : u64

  public fun invert(value : Bool) -> Bool := !value
