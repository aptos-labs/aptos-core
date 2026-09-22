-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: specification and verification.

import Move

/-! Specification functions.  A Move function applied in a specification
denotes its *specification version*: the pure reading of its body (references
erased, `assert!`s dropped, global reads as places), derived at the function's
declaration — the reading under which Move lets a pure function be called in
a specification.  `spec fun f … := body` declares one by hand where there is
no derivation (a native), or a standalone specification function.  A
stateful one reads global places: the clause applying it passes its own
state, and `old(f …)` the pre-state.  The automatic prover unfolds them. -/

namespace Tests.MovePrograms.SpecFunctions

open Move
open scoped Move Move.Spec

module SpecFunctions where

  struct Coin has Key where
    value : U64

  def LIMIT : U64 := 100

  /-! ## Derived specification versions -/

  -- An effectful Move function (it reads through its reference); its
  -- specification version is the same reading, over the value.
  public fun is_big (coin : &Coin) : Action Bool := do
    let current ← *coin
    pure (current.value > LIMIT)

  public fun check (coin : &Coin) : Action Bool := do
    is_big coin

  spec check (coin : Coin) where
    ensures result = is_big coin;
    aborts_if False

  verify check

  -- An `assert!` is dropped by the reading (the specification language's
  -- partial semantics); the function's contract states the abort.
  public fun value_below_limit (coin : &Coin) : Action U64 := do
    let current ← *coin
    assert!(current.value < LIMIT, LIMIT)
    pure current.value

  public fun read_value (coin : &Coin) : Action U64 := do
    value_below_limit coin

  spec read_value (coin : Coin) where
    ensures result = value_below_limit coin;
    aborts_if ¬coin.value < LIMIT with LIMIT

  verify read_value

  -- A pure Move function: its version is its body; the value contract of a
  -- caller applies it.
  fun double (value : U64) : U64 := value + value

  fun quad (value : U64) : U64 := double (double value)

  spec quad (value : U64) where
    requires value * 4 < U64.size;
    ensures result = double (double value)

  verify quad

  -- Vector membership through a reference: the translator's value-level
  -- reading (`vectorContains`), in the version as in the semantics.
  public fun holds (values : &Vector U64) (value : U64) : Action Bool := do
    let element ← &value
    pure (Move.Vector.contains values element)

  public fun holds_twice (values : &Vector U64) (value : U64) : Action Bool := do
    let once ← holds values value
    let again ← holds values value
    pure (once && again)

  spec holds_twice (values : Vector U64) (value : U64) where
    ensures result = holds values value;
    aborts_if False

  verify holds_twice

  -- An element borrow reads the element of the list view.
  public fun first (values : &Vector U64) : Action U64 := do
    let element ← &values[0]
    (*element)

  public fun first_again (values : &Vector U64) : Action U64 := do
    first values

  spec first_again (values : Vector U64) where
    pragma aborts_if_is_partial;
    ensures result = first values

  verify first_again

  -- An abort in value position reads as an unspecified value of the site
  -- (the specification language's partial reading): the contract says
  -- nothing about it, and the rest of the version is exact.
  public fun pick (flag : Bool) (value : U64) : Action U64 := do
    if flag then pure value else abort 7

  public fun pick_again (flag : Bool) (value : U64) : Action U64 := do
    pick flag value

  spec pick_again (flag : Bool) (value : U64) where
    pragma aborts_if_is_partial;
    ensures flag → result = pick flag value

  verify pick_again

  /-! ## Stateful specification functions -/

  -- A standalone one: the balance under an address.
  spec fun balance (addr : Address) : Int := Coin[addr].value

  public fun balance_of (addr : Address) : Action U64 := do
    let value ← &Coin[addr].value
    (*value)

  spec balance_of (addr : Address) where
    ensures result = balance addr;
    aborts_if ¬existsAt<Coin>(addr)

  verify balance_of

  -- The derived version of a global reader is stateful too; versions and
  -- standalone functions compose.
  spec fun is_rich (addr : Address) : Prop := balance_of addr > LIMIT

  public fun rich (addr : Address) : Action Bool := do
    let value ← &Coin[addr].value
    let current ← *value
    pure (current > LIMIT)

  spec rich (addr : Address) where
    ensures result = is_rich addr;
    aborts_if ¬existsAt<Coin>(addr)

  verify rich

  -- `existsAt` in the body is the existence test of the version.
  public fun has_coin (addr : Address) : Action Bool := do
    existsAt Coin addr

  public fun has_coin_too (addr : Address) : Action Bool := do
    has_coin addr

  spec has_coin_too (addr : Address) where
    ensures result = has_coin addr;
    aborts_if False

  verify has_coin_too

  -- `old(f …)` reads the pre-state through a stateful specification function.
  public fun deposit (addr : Address) (amount : U64) : Action Unit := do
    let value ← &mut Coin[addr].value
    value := *value + amount

  spec deposit (addr : Address) (amount : U64) where
    modifies Coin[addr];
    ensures balance_of addr = old(balance_of addr) + amount;
    aborts_if ¬existsAt<Coin>(addr);
    aborts_if ¬old(balance addr).toNat + amount.toNat < U64.size
      with Semantics.Checked.arithmeticAbortCode

  verify deposit

  /-! ## Declared specification versions -/

  -- A native has no body to read: `spec fun` declares its version.
  native fun host_length (values : Vector U64) : U64

  spec fun host_length (values : Vector U64) : Int := Move.Vector.length values

  spec host_length (values : Vector U64) where
    pragma aborts_if_is_partial;
    ensures result = Move.Vector.length values

  fun measure (values : Vector U64) : U64 := host_length values

  spec measure (values : Vector U64) where
    pragma aborts_if_is_partial;
    ensures result = host_length values

  verify measure

  /-! ## Generic specification functions -/

  spec fun holds_in {T} (values : Vector T) (value : T) : Prop :=
    value ∈ values.toList

  public fun push (values : &mut Vector U64) (value : U64) : Action Unit := do
    let current ← *values
    values := current.push value

  spec push (values : &mut Vector U64) (value : U64) where
    requires values.toList.length + 1 < U64.size;
    ensures holds_in values value;
    aborts_if False

  verify push

end Tests.MovePrograms.SpecFunctions
