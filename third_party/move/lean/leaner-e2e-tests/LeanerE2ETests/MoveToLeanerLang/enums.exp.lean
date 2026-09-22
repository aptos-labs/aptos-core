-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import LeanerLang

leaner module 0x42::enums where
  enum Action has Copy, Drop, Store where
    | Idle
    | Transfer (amount : u64)
    | Split (left : u64, right : u64)

  fun total(action : Action) -> u64 :=
    if action is Idle then 0
    else
      if action is Transfer then action.amount else action.left + action.right

  spec total where
    ensures result
        == (match action with
          | Action::Idle {} => 0
          | Action::Transfer { amount := amount } => amount
          | Action::Split { left := left, right := right } => left + right)

  fun classify(action : Action) -> u64 :=
    if action is Idle then 0
    else
      if action is Transfer then 1 else 2

  fun is_transfer(action : Action) -> Bool := action is Transfer

  fun guarded(value : u64) -> u64 := do
    let «_$disc» := value
    return if «_$disc» == 0 then 10
    else
      if «_$disc» >= 1 && «_$disc» < 4 && value != 2 then 20
      else
        if «_$disc» >= 4 && «_$disc» <= 6 then 30 else 40

  fun make(flag : Bool, amount : u64) -> Action :=
    if flag then new Action::Transfer { amount } else new Action::Idle {}
