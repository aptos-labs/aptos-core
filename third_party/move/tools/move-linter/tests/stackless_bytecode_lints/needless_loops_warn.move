module 0x1::needless_loops {
    // Positive cases (should warn)
    fun test_warn_1() {
        let x = 1;
        loop {
            break
        };
    }

    fun test_warn_2(): u64 {
        loop {
            return 42
        }
    }

    fun test_warn_3() {
        loop {
            abort 1
        }
    }

    fun test_warn_4(cond: bool) {
        loop { if (cond) { break } else { break } };
    }

    fun test_warn_5(): u64 {
        loop { if (true) { return 1 } else { return 2 } }
    }

    fun test_warn_6() {
        loop { if (true) { abort 1 } else { abort 2 } }
    }

    fun test_warn_7(): u64 {
        let z = 0;
        loop {
            z = z + 1;
            break;
        };
        z
    }

    fun test_warn_8(cond: bool): u64 {
        let x = 0;
        loop {
            if (cond) {
                x = x + 1;
                break;
            } else {
                x = x + 2;
                break;
            }
        };
        x
    }

    fun test_warn_9(): u64 {
        let i = 0;
        loop {
            if (i > 5) break;
            return 42
        };
        return i
    }

    // Lint skip attribute
    #[lint::skip(needless_loops)]
    fun test_skip_1() {
        loop {
            abort 1
        }
    }

    // Negative cases (should NOT warn)
    fun test_no_warn_1(i0: u64): u64 {
        let i = i0;
        let x = 0;
        loop {
            if (i > 0) { x = x + 1 };
            if (x > 10) {
                i = i - 1;
            };
            if (i == 0) { break };
        };
        x
    }

    fun test_no_warn_2(cond: bool): u64 {
        let y = 0;
        loop {
            if (cond) { return 1 } else { y = y + 1 };
        };
        y
    }

    fun test_no_warn_3(): u64 {
        let i = 0;
        loop {
            if (i >= 10) break;
        };
        i
    }

    fun test_no_warn_4(condition: bool) {
        loop {
            if (condition) {
                break
            };
            // Some other logic could be here
        }
    }
}
