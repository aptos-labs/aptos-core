-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang
import LeanerE2ETests.CheckSupport

/-! A generic specification function reading global memory, expanded at a
call: it reads the resources of the call's type arguments, before and
after. A generic caller calls a generic function twice at its own type
parameter, over a resource with a phantom type parameter. -/

namespace LeanerLang.Tests.Check.Storage.GenericSpecReads

leaner module 0x42::generic_spec_reads where
  struct Coin {T : phantom type} has Key where
    value : u64

  struct USD where
    dummy_field : Bool

  struct EUR where
    dummy_field : Bool

  spec fun increased {T}(addr : Address) : Bool :=
    old(global<Coin<T> >(addr).value) < global<Coin<T> >(addr).value

  fun increment_usd(addr : Address) -> Unit :=
    Coin<USD>[addr].value := Coin<USD>[addr].value + 1
  spec increment_usd where
    ensures increased::<USD>(addr)
    modifies *

  fun increment_usd_claim_eur(addr : Address) -> Unit :=
    Coin<USD>[addr].value := Coin<USD>[addr].value + 1
  spec increment_usd_claim_eur where
    ensures increased::<EUR>(addr) -- error: the EUR coin is unchanged
    modifies *

  fun deposit {C}(addr : Address, amount : u64) -> Unit := do
    let coin := &mut Coin<C>[addr].value
    *coin := *coin + amount
  spec deposit where
    aborts_if !exists<Coin<C> >(addr)
    aborts_if global<Coin<C> >(addr).value + amount > MAX_U64
    ensures global<Coin<C> >(addr).value == old(global<Coin<C> >(addr).value) + amount
    modifies global<Coin<C> >(addr)

  -- A generic caller at its own type parameter, reading in the second call
  -- what the first wrote.
  fun deposit_twice {C}(addr : Address, amount : u64) -> Unit := do
    core.call deposit::<C>(addr, amount)
    core.call deposit::<C>(addr, amount)
  spec deposit_twice where
    aborts_if !exists<Coin<C> >(addr)
    aborts_if global<Coin<C> >(addr).value + 2 * amount > MAX_U64
    ensures global<Coin<C> >(addr).value == old(global<Coin<C> >(addr).value) + 2 * amount
    modifies global<Coin<C> >(addr)

end LeanerLang.Tests.Check.Storage.GenericSpecReads
