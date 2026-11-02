// A `#[module_lock]` caller can inline same-module callees without that attribute:
// removing their frames preserves the caller's lock and reentrancy checks.
// Callees with `#[module_lock]` retain their frames to preserve their lock lifetimes.
module 0xc0ffee::m {
    fun helper(x: u64): u64 {
        x
    }

    #[module_lock]
    fun locked_helper(x: u64): u64 {
        x
    }

    #[module_lock]
    public fun locked(x: u64): u64 {
        helper(x)
    }

    public fun calls_locked(x: u64): u64 {
        locked_helper(x)
    }
}
