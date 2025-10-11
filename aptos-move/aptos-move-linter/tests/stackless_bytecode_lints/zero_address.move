module 0xc0ffee::m {

    public fun example_zero_address(p: address, warn_q: address): address {
        if (p == @0x0) {
            return p
        };
        return warn_q
    }

    public fun example_unchecked_param(warn_p: address): u32 {
        if (warn_p == @0x1) {
            return 1
        };
        2
    }

    public fun example_2_checked_param(p: address): u32 {
        if (p == @0x0) {
            return 2
        };
        if (p == @0x1) {
            return 1
        };
        3
    }

    public fun example_2_checked_param_false_positive(p: address): u32 {
        if (p == @0x1) { // This could trigger a warning, the linter
            return 1     // does not track an order of checks
        };
        if (p == @0x0) {
            return 2
        };
        3
    }


    public fun classic_check_neq(p: address){
        if (p == @0x0) {
            consume(p);
        };
    }

    public fun classic_check_eq(p: address){
        if (p != @0x0) {
            return
        };
        consume(p);
    }

    public fun no_check(warn_p: address){
        consume(warn_p);
    }

    fun consume(_: address) {}

}
