module 0x2::ModifiesSpecFun {
    struct R has key { val: u64 }

    public fun get_addr(a: address): address { a }

    spec fun incr_R(a: address): bool modifies R[get_addr(a)] {
        R[get_addr(a)].val == old(R[get_addr(a)].val) + 1
    }

    fun do_incr(a: address) acquires R {
        R[a].val = R[a].val + 1;
    }
    spec do_incr {
        ensures incr_R(a);
    }
}
