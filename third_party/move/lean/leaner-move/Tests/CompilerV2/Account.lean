-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerMove

/-! A verified Leaner Move module compiled through compiler-v2. -/

leaner module 0x42::Account where
  struct Account has Key where
    balance : u64

  public entry fun open(account : &Signer) -> Unit :=
    move_to<Account>(account, new Account { balance := 0 })

  public fun deposit(addr : Address, amount : u64) -> Unit := do
    let balance := &mut Account[addr].balance
    *balance := *balance + amount
  spec deposit where
    requires exists<Account>(addr)
    modifies global<Account>(addr)
    ensures global<Account>(addr).balance == old(global<Account>(addr).balance) + amount
    aborts_if old(global<Account>(addr).balance) + amount > MAX_U64

  public fun balance(addr : Address) -> u64 := do
    let balance := &Account[addr].balance
    *balance
  spec balance where
    requires exists<Account>(addr)
    ensures result == old(global<Account>(addr).balance)
    aborts_if false
