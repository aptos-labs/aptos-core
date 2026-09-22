-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

open Move
open scoped Move Move.Spec

module BorrowInvalidation where
  struct BorrowedResource has Key where
    value : U64

  fun invalidate_global_owner (addr : Address) : Action U64 := do
    let observation ← &BorrowedResource[addr]
    let removed ← moveFrom BorrowedResource addr
    let _ ← *observation
    pure removed.value

  /--
  error: borrow safety error: cannot move or overwrite an owner while a reference into it is live (conflicts with `observation`)
  -/
  #guard_msgs in
  spec invalidate_global_owner (addr : Address) where
    requires existsAt<BorrowedResource>(addr);
    ensures True
