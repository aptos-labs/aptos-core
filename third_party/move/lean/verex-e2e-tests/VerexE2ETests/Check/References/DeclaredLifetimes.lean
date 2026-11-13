-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Declared lifetimes, in the Rust profile: a result holds the loans of the
parameters whose lifetimes outlive its own, so the other lenders are free while it
lives. Every positive case has a false twin, marked `-- error:`, which must fail.
What may not be accepted is in `DeclaredLifetimeErrors`. -/

verex namespace declared_lifetimes where
  struct R where
    x : u32
    y : u32

  struct Pair {a : lifetime} {b : lifetime} where
    x : &[a] u32
    y : &[b] mut u32

  fun sel {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 :=
    &mut *x

  -- ## The unrelated lender is free

  fun write_other(mut a : u32, mut b : u32) -> R := do
    let r := sel(&mut a, &mut b)
    b := 2u32
    *r := 1u32
    new R { x := a, y := b }
  spec write_other where
    ensures result.x == 1 && result.y == 2

  fun write_other_false(mut a : u32, mut b : u32) -> R := do
    let r := sel(&mut a, &mut b)
    b := 2u32
    *r := 1u32
    new R { x := a, y := b }
  spec write_other_false where
    ensures result.x == 2 -- error: the result writes `a`

  fun sel_written {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 := do
    *y := 7u32
    &mut *x

  -- The other parameter is written by the callee, then the lender read.
  fun read_other(mut a : u32, mut b : u32) -> R := do
    let r := sel_written(&mut a, &mut b)
    let seen := b
    *r := seen
    new R { x := a, y := b }
  spec read_other where
    ensures result.x == 7 && result.y == 7

  fun read_other_false(mut a : u32, mut b : u32) -> R := do
    let r := sel_written(&mut a, &mut b)
    let seen := b
    *r := seen
    new R { x := a, y := b }
  spec read_other_false where
    ensures result.y == b -- error: the callee wrote `b`

  -- ## Aggregates, by the paths their lifetimes are at

  fun take_y {a : lifetime} {b : lifetime}(p : Pair<lifetime a, lifetime b>) -> &[b] mut u32 :=
    move(p.y)

  fun pair_write_shared_lender(mut a : u32, mut b : u32) -> R := do
    let r := take_y(new Pair<lifetime _, lifetime _> { x := &a, y := &mut b })
    a := 1u32
    *r := 2u32
    new R { x := a, y := b }
  spec pair_write_shared_lender where
    ensures result.x == 1 && result.y == 2

  fun pair_write_shared_lender_false(mut a : u32, mut b : u32) -> R := do
    let r := take_y(new Pair<lifetime _, lifetime _> { x := &a, y := &mut b })
    a := 1u32
    *r := 2u32
    new R { x := a, y := b }
  spec pair_write_shared_lender_false where
    ensures result.y == 1 -- error: the reference writes 2

  fun both {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> (&[a] mut u32, &[b] mut u32) :=
    (&mut *x, &mut *y)

  -- ## One lifetime ties both parameters to the result

  fun pick {a : lifetime}(c : Bool, x : &[a] mut u32, y : &[a] mut u32) -> &[a] mut u32 := do
    if c then &mut *x else &mut *y

  fun pick_write(c : Bool, mut a : u32, mut b : u32) -> R := do
    let r := pick(c, &mut a, &mut b)
    *r := 1u32
    new R { x := a, y := b }
  spec pick_write where
    ensures c ==> result.x == 1 && result.y == b
    ensures !c ==> result.y == 1 && result.x == a

  fun pick_write_false(c : Bool, mut a : u32, mut b : u32) -> R := do
    let r := pick(c, &mut a, &mut b)
    *r := 1u32
    new R { x := a, y := b }
  spec pick_write_false where
    ensures result.x == 1 -- error: not when `!c`

  -- ## A bound the signature implies

  fun implied {s : lifetime} {l : lifetime}(x : &[l] u32, y : &[s] &[l] u32) -> &[s] u32 := x
  spec implied where
    ensures result == x

  fun implied_false {s : lifetime} {l : lifetime}(x : &[l] u32, y : &[s] &[l] u32) -> &[s] u32 := x
  spec implied_false where
    ensures result == x + 1 -- error: it is `x`
