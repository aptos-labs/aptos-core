-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Specification functions

An executable function
applied in a specification denotes its derived specification version: the
pure reading of its body (references erased, assertions dropped, global
reads against the clause's state). `spec fun` declares one by hand where
there is no derivation (a native), or a standalone specification function.
-/

leaner module 0x99::spec_functions where
  struct Coin has Key where
    value : u64

  const LIMIT : u64 := 100

  -- ## Derived versions of functions

  -- An effectful function (it reads through its reference); its
  -- specification version is the same reading, over the value.
  public fun is_big(coin : &Coin) -> Bool := do
    let current := *coin
    current.value > LIMIT

  public fun check(coin : &Coin) -> Bool := is_big(coin)
  spec check where
    ensures result == is_big(coin)
    aborts_if false

  -- An assertion is dropped by the reading; the function's contract states
  -- the abort.
  public fun value_below_limit(coin : &Coin) -> u64 := do
    let current := *coin
    assert(current.value < LIMIT, LIMIT)
    current.value

  public fun read_value(coin : &Coin) -> u64 := value_below_limit(coin)
  spec read_value where
    ensures result == value_below_limit(coin)
    aborts_if !(coin.value < LIMIT) with LIMIT

  -- A pure function: its version is its body.
  fun double(value : u64) -> u64 := value + value

  fun quad(value : u64) -> u64 := double(double(value))
  spec quad where
    requires value * 4 <= MAX_U64
    ensures result == double(double(value))

  -- Vector membership through a reference.
  public fun holds(values : &Vector<u64>, value : u64) -> Bool := do
    let element := &value
    core.prim.containsVector(*values, *element)

  public fun holds_twice(values : &Vector<u64>, value : u64) -> Bool := do
    let once := holds(values, value)
    let again := holds(values, value)
    once && again
  spec holds_twice where
    ensures result == holds(values, value)
    aborts_if false

  -- An element borrow reads the element of the vector.
  public fun first(values : &Vector<u64>) -> u64 := do
    let element := &values[0]
    *element

  public fun first_again(values : &Vector<u64>) -> u64 := first(values)
  spec first_again where
    pragma aborts_if_is_partial
    ensures result == first(values)

  -- An abort in value position reads as an unspecified value of the site:
  -- the contract says nothing about it.
  public fun pick(flag : Bool, value : u64) -> u64 :=
    if flag then value else abort(7)

  public fun pick_again(flag : Bool, value : u64) -> u64 := pick(flag, value)
  spec pick_again where
    pragma aborts_if_is_partial
    ensures flag ==> result == pick(flag, value)

  -- ## Stateful specification functions

  public fun balance_of(addr : Address) -> u64 := do
    let value := &Coin[addr].value
    *value
  spec balance_of where
    ensures result == balance(addr)
    aborts_if !exists<Coin>(addr)

  -- A standalone stateful specification function.
  spec fun balance(addr : Address) : Int := global<Coin>(addr).value

  public fun rich(addr : Address) -> Bool := do
    let value := &Coin[addr].value
    let current := *value
    current > LIMIT
  spec rich where
    ensures result == is_rich(addr)
    aborts_if !exists<Coin>(addr)

  -- The derived version of a global reader is stateful too; versions and
  -- standalone functions compose.
  spec fun is_rich(addr : Address) : Bool := balance_of(addr) > LIMIT

  -- `exists` in the body is the existence test of the version.
  public fun has_coin(addr : Address) -> Bool := exists<Coin>(addr)

  public fun has_coin_too(addr : Address) -> Bool := has_coin(addr)
  spec has_coin_too where
    ensures result == has_coin(addr)
    aborts_if false

  -- `old(f(..))` reads the pre-state through a stateful version.
  public fun deposit(addr : Address, amount : u64) -> Unit := do
    let value := &mut Coin[addr].value
    *value := *value + amount
  spec deposit where
    modifies global<Coin>(addr)
    ensures balance_of(addr) == old(balance_of(addr)) + amount
    aborts_if !exists<Coin>(addr)
    aborts_if old(balance(addr)) + amount > MAX_U64

  -- ## Natives and generic specification functions

  -- A native has no body to read: `spec fun` declares its version.
  native fun host_length(values : Vector<u64>) -> u64
  spec host_length where
    pragma aborts_if_is_partial
    ensures result == values.length

  spec fun host_length(values : Vector<u64>) : Int := values.length

  fun measure(values : Vector<u64>) -> u64 := host_length(values)
  spec measure where
    pragma aborts_if_is_partial
    ensures result == host_length(values)

  public fun push(values : &mut Vector<u64>, value : u64) -> Unit := do
    let current := *values
    *values := core.prim.pushVector(current, value)
  spec push where
    requires values.length + 1 <= MAX_U64
    ensures holds_in(values, value)
    aborts_if false

  -- A generic specification function.
  spec fun holds_in {T} (values : Vector<T>, value : T) : Bool := values.contains(value)
