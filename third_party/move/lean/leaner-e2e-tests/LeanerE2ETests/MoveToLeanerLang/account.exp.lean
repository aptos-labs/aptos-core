-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! Balances under an address, with entry functions and contracts. -/
leaner module 0x42::account where
  struct BalanceValue has Copy, Drop, Store where
    value : u64

  /--
  The balance resource.
  -/
  struct Balance has Key where
    balance : BalanceValue

  const E_INSUFFICIENT_BALANCE : u64 := 1

  public entry fun deposit(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    *value := *value + amount

  spec deposit where
    requires exists<Balance>(addr)
    ensures global<Balance>(addr).balance.value
        == old(global<Balance>(addr).balance.value) + amount
    aborts_if global<Balance>(addr).balance.value + amount
        > 18446744073709551615
    modifies global<Balance>(addr)

  public entry fun withdraw(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    let current := *value
    if current < amount then abort(E_INSUFFICIENT_BALANCE)
    *value := *value - amount

  spec withdraw where
    requires exists<Balance>(addr)
    ensures global<Balance>(addr).balance.value
        == old(global<Balance>(addr).balance.value) - amount
    aborts_if global<Balance>(addr).balance.value
        < amount with E_INSUFFICIENT_BALANCE

  public entry fun publish(account : &Signer, amount : u64) -> Unit := do
    move_to<Balance>(
      account, new Balance {
        balance := new BalanceValue { value := amount }
      }
    )

  public fun is_published(addr : Address) -> Bool := exists<Balance>(addr)

  public fun remove(addr : Address) -> u64 := do
    let Balance { balance := balance } := move_from<Balance>(addr)
    return balance.value

  public fun balance_of(addr : Address) -> u64 := Balance[addr].balance.value

  spec balance_of where
    aborts_if !exists<Balance>(addr)
    ensures result == global<Balance>(addr).balance.value
