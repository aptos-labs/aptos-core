#![allow(dead_code)]

pub fn inline_assembly() {
    // This fixture is intentionally outside the accepted subset. M0 must
    // observe the MIR tag so the mapper can reject it without erasing it.
    unsafe {
        core::arch::asm!("");
    }
}
