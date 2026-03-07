module 0x42::types {
    struct Box<T> has drop, copy, store { val: T }
}

module 0x42::impl_mod {
    use 0x42::types::Box;

    // Warning: Box is defined in 0x42::types, not here — not registered as receiver
    fun unbox(self: Box<u64>): u64 { abort 0 }

    // Error: unbox is not a receiver on Box, so dot-call fails
    fun test_unbox_as_receiver() {
        let b = Box<u64> { val: 42 };
        b.unbox();
    }
}
