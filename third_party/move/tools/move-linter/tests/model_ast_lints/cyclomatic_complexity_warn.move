module 0xc0ffee::complexity {
    public fun high_complexity(a: bool, b: bool): bool {
        assert!(a || b, 100);
        if (a) {
            return b;
        };

        if (b) {
            return a;
        };

        let c = a && b;

        loop {
            if (c) {
                return c;
            } else {
                if (a || b) {
                    return a;
                };
            };
            break;
        };
        a
    }


    public fun low_complexity(): bool {
        let b = 1;
        inline_fun(b) == 1
    }


    inline fun inline_fun(b: u64): u64 {
            if (b == 1) {
                1
            }else if (b == 2) {
                0
            }else if (b == 3) {
                1
            }else if (b == 4) {
                0
            }else if (b == 5) {
                1
            }else if (b == 6) {
                0
            }else if (b == 7) {
                1
            }else if (b == 8) {
                0
            }else if (b == 9) {
                1
            }else if (b == 10) {
                0
            }else {
                12
            }
    }
}
