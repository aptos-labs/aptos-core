-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

/-! A nested-resource account: a field-focused deposit and a guarded
withdrawal. -/

leaner module 0x42::account where
  struct BalanceValue has Copy, Drop, Store where
    value : u64

  struct Balance has Key where
    balance : BalanceValue

  const E_INSUFFICIENT_BALANCE : u64 := 1

  public entry fun deposit(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    *value := *value + amount
  spec deposit where
    requires exists<Balance>(addr)
    modifies global<Balance>(addr)
    ensures global<Balance>(addr).balance.value
        == old(global<Balance>(addr).balance.value) + amount
    aborts_if old(global<Balance>(addr).balance.value) + amount
        > MAX_U64

  public entry fun withdraw(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    let current := *value
    if current < amount then abort(E_INSUFFICIENT_BALANCE)
    *value := *value - amount
  spec withdraw where
    requires exists<Balance>(addr)
    modifies global<Balance>(addr)
    ensures global<Balance>(addr).balance.value
        == old(global<Balance>(addr).balance.value) - amount
    aborts_if old(global<Balance>(addr).balance.value) < amount
        with E_INSUFFICIENT_BALANCE

