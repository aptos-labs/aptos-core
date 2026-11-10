// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct R {
    pub x: u32,
    pub y: u32,
}

pub struct Holder<'a> {
    pub r: &'a mut u32,
}

pub fn both<'a, 'b>(a: &'a mut u32, b: &'b mut u32) -> (&'a mut u32, &'b mut u32) {
    (a, b)
}

pub fn swp<'a>(a: &'a mut u32, b: &'a mut u32) -> (&'a mut u32, &'a mut u32) {
    (b, a)
}

pub fn cond_swp<'a>(c: bool, a: &'a mut u32, b: &'a mut u32) -> (&'a mut u32, &'a mut u32) {
    if c { (b, a) } else { (a, b) }
}

pub fn write_first<'a>(a: &'a mut u32, b: &'a mut u32) -> (&'a mut u32, &'a mut u32) {
    *a = 5;
    *b = 6;
    (a, b)
}

pub fn mixed(a: &mut u32) -> (&mut u32, u32) {
    let old = *a;
    (a, old)
}

pub fn ret_destruct(mut a: u32, mut b: u32) -> R {
    let (p, q) = both(&mut a, &mut b);
    *p = 1;
    *q = 2;
    R { x: a, y: b }
}

pub fn ret_destruct_false(mut a: u32, mut b: u32) -> R {
    let (p, q) = both(&mut a, &mut b);
    *p = 1;
    *q = 2;
    R { x: a, y: b }
}

pub fn ret_swap(mut a: u32, mut b: u32) -> R {
    let (p, q) = swp(&mut a, &mut b);
    *p = 1;
    *q = 2;
    R { x: a, y: b }
}

pub fn ret_swap_false(mut a: u32, mut b: u32) -> R {
    let (p, q) = swp(&mut a, &mut b);
    *p = 1;
    *q = 2;
    R { x: a, y: b }
}

pub fn ret_cond_swap(c: bool, mut a: u32, mut b: u32) -> R {
    let (p, q) = cond_swp(c, &mut a, &mut b);
    *p = 1;
    *q = 2;
    R { x: a, y: b }
}

pub fn ret_cond_swap_false(c: bool, mut a: u32, mut b: u32) -> R {
    let (p, q) = cond_swp(c, &mut a, &mut b);
    *p = 1;
    *q = 2;
    R { x: a, y: b }
}

pub fn ret_write_first(mut a: u32, mut b: u32) -> R {
    let (p, _q) = write_first(&mut a, &mut b);
    *p = 1;
    R { x: a, y: b }
}

pub fn ret_write_first_false(mut a: u32, mut b: u32) -> R {
    let (p, _q) = write_first(&mut a, &mut b);
    *p = 1;
    R { x: a, y: b }
}

pub fn ret_mixed(mut a: u32) -> R {
    let (p, o) = mixed(&mut a);
    *p = 1;
    R { x: a, y: o }
}

pub fn ret_mixed_false(mut a: u32) -> R {
    let (p, o) = mixed(&mut a);
    *p = 1;
    R { x: a, y: o }
}

pub fn take(h: Holder<'_>) -> &mut u32 {
    h.r
}

pub fn take_false(h: Holder<'_>) -> &mut u32 {
    h.r
}

pub fn tup_take(t: (&mut u32, u32)) -> &mut u32 {
    t.0
}

pub fn tup_take_false(t: (&mut u32, u32)) -> &mut u32 {
    t.0
}

pub fn field_ref<'s, 'a>(it: &'s mut Holder<'a>) -> &'s mut u32 {
    &mut *it.r
}

pub fn field_ref_false<'s, 'a>(it: &'s mut Holder<'a>) -> &'s mut u32 {
    &mut *it.r
}

pub fn replace_nested<'a>(selected: &mut &'a mut u32, replacement: &'a mut u32) {
    *selected = replacement;
}

pub fn replace_nested_false<'a>(selected: &mut &'a mut u32, replacement: &'a mut u32) {
    *selected = replacement;
}
