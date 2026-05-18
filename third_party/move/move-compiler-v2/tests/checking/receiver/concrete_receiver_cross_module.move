module 0x42::definer {
    struct Box<T> has drop, copy, store { val: T }

    // OK: concrete receiver in the defining module
    public fun unbox_u64(self: Box<u64>): u64 { self.val }

    public fun new_u64(val: u64): Box<u64> { Box { val } }
}

module 0x42::caller {
    use 0x42::definer;

    // Cross-module CALL (not definition) — should work
    fun test_cross_module_call() {
        let b = definer::new_u64(42);
        assert!(b.unbox_u64() == 42, 0);
    }
}
