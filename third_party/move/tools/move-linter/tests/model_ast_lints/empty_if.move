module 0xc0ffee::m {

    public fun empty_if(x: u64) {
        if (x > 0) {
        } else {
            abort(0)
        };
    }

    public fun empty_if_wo_else(x: u64) {
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

    #[lint::skip(empty_if)]
    public fun skip_empty_if(x: u64) {
        if (x > 0) {
        } else {
            abort(0)
        };
    }

}
