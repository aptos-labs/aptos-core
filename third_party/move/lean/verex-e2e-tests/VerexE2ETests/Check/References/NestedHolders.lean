-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Mutable references held inside aggregates, in the Rust profile: a tuple a
call returns, then a struct holding one by value. A loan dies at its path in
its holder, and a reference taken out of an aggregate is a reborrow through it,
so nothing resolves a stale copy: every positive case has a false twin, marked
`-- error:`, which must fail. -/

verex namespace nested_holders where
  struct R where
    x : u32
    y : u32

  struct Holder where
    r : &mut u32

  struct Two where
    a : &mut u32
    b : &mut u32

  -- ## A tuple a call returns

  fun both(a : &mut u32, b : &mut u32) -> (&mut u32, &mut u32) :=
    (&mut *a, &mut *b)

  fun swp(a : &mut u32, b : &mut u32) -> (&mut u32, &mut u32) :=
    (&mut *b, &mut *a)

  fun cond_swp(c : Bool, a : &mut u32, b : &mut u32) -> (&mut u32, &mut u32) := do
    if c then
      (&mut *b, &mut *a)
    else
      (&mut *a, &mut *b)

  fun write_first(a : &mut u32, b : &mut u32) -> (&mut u32, &mut u32) := do
    *a := 5u32
    *b := 6u32
    (&mut *a, &mut *b)

  fun mixed(a : &mut u32) -> (&mut u32, u32) := do
    let old_value := *a
    (&mut *a, old_value)

  fun ret_destruct(mut a : u32, mut b : u32) -> R := do
    let t := both(&mut a, &mut b)
    let p := move(t[0u32])
    let q := move(t[1u32])
    *p := 1u32
    *q := 2u32
    new R { x := a, y := b }
  spec ret_destruct where
    ensures result.x == 1 && result.y == 2

  fun ret_destruct_false(mut a : u32, mut b : u32) -> R := do
    let t := both(&mut a, &mut b)
    let p := move(t[0u32])
    let q := move(t[1u32])
    *p := 1u32
    *q := 2u32
    new R { x := a, y := b }
  spec ret_destruct_false where
    ensures result.x == 2 -- error: `p` writes `a`

  fun ret_swap(mut a : u32, mut b : u32) -> R := do
    let t := swp(&mut a, &mut b)
    let p := move(t[0u32])
    let q := move(t[1u32])
    *p := 1u32
    *q := 2u32
    new R { x := a, y := b }
  spec ret_swap where
    ensures result.x == 2 && result.y == 1

  fun ret_swap_false(mut a : u32, mut b : u32) -> R := do
    let t := swp(&mut a, &mut b)
    let p := move(t[0u32])
    let q := move(t[1u32])
    *p := 1u32
    *q := 2u32
    new R { x := a, y := b }
  spec ret_swap_false where
    ensures result.x == 1 -- error: the callee swapped them

  fun ret_cond_swap(c : Bool, mut a : u32, mut b : u32) -> R := do
    let t := cond_swp(c, &mut a, &mut b)
    let p := move(t[0u32])
    let q := move(t[1u32])
    *p := 1u32
    *q := 2u32
    new R { x := a, y := b }
  spec ret_cond_swap where
    ensures c ==> result.x == 2 && result.y == 1
    ensures !c ==> result.x == 1 && result.y == 2

  fun ret_cond_swap_false(c : Bool, mut a : u32, mut b : u32) -> R := do
    let t := cond_swp(c, &mut a, &mut b)
    let p := move(t[0u32])
    let q := move(t[1u32])
    *p := 1u32
    *q := 2u32
    new R { x := a, y := b }
  spec ret_cond_swap_false where
    ensures result.x == 1 -- error: not where `c` holds

  -- A component left in the tuple dies there, keeping what the callee wrote.
  fun ret_write_first(mut a : u32, mut b : u32) -> R := do
    let t := write_first(&mut a, &mut b)
    let p := move(t[0u32])
    *p := 1u32
    new R { x := a, y := b }
  spec ret_write_first where
    ensures result.x == 1 && result.y == 6

  fun ret_write_first_false(mut a : u32, mut b : u32) -> R := do
    let t := write_first(&mut a, &mut b)
    let p := move(t[0u32])
    *p := 1u32
    new R { x := a, y := b }
  spec ret_write_first_false where
    ensures result.y == b -- error: the callee wrote `b`

  fun ret_mixed(mut a : u32) -> R := do
    let t := mixed(&mut a)
    let p := move(t[0u32])
    let o := copy(t[1u32])
    *p := 1u32
    new R { x := a, y := o }
  spec ret_mixed where
    ensures result.x == 1 && result.y == a

  fun ret_mixed_false(mut a : u32) -> R := do
    let t := mixed(&mut a)
    let p := move(t[0u32])
    let o := copy(t[1u32])
    *p := 1u32
    new R { x := a, y := o }
  spec ret_mixed_false where
    ensures result.x == a -- error: `p` wrote `a`

  -- ## A struct holding one by value

  -- Constructed, written through, and read after it dies.
  fun hold_write_read(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 5u32
    x
  spec hold_write_read where
    ensures result == 5

  fun hold_write_read_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 5u32
    x
  spec hold_write_read_false where
    ensures result == x -- error: the holder wrote `x`

  fun hold_param(x : &mut u32) -> Unit := do
    let h := new Holder { r := &mut *x }
    *h.r := 3u32
  spec hold_param where
    ensures x == 3

  fun hold_param_false(x : &mut u32) -> Unit := do
    let h := new Holder { r := &mut *x }
    *h.r := 3u32
  spec hold_param_false where
    ensures x == old(x) -- error: the holder wrote `x`

  -- A reference moved out: the aggregate's component stays true.
  fun hold_move_out(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let r := move(h.r)
    *r := 5u32
    x
  spec hold_move_out where
    ensures result == 5

  fun hold_move_out_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let r := move(h.r)
    *r := 5u32
    x
  spec hold_move_out_false where
    ensures result == x -- error: a stale copy resolved would prove this

  -- A vacuous path would prove anything: this holds of no run.
  fun hold_move_out_vacuous(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let r := move(h.r)
    *r := 5u32
    x
  spec hold_move_out_vacuous where
    ensures false -- error: the path is not vacuous

  -- A drop denotes nothing; the loans the value holds die there.
  fun hold_drop(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 4u32
    drop(h)
    x
  spec hold_drop where
    ensures result == 4

  fun hold_drop_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 4u32
    drop(h)
    x
  spec hold_drop_false where
    ensures result == 5 -- error: it wrote 4

  -- Passed to a callee by value, whose clauses read through the holder.
  fun store(h : Holder) -> Unit := do
    *h.r := 7u32
  spec store where
    ensures h.r == 7

  fun store_false(h : Holder) -> Unit := do
    *h.r := 7u32
  spec store_false where
    ensures h.r == old(h).r -- error: it wrote 7

  fun hold_callee_value(mut x : u32) -> u32 := do
    store(new Holder { r := &mut x })
    x
  spec hold_callee_value where
    ensures result == 7

  fun hold_callee_value_false(mut x : u32) -> u32 := do
    store(new Holder { r := &mut x })
    x
  spec hold_callee_value_false where
    ensures result == x -- error: the callee wrote it

  -- Passed by `&mut`, and the holder written after it comes back.
  fun store_mut(h : &mut Holder) -> Unit := do
    *h.r := 7u32
  spec store_mut where
    ensures h.r == 7

  fun store_mut_false(h : &mut Holder) -> Unit := do
    *h.r := 7u32
  spec store_mut_false where
    ensures h.r == 8 -- error: it wrote 7

  fun hold_callee_mut(mut x : u32) -> u32 := do
    let mut h := new Holder { r := &mut x }
    store_mut(&mut h)
    x
  spec hold_callee_mut where
    ensures result == 7

  fun hold_callee_mut_false(mut x : u32) -> u32 := do
    let mut h := new Holder { r := &mut x }
    store_mut(&mut h)
    x
  spec hold_callee_mut_false where
    ensures result == x -- error: the callee wrote it

  fun hold_callee_mut_after(mut x : u32) -> u32 := do
    let mut h := new Holder { r := &mut x }
    store_mut(&mut h)
    *h.r := 8u32
    x
  spec hold_callee_mut_after where
    ensures result == 8

  fun hold_callee_mut_after_false(mut x : u32) -> u32 := do
    let mut h := new Holder { r := &mut x }
    store_mut(&mut h)
    *h.r := 8u32
    x
  spec hold_callee_mut_after_false where
    ensures result == 7 -- error: the later write wins

  -- Returned from a callee.
  fun mk(x : &mut u32) -> Holder :=
    new Holder { r := &mut *x }

  fun hold_return(mut x : u32) -> u32 := do
    let h := mk(&mut x)
    *h.r := 9u32
    x
  spec hold_return where
    ensures result == 9

  fun hold_return_false(mut x : u32) -> u32 := do
    let h := mk(&mut x)
    *h.r := 9u32
    x
  spec hold_return_false where
    ensures result == x -- error: written through the returned holder

  -- A reference taken out of a by-value aggregate parameter and returned.
  fun take(h : Holder) -> &mut u32 :=
    &mut *h.r

  fun hold_take(mut x : u32) -> u32 := do
    let r := take(new Holder { r := &mut x })
    *r := 2u32
    x
  spec hold_take where
    ensures result == 2

  fun hold_take_false(mut x : u32) -> u32 := do
    let r := take(new Holder { r := &mut x })
    *r := 2u32
    x
  spec hold_take_false where
    ensures result == x -- error: written through the taken reference

  fun take_moved(h : Holder) -> &mut u32 :=
    move(h.r)

  fun hold_take_moved(mut x : u32) -> u32 := do
    let r := take_moved(new Holder { r := &mut x })
    *r := 2u32
    x
  spec hold_take_moved where
    ensures result == 2

  fun hold_take_moved_false(mut x : u32) -> u32 := do
    let r := take_moved(new Holder { r := &mut x })
    *r := 2u32
    x
  spec hold_take_moved_false where
    ensures result == x -- error: written through the taken reference

  fun tup_take(t : (&mut u32, u32)) -> &mut u32 :=
    move(t[0u32])

  fun tup_take_caller(mut x : u32) -> u32 := do
    let r := tup_take((&mut x, 1u32))
    *r := 2u32
    x
  spec tup_take_caller where
    ensures result == 2

  fun tup_take_caller_false(mut x : u32) -> u32 := do
    let r := tup_take((&mut x, 1u32))
    *r := 2u32
    x
  spec tup_take_caller_false where
    ensures result == x -- error: written through the taken reference

  -- Through `&mut` of a holder.
  fun field_ref(it : &mut Holder) -> &mut u32 :=
    &mut *it.r

  fun hold_field_ref(mut x : u32) -> u32 := do
    let mut h := new Holder { r := &mut x }
    let r := field_ref(&mut h)
    *r := 6u32
    x
  spec hold_field_ref where
    ensures result == 6

  fun hold_field_ref_false(mut x : u32) -> u32 := do
    let mut h := new Holder { r := &mut x }
    let r := field_ref(&mut h)
    *r := 6u32
    x
  spec hold_field_ref_false where
    ensures result == x -- error: written through the returned reborrow

  -- Two writes around a reborrow through the holder.
  fun two_writes(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 1u32
    let r := &mut *h.r
    *r := 2u32
    *h.r := 3u32
    x
  spec two_writes where
    ensures result == 3
  fun two_writes_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 1u32
    let r := &mut *h.r
    *r := 2u32
    *h.r := 3u32
    x
  spec two_writes_false where
    ensures x == 3 -- error: that is the value on entry

  -- Two loans of one struct.
  fun two_local(mut a : u32, mut b : u32) -> R := do
    let t := new Two { a := &mut a, b := &mut b }
    *t.a := 1u32
    *t.b := 2u32
    new R { x := a, y := b }
  spec two_local where
    ensures result.x == 1 && result.y == 2
  fun two_local_false(mut a : u32, mut b : u32) -> R := do
    let t := new Two { a := &mut a, b := &mut b }
    *t.a := 1u32
    *t.b := 2u32
    new R { x := a, y := b }
  spec two_local_false where
    ensures result.x == 2 -- error: `a` holds 1

  -- Re-pointed.
  fun reassign(mut a : u32, mut b : u32) -> R := do
    let mut h := new Holder { r := &mut a }
    *h.r := 1u32
    h.r := &mut b
    *h.r := 2u32
    new R { x := a, y := b }
  spec reassign where
    ensures result.x == 1 && result.y == 2
  fun reassign_false(mut a : u32, mut b : u32) -> R := do
    let mut h := new Holder { r := &mut a }
    *h.r := 1u32
    h.r := &mut b
    *h.r := 2u32
    new R { x := a, y := b }
  spec reassign_false where
    ensures b == 2 -- error: that is the value on entry

  -- A call mid-way, the holder written both sides.
  fun bump(r : &mut u32) -> Unit := do
    *r := *r + 1u32
  spec bump where
    requires r < 100
    ensures r == old(r) + 1

  fun call_through(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 1u32
    bump(&mut *h.r)
    *h.r := *h.r + 1u32
    x
  spec call_through where
    ensures result == 3
  fun call_through_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    *h.r := 1u32
    bump(&mut *h.r)
    *h.r := *h.r + 1u32
    x
  spec call_through_false where
    ensures result == 2 -- error: three writes

  -- by value with entry view
  fun add_one(h : Holder) -> Unit := do
    *h.r := *h.r + 1u32
  spec add_one where
    requires h.r < 100
    ensures h.r == old(h).r + 1
  fun add_one_false(h : Holder) -> Unit := do
    *h.r := *h.r + 1u32
  spec add_one_false where
    requires h.r < 100
    ensures h.r == old(h).r -- error: it added one

  -- A branch that returns early, the holder resolved there.
  fun early(c : Bool, mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    if c then
      *h.r := 1u32
    else
      *h.r := 2u32
    x
  spec early where
    ensures c ==> result == 1
    ensures !c ==> result == 2
  fun early_false(c : Bool, mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    if c then
      *h.r := 1u32
    else
      *h.r := 2u32
    x
  spec early_false where
    ensures result == 1 -- error: not where `c` fails

  -- A copy of the bare field.
  fun field_value(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let r := h.r
    *r := 5u32
    x
  spec field_value where
    ensures result == 5
  fun field_value_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let r := h.r
    *r := 5u32
    x
  spec field_value_false where
    ensures result == x -- error: written through the field

  fun returned_param(x : &mut u32) -> &mut u32 := do
    let h := new Holder { r := x }
    move(h.r)
  spec returned_param where
    ensures result == old(x)

  fun returned_param_false(x : &mut u32) -> &mut u32 := do
    let h := new Holder { r := x }
    move(h.r)
  spec returned_param_false where
    ensures result == old(x) + 1 -- error: it writes nothing

  fun returned_param_user(mut y : u32) -> u32 := do
    let r := returned_param(&mut y)
    *r := 4u32
    y
  spec returned_param_user where
    ensures result == 4

  fun returned_param_user_false(mut y : u32) -> u32 := do
    let r := returned_param(&mut y)
    *r := 4u32
    y
  spec returned_param_user_false where
    ensures result == y -- error: written through the result

  fun whole_move(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let k := h
    *k.r := 8u32
    x
  spec whole_move where
    ensures result == 8

  fun whole_move_false(mut x : u32) -> u32 := do
    let h := new Holder { r := &mut x }
    let k := h
    *k.r := 8u32
    x
  spec whole_move_false where
    ensures result == x -- error: written through the moved holder

  -- A holder written in a loop: its lender is written.
  fun loop_holder(mut x : u32) -> u32 := do
    x := 0u32
    let h := new Holder { r := &mut x }
    let mut i := 0u32
    while i < 3u32 do
      *h.r := i
      i := i + 1u32
    where
      invariant i <= 3
    x
  spec loop_holder where
    ensures result == 0 -- error: written in the loop

  fun loop_holder_any(mut x : u32) -> u32 := do
    x := 0u32
    let h := new Holder { r := &mut x }
    let mut i := 0u32
    while i < 3u32 do
      *h.r := i
      i := i + 1u32
    where
      invariant i <= 3
    x
  spec loop_holder_any where
    ensures false -- error: the path is not vacuous

/-! A holder at a type parameter, and an enum holding a reference. -/

verex namespace generic_struct where
  struct Wrap {T} where
    value : T

  fun f(x0 : u32) -> u32 := do
    let mut x := x0
    let w := new Wrap<&mut u32> { value := &mut x }
    *w.value := 1u32
    x
  spec f where
    ensures result == 1

  fun f_false(x0 : u32) -> u32 := do
    let mut x := x0
    let w := new Wrap<&mut u32> { value := &mut x }
    *w.value := 1u32
    x
  spec f_false where
    ensures result == x0 -- error: written through the holder

  fun f_vacuous(x0 : u32) -> u32 := do
    let mut x := x0
    let w := new Wrap<&mut u32> { value := &mut x }
    *w.value := 1u32
    x
  spec f_vacuous where
    ensures false -- error: the path is not vacuous

verex namespace enum_holder where
  enum Slot where
    | Empty
    | Full (r : &mut u32)

  fun f(x0 : u32) -> u32 := do
    let mut x := x0
    let s := new Slot::Full { r := &mut x }
    drop(s)
    x
  spec f where
    ensures result == x0

  fun f_false(x0 : u32) -> u32 := do
    let mut x := x0
    let s := new Slot::Full { r := &mut x }
    drop(s)
    x
  spec f_false where
    ensures result == x0 + 1 -- error: nothing is written

  fun f_vacuous(x0 : u32) -> u32 := do
    let mut x := x0
    let s := new Slot::Full { r := &mut x }
    drop(s)
    x
  spec f_vacuous where
    ensures false -- error: the path is not vacuous

-- A holder written while a loan moved out of it lives: the loan's death names the place
-- holding it then.
verex namespace reinitialized where
  struct Holder where
    r : &mut u32

  fun f(mut a : u32, mut b : u32) -> u32 := do
    let mut k := new Holder { r := &mut a }
    let h := k
    k := new Holder { r := &mut b }
    *h.r := 1u32
    *k.r := 2u32
    b
  spec f where
    ensures result == 2

  fun f_false(mut a : u32, mut b : u32) -> u32 := do
    let mut k := new Holder { r := &mut a }
    let h := k
    k := new Holder { r := &mut b }
    *h.r := 1u32
    *k.r := 2u32
    b
  spec f_false where
    ensures result == 1 -- error: the second holder writes `b`

-- A reference overwritten while the one it reborrows lives: its loan ends at the overwrite.
verex namespace overwritten_live where
  struct R where
    x : u32
    y : u32

  fun f(mut a : u32, mut b : u32) -> R := do
    let p := &mut a
    let mut s := &mut *p
    *s := 1u32
    s := &mut b
    *p := 5u32
    *s := 6u32
    new R { x := a, y := b }
  spec f where
    ensures result.x == 5 && result.y == 6

-- The same in a Move module.
verex module 0x42::overwritten_live_move where
  fun f(a : u64, b : u64) -> (u64, u64) := do
    let mut x := a
    let mut y := b
    let p := &mut x
    let mut s := &mut *p
    *s := 1
    s := &mut y
    *p := 5
    *s := 6
    (x, y)
  spec f where
    ensures spec.result[0] == 5 && spec.result[1] == 6
