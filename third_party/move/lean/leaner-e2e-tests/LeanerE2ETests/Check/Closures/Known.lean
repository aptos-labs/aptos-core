-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Known closures

An invocation of a closure whose target and captures a proof sees is a call
of its target, verified through the target's contract: captured parameters
in any position, a closure invoked twice, held in a struct field or a
vector, and a generic target at its type arguments, concrete or a generic
caller's own parameters, with a function type over those parameters in the
module.
-/

namespace LeanerLang.Tests.Check.Closures.Known

leaner module 0x99::known_closures where
  struct Op has Copy, Drop where
    f : Fn(u64) -> u64 has Copy, Drop

  public fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  public fun sub(x : u64, y : u64) -> u64 := x - y
  spec sub where
    ensures result == x - y
    aborts_if x < y

  public fun pick {T has Copy, Drop}(first : Bool, a : T, b : T) -> T :=
    if first then a else b
  spec pick where
    ensures result == (if first then a else b)
    aborts_if false

  public fun leading(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)
  spec leading where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  public fun trailing(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](sub, _, y)
    invoke(f, x)
  spec trailing where
    ensures result == x - y
    aborts_if x < y

  public fun twice(x : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, 1)
    invoke(f, invoke(f, x))
  spec twice where
    ensures result == x + 2
    aborts_if x + 2 > MAX_U64

  public fun field(x : u64, y : u64) -> u64 := do
    let op := new Op { f := function[Fn(u64) -> u64 has Copy, Drop](add, x) }
    invoke(op.f, y)
  spec field where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  public fun element(x : u64, y : u64) -> u64 := do
    let fs := vector<Fn(u64) -> u64 has Copy, Drop>[function[Fn(u64) -> u64 has Copy, Drop](add, x)]
    invoke(fs[0], y)
  spec element where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  public fun generic(first : Bool, a : u64, b : u64) -> u64 := do
    let f := function[Fn(u64, u64) -> u64 has Copy, Drop](pick::<u64>, first, _, _)
    invoke(f, a, b)
  spec generic where
    ensures result == (if first then a else b)
    aborts_if false

  public fun generic_at {T has Copy, Drop}(first : Bool, a : T, b : T) -> T := do
    let f := function[Fn(T, T) -> T has Copy, Drop](pick::<T>, first, _, _)
    invoke(f, a, b)
  spec generic_at where
    ensures result == (if first then a else b)
    aborts_if false

  public fun created_at {T has Copy, Drop}(first : Bool, a : T) -> Fn(T) -> T has Copy, Drop :=
    function[Fn(T) -> T has Copy, Drop](pick::<T>, first, a, _)
  spec created_at where
    aborts_if false

  public fun wrong(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)
  spec wrong where
    ensures result == x + y + 1 -- error: the closure adds its capture and argument only
    aborts_if x + y > MAX_U64

end LeanerLang.Tests.Check.Closures.Known
