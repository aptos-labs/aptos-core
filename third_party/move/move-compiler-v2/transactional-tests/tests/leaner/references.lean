--# publish --print-bytecode

import Move

namespace LeanerTxnReferences

open Move
open scoped Move Move.Compiler

@[move_struct]
structure BalanceValue where
  value : U64
  deriving Copy, Drop, Store

@[move_struct]
structure Balance where
  balance : BalanceValue
  deriving Key

@[move_fun]
def readBalance (addr : Address) : Action U64 := do
  let value ← &Balance[addr].balance.value
  (*value)

@[move_fun]
def addToBalance (addr : Address) (amount : U64) : Action Unit := do
  let value ← &mut Balance[addr].balance.value
  value := *value + amount

@[move_fun]
def deposit (addr : Address) (amount : U64) : Action Unit := do
  addToBalance addr amount

#export_leaner "LeanerReferences" structs [BalanceValue, Balance]
  functions [readBalance, addToBalance, deposit]

end LeanerTxnReferences

--# run 0x0::LeanerReferences::deposit --args @0x42 5u64

--# run 0x0::LeanerReferences::readBalance --args @0x42
