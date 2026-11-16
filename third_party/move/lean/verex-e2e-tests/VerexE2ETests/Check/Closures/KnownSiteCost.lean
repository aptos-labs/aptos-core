-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-! The same closure target, verified in a module of three sites and in a
module of nine, fits the same budget. The same `generic` is specified and
verified in both modules below; the second declares six more sites than the
first, whose targets carry no specification of their own. The two controls
fit that one budget; they do not bound the cost of any number of sites.
-/
namespace VerexSL.Tests.Check.Closures.KnownSiteCost

verex module 0x9a::three_sites where
  public fun add(x : u64, y : u64) -> u64 := x + y
  spec add where
    ensures result == x + y
    aborts_if x + y > MAX_U64

  public fun pick {T has Copy, Drop}(first : Bool, a : T, b : T) -> T :=
    if first then a else b
  spec pick where
    ensures result == (if first then a else b)
    aborts_if false

  public fun generic(first : Bool, a : u64, b : u64) -> u64 := do
    let f := function[Fn(u64, u64) -> u64 has Copy, Drop](pick::<u64>, first, _, _)
    invoke(f, a, b)
  spec generic where
    ensures result == (if first then a else b)
    aborts_if false

  public fun generic_at {T has Copy, Drop}(first : Bool, a : T, b : T) -> T := do
    let f := function[Fn(T, T) -> T has Copy, Drop](pick::<T>, first, _, _)
    invoke(f, a, b)

  public fun created_at {T has Copy, Drop}(first : Bool, a : T) -> Fn(T) -> T has Copy, Drop :=
    function[Fn(T) -> T has Copy, Drop](pick::<T>, first, a, _)

verex module 0x9b::nine_sites where
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

  public fun generic(first : Bool, a : u64, b : u64) -> u64 := do
    let f := function[Fn(u64, u64) -> u64 has Copy, Drop](pick::<u64>, first, _, _)
    invoke(f, a, b)
  spec generic where
    ensures result == (if first then a else b)
    aborts_if false

  public fun generic_at {T has Copy, Drop}(first : Bool, a : T, b : T) -> T := do
    let f := function[Fn(T, T) -> T has Copy, Drop](pick::<T>, first, _, _)
    invoke(f, a, b)

  public fun created_at {T has Copy, Drop}(first : Bool, a : T) -> Fn(T) -> T has Copy, Drop :=
    function[Fn(T) -> T has Copy, Drop](pick::<T>, first, a, _)

  public fun leading(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, x)
    invoke(f, y)

  public fun trailing(x : u64, y : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](sub, _, y)
    invoke(f, x)

  public fun twice(x : u64) -> u64 := do
    let f := function[Fn(u64) -> u64 has Copy, Drop](add, 1)
    invoke(f, invoke(f, x))

  public fun field(x : u64, y : u64) -> u64 := do
    let op := new Op { f := function[Fn(u64) -> u64 has Copy, Drop](add, x) }
    invoke(op.f, y)

  public fun element(x : u64, y : u64) -> u64 := do
    let fs := vector<Fn(u64) -> u64 has Copy, Drop>[function[Fn(u64) -> u64 has Copy, Drop](add, x)]
    invoke(fs[0], y)

end VerexSL.Tests.Check.Closures.KnownSiteCost
