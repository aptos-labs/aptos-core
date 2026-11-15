-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! What a shared view of a holder does not allow, each rejected: a shared view holds
no loan, so nothing can be taken out of it or lent through it, and it is taken only of
a holder whose references are live. -/

-- A mutable reference moved out of a shared view.
verex namespace move_out where
  struct Holder where
    r : &mut u32

  fun f(h : &Holder) -> u32 := do
    let r := move((*h).r)
    *r

  spec f where
    ensures true

-- A mutable borrow through a shared view.
verex namespace borrow_through where
  struct Holder where
    r : &mut u32

  fun f(h : &Holder) -> Unit := do
    let r := &mut *((*h).r)
    *r := 1u32

  spec f where
    ensures true

-- A write through a copy of the reference a shared view holds.
verex namespace write_through_copy where
  struct Holder where
    r : &mut u32

  fun f(h : &Holder) -> Unit := do
    let r := copy((*h).r)
    *r := 1u32
  spec f where
    ensures true

-- A shared view of a holder after a write of the lender: the view is a use of the
-- holder, so the analysis keeps its loan live over the write.
verex namespace after_death where
  struct Holder where
    r : &mut u32

  fun f(x0 : u32) -> u32 := do
    let mut x := x0
    let h := new Holder { r := &mut x }
    x := 1u32
    let v := &h
    *((*v).r)

  spec f where
    ensures true

-- A shared view of a holder while a field's own loan is live.
verex namespace during_field_loan where
  struct Pair where
    a : u32
    b : u32

  fun f(x0 : u32) -> u32 := do
    let mut p := new Pair { a := x0, b := x0 }
    let r := &mut p.a
    let v := &p
    *r := v.b
    p.a

  spec f where
    ensures true

-- A shared view of a holder a reference was moved out of: the move is a reborrow
-- through the holder, live while the moved reference is.
verex namespace partially_moved where
  struct Holder where
    r : &mut u32

  fun f(x0 : u32) -> u32 := do
    let mut x := x0
    let h := new Holder { r := &mut x }
    let r := move(h.r)
    let v := &h
    *r := 1u32
    *((*v).r)
  spec f where
    ensures true

-- The prophecy of a reference through a shared view: a view holds what is current.
verex namespace final_through_view where
  struct Holder where
    r : &mut u32

  fun f(h : &Holder) -> u32 := *((*h).r)
  spec f where
    ensures final(h.r) == result

-- A returned value holding a loan that died: no residue of a retired borrow leaves a
-- function, of a local loan or of a reborrow of an argument.
verex namespace returned_residue where
  struct Holder where
    r : &mut u32

  fun f(x0 : u32) -> (u32, Holder) := do
    let mut x := x0
    let h := new Holder { r := &mut x }
    x := 1u32
    (x, h)
  spec f where
    ensures true

verex namespace returned_residue_of_argument where
  struct Holder where
    r : &mut u32

  fun f(p : &mut u32) -> (u32, Holder) := do
    let h := new Holder { r := &mut *p }
    *p := 1u32
    (0u32, h)
  spec f where
    ensures true
