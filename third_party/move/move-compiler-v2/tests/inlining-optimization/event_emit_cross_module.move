// A callee that calls `0x1::event::emit<T>` must not be inlined into a caller
// that lives in a different module than `T`. Moving the `event::emit` bytecode
// across modules would violate the extended-check invariant that requires
// `event::emit<T>` to be invoked from `T`'s defining module.
//
// The callee below does not pack, destructure, or borrow E — so the only
// reason it is ineligible for inlining is the cross-module `event::emit<E>`.
module 0x1::event {
    public fun emit<T: store + drop>(_msg: T) {}
}

module 0x1::emitter {
    friend 0x1::caller;

    struct E has drop, store { v: u64 }

    public(friend) fun emit_e(e: E) {
        0x1::event::emit(e)
    }
}

module 0x1::caller {
    public fun trigger(e: 0x1::emitter::E) {
        0x1::emitter::emit_e(e);
    }
}
