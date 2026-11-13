// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn whole(mut a: u32, mut b: u32) -> u32 {
    let t = (&mut a, &mut b);
    *t.0 = 1;
    *t.1 = 2;
    a
}

pub fn nested(mut a: u32, mut b: u32) -> u32 {
    let t = ((&mut a, 7u32), &mut b);
    *(t.0).0 = t.0.1;
    *t.1 = 2;
    a
}

pub fn read(a: u32, b: u32) -> u32 {
    let t = ((a, b), a);
    (t.0).1
}

pub fn through(t: &mut (u32, u32)) {
    let (a, b) = t;
    *a = 1;
    *b = 2;
}
