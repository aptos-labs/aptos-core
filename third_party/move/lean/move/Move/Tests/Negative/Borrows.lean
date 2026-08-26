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

/-- The source-spec encoding must not confuse a shadowing local with the
still-live mutable reference.  Rejecting this unsupported form is sound: it
prevents a later owner refresh from reading the shadow instead of the loan. -/
fun shadowed_mutable_reference : Action U64 := do
  let mut owner : U64 := 0
  let valueRef ← &mut owner
  valueRef := 1
  let valueRef : U64 := 2
  let output := valueRef
  pure output

/--
error: automatic source specifications do not support shadowing a mutable-reference local
-/
#guard_msgs in
spec shadowed_mutable_reference where
  ensures result = 2

/-- The initializer of a shadowing binding is still in the old reference's
scope, so it must be rejected as well. -/
fun shadowed_reference_initializer : Action U64 := do
  let mut owner : U64 := 0
  let valueRef ← &mut owner
  let valueRef ← *valueRef
  let output := valueRef
  pure output

/--
error: automatic source specifications do not support shadowing a mutable-reference local
-/
#guard_msgs in
spec shadowed_reference_initializer where
  ensures result = 0

end Move.Tests.Negative.Borrows
