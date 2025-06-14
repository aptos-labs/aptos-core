module 0xc0ffee::m {

    struct Counter has key, store, drop { i: u64 }

    struct NestedCounter has key, store, drop {
        n: Counter
    }

    struct HyperNestedCounter has key, store, drop {
        n: NestedCounter
    }

    public fun test1(x: u64) {
        if (x % x == 2) {
            abort 1;
        };
        if ((x+1) ^ (x+1) == 2) {
            abort 1;
        };
        if (x <= x) {
            abort 1;
        };
        if (x >= x) {
            abort 1;
        };
        if (x == x) {
            abort 1;
        };
        if (x | x == 2) {
            abort 1;
        };
        if (x & x == 2) {
            abort 1;
        };
        if (x / x == 2) {
            abort 1;
        };

        let c = Counter { i: x };
        let nc = NestedCounter { n: c };
        let hnc = HyperNestedCounter { n: nc };

        if (hnc.n.n.i != hnc.n.n.i) {
            abort 1;
        };
        if (x < x) {
            abort 1;
        };
        if (x > x) {
            abort 1;
        };

        if (a_fn(x) > a_fn(x)) { // This should not warn
            abort 1;
        };
    }

    #[lint::skip(equal_operands_in_pure_bin_op)]
    public fun test2(x: u64) {
        if (x % x == 0) {
            abort 1;
        };
    }

    public fun a_fn(a: u64): u64 {
        a
    }


}
