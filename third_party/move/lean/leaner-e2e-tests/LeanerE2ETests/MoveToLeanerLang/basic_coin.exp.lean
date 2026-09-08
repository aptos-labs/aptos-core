-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A minimal coin module exercising declarations, bodies, and specs. -/
leaner module 0x42::basic_coin where
  -- An ordinary comment before the struct.
  /--
  The coin resource.
  -/
  struct Coin has Key where
    value : u64

  const E_INSUFFICIENT : u64 := 1

  -- trailing comment
  /--
  Withdraws `amount` from `addr`.
  -/
  public fun withdraw(addr : Address, amount : u64) -> Unit := do
    let balance := &mut Coin[addr].value
    assert!(*balance >= amount, E_INSUFFICIENT)
    *balance := *balance - amount

  spec withdraw where
    pragma aborts_if_is_partial
    aborts_if !exists<Coin>(addr)
    aborts_if global<Coin>(addr).value < amount with E_INSUFFICIENT
    ensures global<Coin>(addr).value == old(global<Coin>(addr).value) - amount

  -- block comment
  public fun balance_of(addr : Address) -> u64 := Coin[addr].value

  spec balance_of where
    aborts_if !exists<Coin>(addr)
    ensures result == global<Coin>(addr).value

  spec fun total(a : Address, b : Address) : Int :=
    global<Coin>(a).value + global<Coin>(b).value
