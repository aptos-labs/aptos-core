// Tests that #[immutable] is rejected on native and inline functions.
module 0x42::immutable_attr_invalid {
    // Error: #[immutable] on a native function.
    #[immutable]
    native fun native_fn(): u64;

    // Error: #[immutable] on an inline function.
    #[immutable]
    inline fun inline_fn(): u64 { 42 }

    public fun use_both(): u64 {
        inline_fn()
    }
}
