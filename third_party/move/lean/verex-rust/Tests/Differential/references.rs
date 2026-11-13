// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![feature(core_intrinsics)]
#![allow(internal_features)]

pub fn write(a: &mut u8, b: u8) {
    *a = b;
}

pub fn write_nothing(_a: &mut u8) {}

pub fn increment(a: &mut u8) {
    *a += 1;
}

pub fn write_if(a: &mut i8, b: bool) -> i8 {
    if b {
        *a = -*a;
    }
    *a
}

pub fn swap(a: &mut u8, b: &mut u8) {
    let held = *a;
    *a = *b;
    *b = held;
}

pub fn local_borrow(a: u8) -> u8 {
    let mut held = a;
    let r = &mut held;
    *r = core::intrinsics::wrapping_add(*r, 1);
    held
}

pub fn two_local_borrows(a: u8, b: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let p = &mut x;
    let q = &mut y;
    let held = *p;
    *p = *q;
    *q = held;
    (x, y)
}

pub fn reborrow(a: &mut u8) -> u8 {
    let r = &mut *a;
    *r = core::intrinsics::wrapping_mul(*r, 2);
    *a
}

pub fn through_callee(a: u8) -> u8 {
    let mut held = a;
    increment(&mut held);
    held
}

pub fn through_callee_twice(a: &mut u8, b: u8) -> u8 {
    write(a, b);
    increment(a);
    *a
}

pub struct Pair {
    pub first: u8,
    pub second: i8,
}

pub fn struct_place(a: Pair, b: u8) -> Pair {
    let mut p = a;
    p.first = b;
    let r = &mut p.second;
    *r = core::intrinsics::wrapping_add(*r, 1);
    p
}

pub fn struct_by_reference(a: &mut Pair) -> u8 {
    a.first = core::intrinsics::wrapping_add(a.first, 1);
    a.first
}

pub fn select<'a>(a: &'a mut u8, b: &'a mut u8, first: bool) -> &'a mut u8 {
    if first { a } else { b }
}

pub fn through_select(a: u8, b: u8, first: bool) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let r = select(&mut x, &mut y, first);
    *r = 9;
    (x, y)
}

pub fn pass_through(a: &mut u8) -> &mut u8 {
    a
}

pub fn through_pass(a: u8) -> u8 {
    let mut x = a;
    let r = pass_through(&mut x);
    *r = 9;
    x
}

pub fn array_place(a: [u8; 3], i: usize, b: u8) -> [u8; 3] {
    let mut held = a;
    held[i] = b;
    held
}

pub fn array_read(a: [u8; 3], i: usize) -> u8 {
    a[i]
}

pub fn array_borrow(a: [u8; 3], i: usize) -> [u8; 3] {
    let mut held = a;
    let r = &mut held[i];
    *r = core::intrinsics::wrapping_add(*r, 1);
    held
}

pub fn literal_read(a: [u8; 3]) -> u8 {
    a[1]
}

pub fn make_array(a: u8) -> [u8; 2] {
    [a, a]
}

pub fn read_of_call(a: u8, i: usize) -> u8 {
    make_array(a)[i]
}

pub fn nested_read(a: [[u8; 2]; 2], i: usize, j: usize) -> u8 {
    a[i][j]
}

pub fn struct_element(a: [Pair; 2], i: usize) -> i8 {
    a[i].second
}

pub fn struct_element_write(a: [Pair; 2], i: usize, b: u8) -> [Pair; 2] {
    let mut held = a;
    held[i].first = b;
    held
}

pub fn struct_element_borrow(a: [Pair; 2], i: usize) -> [Pair; 2] {
    let mut held = a;
    let r = &mut held[i].second;
    *r = core::intrinsics::wrapping_add(*r, 1);
    held
}

pub fn computed_index(a: [u8; 4], i: usize) -> u8 {
    a[i + 1]
}

pub fn slice_read(a: &[u8], i: usize) -> u8 {
    a[i]
}

pub fn slice_len(a: &[u8]) -> usize {
    a.len()
}

pub fn loop_write(a: &mut u8, n: u8) {
    let mut i = 0u8;
    while i < n {
        *a = core::intrinsics::wrapping_add(*a, i);
        i += 1;
    }
}

pub fn loop_borrow(n: u8) -> u8 {
    let mut total = 0u8;
    let mut i = 0u8;
    while i < n {
        let r = &mut total;
        *r = core::intrinsics::wrapping_add(*r, i);
        i += 1;
    }
    total
}

pub fn panic_after_write(a: &mut u8, b: u8) -> u8 {
    *a = 1;
    *a = 10 / b;
    *a
}

pub fn deref_of_call(a: u8) -> u8 {
    let mut x = a;
    *pass_through(&mut x)
}

pub fn make_pair(a: u8, b: i8) -> Pair {
    Pair { first: a, second: b }
}

pub fn field_of_call(a: u8, b: i8) -> i8 {
    make_pair(a, b).second
}

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct Two<'a> {
    pub a: &'a mut u8,
    pub b: &'a mut u8,
}

