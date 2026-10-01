-- Copyright © Aptos Foundation

--# publish --print-bytecode

import LeanerMove

leaner module 0x0::LeanerReferences where
  struct BalanceValue has Copy, Drop, Store where
    value : u64

  struct Balance has Key where
    balance : BalanceValue

  fun read_balance(addr : Address) -> u64 := do
    let value := &Balance[addr].balance.value
    *value

  fun add_to_balance(addr : Address, amount : u64) -> Unit := do
    let value := &mut Balance[addr].balance.value
    *value := *value + amount

  fun deposit(addr : Address, amount : u64) -> Unit := add_to_balance(addr, amount)

--# run 0x0::LeanerReferences::deposit --args @0x42 5u64

--# run 0x0::LeanerReferences::read_balance --args @0x42
