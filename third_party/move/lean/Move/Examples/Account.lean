-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move.Attributes
import Move.Syntax

/-! The golden Lean-authored Move contract used to pin the prototype DevX. -/

namespace Move.Examples.Account

open Move
open scoped Move

@[move_struct]
structure BalanceValue where
  value : U64
  deriving Copy, Drop, Store

@[move_struct]
structure Balance where
  balance : BalanceValue
  deriving Key

def E_INSUFFICIENT_BALANCE : U64 := 1

@[move_entry]
def deposit (addr : Address) (amount : U64) : Action Unit := do
  let value ← &mut Balance[addr].balance.value
  value := *value + amount

@[move_entry]
def withdraw (addr : Address) (amount : U64) : Action Unit := do
  let value ← &mut Balance[addr].balance.value
  let old ← *value
  if old < amount then
    abort E_INSUFFICIENT_BALANCE
  value := *value - amount

end Move.Examples.Account
