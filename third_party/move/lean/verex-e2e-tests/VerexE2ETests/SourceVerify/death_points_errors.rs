// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Where a loan dies: every holder is dead from the anchor on, on every path
// reaching it. Each function has a false twin and an `ensures false` twin.

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct Two<'a> {
    pub a: &'a mut u8,
    pub b: &'a mut u8,
}

pub fn bump(r: &mut u8, v: u8) {
    *r = v;
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

pub fn whole_repoint_trailing_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn whole_repoint_trailing_vacuous(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn cond_store_trailing_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn cond_store_trailing_vacuous(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn reborrow_chain_false(a: u8, v: u8) -> u8 {
    let mut x = a;
    let s = &mut x;
    let t = &mut *s;
    *t = v;
    x
}

pub fn reborrow_chain_vacuous(a: u8, v: u8) -> u8 {
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

pub fn cond_consume_false(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    if c {
        bump(r, v);
    } else {
        *r = 1;
    }
    x
}

pub fn cond_consume_vacuous(c: bool, a: u8, v: u8) -> u8 {
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

pub fn sibling_live_false(c: bool, p: u8, q: u8, v: u8) -> (u8, u8) {
    let mut x = p;
    let mut y = q;
    let t = Two { a: &mut x, b: &mut y };
    if c {
        *t.a = v;
    }
    *t.b = v;
    (x, y)
}

pub fn sibling_live_vacuous(c: bool, p: u8, q: u8, v: u8) -> (u8, u8) {
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

pub fn then_reads_else_repoints_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn then_reads_else_repoints_vacuous(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn loop_local_write_false(a: u8, n: u8) -> u8 {
    let mut x = a;
    let r = &mut x;
    let mut i = 0u8;
    while i < n {
        *r = i;
        i += 1;
    }
    x
}

pub fn loop_local_write_vacuous(a: u8, n: u8) -> u8 {
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

pub fn break_skips_repoint_false(c: bool, n: u8, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn break_skips_repoint_vacuous(c: bool, n: u8, a: u8, b: u8, v: u8) -> (u8, u8) {
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
