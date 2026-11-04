// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Holder<'a> {
    pub r: &'a mut u8,
}

pub struct Two<'a> {
    pub a: &'a mut u8,
    pub b: &'a mut u8,
}

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

pub fn cond_move_local_false(c: bool, a: u8, v: u8) -> u8 {
    let mut x = a;
    let h = Holder { r: &mut x };
    if c {
        let r = h.r;
        *r = v;
    }
    x
}

pub fn cond_move_local_vacuous(c: bool, a: u8, v: u8) -> u8 {
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

pub fn cond_move_param_by_value_false(c: bool, h: Holder<'_>, v: u8) {
    if c {
        let r = h.r;
        *r = v;
    }
}

pub fn cond_move_param_by_value_vacuous(c: bool, h: Holder<'_>, v: u8) {
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

pub fn cond_move_lent_false(c: bool, h: &mut Holder<'_>, v: u8) {
    if c {
        let r = &mut *h.r;
        *r = v;
    }
}

pub fn cond_move_lent_vacuous(c: bool, h: &mut Holder<'_>, v: u8) {
    if c {
        let r = &mut *h.r;
        *r = v;
    }
}

pub fn cond_return(c: bool, a: &mut u8, b: &mut u8, v: u8) {
    let r = cond_returned(c, Holder { r: a }, b);
    *r = v;
}

pub fn cond_return_false(c: bool, a: &mut u8, b: &mut u8, v: u8) {
    let r = cond_returned(c, Holder { r: a }, b);
    *r = v;
}

pub fn cond_return_vacuous(c: bool, a: &mut u8, b: &mut u8, v: u8) {
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

pub fn cond_store_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn cond_store_vacuous(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn cond_two_components_false(c: bool, a: &mut u8, b: &mut u8, v: u8) {
    let t = Two { a, b };
    if c {
        let p = t.a;
        *p = v;
    } else {
        let q = t.b;
        *q = v;
    }
}

pub fn cond_two_components_vacuous(c: bool, a: &mut u8, b: &mut u8, v: u8) {
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

pub fn cond_nested_false(c: bool, a: &mut u8, v: u8) {
    let o = Outer { a: Holder { r: a } };
    if c {
        let r = o.a.r;
        *r = v;
    }
}

pub fn cond_nested_vacuous(c: bool, a: &mut u8, v: u8) {
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

pub fn cond_callee_false(c: bool, a: &mut u8, v: u8) {
    let h = Holder { r: a };
    if c {
        bump(h.r, v);
    }
}

pub fn cond_callee_vacuous(c: bool, a: &mut u8, v: u8) {
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

pub fn cond_loop_false(a: &mut u8, n: u8) {
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

pub fn cond_loop_vacuous(a: &mut u8, n: u8) {
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

pub fn cond_store_entry_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn whole_repoint_false(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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

pub fn whole_repoint_vacuous(c: bool, a: u8, b: u8, v: u8) -> (u8, u8) {
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
