module 0xc0ffee::m {

    public fun empty_if(x: u64) {
        if (x > 0) {
        } else {
            return;
        };
    }

    public fun empty_if_no_else(x: u64) {
        if (x > 0) {
        };
    }

    public fun non_empty_if(x: u64): u64 {
        if (x > 0) {
            x = x + 1;
        };
        x
    }

    public fun empty_if_with_void(x: u64): u64 {
        if (x > 0) {
            ()
        };
        x
    }

    public fun empty_if_with_empty_else(x: u64): u64 {
        if (x > 35) {
        } else { };
        0
    }

    public fun empty_if_with_non_empty_else(x: u64): u64 {
        if (x > 35) {
        } else {
            return 2;
         };
        0
    }

    public fun aborts(x: u64) {
        assert!(x > 10);
    }

    #[lint::skip(empty_if)]
    public fun skip_empty_if(x: u64) {
        if (x > 35) {
        } else { };
    }

    public fun dont_flag_abort_on_else(x: u64) {
        if (x > 0) {
        } else {
            abort(0)
        };
    }

}
