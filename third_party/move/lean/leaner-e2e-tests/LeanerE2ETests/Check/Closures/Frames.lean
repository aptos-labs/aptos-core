-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Frames of function-valued parameters

A function-valued parameter without `modifies_of` keeps global memory: a
higher-order function assumes it, so that an iteration invoking the
parameter keeps the state its precondition speaks about. A caller
establishes it from the target's theorem, or from its body whatever its
precondition. A parameter may take a shared reference, which is the value
it observes, and memory holding an enum resource or an instance of a
generic declaration is typed.
`modifies_of<f>(a) global<R>(a)` widens the frame to the
targets at an invocation's arguments, and `modifies_of<f> *` leaves memory
open.
`result_of` of a literal closure is read where the invocation does not
abort, and the aborts of a target whose specification states none are read
from its body.
-/

namespace LeanerLang.Tests.Check.Closures.Frames

leaner module 0x99::closure_frames where
  struct Counter has Key where
    value : u64

  struct Config has Key where
    active : Bool

  enum Mode has Key where
    | Off
    | On (level : u64)

  struct Box {T has Copy, Drop, Store} has Copy, Drop, Store where
    value : T

  struct Holder has Key where
    boxed : Box<u64>

  public fun apply(f : Fn(u64) -> u64, x : u64) -> u64 := invoke(f, x)
  spec apply where
    pragma opaque
    requires !aborts_of<f>(x)
    aborts_if false
    ensures ensures_of<f>(x, result)

  public fun keeps(f : Fn(u64) -> u64, x : u64, addr : Address) -> u64 := invoke(f, x)
  spec keeps where
    pragma aborts_if_is_partial
    ensures exists<Counter>(addr) == old(exists<Counter>(addr))

  -- Memory holding an enum resource, or a resource with a field of a generic
  -- declaration's instance, is typed at its declaration's fields.
  public fun keeps_mode(f : Fn(u64) -> u64, x : u64, addr : Address) -> u64 := invoke(f, x)
  spec keeps_mode where
    pragma aborts_if_is_partial
    ensures exists<Mode>(addr) == old(exists<Mode>(addr))
    ensures exists<Holder>(addr) == old(exists<Holder>(addr))

  public fun count(v : &Vector<u64>, pred : Fn(&u64) -> Bool has Copy, Drop) -> u64 := do
    let i := 0
    let n := 0
    let len := v.length
    while i < len do
      if invoke(pred, &v[i]) then n := n + 1
      i := i + 1
    where
      invariant i <= len
      invariant n <= i
    n
  spec count where
    requires ∀ (x in 0 .. v.length), !aborts_of<pred>(v[x])
    aborts_if false
    ensures result <= v.length

  public fun twice(f : Fn(u64) -> u64 has Copy, x : u64) -> u64 := invoke(f, x) + invoke(f, x)
  spec twice where
    pragma opaque
    ensures result == result_of<f>(x) + result_of<f>(x)

  fun plus_one(x : u64) -> u64 := x + 1
  spec plus_one where
    aborts_if x == MAX_U64
    ensures result == x + 1

  fun positive(x : &u64) -> Bool := *x > 0
  spec positive where
    ensures result == (x > 0)

  fun peek(addr : Address, x : u64) -> u64 := do
    let value := &Counter[addr].value
    if *value == 0 then x else x
  spec peek where
    requires exists<Counter>(addr)
    ensures result == x

  fun bump(addr : Address, x : u64) -> u64 := do
    let value := &mut Counter[addr].value
    *value := x
    x
  spec bump where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures result == x

  public fun apply_plus_one() -> u64 := apply(function[Fn(u64) -> u64](plus_one), 1)
  spec apply_plus_one where
    ensures result == 2

  public fun twice_plus_one() -> u64 := twice(function[Fn(u64) -> u64 has Copy](plus_one), 1)
  spec twice_plus_one where
    ensures result == 4

  -- `positive` states no aborts; its body has none.
  public fun count_positive(v : &Vector<u64>) -> u64 :=
    count(v, function[Fn(&u64) -> Bool has Copy, Drop](positive))
  spec count_positive where
    ensures result <= v.length

  -- `peek`'s body keeps memory, whatever its precondition.
  public fun apply_peek(addr : Address, x : u64) -> u64 :=
    apply(function[Fn(u64) -> u64](peek, addr), x)
  spec apply_peek where
    requires exists<Counter>(addr)
    ensures result == x

  public fun apply_bump_incorrect(addr : Address, x : u64) -> u64 :=
    apply(function[Fn(u64) -> u64](bump, addr), x) -- error: `bump` changes memory
  spec apply_bump_incorrect where
    requires exists<Counter>(addr)
    ensures result == x

  -- ## Declared frames

  public fun apply_writes(f : Fn(Address) -> u64, x : Address) -> u64 := invoke(f, x)
  spec apply_writes where
    pragma opaque
    pragma aborts_if_is_partial
    modifies_of<f>(a : Address) global<Counter>(a)
    ensures exists<Config>(x) == old(exists<Config>(x))
    modifies *

  public fun apply_any(f : Fn(Address) -> u64, x : Address) -> u64 := invoke(f, x)
  spec apply_any where
    pragma aborts_if_is_partial
    modifies_of<f> *
    ensures exists<Config>(x) == old(exists<Config>(x)) -- error: `f` may change any memory
    modifies *

  fun set_counter(addr : Address) -> u64 := do
    let value := &mut Counter[addr].value
    *value := 7
    7
  spec set_counter where
    requires exists<Counter>(addr)
    modifies global<Counter>(addr)
    ensures result == 7

  fun set_config(addr : Address) -> u64 := do
    let active := &mut Config[addr].active
    *active := true
    7
  spec set_config where
    requires exists<Config>(addr)
    modifies global<Config>(addr)
    ensures result == 7

  public fun writes_counter(addr : Address) -> u64 :=
    apply_writes(function[Fn(Address) -> u64](set_counter), addr)
  spec writes_counter where
    requires exists<Counter>(addr)
    modifies *

  public fun writes_config_incorrect(addr : Address) -> u64 :=
    apply_writes(function[Fn(Address) -> u64](set_config), addr) -- error: outside the frame
  spec writes_config_incorrect where
    requires exists<Config>(addr)
    modifies *

end LeanerLang.Tests.Check.Closures.Frames
