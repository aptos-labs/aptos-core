-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import Move

/-!
# Source borrow diagnostics

These tests exercise retained-source extraction, not just the policy core.
-/

namespace Move.Tests.Negative.Borrows

open Move
open scoped Move Move.Spec

/-- Merely creating a competing mutable handle does not create a prophecy. -/
fun discard_competing_handle : Action U64 := do
  let mut owner : U64 := 0
  let selected ← &mut owner
  let _discarded ← &mut owner
  selected := 1
  let result ← *selected
  pure result

spec discard_competing_handle where
  ensures result = 1

verify discard_competing_handle

fun poisoned_use : Action U64 := do
  let mut owner : U64 := 0
  let selected ← &mut owner
  let poisoned ← &mut owner
  selected := 1
  let result ← *poisoned
  pure result

/--
error: borrow safety error `poisoned`: reference was poisoned by an overlapping write (conflicts with `selected`)
-/
#guard_msgs in
spec poisoned_use where
  ensures True

end Move.Tests.Negative.Borrows
