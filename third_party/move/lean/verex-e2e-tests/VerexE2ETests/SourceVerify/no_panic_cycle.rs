// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// `ping` panics; nothing specifies `ping` or `pong`, which call each other.

pub fn ping(n: u32) -> u32 {
    if n > 100 {
        pong(n)
    } else {
        100 / n
    }
}

pub fn pong(n: u32) -> u32 {
    ping(n - 100)
}

pub fn caller(n: u32) -> u32 {
    ping(n)
}

// `tick` is specified; `tock`, on its cycle, is not.

pub fn tick(n: u32) -> u32 {
    if n > 100 {
        tock(n)
    } else {
        n
    }
}

pub fn tock(n: u32) -> u32 {
    tick(n - 100)
}
