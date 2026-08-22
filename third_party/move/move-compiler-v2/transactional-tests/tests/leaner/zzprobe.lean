--# publish

import Move

open scoped Move

module ZZProbe where

  struct Vault has Key where
    balance : U64

  @[move_public]
  fun publish (signer : &Signer) (amount : U64) : Action Unit :=
    moveTo signer ({ balance := amount } : Vault)

  @[move_public]
  fun read (addr : Address) : Action U64 := do
    let value ← &Vault[addr].balance
    (*value)

--# run --signers 0x42 --args 10u64 -- 0x0::ZZProbe::publish

--# run 0x0::ZZProbe::read --args @0x42
