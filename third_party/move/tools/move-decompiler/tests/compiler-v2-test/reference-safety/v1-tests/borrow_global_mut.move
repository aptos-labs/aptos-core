module 0x8675309::M {
    struct R has key { f: u64 }

    fun t0(addr: address) acquires R {
        let f = borrow_global_mut<R>(addr).f;
        let r1 = borrow_global_mut<R>(addr);
        r1.f = f
    }
}
