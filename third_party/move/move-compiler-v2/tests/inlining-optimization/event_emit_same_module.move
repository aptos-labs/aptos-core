// Positive control: when the callee emits an event whose struct is defined in
// the same module as the caller, inlining is still allowed. The final bytecode
// for `trigger` should contain the inlined body of `emit_e`, not a call to it.
module 0x1::event {
    public fun emit<T: store + drop>(_msg: T) {}
}

module 0x1::both {
    struct E has drop, store { v: u64 }

    fun emit_e(e: E) {
        0x1::event::emit(e)
    }

    public fun trigger(e: E) {
        emit_e(e)
    }
}