pub fn holder_move_out(a: u8, v: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let r = h.r;
    *r = v;
    x
}

pub fn holder_two_moved_ab(a: u8, b: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let t = Two { a: &mut x, b: &mut y };
    let p = t.a;
    let q = t.b;
    *p = b;
    *q = a;
    (x, y)
}

pub fn holder_two_moved_ba(a: u8, b: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let t = Two { a: &mut x, b: &mut y };
    let q = t.b;
    let p = t.a;
    *q = a;
    *p = b;
    (x, y)
}

pub fn holder_take(h: Holder<'_>) -> &mut u8 {
    h.r
}

pub fn holder_take_returned(a: u8, v: u8) -> u8 {
    let mut x = a;
    let r = holder_take(Holder { r: &mut x });
    *r = v;
    x
}

pub fn holder_write(h: &mut Holder<'_>, v: u8) {
    *h.r = v;
}

pub fn holder_by_mut_callee(a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut h = Holder { r: &mut x };
    holder_write(&mut h, v);
    let held = *h.r;
    *h.r = core::intrinsics::wrapping_add(held, 1);
    x
}

pub fn holder_repoint_after_dead(a: u8, b: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut h = Holder { r: &mut x };
    let first = *h.r;
    *h.r = core::intrinsics::wrapping_add(first, 1);
    h.r = &mut y;
    let second = *h.r;
    *h.r = core::intrinsics::wrapping_add(second, 2);
    (x, y)
}

pub fn holder_loop_repoint(a: [u8; 2], i: usize) -> [u8; 2] {
    let mut held = a;
    let mut first = 0u8;
    let mut h = Holder { r: &mut first };
    let mut k = 0usize;
    loop {
        *h.r = 7;
        if k == i {
            h.r = &mut held[k];
            break;
        }
        k += 1;
    }
    *h.r = 9;
    held
}

pub fn pair_swapped<'a>(a: &'a mut u8, b: &'a mut u8) -> (&'a mut u8, &'a mut u8) {
    (b, a)
}

pub fn tuple_from_call_destructured(a: u8, b: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let (p, q) = pair_swapped(&mut x, &mut y);
    *p = 1;
    *q = 2;
    (x, y)
}

pub fn holder_outer_dies_first(a: u8, v: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    let r = &mut *h.r;
    *r = v;
    *r = core::intrinsics::wrapping_add(*r, 1);
    *r = core::intrinsics::wrapping_add(*r, 1);
    x
}

// The same over lent references, whose prophecies the check perturbs.

pub fn lent_holder_move_out(a: &mut u8, v: u8) {
    let h = Holder { r: a };
    let r = h.r;
    *r = v;
}

pub fn lent_holder_two_moved_ab(a: &mut u8, b: &mut u8) {
    let held = *a;
    let t = Two { a, b };
    let p = t.a;
    let q = t.b;
    *p = *q;
    *q = held;
}

pub fn lent_holder_two_moved_ba(a: &mut u8, b: &mut u8) {
    let held = *a;
    let t = Two { a, b };
    let q = t.b;
    let p = t.a;
    *p = *q;
    *q = held;
}

pub fn lent_holder_take_returned(a: &mut u8, v: u8) {
    let r = holder_take(Holder { r: a });
    *r = v;
}

pub fn lent_holder_by_mut_callee(a: &mut u8, v: u8) {
    let mut h = Holder { r: a };
    holder_write(&mut h, v);
    let held = *h.r;
    *h.r = core::intrinsics::wrapping_add(held, 1);
}

pub fn lent_holder_repoint_after_dead(a: &mut u8, b: &mut u8) {
    let mut h = Holder { r: &mut *a };
    let first = *h.r;
    *h.r = core::intrinsics::wrapping_add(first, 1);
    h.r = &mut *b;
    let second = *h.r;
    *h.r = core::intrinsics::wrapping_add(second, 2);
}

pub fn lent_tuple_from_call_destructured(a: &mut u8, b: &mut u8) {
    let (p, q) = pair_swapped(a, b);
    *p = 1;
    *q = 2;
}

pub fn lent_holder_outer_dies_first(a: &mut u8, v: u8) {
    let h = Holder { r: a };
    let r = &mut *h.r;
    *r = v;
    let held = *r;
    *r = core::intrinsics::wrapping_add(held, 1);
}

// Moves out of aggregates on some paths.

pub struct Outer<'a> {
    pub a: Holder<'a>,
}

pub fn cond_returned<'a>(c: bool, h: Holder<'a>, y: &'a mut u8) -> &'a mut u8 {
    if c { h.r } else { y }
}

pub fn bump(r: &mut u8, v: u8) {
    *r = v;
}

pub fn cond_move_local(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    if c {
        let r = h.r;
        *r = v;
    }
    x
}

pub fn cond_move_param_by_value(c: bool, h: Holder<'_>, v: u8) {
    if c {
        let r = h.r;
        *r = v;
    }
}

