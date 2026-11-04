// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A cursor over `Option<&mut T>`: each function has a false twin and an `ensures false`
// twin.

pub fn next(r: &mut u8) -> Option<&mut u8> {
    if *r == 0 {
        None
    } else {
        *r -= 1;
        Some(r)
    }
}

pub fn enum_cursor(a: &mut u8) {
    let mut cur = Some(a);
    while let Some(r) = cur {
        cur = next(r);
    }
}

pub fn enum_cursor_false(a: &mut u8) {
    let mut cur = Some(a);
    while let Some(r) = cur {
        cur = next(r);
    }
}

pub fn enum_cursor_vacuous(a: &mut u8) {
    let mut cur = Some(a);
    while let Some(r) = cur {
        cur = next(r);
    }
}

pub fn local_enum_cursor(a: u8) -> u8 {
    let mut x = a;
    let mut cur = Some(&mut x);
    while let Some(r) = cur {
        cur = next(r);
    }
    x
}

pub fn local_enum_cursor_false(a: u8) -> u8 {
    let mut x = a;
    let mut cur = Some(&mut x);
    while let Some(r) = cur {
        cur = next(r);
    }
    x
}

pub fn local_enum_cursor_vacuous(a: u8) -> u8 {
    let mut x = a;
    let mut cur = Some(&mut x);
    while let Some(r) = cur {
        cur = next(r);
    }
    x
}
