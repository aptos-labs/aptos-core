// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Inline assembly remains unsupported; its caller must not silently verify.
pub fn assembly() -> u64 {
    unsafe { core::arch::asm!("", options(nomem, nostack)); }
    5
}

pub fn no_contract() -> u64 {
    assembly()
}
