--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectGlobalReturn where

  struct Counter has Key where
    value : U64

  fun run (address : Address) : Action (&U64) := do
    &Counter[address].value

  spec run (_address : Address) where
    ensures True
