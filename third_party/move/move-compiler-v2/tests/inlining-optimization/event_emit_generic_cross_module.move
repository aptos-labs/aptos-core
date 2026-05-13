// A generic callee that calls `0x1::event::emit<T>` on its own type parameter
// must not be inlined across modules. After inlining, the caller's type argument
// concretizes `T`, potentially into a struct outside the caller's module — which
// would violate the extended-check invariant on `event::emit<T>`.
module 0x1::event {
    public fun emit<T: store + drop>(_msg: T) {}
}

module 0x1::wrapper {
    friend 0x1::caller;

    public(friend) fun emit_wrapper<T: store + drop>(e: T) {
        0x1::event::emit(e)
    }
}

module 0x1::caller {
    struct E has drop, store { v: u64 }

    public fun trigger(e: E) {
        0x1::wrapper::emit_wrapper(e);
    }
}
