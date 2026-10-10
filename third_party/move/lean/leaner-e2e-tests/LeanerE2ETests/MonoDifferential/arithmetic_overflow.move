// Both engines detect the u8 overflow; they classify it differently. MonoVM
// reports a runtime failure, the Leaner path an abort carrying a code. The
// expectation is that the Leaner side should report the under/overflow too
// rather than synthesize an abort code, but neither side is patched to
// imitate the other: the recorded ERROR line keeps the open divergence
// visible — see the findings register in designs/monovm-link-design.md.

// RUN: publish
module 0x49::overflowing {
    public fun add_beyond(x: u8): u8 {
        x + 200
    }
}

// RUN: execute 0x49::overflowing::add_beyond --args 100
