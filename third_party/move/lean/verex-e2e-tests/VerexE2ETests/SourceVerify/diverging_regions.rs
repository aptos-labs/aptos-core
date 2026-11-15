// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub enum Color {
    Red,
    Green,
    Blue,
}

pub fn spin(_value: u64) -> ! {
    loop {}
}

pub fn die() -> ! {
    std::process::abort()
}

pub fn own_function(c: u64) -> u64 {
    if c == 0 {
        die()
    }
    c
}

pub fn spinning_function(c: u64) -> u64 {
    if c == 0 {
        spin(c)
    }
    c
}

pub fn explicit(c: u64) -> u64 {
    if c == 0 {
        std::process::exit(1)
    }
    c
}

pub fn process_abort(c: u64) -> u64 {
    if c == 0 {
        std::process::abort()
    }
    c
}

pub fn computes(c: u64, x: u64) -> u64 {
    if c == 0 {
        let y = x + 1;
        let z = y * y;
        spin(z / x)
    }
    c
}

pub fn nested(a: bool, b: bool) -> u64 {
    let mut x = 0;
    if a {
        if b {
            die()
        }
        x = 1;
    }
    x
}

pub fn arm(color: Color) -> u64 {
    match color {
        Color::Red => die(),
        Color::Green => 1,
        Color::Blue => 2,
    }
}

pub fn no_region(color: Color) -> u64 {
    match color {
        Color::Red => 0,
        Color::Green => 1,
        Color::Blue => 2,
    }
}

pub fn loop_exit(limit: u64) -> u64 {
    let mut count = 0;
    while count < limit {
        count += 1;
    }
    if count != limit {
        die()
    }
    count
}

pub fn only_exit(c: bool) -> u64 {
    loop {
        if c {
            die()
        }
    }
}
