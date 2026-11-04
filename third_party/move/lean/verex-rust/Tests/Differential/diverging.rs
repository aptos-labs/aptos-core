// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub enum Color {
    Red,
    Green,
    Blue,
}

// The interpreter and the denotation see only `abort`, whatever the callee was.
pub fn die() -> ! {
    std::process::abort()
}

pub fn exits(_code: i32) -> ! {
    std::process::exit(_code)
}

pub fn own_function(c: u8) -> u8 {
    if c == 0 {
        die()
    }
    c
}

pub fn library_function(c: u8) -> u8 {
    if c == 0 {
        std::process::exit(1)
    }
    c
}

pub fn computes(c: u8, x: u8) -> u8 {
    if c == 0 {
        let y = x.wrapping_add(1);
        exits((y / x) as i32)
    }
    c
}

pub fn nested(a: u8, b: u8) -> u8 {
    let mut x = 0;
    if a > 10 {
        if b > 10 {
            die()
        }
        x = a - b;
    }
    x + 1
}

// The arms that continue meet, and what follows stays out of them.
pub fn arms_meet(a: u8, b: u8) -> u8 {
    let mut x = 0;
    if b > 10 {
        match a {
            0 => die(),
            1 => x = 10,
            _ => x = 20,
        }
        x = x + 1;
    }
    x + 2
}

pub fn arm(a: u8) -> u8 {
    match a {
        0 => die(),
        1 => 10,
        2 => exits(3),
        _ => 20,
    }
}

pub fn variant(a: u8) -> u8 {
    let color = if a == 0 {
        Color::Red
    } else if a == 1 {
        Color::Green
    } else {
        Color::Blue
    };
    match color {
        Color::Red => die(),
        Color::Green => 1,
        Color::Blue => 2,
    }
}

pub fn exit_of_loop(n: u8) -> u8 {
    let mut i = 0u8;
    loop {
        if i == n {
            die()
        }
        i += 1;
    }
}

pub fn after_a_loop(n: u8) -> u8 {
    let mut i = 0u8;
    while i < n {
        i += 1;
    }
    if i == 5 {
        die()
    }
    i
}
