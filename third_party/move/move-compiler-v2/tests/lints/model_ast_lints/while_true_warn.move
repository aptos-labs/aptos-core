module 0xc0ffee::m {
    public fun test_warn_1(x: u64) {
        while (true) {
            if (x > 10) { break; } else { test_warn_1(x + 1); }
        }
    }

    public fun test_warn_2(x: u64) {
        let i = x;
        let __update_iter_flag = false;
        while (true) {
            if (__update_iter_flag) { i = i + 1; } else { __update_iter_flag = true; }
        }
    }

    public fun test_no_warn_1(x: u64) {
        loop {
            if (x > 10) { break; } else { test_no_warn_1(x + 1); }
        }
    }

    public fun test_no_warn_2(x: u64) {
        for (i in x..10) {
            test_no_warn_2(i);
        }
    }
}

module 0xc0ffee::no_warn {
    #[lint::skip(while_true)]
    public fun test_warn_1(x: u64) {
        while (true) {
            if (x > 10) { break; } else { test_warn_1(x + 1); }
        }
    }
}
