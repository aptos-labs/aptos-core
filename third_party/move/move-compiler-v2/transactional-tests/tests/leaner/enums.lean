--# publish

import Move

module LeanerEnums where

  @[move_enum]
  inductive Action where
    | idle
    | transfer (amount : U64)
    | split (left right : U64)
    deriving Copy, Drop, Store

  /-! ## Functions -/

  fun total (action : Action) : U64 :=
    match action with
    | .idle => 0
    | .transfer amount => amount
    | .split left right => left + right

  fun idle_total : U64 := total .idle

  fun transfer_total (amount : U64) : U64 := total (.transfer amount)

  fun split_total (left right : U64) : U64 := total (.split left right)

/-! ## Tests -/

--# run 0x0::LeanerEnums::idle_total

--# run 0x0::LeanerEnums::transfer_total --args 9u64

--# run 0x0::LeanerEnums::split_total --args 4u64 5u64
