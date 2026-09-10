--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectSuspendedParent where

  struct Pair has Copy, Drop, Store where
    left : U64
    right : U64

  fun run : Action U64 := do
    let owner : Pair := { left := 1, right := 2 }
    let parent ← &mut owner
    let child ← &mut parent.left
    child := 8
    let parentValue ← *parent
    let _childValue ← *child
    pure parentValue.right

  spec run where
    ensures True
