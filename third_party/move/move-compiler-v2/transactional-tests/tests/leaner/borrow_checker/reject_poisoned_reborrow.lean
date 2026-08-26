--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectPoisonedReborrow where

  struct Box has Copy, Drop, Store where
    value : U64

  fun run : Action U64 := do
    let owner : Box := { value := 0 }
    let selected ← &mut owner
    let poisoned ← &mut owner
    selected := { value := 1 }
    let child ← &mut poisoned.value
    let result ← *child
    pure result

  spec run where
    ensures True
