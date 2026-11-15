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

pub fn direct(c: u64) -> u64 {
    if c == 0 {
        die()
    }
    c
}

pub fn explicit(c: u64) -> u64 {
    if c == 0 {
        std::process::exit(1)
    }
    c
}

// The region computes before its call, including a check of its own.
pub fn computes(c: u64, x: u64) -> u64 {
    if c == 0 {
        let y = x + 1;
        let z = y * y;
        spin(z / x)
    }
    c
}

// Both ways through the entry of the region reach a call.
pub fn both_ways(c: bool) -> u64 {
    if c {
        spin(1)
    } else {
        die()
    }
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

// The impossible default of the switch stays impossible.
pub fn arm(color: Color) -> u64 {
    match color {
        Color::Red => die(),
        Color::Green => 1,
        Color::Blue => 2,
    }
}

// Every arm diverges, so the switch goes with its impossible default.
pub fn every_arm(color: Color) -> u64 {
    match color {
        Color::Red => die(),
        Color::Green => spin(1),
        Color::Blue => die(),
    }
}

// No region: nothing but the impossible default fails.
pub fn no_region(color: Color) -> u64 {
    match color {
        Color::Red => 0,
        Color::Green => 1,
        Color::Blue => 2,
    }
}

// The loop has no way out but the call and is not in the region.
pub fn loop_exit(limit: u64) -> u64 {
    let mut count = 0;
    loop {
        if count > limit {
            die()
        }
        count += 1;
    }
}

pub fn before_a_loop(limit: u64) -> u64 {
    let mut count = 0;
    while count < limit {
        count += 1;
    }
    die()
}
