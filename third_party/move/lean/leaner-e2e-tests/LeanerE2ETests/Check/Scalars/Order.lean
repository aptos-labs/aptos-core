-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerE2ETests.CheckSupport

/-!
# Structural order and authored proofs

`core.prim.compare` is `std::cmp::compare`: the structural order of two
values of one type, `-1`, `0`, or `1`. On integers it is their natural order;
at a type parameter it is an arbitrary total order, whose laws
(`RuntimeValue.order_trans` and friends) an authored proof uses. A
`verify f by tactics` item runs the automatic closer first and hands the
obligations it leaves to the tactics.
-/

leaner module 0x42::order where
  fun smaller(a : u64, b : u64) -> u64 := if core.prim.compare(a, b) < 0 then a else b
  spec smaller where
    ensures result <= a && result <= b
    aborts_if false

  fun precedes {T} (a : &T, b : &T) -> Bool := core.prim.compare(a, b) < 0
  spec precedes where
    ensures result == (core.prim.compare(a, b) < 0)
    aborts_if false

  -- Transitivity is a law of the order, not a fact the closer derives.
  fun ordered3 {T} (a : T, b : T, c : T) -> Bool :=
    core.prim.compare(&a, &b) < 0 && core.prim.compare(&b, &c) < 0
  spec ordered3 where
    ensures result ==> core.prim.compare(a, c) < 0
    aborts_if false
  verify ordered3 by
    intro right
    exact Std.TransCmp.lt_trans ‹_› right
