-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: expected failures and diagnostics.

import Move

/-! Diagnostics of specification functions: a Move function without a pure
reading (an imperative body, a mutable-reference parameter, a native,
recursion) applied in a specification, `spec fun` on a function whose version
is derived, `old` inside a specification function, a stateful function read
by a specification whose function does not use the resource, and a second
declaration. -/

open Move
open scoped Move Move.Spec

module Tests.Negative.SpecFunctions where

  struct Coin has Key where
    value : U64

  -- A body that reassigns a local has no pure reading.
  public fun count_up (limit : U64) : Action U64 := do
    let mut total := 0
    total := total + limit
    pure total

  public fun use_count (limit : U64) : Action U64 := do
    count_up limit

  /--
  error: Move function `Tests.Negative.SpecFunctions.count_up` has no specification version: reassigns a local (or writes through a reference); `spec fun count_up …` can declare one
  -/
  #guard_msgs in
  spec use_count (limit : U64) where
    ensures result = count_up limit;
    aborts_if False

  -- A mutable-reference parameter: not pure.
  public fun bump (coin : &mut Coin) : Action U64 := do
    let current ← *coin
    coin := { value := current.value + 1 }
    pure current.value

  public fun use_bump (coin : &mut Coin) : Action U64 := do
    bump coin

  /--
  error: Move function `Tests.Negative.SpecFunctions.bump` has no specification version: it takes a mutable reference; `spec fun bump …` can declare one
  -/
  #guard_msgs in
  spec use_bump (coin : &mut Coin) where
    ensures result = bump coin;
    aborts_if False

  -- A native has no body to read (its contract summarizes it for callers).
  native fun host_value (coin : Coin) : U64

  spec host_value (coin : Coin) where
    pragma aborts_if_is_partial;
    ensures True

  public fun use_host (coin : &Coin) : Action U64 := do
    let current ← *coin
    pure (host_value current)

  /--
  error: Move function `Tests.Negative.SpecFunctions.host_value` has no specification version: it has no body (a native); `spec fun host_value …` can declare one
  -/
  #guard_msgs in
  spec use_host (coin : Coin) where
    ensures result = host_value coin;
    aborts_if False

  -- A derived version cannot be redeclared.
  public fun is_empty (coin : &Coin) : Action Bool := do
    let current ← *coin
    pure (current.value == 0)

  /--
  error: Move function `Tests.Negative.SpecFunctions.is_empty` has a derived specification version; `spec fun` declares one only for a Move function without (a native, or a body with no pure reading)
  -/
  #guard_msgs in
  spec fun is_empty (coin : Coin) : Prop := coin.value = 0

  /--
  error: specification functions use mathematical `Int`, not Move integer type `Move.U64`
  -/
  #guard_msgs in
  spec fun bounded_parameter (value : U64) : Int := value

  /--
  error: specification functions return mathematical `Int`, not Move integer type `Move.U64`
  -/
  #guard_msgs in
  spec fun bounded_result (value : Int) : U64 := value

  /--
  error: specification functions use mathematical `Int`, not Move integer type `Move.U64`
  -/
  #guard_msgs in
  spec opaque bounded_opaque (value : U64) : Int

  /--
  error: specification function result inferred as a Move integer; declare and return `Int`
  -/
  #guard_msgs in
  spec fun inferred_bounded_result := U64.ofNat 0

  /--
  error: `old` cannot be used in a specification function; the specification that applies the function chooses the state it reads (`old(f …)` reads the pre-state)
  -/
  #guard_msgs in
  spec fun previous_balance (addr : Address) : Int := old(Coin[addr].value)

  spec fun balance (addr : Address) : Int := Coin[addr].value

  /--
  error: specification function `Tests.Negative.SpecFunctions.balance` is already declared
  -/
  #guard_msgs in
  spec fun balance (addr : Address) : Int := Coin[addr].value

  public fun unrelated (addr : Address) : Action Bool := do
    pure (addr == addr)

  /--
  error: specification function `Tests.Negative.SpecFunctions.balance` reads resource `Tests.Negative.SpecFunctions.Coin`, which the specified function does not use
  -/
  #guard_msgs in
  spec unrelated (addr : Address) where
    ensures balance addr = balance addr;
    aborts_if False

  /--
  error: `old` expects a global resource place
  -/
  #guard_msgs in
  spec unrelated (addr : Address) where
    ensures old(addr) = addr;
    aborts_if False
