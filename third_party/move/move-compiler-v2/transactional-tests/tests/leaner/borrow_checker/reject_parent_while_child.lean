-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectSuspendedParent where
  struct Pair has Copy, Drop, Store where
    left : u64
    right : u64

  fun run() -> u64 := do
    let mut owner := new Pair { left := 1, right := 2 }
    let parent := &mut owner
    let child := &mut parent.left
    *child := 8
    let parentValue := *parent
    let _childValue := *child
    parentValue.right
