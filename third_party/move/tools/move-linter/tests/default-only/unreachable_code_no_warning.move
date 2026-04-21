// Functions with no unreachable code — no diagnostics expected.
module 0xc0ffee::m {
    public fun early_return(x: u64): u64 {
        if (x == 0) {
            return 0
        };
        x + 1
    }

    public fun simple(): u64 {
        42
    }

    public fun loops_and_breaks(x: u64): u64 {
        let y = 0;
        while (y < x) {
            y = y + 1;
            if (y == 5) break;
        };
        y
    }
}
