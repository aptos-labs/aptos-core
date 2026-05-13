module 0x2::ModifiesReceiverCall {
    struct State has key { value: u64 }

    struct Handle has copy, drop {
        inner: address,
    }

    public fun addr_of(self: &Handle): address {
        self.inner
    }

    public fun increment(handle: Handle) acquires State {
        let s = borrow_global_mut<State>(handle.inner);
        s.value = s.value + 1;
    }
    spec increment {
        aborts_if !exists<State>(handle.addr_of());
        modifies global<State>(handle.addr_of());
    }
}
