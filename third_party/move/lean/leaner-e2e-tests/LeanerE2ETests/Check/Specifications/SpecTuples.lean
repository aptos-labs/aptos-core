-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Specifications read tuples: a specification function's tuple result
compared with a tuple literal, and a function a clause calls that
destructures an enum value and returns a tuple. -/

leaner module 0x42::spec_tuples where
  enum PairBox has Copy, Drop where
    | Pair (x : u64, y : u64)

  spec fun swap(x : Int, y : Int) : (u64, u64) := (y, x)

  fun swapped() -> Bool := true
  spec swapped where
    ensures swap(1, 2) == (2, 1)

  fun unbox_pair(box : PairBox) -> (u64, u64) := do
    let PairBox::Pair { x := x, y := y } := box
    (x, y)

  fun unboxed() -> Bool := true
  spec unboxed where
    ensures unbox_pair(new PairBox::Pair { x := 1, y := 2 }) == (1, 2)
