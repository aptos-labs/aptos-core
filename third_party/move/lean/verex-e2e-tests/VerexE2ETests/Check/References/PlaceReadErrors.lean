-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexE2ETests.CheckSupport

/-!
# Place borrows no local holds, in conflict

A mutable borrow of a place that no local holds is read as a shared borrow
only after the borrow analysis checked it as a mutable one: a live shared
loan of the place still rejects it.
-/

namespace VerexSL.Tests.Check.References.PlaceReadErrors

verex module 0x42::place_read_errors where
  struct Cell has Copy, Drop where
    value : u64

  fun discarded_while_shared(c : Cell) -> u64 := do
    let mut d := c
    let s := &d.value
    let _ := &mut d.value
    *s
  spec discarded_while_shared where
    ensures result == c.value

  fun read_while_shared(c : Cell) -> u64 := do
    let mut d := c
    let s := &d
    let v := (&mut d).value
    v + s.value
  spec read_while_shared where
    pragma verify = false

end VerexSL.Tests.Check.References.PlaceReadErrors
