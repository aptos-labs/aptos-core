-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! A loan dying on the path that did not store it into a place: the place holds
another reference there, and the death says nothing about it. -/

verex module 0x42::conditional_holders where
  fun repoint(c : Bool, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let r := &mut x
    let mut s := &mut y
    if c then
      s := r
    else
      ()
    *s := v
    (x, y)
  spec repoint where
    ensures c ==> spec.result[0] == v && spec.result[1] == b
    ensures !c ==> spec.result[0] == a && spec.result[1] == v

  fun repoint_false(c : Bool, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let r := &mut x
    let mut s := &mut y
    if c then
      s := r
    else
      ()
    *s := v
    (x, y)
  spec repoint_false where
    ensures !c ==> b == v -- error: that is the value on entry

  fun repoint_vacuous(c : Bool, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let r := &mut x
    let mut s := &mut y
    if c then
      s := r
    else
      ()
    *s := v
    (x, y)
  spec repoint_vacuous where
    ensures false -- error: the paths are not vacuous

/-! A reference moved out of an aggregate on some paths: what the aggregate still
holds dies on the others. -/

verex namespace conditional_move where
  struct Holder where
    r : &mut u32

  fun f(c : Bool, x0 : u32) -> u32 := do
    let mut x := x0
    let h := new Holder { r := &mut x }
    if c then
      let r := move(h.r)
      *r := 1u32
    else
      ()
    x
  spec f where
    ensures c ==> result == 1
    ensures !c ==> result == x0

  fun f_false(c : Bool, x0 : u32) -> u32 := do
    let mut x := x0
    let h := new Holder { r := &mut x }
    if c then
      let r := move(h.r)
      *r := 1u32
    else
      ()
    x
  spec f_false where
    ensures result == 1 -- error: not on the other path

  fun f_vacuous(c : Bool, x0 : u32) -> u32 := do
    let mut x := x0
    let h := new Holder { r := &mut x }
    if c then
      let r := move(h.r)
      *r := 1u32
    else
      ()
    x
  spec f_vacuous where
    ensures false -- error: the paths are not vacuous

/-! After a condition one branch runs: one branch overwriting a holder says nothing
of the other reading it. -/

verex namespace condition_choice where
  fun bump(r : &mut u64, v : u64) -> Unit := do
    *r := v

  fun f(c : Bool, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    let w := &mut y
    if (do bump(w, b); c) then
      s := w
    else
      *s := 7u64
    *s := v
    (x, y)
  spec f where
    ensures c ==> spec.result[1] == v && spec.result[0] == a
    ensures !c ==> spec.result[0] == v && spec.result[1] == b

  fun f_false(c : Bool, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    let w := &mut y
    if (do bump(w, b); c) then
      s := w
    else
      *s := 7u64
    *s := v
    (x, y)
  spec f_false where
    ensures spec.result[1] == v -- error: not on the other path

  fun f_vacuous(c : Bool, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    let w := &mut y
    if (do bump(w, b); c) then
      s := w
    else
      *s := 7u64
    *s := v
    (x, y)
  spec f_vacuous where
    ensures false -- error: the paths are not vacuous

/-! A failing guard reaches the arms after its own. -/

verex namespace guard_fall_through where
  enum Choice where
    | A (k : u64)
    | B

  fun f(o : Choice, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    match o with
    | Choice::A { k := k } if k > 0u64 =>
      s := &mut y
    | _ =>
      *s := 7u64
    *s := v
    (x, y)
  spec f where
    ensures spec.result[0] == v || spec.result[1] == v

  fun f_false(o : Choice, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    match o with
    | Choice::A { k := k } if k > 0u64 =>
      s := &mut y
    | _ =>
      *s := 7u64
    *s := v
    (x, y)
  spec f_false where
    ensures spec.result[1] == v -- error: not on the other path

  fun f_vacuous(o : Choice, a : u64, b : u64, v : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    match o with
    | Choice::A { k := k } if k > 0u64 =>
      s := &mut y
    | _ =>
      *s := 7u64
    *s := v
    (x, y)
  spec f_vacuous where
    ensures false -- error: the paths are not vacuous

/-! A jump past an overwrite: the path that takes it still reads the holder. -/

verex namespace jump_past_overwrite where
  fun f(c : Bool, n : u8, a : u8, b : u8, v : u8) -> (u8, u8) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    let mut i := 0u8
    while i < n do
      if c then break
      s := &mut y
      i := i + 1u8
    *s := v
    (x, y)
  spec f where
    pragma unroll = 2
    requires n <= 2
    ensures c || n == 0 ==> spec.result[0] == v && spec.result[1] == b
    ensures !c && n > 0 ==> spec.result[0] == a && spec.result[1] == v

  fun f_false(c : Bool, n : u8, a : u8, b : u8, v : u8) -> (u8, u8) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    let mut i := 0u8
    while i < n do
      if c then break
      s := &mut y
      i := i + 1u8
    *s := v
    (x, y)
  spec f_false where
    pragma unroll = 2
    requires n <= 2
    ensures spec.result[1] == v -- error: not on the other path

  fun f_vacuous(c : Bool, n : u8, a : u8, b : u8, v : u8) -> (u8, u8) := do
    let mut x := a
    let mut y := b
    let mut s := &mut x
    let mut i := 0u8
    while i < n do
      if c then break
      s := &mut y
      i := i + 1u8
    *s := v
    (x, y)
  spec f_vacuous where
    pragma unroll = 2
    requires n <= 2
    ensures false -- error: the paths are not vacuous

/-! A local bound inside an operand is bound afresh on the next iteration: reading it
there says nothing of the loan it held, which ends within the operand. -/

verex namespace operand_binding where
  fun add(x : u8, y : u8) -> u8 := x % 100u8 + y % 100u8

  fun f(a : u8, n : u8) -> u8 := do
    let mut acc := a
    let mut count := n
    while count > 0u8 do
      acc := add(
        acc,
        do
          let t := &mut count
          *t := *t - 1u8
          1u8)
    acc
  spec f where
    pragma unroll = 2
    requires n <= 2
    panics_if false
    ensures n == 0 ==> result == a

  fun f_false(a : u8, n : u8) -> u8 := do
    let mut acc := a
    let mut count := n
    while count > 0u8 do
      acc := add(
        acc,
        do
          let t := &mut count
          *t := *t - 1u8
          1u8)
    acc
  spec f_false where
    pragma unroll = 2
    requires n <= 2
    ensures result == a + 1 -- error: not when no iteration runs

  fun f_vacuous(a : u8, n : u8) -> u8 := do
    let mut acc := a
    let mut count := n
    while count > 0u8 do
      acc := add(
        acc,
        do
          let t := &mut count
          *t := *t - 1u8
          1u8)
    acc
  spec f_vacuous where
    pragma unroll = 2
    requires n <= 2
    ensures false -- error: the paths are not vacuous
