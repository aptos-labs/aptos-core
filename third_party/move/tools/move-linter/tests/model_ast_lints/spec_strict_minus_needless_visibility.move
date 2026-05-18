// @checks=strict,-needless_visibility
// Verifies that `--checks=strict,-needless_visibility` runs the strict tier
// (so `use_receiver_style` fires) but suppresses `needless_visibility`.

module 0xc0ffee::m {
    struct S has drop {}

    // Without `-needless_visibility`, this would warn:
    // "package function `helper` is only called from the same module".
    package fun helper(self: &S): u64 {
        let _ = self;
        0
    }

    public fun caller(s: &S): u64 {
        // `use_receiver_style` (strict tier) DOES still warn here:
        // `helper(s)` could be written `s.helper()`.
        helper(s)
    }
}
