module 0x42::m {
    struct R has key {
        f: u64,
    }

    fun acq(addr: address): R acquires R {
        move_from(addr)
    }

    // Both `r` and `f_ref`, which is derived from `r`, conflict with the call.
    fun t(addr: address): u64 acquires R {
        let r = borrow_global<R>(addr);
        let f_ref = &r.f;
        let R { f } = acq(addr);
        let x = *f_ref;
        x + r.f + f
    }
}
