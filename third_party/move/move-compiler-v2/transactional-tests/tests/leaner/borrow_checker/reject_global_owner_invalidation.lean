--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectGlobalOwner where

  struct Counter has Key where
    value : U64

  fun run (address : Address) : Action U64 := do
    let observation ← &Counter[address].value
    let removed ← moveFrom Counter address
    let _observed ← *observation
    pure removed.value

  spec run (_address : Address) where
    ensures True
