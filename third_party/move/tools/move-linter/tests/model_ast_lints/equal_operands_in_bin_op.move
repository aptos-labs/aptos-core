module 0xc0ffee::m {
    use std::vector;

    struct Counter has key, store, drop { i: u64 }

    struct NestedCounter has key, store, drop {
        n: Counter
    }

    struct HyperNestedCounter has key, store, drop {
        n: NestedCounter
    }

    const TWO: u64 = 2;

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

        if (TWO == TWO) {
            abort 1;
        };

        if (TWO == 2) {
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

        let b = true;

        if (b || b) { // This should trigger `simpler_boolean_expression` instead of `equal_operands_in_bin_op`
            abort 1;
        };

        if (b && b) { // This should trigger `simpler_boolean_expression` instead of `equal_operands_in_bin_op`
            abort 1;
        };

        if (a_fn(x) > a_fn(x)) { // This should not warn
            abort 1;
        };

        let arr = vector::empty<u64>();
        vector::push_back(&mut arr, x);

        if (arr[0] == arr[0]) { // TODO: This should warn but it needs to support well known functions.
            abort 1;
        };
    }

    #[lint::skip(equal_operands_in_bin_op)]
    public fun test2(x: u64) {
        if (x % x == 0) {
            abort 1;
        };
    }

    public fun a_fn(a: u64): u64 {
        a
    }


}
