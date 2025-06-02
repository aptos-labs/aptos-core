module 0x42::m {

    struct S<T> { x: T }

    // Call styles

    fun receiver_needs_type_args<T, R>(self: S<T>, _y: T) {
        abort 1
    }

    fun test_call_styles(s: S<u64>, x: u64) {
        s.receiver_needs_type_args(x)
    }
}
