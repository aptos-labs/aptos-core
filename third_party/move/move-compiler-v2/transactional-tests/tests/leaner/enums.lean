-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerEnums where
  enum Action has Copy, Drop, Store where
    | Idle
    | Transfer (amount : u64)
    | Split (left : u64, right : u64)

  fun total(action : Action) -> u64 :=
    match action with
      | Action::Idle {} => 0
      | Action::Transfer { amount := amount } => amount
      | Action::Split { left := left, right := right } => left + right

  fun idle_total() -> u64 := total(new Action::Idle {})

  fun transfer_total(amount : u64) -> u64 := total(new Action::Transfer { amount })

  fun split_total(left : u64, right : u64) -> u64 := total(new Action::Split { left, right })

--# run 0x0::LeanerEnums::idle_total

--# run 0x0::LeanerEnums::transfer_total --args 9u64

--# run 0x0::LeanerEnums::split_total --args 4u64 5u64
