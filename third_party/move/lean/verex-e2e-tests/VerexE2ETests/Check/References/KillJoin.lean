-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A conditional NLL kill joined with a path that keeps the concrete place:
the merged state must remember the concrete place, not only `.external`, so
the write through the original reference while a reborrow of it is live is
a conflict on both paths. Both orderings of the kill branch are tested. -/

-- Kill in the then-branch: the else path keeps the concrete place.
-- After the join, `t` reborrows `*s`; the write `*s := 2` conflicts.
verex namespace kill_join_then where
  fun f(c : Bool, x0 : u32, y0 : u32) -> u32 := do
    let mut x := x0
    let mut y := y0
    let mut s := &mut x
    if c then
      s := &mut y
    else
      ()
    let t := &mut *s
    *s := 2u32
    *t := 1u32
    x
  spec f where
    ensures true

-- Kill in the else-branch: the then path keeps the concrete place.
verex namespace kill_join_else where
  fun f(c : Bool, x0 : u32, y0 : u32) -> u32 := do
    let mut x := x0
    let mut y := y0
    let mut s := &mut x
    if c then
      ()
    else
      s := &mut y
    let t := &mut *s
    *s := 2u32
    *t := 1u32
    x
  spec f where
    ensures true