pub fn cond_move_lent(c: bool, h: &mut Holder<'_>, v: u8) {
    if c {
        let r = &mut *h.r;
        *r = v;
    }
}

pub fn cond_return(c: bool, a: &mut u8, b: &mut u8, v: u8) {
    let r = cond_returned(c, Holder { r: a }, b);
    *r = v;
}

pub fn cond_store(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let h = Holder { r: &mut x };
    let mut k = Holder { r: &mut y };
    if c {
        k.r = h.r;
    }
    *k.r = v;
    (x, y)
}

pub fn cond_two_components(c: bool, a: &mut u8, b: &mut u8, v: u8) {
    let t = Two { a, b };
    if c {
        let p = t.a;
        *p = v;
    } else {
        let q = t.b;
        *q = v;
    }
}

pub fn cond_nested(c: bool, a: &mut u8, v: u8) {
    let o = Outer { a: Holder { r: a } };
    if c {
        let r = o.a.r;
        *r = v;
    }
}

pub fn cond_callee(c: bool, a: &mut u8, v: u8) {
    let h = Holder { r: a };
    if c {
        bump(h.r, v);
    }
}

pub fn cond_loop(a: &mut u8, n: u8) {
    let h = Holder { r: a };
    let mut i = 0u8;
    while i < n {
        if i % 2 == 1 {
            let r = h.r;
            *r = i;
            break;
        }
        *h.r = i;
        i += 1;
    }
}

// Enums holding a mutable reference.

pub enum Slot<'a> {
    Left(&'a mut u8),
    Right(&'a mut u8),
    Empty,
}

pub fn slot_write(s: Slot<'_>, v: u8) {
    match s {
        Slot::Left(r) => *r = v,
        Slot::Right(r) => *r = core::intrinsics::wrapping_add(v, 1),
        Slot::Empty => {}
    }
}

pub fn opt_pick(c: bool, a: &mut u8) -> Option<&mut u8> {
    if c { Some(a) } else { None }
}

pub fn opt_by_value(o: Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_by_value_caller(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let o = if c { Some(&mut x) } else { None };
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn opt_returned(c: bool, a: &mut u8, v: u8) {
    if let Some(r) = opt_pick(c, a) {
        *r = v;
    }
}

pub fn opt_by_ref(o: &mut Option<u8>, v: u8) {
    if let Some(r) = o {
        *r = v;
    }
}

pub fn opt_ref_of_ref(o: &mut Option<&mut u8>, v: u8) {
    if let Some(r) = o {
        **r = v;
    }
}

pub fn opt_none_on_one_path(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let mut o = Some(&mut x);
    if c {
        o = None;
    }
    if let Some(r) = o {
        *r = v;
    }
    x
}

pub fn slot_repointed(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = Slot::Left(&mut x);
    if c {
        s = Slot::Right(&mut y);
    }
    slot_write(s, v);
    (x, y)
}

pub fn whole_repoint(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let r = &mut x;
    let mut s = &mut y;
    if c {
        s = r;
    }
    *s = v;
    (x, y)
}

pub fn whole_repoint_trailing(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let r = &mut x;
    let mut s = &mut y;
    let mut i = 0u8;
    if c {
        s = r;
        i += 1;
    }
    *s = v;
    (x, y)
}

pub fn cond_store_trailing(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let h = Holder { r: &mut x };
    let mut k = Holder { r: &mut y };
    let mut i = 0u8;
    if c {
        k.r = h.r;
        i += 1;
    }
    *k.r = v;
    (x, y)
}

pub fn reborrow_chain(a: u8, v: u8) -> u8 {
    let mut x = a;
    let s = &mut x;
    let t = &mut *s;
    *t = v;
    x
}

pub fn cond_consume(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    if c {
        bump(r, v);
    } else {
        *r = 1;
    }
    x
}

pub fn sibling_live(c: bool, p: u8, q: u8, v: u8) -> (u8, u8) {
    let mut x = p;
    let mut y = q;
    let t = Two { a: &mut x, b: &mut y };
    if c {
        *t.a = v;
    }
    *t.b = v;
    (x, y)
}

pub fn then_reads_else_repoints(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = &mut x;
    let w = &mut y;
    bump(w, b);
    if c {
        *s = v;
    } else {
        s = w;
    }
    *s = v;
    (x, y)
}

pub fn loop_local_write(a: u8, n: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let mut i = 0u8;
    while i < n {
        *r = i;
        i += 1;
    }
    x
}

pub fn break_skips_repoint(c: bool, n: u8, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = &mut x;
    let mut i = 0u8;
    while i < n {
        if c {
            break;
        }
        s = &mut y;
        i += 1;
    }
    *s = v;
    (x, y)
}

pub fn guard_falls_through(o: Option<u8>, a: u8, b: u8, v: u8) -> (u8, u8) {
    let mut x = a;
    let mut y = b;
    let mut s = &mut x;
    match o {
        Some(k) if k > 0 => {
            s = &mut y;
        }
        _ => {}
    }
    *s = v;
    (x, y)
}
