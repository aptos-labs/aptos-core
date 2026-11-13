-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Structural order and authored proofs

`core.prim.compare` is `std::cmp::compare`: the structural order of two
values of one type, `-1`, `0`, or `1`. On integers it is their natural order;
at a type parameter it is an arbitrary total order, whose laws
(`Std.TransCmp`, a typed order's) an authored proof uses. A
`verify f by lean => tactics` item runs the automatic solver first and hands the
obligations it leaves to the tactics.
-/

verex module 0x42::order where
  fun smaller(a : u64, b : u64) -> u64 := if core.prim.compare(a, b) < 0 then a else b
  spec smaller where
    ensures result <= a && result <= b
    aborts_if false

  fun precedes {T} (a : &T, b : &T) -> Bool := core.prim.compare(a, b) < 0
  spec precedes where
    ensures result == (core.prim.compare(a, b) < 0)
    aborts_if false

  -- Transitivity is a law of the order, not a fact the solver derives.
  fun ordered3 {T} (a : T, b : T, c : T) -> Bool :=
    core.prim.compare(&a, &b) < 0 && core.prim.compare(&b, &c) < 0
  spec ordered3 where
    ensures result ==> core.prim.compare(a, c) < 0
    aborts_if false
  verify ordered3 by lean =>
    verex_obligations
    all_goals (first | verex_decide | (verex_prepare; all_goals try verex_decide))
    intro left right
    exact Std.TransCmp.lt_trans left right
