-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! What the denotation of references held inside aggregates does not carry yet,
each rejected by name rather than given a meaning. -/

-- A value holding a mutable reference moved out of an aggregate.
verex namespace nested_move where
  struct Holder where
    r : &mut u32

  struct Outer where
    inner : Holder

  fun f(mut x : u32) -> u32 := do
    let o := new Outer { inner := new Holder { r := &mut x } }
    let h := move(o.inner)
    *h.r := 1u32
    x
  spec f where
    ensures result == 1

-- A write of a lender a local may still hold, on the path it was moved and rewritten.
verex namespace rewritten_on_a_path where
  struct Holder where
    r : &mut u32

  fun f(c : Bool, mut a : u32, mut b : u32, mut d : u32) -> u32 := do
    let mut k := new Holder { r := &mut a }
    let mut h := new Holder { r := &mut d }
    if c then
      h := k
      k := new Holder { r := &mut b }
    *h.r := 1u32
    a := 3u32
    *k.r := 2u32
    a
  spec f where
    ensures true

-- A reference overwritten in a parameter: no loan of the frame ends there.
verex namespace overwritten_parameter where
  struct Holder where
    r : &mut u32

  fun f(mut h : Holder, y : &mut u32) -> Unit := do
    h.r := &mut *y
  spec f where
    ensures true

-- A reference overwritten through a reference.
verex namespace overwritten_through where
  struct Holder where
    r : &mut u32

  fun f(h : &mut Holder, y : &mut u32) -> Unit := do
    h.r := &mut *y
  spec f where
    ensures true

-- A generic function instantiated at a mutable reference: it verifies at the rows.
verex namespace generic_call where
  fun id {T}(value : T) -> T :=
    value

  fun f(mut x : u32) -> u32 := do
    let r := id(&mut x)
    *r := 1u32
    x
  spec f where
    ensures result == 1

-- A drop of a reference the caller lends: it has no death here.
verex namespace dropped_reference where
  fun drop_reference(reference : &mut u32) -> Unit := do
    drop(reference)
  spec drop_reference where
    ensures true

-- A drop of a holder the caller passes by value.
verex namespace dropped_holder where
  struct Holder where
    r : &mut u32

  fun drop_holder(h : Holder) -> Unit := do
    drop(h)
  spec drop_holder where
    ensures true

-- A holder overwritten while its loan lives on through another: the loan's death
-- A reference the caller lends, overwritten through a local it moved to.
verex namespace overwritten_via_local where
  struct Holder where
    r : &mut u32

  fun f(h : Holder, y : &mut u32) -> Holder := do
    let mut k := h
    k.r := &mut *y
    k
  spec f where
    ensures true
