--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowPermissiveUnused where

  /-- Leaner permits overlapping unactivated handles when the competing
  handle dies unused before the selected handle is activated. -/
  fun run : Action U64 := do
    let owner : U64 := 0
    let selected ← &mut owner
    let _competing ← &mut owner
    selected := 5
    let result ← *selected
    pure result

  spec run where
    ensures True

--# run 0x0::LeanerBorrowPermissiveUnused::run
