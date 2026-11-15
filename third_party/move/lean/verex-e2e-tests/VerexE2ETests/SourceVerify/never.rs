// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub fn abort_now() -> ! {
    std::process::abort()
}

pub fn spin() -> ! {
    loop {}
}

pub fn spin_unless(flag: bool, value: u32) -> u32 {
    if flag {
        loop {}
    }
    value
}
