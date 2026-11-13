// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// False twins of `declared_lifetimes`: each specification fails where it should.

pub struct R {
    pub x: u32,
    pub y: u32,
}

pub fn sel<'a, 'b>(x: &'a mut u32, _y: &'b mut u32) -> &'a mut u32 {
    x
}

pub fn write_other(mut a: u32, mut b: u32) -> R {
    let r = sel(&mut a, &mut b);
    b = 2;
    *r = 1;
    R { x: a, y: b }
}

pub struct Pair<'a, 'b> {
    pub x: &'a u32,
    pub y: &'b mut u32,
}

pub fn take_x<'a, 'b>(p: Pair<'a, 'b>) -> &'a u32 {
    p.x
}

pub fn take_y<'a, 'b>(p: Pair<'a, 'b>) -> &'b mut u32 {
    p.y
}

pub fn pair_take_x(a: u32, mut b: u32) -> u32 {
    let r = {
        let p = Pair { x: &a, y: &mut b };
        take_x(p)
    };
    b = 2;
    if *r == a { b } else { a }
}

pub fn pair_take_y(mut a: u32, mut b: u32) -> R {
    let r = {
        let p = Pair { x: &a, y: &mut b };
        take_y(p)
    };
    a = 1;
    *r = 2;
    R { x: a, y: b }
}

pub fn both<'a, 'b>(a: &'a mut u32, b: &'b mut u32) -> (&'a mut u32, &'b mut u32) {
    (a, b)
}

pub fn tuple_write_second_lender(mut a: u32, mut b: u32) -> R {
    let (p, q) = both(&mut a, &mut b);
    *q = 2;
    b = 3;
    *p = 1;
    R { x: a, y: b }
}

pub fn pick<'a, 'b: 'a>(c: bool, x: &'a mut u32, y: &'b mut u32) -> &'a mut u32 {
    if c { x } else { y }
}

pub fn pick_write(c: bool, mut a: u32, mut b: u32) -> R {
    let r = pick(c, &mut a, &mut b);
    *r = 1;
    R { x: a, y: b }
}

pub fn elided_two(x: &mut u32, y: &u32) -> u32 {
    *x = *y;
    *x
}

pub fn elided_result(x: &mut u32) -> &mut u32 {
    x
}

pub fn elided_write(mut a: u32) -> u32 {
    let r = elided_result(&mut a);
    *r = 4;
    a
}

pub struct S {
    pub v: u32,
}

impl S {
    pub fn get(&mut self, _other: &u32) -> &mut u32 {
        &mut self.v
    }
}

pub fn method_write_other(mut s: S, mut b: u32) -> R {
    let r = s.get(&b);
    b = 2;
    *r = 1;
    R { x: s.v, y: b }
}

pub fn generic_keep<'a, T>(x: &'a mut T, _y: &u32) -> &'a mut T {
    x
}

pub fn generic_caller(mut a: u32, b: u32) -> u32 {
    let r = generic_keep(&mut a, &b);
    *r = 1;
    a
}
