module 0x42::M {

    struct R has key { f: u64 }

    inline fun my_borrow(): &R {
        borrow_global<R>(@0x42)
    }

    public fun test_resource(s: &signer) acquires R {
        move_to<R>(s, R{f:1});
        assert!(my_borrow().f == 1, 1);
    }
}
