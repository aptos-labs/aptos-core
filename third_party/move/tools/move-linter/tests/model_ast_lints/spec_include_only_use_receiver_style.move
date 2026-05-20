// @checks=use_receiver_style
// Verifies that `--checks=use_receiver_style` runs ONLY the named lint:
// neither `needless_visibility` nor any other default-tier lint should fire.

module 0xc0ffee::m {
    struct S has drop {}

    // `needless_visibility` would normally warn here, but we did not select
    // any tier — only `use_receiver_style` is enabled.
    package fun helper(self: &S): u64 {
        let _ = self;
        0
    }

    public fun caller(s: &S): u64 {
        // `use_receiver_style` fires here.
        helper(s)
    }
}
