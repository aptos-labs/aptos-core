--# publish

import Move

open Move
open scoped Move Move.Spec

module LeanerBorrowRejectNestedPoison where

  struct Inner has Copy, Drop, Store where
    value : U64

  struct Outer has Copy, Drop, Store where
    inner : Inner

  fun run : Action U64 := do
    let owner : Outer := { inner := { value := 0 } }
    let outer ← &mut owner
    let inner ← &mut outer.inner
    let field ← &mut inner.value
    let whole ← &mut owner
    whole := { inner := { value := 1 } }
    let result ← *field
    pure result

  spec run where
    ensures True
