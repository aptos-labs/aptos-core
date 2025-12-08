module 0xc0ffee::m {
    struct A(bool) has key, drop;
    struct B(bool) has key, drop;


    public fun no_warn_1(): bool {
        exists<A>(@0xc0ffee) && exists<B>(@0xc0ffee)
    }

    public fun no_warn_2(): bool {
        borrow_global<A>(@0xc0ffee).0 && borrow_global<B>(@0xc0ffee).0
    }

    public fun no_warn_3(): bool {
        move_from<A>(@0xc0ffee).0 && move_from<B>(@0xc0ffee).0
    }

    public fun warn_1(): bool {
        exists<A>(@0xc0ffee) && exists<A>(@0xc0ffee)
    }

    public fun warn_2(): bool {
        borrow_global<A>(@0xc0ffee).0 && borrow_global<A>(@0xc0ffee).0
    }

    public fun warn_3(): bool {
        move_from<A>(@0xc0ffee).0 && move_from<A>(@0xc0ffee).0
    }
}
