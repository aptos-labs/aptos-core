module 0xc0ffee::m {
    struct W(u64);

    public inline fun destroy1(w: W) {
        let W(_) = w;
    }

    package inline fun destroy2(w: W) {
        let W(_) = w;
    }

    public inline fun get_field(w: W): u64 {
        w.0
    }
}

module 0xc0ffee::n {
    enum E {
        A(u64),
        B(u64),
    }

    public(friend) inline fun fetch(e: E): u64 {
        match (e) {
            E::A(x) => x,
            E::B(x) => x,
        }
    }

    inline fun no_warn(): E {
        E::A(0)
    }
}

module 0xc0ffee::o {
    struct R has key { f: u64 }

    public inline fun my_borrow(): &R {
        borrow_global<R>(@0xc0ffee)
    }
}
