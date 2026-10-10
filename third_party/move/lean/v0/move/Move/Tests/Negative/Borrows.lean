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

/-- A shadowing value local has a distinct retained-source identity from the
mutable reference whose loan has ended. -/
fun shadowed_mutable_reference : Action U64 := do
  let mut owner : U64 := 0
  let valueRef ← &mut owner
  valueRef := 1
  let valueRef : U64 := 2
  let output := valueRef
  pure output

spec shadowed_mutable_reference where
  ensures result = 2;
  aborts_if False

verify shadowed_mutable_reference

/-- A shadowing initializer still sees the preceding reference; only the new
binder and its following uses receive the fresh identity. -/
fun shadowed_reference_initializer : Action U64 := do
  let mut owner : U64 := 0
  let valueRef ← &mut owner
  let valueRef ← *valueRef
  let output := valueRef
  pure output

spec shadowed_reference_initializer where
  ensures result = 0;
  aborts_if False

verify shadowed_reference_initializer

/-- A branch-local shadow does not capture uses of the outer reference after
the branch rejoins. -/
fun nested_shadowed_reference (takeBranch : Bool) : Action U64 := do
  let mut owner : U64 := 0
  let valueRef ← &mut owner
  valueRef := 1
  if takeBranch then
    let valueRef : U64 := 2
    let _ignored := valueRef
  let output ← *valueRef
  pure output

spec nested_shadowed_reference (takeBranch : Bool) where
  ensures result = 1;
  aborts_if False

verify nested_shadowed_reference

/-- Pattern binders participate in the same lexical alpha-renaming. -/
fun pattern_shadowed_reference : Action U64 := do
  let mut owner : U64 := 0
  let valueRef ← &mut owner
  valueRef := 1
  let pair := (2, 3)
  let (valueRef, other) := pair
  pure (valueRef + other)

spec pattern_shadowed_reference where
  ensures result = 5;
  aborts_if False

verify pattern_shadowed_reference

end Move.Tests.Negative.Borrows
