-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

-- Enum declaration locals bind the whole tagged value as `this`, irrespective
-- of the number or type of fields in the selected variant.
verex module 0x42::ghost_initializers where
  enum Slot has Copy, Drop where
    | Empty
    | Full (item : u64)
  spec Slot where
    ghost occupied : Bool := this is Full

  fun full() -> Slot := new Slot::Full { item := 1 }
  spec full where
    ensures result.occupied
    aborts_if false
  verify full

  fun empty() -> Slot := new Slot::Empty {}
  spec empty where
    ensures !result.occupied
    aborts_if false
  verify empty

  struct Counter has Copy, Drop where
    value : u64
  spec Counter where
    ghost initial : Int := this.value

  fun fresh() -> Counter := new Counter { value := 7 }
  spec fresh where
    ensures result.initial == 7
    aborts_if false
  verify fresh
