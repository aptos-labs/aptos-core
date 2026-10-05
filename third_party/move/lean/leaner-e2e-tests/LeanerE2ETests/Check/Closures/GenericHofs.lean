-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Generic higher-order functions

A generic function invoking a function value it cannot see over its type
parameters: the value is typed at the semantic types its frame gives the
parameters, as its callers establish at their type arguments, a literal
closure by its target's types and a function value of their own by their
own typing. A callee inlined at type arguments invokes at the caller's
types.
-/

namespace LeanerLang.Tests.Check.Closures.GenericHofs

leaner module 0x99::generic_hofs where
  fun map {T} {R}(f : Fn(T) -> R, x : T) -> R := invoke(f, x)
  spec map where
    ensures ensures_of<f>(x, result)

  fun apply_both {A} {B} {R}(f : Fn(A, B) -> R, x : A, y : B) -> R := invoke(f, x, y)
  spec apply_both where
    ensures ensures_of<f>(x, y, result)

  fun triple(y : u64) -> u64 := y * 3
  spec triple where
    ensures result == y * 3

  fun pick(flag : Bool, y : u64) -> u64 := if flag then y else 0
  spec pick where
    ensures result == (if flag then y else 0)

  fun map_triple(x : u64) -> u64 := map(function[Fn(u64) -> u64](triple), x)
  spec map_triple where
    ensures result == x * 3

  fun apply_pick(flag : Bool, x : u64) -> u64 :=
    apply_both(function[Fn(Bool, u64) -> u64](pick), flag, x)
  spec apply_pick where
    ensures flag ==> result == x
    ensures !flag ==> result == 0

  -- A function value of the caller's own, passed through.
  fun map_own(f : Fn(u64) -> u64, x : u64) -> u64 := map(f, x)
  spec map_own where
    ensures ensures_of<f>(x, result)

  fun map_triple_incorrect(x : u64) -> u64 := map(function[Fn(u64) -> u64](triple), x)
  spec map_triple_incorrect where
    ensures result == x * 2 -- error: the closure triples

end LeanerLang.Tests.Check.Closures.GenericHofs
