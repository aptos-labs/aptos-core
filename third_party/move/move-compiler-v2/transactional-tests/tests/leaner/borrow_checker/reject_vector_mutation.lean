-- Copyright © Aptos Foundation

--# publish

import LeanerMove

leaner module 0x0::LeanerBorrowRejectVectorMutation where
  -- The write through `vectorRef` conflicts with the element borrow
  -- `observation`, which is still used.
  fun run() -> u64 := do
    let mut values := vector<u64>[1, 2]
    let vectorRef := &mut values
    let observation := &vectorRef[0]
    *vectorRef := core.prim.insertVector(*vectorRef, 2, 3)
    let result := *observation
    result
