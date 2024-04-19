//# publish
module 0x42::m {
    fun assert_approx_the_same(x: u256, y: u256, precision: u128) {
        if (x < y) {
            let tmp = x;
            x = y;
            y = tmp;
        };
        let mult = 1u256;
        let n = 10u256;
        while (precision > 0) {
            if (precision % 2 == 1) {
                mult = mult * n;
            };
            precision = precision / 2;
            n = n * n; // previous bug: alive in same instr but not after
        };
        assert!((x - y) * mult < x, 0);
    }

    fun exec() {
        assert_approx_the_same(5672, 5672, 0)
    }
}

//# run 0x42::m::exec
