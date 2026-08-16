--# publish

import Move

move_module LeanerEnums where

  @[move_enum]
  inductive Action where
    | idle
    | transfer (amount : U64)
    | split (left right : U64)
    deriving Copy, Drop, Store

  fun total (action : Action) : U64 :=
    match action with
    | .idle => 0
    | .transfer amount => amount
    | .split left right => left + right

  fun idleTotal : U64 := total .idle

  fun transferTotal (amount : U64) : U64 := total (.transfer amount)

  fun splitTotal (left right : U64) : U64 := total (.split left right)

--# run 0x0::LeanerEnums::idleTotal

--# run 0x0::LeanerEnums::transferTotal --args 9u64

--# run 0x0::LeanerEnums::splitTotal --args 4u64 5u64
