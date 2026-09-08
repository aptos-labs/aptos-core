-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0
-- Test category: expected failures and diagnostics.

import Move

/-! Diagnostics for invalid Move surface control forms. These constructs are
rejected before an IR can be exported. -/

open Move
open scoped Move

namespace Tests.Negative.Surface

/--
error: `break` must be nested inside a loop
-/
#guard_msgs in
def bareBreak (n : U64) : U64 := Id.run do
  break
  n

/--
error: `continue` must be nested inside a loop
-/
#guard_msgs in
def bareContinue (n : U64) : U64 := Id.run do
  continue
  n

/--
error: `continue f` cannot appear inside `loop` / `while`; use `continue`
-/
#guard_msgs in
partial def continueCallInside (n : U64) : U64 := Id.run do
  let mut n := n
  while 0 < n do
    continue continueCallInside (n - 1)
  n

/--
error: unknown loop label `missing`
-/
#guard_msgs in
def unknownLabel (n : U64) : U64 := Id.run do
  loop@outer
    break@missing
  n

/--
error: duplicate active loop label `outer`
-/
#guard_msgs in
def duplicateLabel (n : U64) : U64 := Id.run do
  loop@outer
    loop@outer
      break
  n

/--
error: Unknown identifier `Move.loopEnter`
-/
#guard_msgs in
def forgedLoopMarker : Nat :=
  Move.loopEnter 0 0 0 ()

end Tests.Negative.Surface
