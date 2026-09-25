-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-!
# Diagnostics of specification functions

One module per case: a native applied in a specification, `spec fun` over a function whose version
is derived, and a second declaration are rejected; so are a recursive
specification function without a `decreases` measure and one whose
recursive call does not decrease it.

Accepted, by contrast: a body that reassigns a local has a pure
reading (the assignment rebinds the local). A function taking a mutable
reference reads its entry value when applied to `old(..)`; applied to the
bare parameter, which denotes the exit value in `ensures`, the reading
differs from the result, and the clause fails. A clause reading a resource
through a specification function brings that resource into the contract, as
a direct read does. The Move surface declares specification functions over
its bounded integer types, and a specification function may read `old`, the
pre-state of the specification applying it.
-/

leaner module 0x42::reassigns where
  public fun count_up(limit : u64) -> u64 := do
    let mut total : u64 := 0
    total := total + limit
    total

  public fun use_count(limit : u64) -> u64 := count_up(limit)
  spec use_count where
    ensures result == count_up(limit)
    aborts_if false

leaner module 0x42::mutable_parameter where
  struct Coin has Key where
    value : u64

  public fun bump(coin : &mut Coin) -> u64 := do
    let current := *coin
    *coin := new Coin { value := current.value + 1 }
    current.value

  public fun use_bump(coin : &mut Coin) -> u64 := bump(coin)
  spec use_bump where
    ensures result == bump(old(coin))
    aborts_if old(coin).value + 1 > MAX_U64

  public fun use_bump_exit(coin : &mut Coin) -> u64 := bump(coin)
  spec use_bump_exit where
    pragma aborts_if_is_partial
    ensures result == bump(coin)

leaner module 0x42::native_version where
  struct Coin has Key where
    value : u64

  native fun host_value(coin : Coin) -> u64
  spec host_value where
    pragma aborts_if_is_partial
    ensures true

  public fun use_host(coin : &Coin) -> u64 := host_value(*coin)
  spec use_host where
    ensures result == host_value(coin)
    aborts_if false

leaner module 0x42::redeclared_version where
  struct Coin has Key where
    value : u64

  public fun is_empty(coin : &Coin) -> Bool := (*coin).value == 0

  spec fun is_empty(coin : Coin) : Bool := coin.value == 0

leaner module 0x42::bounded_parameter where
  spec fun bounded_parameter(value : u64) : Int := value

leaner module 0x42::bounded_result where
  spec fun bounded_result(value : Int) : u64 := value

leaner module 0x42::bounded_opaque where
  opaque spec fun bounded_opaque(value : u64) : Int

leaner module 0x42::old_in_spec_function where
  struct Coin has Key where
    value : u64

  spec fun previous_balance(addr : Address) : Int := old(global<Coin>(addr).value)

leaner module 0x42::declared_twice where
  struct Coin has Key where
    value : u64

  spec fun balance(addr : Address) : Int := global<Coin>(addr).value
  spec fun balance(addr : Address) : Int := global<Coin>(addr).value

leaner module 0x42::unrelated_reader where
  struct Coin has Key where
    value : u64

  public fun unrelated(addr : Address) -> Bool := addr == addr
  spec unrelated where
    ensures result && balance(addr) == balance(addr)
    aborts_if false

  spec fun balance(addr : Address) : Int := global<Coin>(addr).value

leaner module 0x42::undecreasing where
  fun zero() -> u64 := 0
  spec zero where
    ensures loops(1) == loops(1)
    aborts_if false

  spec fun loops(n : Int) : Int := loops(n) + 1

leaner module 0x42::misdecreasing where
  spec fun grows(n : Int) : Int decreases n := if n > 0 then grows(n + 1) else 0
