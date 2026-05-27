address 0x0 {
module A {

    struct S has key {
        x: u64
    }
    inline fun mutate_at(addr: address, f: |address|)  {
        f(addr)
    }


    fun call_mutate_at(addr: address) acquires S {
        mutate_at(addr, |addr| {
            let s = borrow_global_mut<S>(addr);
            s.x = 2;
        } spec {
            modifies global<S>(addr);
            pragma opaque;
            aborts_if !exists<S>(addr);
            ensures global<S>(addr).x == 2;
        })
    }
    spec call_mutate_at {
        pragma opaque;
        modifies global<S>(addr);
        aborts_if !exists<S>(addr);
        ensures global<S>(addr).x == 2;
    }

    fun call_mutate_inline_no_modifies(addr: address) acquires S {
        mutate_at(addr, |addr| {
            let s = borrow_global_mut<S>(addr);
            s.x = 2;
        } spec {
            pragma opaque;
            aborts_if !exists<S>(addr);
            ensures global<S>(addr).x == 2;
        })
    }

    fun call_mutate_inline_caller_no_modifies(addr: address) acquires S {
        mutate_at(addr, |addr| {
            let s = borrow_global_mut<S>(addr);
            s.x = 2;
        } spec {
            pragma opaque;
            modifies global<S>(addr);
            aborts_if !exists<S>(addr);
            ensures global<S>(addr).x == 2;
        })
    }
    spec call_mutate_inline_caller_no_modifies {
        pragma opaque;
        aborts_if !exists<S>(addr);
        ensures global<S>(addr).x == 2;
    }

}
}
