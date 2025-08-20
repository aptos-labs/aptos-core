module 0xc0ffee::m {
    fun bar() {}

    //1: Redundant with && — Le + Lt (x <= 400 && x < 500)
    public fun test1_warn(x: u64) {
        if (x <= 400 && x < 500) { bar() };
    }

    //1b: reverse order — Lt + Le (x < 500 && x <= 400)
    public fun test1b_warn(x: u64) {
        if (x < 500 && x <= 400) { bar() };
    }

    // 2: Contradiction with && — Le + Gt (x <= 400 && x > 500)
    public fun test2_warn(x: u64) {
        if (x <= 400 && x > 500) { bar() };
    }

    // 2b: reverse order — Gt + Le (x > 500 && x <= 400)
    public fun test2b_warn(x: u64) {
        if (x > 500 && x <= 400) { bar() };
    }

    // 3: Redundant with || — Gt + Ge (x > 10 || x >= 5)
    public fun test3_warn(x: u64) {
        if (x > 10 || x >= 5) { bar() };
    }

    // 3b: reverse order — Ge + Gt (x >= 5 || x > 10)
    public fun test3b_warn(x: u64) {
        if (x >= 5 || x > 10) { bar() };
    }

    // 4: Redundant with || — Lt + Le (x < 5 || x <= 10)
    public fun test5_warn(x: u64) {
        if (x < 5 || x <= 10) { bar() };
    }

    // 4b: reverse order — Le + Lt (x <= 10 || x < 5)
    public fun test5b_warn(x: u64) {
        if (x <= 10 || x < 5) { bar() };
    }

    // 5: Tautology with || — Lt + Ge (x < 5 || x >= 5)
    public fun test4_warn(x: u64) {
        if (x < 5 || x >= 5) { bar() };
    }

    // 5b: reverse order — Ge + Lt (x >= 5 || x < 5)
    public fun test4b_warn(x: u64) {
        if (x >= 5 || x < 5) { bar() };
    }

    // 6: Tautology with || — Le + Gt (x <= 5 || x > 5)
    public fun test6_warn(x: u64) {
        if (x <= 5 || x > 5) { bar() };
    }

    // 6b: reverse order — Gt + Le (x > 5 || x <= 5)
    public fun test6b_warn(x: u64) {
        if (x > 5 || x <= 5) { bar() };
    }

    // 7: Redundant with && — Eq + Lt (x == 5 && x < 10)
    public fun test7_warn(x: u64) {
        if (x == 5 && x < 10) { bar() };
    }

    // 7b: reverse order — Lt + Eq (x < 10 && x == 5)
    public fun test7b_warn(x: u64) {
        if (x < 10 && x == 5) { bar() };
    }

    // 8: Redundant with && — Eq + Le (x == 5 && x <= 5)
    public fun test8_warn(x: u64) {
        if (x == 5 && x <= 5) { bar() };
    }

    // 8b: reverse order — Le + Eq (x <= 5 && x == 5)
    public fun test8b_warn(x: u64) {
        if (x <= 5 && x == 5) { bar() };
    }

    // 9: Redundant with && — Eq + Gt (x == 5 && x > 3)
    public fun test9_warn(x: u64) {
        if (x == 5 && x > 3) { bar() };
    }

    // 9b: reverse order — Gt + Eq (x > 3 && x == 5)
    public fun test9b_warn(x: u64) {
        if (x > 3 && x == 5) { bar() };
    }

    // 10: Redundant with && — Eq + Ge (x == 5 && x >= 0)
    public fun test10_warn(x: u64) {
        if (x == 5 && x >= 0) { bar() };
    }

    // 10b: reverse order — Ge + Eq (x >= 0 && x == 5)
    public fun test10b_warn(x: u64) {
        if (x >= 0 && x == 5) { bar() };
    }

    // 11: Redundant with && — Eq + Neq (x == 5 && x != 6)
    public fun test11_warn(x: u64) {
        if (x == 5 && x != 6) { bar() };
    }

    // 11b: reverse order — Neq + Eq (x != 6 && x == 5)
    public fun test11b_warn(x: u64) {
        if (x != 6 && x == 5) { bar() };
    }

    // 12: Contradiction with && — Eq + Neq (x == 5 && x != 5)
    public fun test12_warn(x: u64) {
        if (x == 5 && x != 5) { bar() };
    }

    // 12b: reverse order — Neq + Eq (x != 5 && x == 5)
    public fun test12b_warn(x: u64) {
        if (x != 5 && x == 5) { bar() };
    }

    // 13: Redundant with && — Lt + Neq (x < 10 && x != 10)
    public fun test13_warn(x: u64) {
        if (x < 10 && x != 10) { bar() };
    }

    // 13b: reverse order — Neq + Lt (x != 10 && x < 10)
    public fun test13b_warn(x: u64) {
        if (x != 10 && x < 10) { bar() };
    }

    // 14: Redundant with && — Gt + Neq (x > 10 && x != 10)
    public fun test14_warn(x: u64) {
        if (x > 10 && x != 10) { bar() };
    }

    // 14b: reverse order — Neq + Gt (x != 10 && x > 10)
    public fun test14b_warn(x: u64) {
        if (x != 10 && x > 10) { bar() };
    }

    // 15: Redundant with && — Le + Le (x <= 5 && x <= 6)
    public fun test15_warn(x: u64) {
        if (x <= 5 && x <= 6) { bar() };
    }

    // 15b: reverse order — Le + Le (x <= 6 && x <= 5)
    public fun test15b_warn(x: u64) {
        if (x <= 6 && x <= 5) { bar() };
    }

    // 16: Redundant with && — Lt + Lt (x < 5 && x < 6)
    public fun test16_warn(x: u64) {
        if (x < 5 && x < 6) { bar() };
    }

    // 16b: reverse order — Lt + Lt (x < 6 && x < 5)
    public fun test16b_warn(x: u64) {
        if (x < 6 && x < 5) { bar() };
    }

    // 17: Redundant with && — Ge + Ge (x >= 6 && x >= 5)
    public fun test17_warn(x: u64) {
        if (x >= 6 && x >= 5) { bar() };
    }

    // 17b: reverse order — Ge + Ge (x >= 5 && x >= 6)
    public fun test17b_warn(x: u64) {
        if (x >= 5 && x >= 6) { bar() };
    }

    // 18: Redundant with && — Gt + Gt (x > 6 && x > 5)
    public fun test18_warn(x: u64) {
        if (x > 6 && x > 5) { bar() };
    }

    // 18b: reverse order — Gt + Gt (x > 5 && x > 6)
    public fun test18b_warn(x: u64) {
        if (x > 5 && x > 6) { bar() };
    }

    // 19: Contradiction with && — Lt + Ge (x < 10 && x >= 10)
    public fun test19_warn(x: u64) {
        if (x < 10 && x >= 10) { bar() };
    }

    // 19b: reverse order — Ge + Lt (x >= 10 && x < 10)
    public fun test19b_warn(x: u64) {
        if (x >= 10 && x < 10) { bar() };
    }

    // 20: Contradiction with && — Eq + Eq (x == 5 && x == 6)
    public fun test20_warn(x: u64) {
        if (x == 5 && x == 6) { bar() };
    }

    // 21: Redundant with && — Ge + Gt (x >= 10 && x > 5)
    public fun test21_warn(x: u64) {
        if (x >= 10 && x > 5) { bar() };
    }

    // 21b: reverse order — Gt + Ge (x > 5 && x >= 10)
    public fun test21b_warn(x: u64) {
        if (x > 5 && x >= 10) { bar() };
    }

    // 22: Redundant with && — Neq + Neq (x != 5 && x != 5)
    public fun test22_warn(x: u64) {
        if (x != 5 && x != 5) { bar() };
    }

    // Skip lint
    #[lint::skip(redundant_comparison)]
    public fun test23_warn(x: u64) {
        if (x < 5 || x <= 10) { bar() };
    }

    // Redundant cases that are not detected

    // Missed redundant case due to nesting on the left (And): x < 6 is implied by x <= 5
    public fun and_nested_left(x: u64) {
        let y = 100;
        if ((x <= 5 && x < y) && x < 6) { bar() };
    }

    // Missed redundant case due to nesting on the right (And): x < 6 is implied by x <= 5
    public fun and_nested_right(x: u64) {
        let y = 100;
        if (x < 6 && (x <= 5 && x < y)) { bar() };
    }

    // Missed redundant case due to nesting on the left (Or): x >= 5 is redundant given x > 10
    public fun or_nested_left(x: u64) {
        let y = 100;
        if ((x > 10 || x > y) || x >= 5) { bar() };
    }

    // Missed redundant case due to nesting on the right (Or): x >= 5 is redundant given x > 10
    public fun or_nested_right(x: u64) {
        let y = 100;
        if (x >= 5 || (x > 10 || x > y)) { bar() };
    }
}
