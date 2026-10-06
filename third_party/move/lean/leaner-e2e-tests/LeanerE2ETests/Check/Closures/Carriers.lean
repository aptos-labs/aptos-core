-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Function values typed by their carriers

A function value a proof does not see is typed by its carrier where the
unit's native and semantic readings agree: its invocation takes no typing
assumption, and memory with a function-typed field is typed. The unit
declares a phantom type parameter, whose arguments a native nominal type
keeps, so its readings agree.
-/

namespace LeanerLang.Tests.Check.Closures.Carriers

leaner module 0x99::carriers where
  struct Tag {T : phantom type} has Copy, Drop, Store where
    id : u64

  struct Hook has Key where
    f : Fn(u64) -> u64 has Copy, Store

  struct Tagged has Key where
    g : Fn(Tag<u64>) -> u64 has Copy, Store

  fun apply(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec apply where
    ensures ensures_of<f>(x, result)

  fun triple(y : u64) -> u64 := y * 3
  spec triple where
    ensures result == y * 3

  fun apply_triple(x : u64) -> u64 := apply(function[Fn(u64) -> u64](triple), x)
  spec apply_triple where
    ensures result == x * 3

  fun apply_triple_incorrect(x : u64) -> u64 := apply(function[Fn(u64) -> u64](triple), x)
  spec apply_triple_incorrect where
    ensures result == x * 2 -- error: the closure triples

end LeanerLang.Tests.Check.Closures.Carriers
