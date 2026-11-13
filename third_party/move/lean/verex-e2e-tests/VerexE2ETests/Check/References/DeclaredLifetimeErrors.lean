-- Copyright © Aptos Foundation
-- SPDX-License-Identifier: Apache-2.0

import VerexSL

/-! Bodies and callers the declared lifetimes do not license. Rustc does not guard
VerexSL: a caller reads from a signature which loans a result holds, so a body
whose result holds another would free a lender while it is borrowed. -/

-- A parameter returned at an unrelated lifetime.
verex namespace wrong_parameter where
  fun sel {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 := y

  spec sel where
    ensures true

-- A reborrow of it.
verex namespace wrong_reborrow where
  fun sel {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 :=
    &mut *y

  spec sel where
    ensures true

-- Through locals, whose lifetimes are inferred.
verex namespace wrong_through_locals where
  fun sel {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 := do
    let r : &mut u32 := y
    let s : &mut u32 := r
    s

  spec sel where
    ensures true

-- On one path only.
verex namespace wrong_on_one_path where
  fun sel {a : lifetime} {b : lifetime}(c : Bool, x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 := do
    if c then &mut *x else &mut *y

  spec sel where
    ensures true

-- By an early return.
verex namespace wrong_early_return where
  fun sel {a : lifetime} {b : lifetime}(c : Bool, x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 := do
    if c then return &mut *y
    &mut *x

  spec sel where
    ensures true

-- Through a call whose declared result holds it.
verex namespace wrong_through_call where
  fun keep {c : lifetime} {d : lifetime}(p : &[c] mut u32, q : &[d] mut u32) -> &[c] mut u32 :=
    &mut *p

  fun sel {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 :=
    keep(&mut *y, &mut *x)

  spec sel where
    ensures true

-- A shared reference: its snapshot is sound only while its lender is not written.
verex namespace wrong_shared where
  fun sel {a : lifetime} {b : lifetime}(x : &[a] u32, y : &[b] u32) -> &[a] u32 := y

  spec sel where
    ensures true

-- The reference component of a parameter held by value.
verex namespace wrong_component where
  struct Pair {a : lifetime} {b : lifetime} where
    x : &[a] u32
    y : &[b] mut u32

  fun take {a : lifetime} {b : lifetime}(p : Pair<lifetime a, lifetime b>) -> &[a] mut u32 :=
    move(p.y)

  spec take where
    ensures true

-- The components of a tuple result crossed.
verex namespace wrong_tuple_component where
  fun both {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> (&[a] mut u32, &[b] mut u32) :=
    (&mut *y, &mut *x)

  spec both where
    ensures true

-- A struct result holding the other parameter.
verex namespace wrong_struct_result where
  struct Holder {a : lifetime} where
    r : &[a] mut u32

  fun make {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> Holder<lifetime a> :=
    new Holder<lifetime _> { r := &mut *y }

  spec make where
    ensures true

-- A relation nothing states: no outlives bound can be read or written.
verex namespace wrong_no_relation where
  fun shrink {s : lifetime} {l : lifetime}(x : &[l] mut u32) -> &[s] mut u32 := &mut *x

  spec shrink where
    ensures true

-- A bound nothing declares between two lifetimes of one reference's referent.
verex namespace wrong_implied_direction where
  fun lengthen {s : lifetime} {l : lifetime}(x : &[s] u32, y : &[s] &[l] u32) -> &[l] u32 := x

  spec lengthen where
    ensures true

-- A generic referent changes nothing.
verex namespace wrong_generic where
  fun sel {T} {a : lifetime} {b : lifetime}(x : &[b] mut T, y : &[a] mut T) -> &[a] mut T :=
    &mut *x

  spec sel where
    ensures true

-- A nominal's arguments are not related, whatever its declaration would bound.
verex namespace wrong_nominal_arguments where
  struct Ordered {a : lifetime} {b : lifetime} where
    short : &[a] u32
    long : &[b] u32

  fun long_as_short {x : lifetime} {y : lifetime}(o : Ordered<lifetime x, lifetime y>) -> &[x] u32 :=
    copy(o.long)

  spec long_as_short where
    ensures true

-- The lender of the parameter a result does hold.
verex namespace caller_writes_held_lender where
  fun sel {a : lifetime} {b : lifetime}(x : &[a] mut u32, y : &[b] mut u32) -> &[a] mut u32 :=
    &mut *x

  fun f(mut a : u32, mut b : u32) -> u32 := do
    let r := sel(&mut a, &mut b)
    a := 2u32
    *r := 1u32
    a

  spec f where
    ensures true

-- One lifetime ties the other parameter's lender too.
verex namespace caller_writes_tied_lender where
  fun pick {a : lifetime}(x : &[a] mut u32, y : &[a] mut u32) -> &[a] mut u32 :=
    &mut *x

  fun f(mut a : u32, mut b : u32) -> u32 := do
    let r := pick(&mut a, &mut b)
    b := 2u32
    *r := 1u32
    b

  spec f where
    ensures true

-- A bound the signature implies ties it as well.
verex namespace caller_writes_implied_lender where
  fun implied {s : lifetime} {l : lifetime}(x : &[l] u32, y : &[s] &[l] u32) -> &[s] u32 := x

  fun f(mut a : u32, b : u32) -> u32 := do
    let p := &b
    let r := implied(&a, &p)
    a := 2u32
    *r

  spec f where
    ensures true

-- The holder behind the reference a result is read through.
verex namespace caller_uses_holder_behind_result where
  struct Holder {a : lifetime} where
    r : &[a] mut u32

  fun inner {s : lifetime} {a : lifetime}(h : &[s] mut Holder<lifetime a>) -> &[s] mut u32 :=
    &mut *(*h).r

  fun f(mut x : u32) -> u32 := do
    let mut h := new Holder<lifetime _> { r := &mut x }
    let r := inner(&mut h)
    *h.r := 2u32
    *r := 1u32
    x

  spec f where
    ensures true

-- A lifetime no parameter is declared at is no licence: an elided one relates to all.
verex namespace caller_writes_elided_lender where
  fun sel(x : &mut u32, y : &mut u32) -> &mut u32 := &mut *x

  fun f(mut a : u32, mut b : u32) -> u32 := do
    let r := sel(&mut a, &mut b)
    b := 2u32
    *r := 1u32
    b

  spec f where
    ensures true

-- A type argument's regions may be related by what the unit does not see.
verex namespace caller_writes_type_argument_lender where
  fun first {T} {a : lifetime}(x : &[a] mut u32, y : T) -> &[a] mut u32 := &mut *x

  fun f(mut a : u32, mut b : u32) -> u32 := do
    let r := first(&mut a, &mut b)
    b := 2u32
    *r := 1u32
    b

  spec f where
    ensures true
