// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn die() -> ! {
    std::process::abort()
}

pub fn spin(_value: u64) -> ! {
    loop {}
}

// A call of `die` is a panic, whatever a contract of `die` says.
pub fn unpermitted(c: u64) -> u64 {
    if c == 0 {
        die()
    }
    c
}

pub fn wrong_condition(c: u64) -> u64 {
    if c == 0 {
        die()
    }
    c
}

pub fn wrong_result(c: u64) -> u64 {
    if c == 0 {
        std::process::exit(1)
    }
    c
}

pub fn unpermitted_abort(c: u64) -> u64 {
    if c == 0 {
        std::process::abort()
    }
    c
}

// A callee that spins is a panic, which is sound and incomplete.
pub fn spinning(c: u64) -> u64 {
    if c == 0 {
        spin(c)
    }
    c
}

pub fn two_regions(c: u64) -> u64 {
    if c == 0 {
        die()
    }
    if c == 1 {
        die()
    }
    c
}

pub fn only_exit(c: bool) -> u64 {
    loop {
        if c {
            die()
        }
    }
}
