module 0xc0ffee::m {
    fun foo(x: u64): bool {
        x > 10
    }

    fun bar() {}

    /****** Cases with warnings *****/

    // Simple nested if case
    public fun test_warn_1(a: bool, b: bool) {
        if (a) {
            if (b) {
                bar();
            }
        }
    }

    // Nested if with function calls in conditions
    public fun test_warn_2(x: u64, y: u64) {
        if (foo(x)) {
            if (foo(y)) {
                bar();
            }
        }
    }

    // Nested if with complex conditions
    public fun test_warn_3(x: u64, y: u64) {
        if (x > 5) {
            if (y < 20) {
                bar();
            }
        }
    }

    // Multiple levels of nesting (should warn on the first level)
    public fun test_warn_4(a: bool, b: bool, c: bool) {
        if (a) {
            if (b) {
                if (c) {
                    bar();
                }
            }
        }
    }

    // /****** Cases without warnings *****/

    // Outer if has else clause
    public fun test_no_warn_1(a: bool, b: bool) {
        if (a) {
            if (b) {
                bar();
            }
        } else {
            bar();
        }
    }

    // Inner if has else clause
    public fun test_no_warn_2(a: bool, b: bool) {
        if (a) {
            if (b) {
                bar();
            } else {
                bar();
            }
        }
    }

    // Both ifs have else clauses
    public fun test_no_warn_3(a: bool, b: bool) {
        if (a) {
            if (b) {
                bar();
            } else {
                bar();
            }
        } else {
            bar();
        }
    }

    // No nesting - just a single if
    public fun test_no_warn_4(a: bool) {
        if (a) {
            bar();
        }
    }

    // Different statements between ifs
    public fun test_no_warn_5(a: bool, b: bool) {
        if (a) {
            bar();
            if (b) {
                bar();
            }
        }
    }

    #[lint::skip(nested_if)]
    public fun test_no_warn_6(a: bool, b: bool) {
        if (a) {
            if (b) {
                bar();
            }
        }
    }
}
