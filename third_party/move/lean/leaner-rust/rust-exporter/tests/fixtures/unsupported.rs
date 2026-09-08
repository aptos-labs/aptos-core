#![allow(dead_code)]

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub fn inline_assembly() {
    // This fixture is intentionally outside the accepted subset. M0 must
    // observe the MIR tag so the mapper can reject it without erasing it.
    unsafe {
        core::arch::asm!("");
    }
}
