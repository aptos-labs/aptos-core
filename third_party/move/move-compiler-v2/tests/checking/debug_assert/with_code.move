// 2-arg forms: with a u64 abort code, and with a vector<u8> abort message.
module 0x42::m {
    const E_INVARIANT: u64 = 0xDEAD;

    public fun two_arg_u64(x: u64) {
        debug_assert!(x > 0, E_INVARIANT);
    }

    public fun two_arg_msg(x: u64) {
        debug_assert!(x > 0, b"x must be positive");
    }
}
