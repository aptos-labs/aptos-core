//#publish
module 0xcafe::vectors {
    use std::vector;
    public entry fun guess_flips(flips: vector<u8>) {
        loops_with_break(&flips);
        let _v = copy flips; // this is ok

        // this will fail, a UNKNOWN_INVARIANT_VIOLATION_ERROR (code 2000)
        let _v2 =  flips;
    }

    // no function call
    public entry fun guess_flips_directly(flips: vector<u8>) {
        let i = 0;
        while (i < vector::length(&flips)) {
            if (*vector::borrow(&flips, i) != 0) {
                break
            };
            i = i + 1;
        };
        let _v = copy flips; // this is ok
        let _v2 =  flips; // this is ok
    }

    // call function, no inline, with `break`
    public entry fun guess_with_break_without_inline(flips: vector<u8>) {
        loops_with_break_no_inline(&flips);
        let _v = copy flips; // this is ok
        let _v2 =  flips; // this is ok
    }

    // call `inline` function, without `break`
    public entry fun guess_without_break_with_inline(flips: vector<u8>) {
        loops_without_break(&flips);
        let _v = flips; // this is ok
        let _v2 = copy flips; // this is ok
    }


    inline fun loops_with_break(flips: &vector<u8>) {
        let i = 0;
        while (i < vector::length(flips)) {
            if (*vector::borrow(flips, i) != 0) {
                break
            };
            i = i + 1;
        };
    }

    fun loops_with_break_no_inline(flips: &vector<u8>) {
        let i = 0;
        while (i < vector::length(flips)) {
            if (*vector::borrow(flips, i) != 0) {
                break
            };
            i = i + 1;
        };
    }

    inline fun loops_without_break(flips: &vector<u8>) {
        let i = 0;
        while (i < vector::length(flips)) {
            assert!(*vector::borrow(flips, i) == 0, 3);
            i = i + 1;
        };
    }

    // #[test]
    fun test_guess_with_inline_break() {
        let flips = vector[0, 0, 0,0];
        guess_flips(flips);
    }

    // #[test]
    fun test_guess_directly() {
        let flips = vector[0, 0, 0,0];
        guess_flips_directly(flips);
    }

    // #[test]
    fun test_guess_with_break_no_inline() {
        let flips = vector[0, 0, 0,0];
        guess_with_break_without_inline(flips);
    }

    // #[test]
    fun test_guess_without_break() {
        let flips = vector[0, 0, 0,0];
        guess_without_break_with_inline(flips);
    }
}

//#run 0xcafe::vectors::test_guess_with_inline_break

//#run 0xcafe::vectors::test_guess_directly

//#run 0xcafe::vectors::test_guess_with_break_no_inline

//#run 0xcafe::vectors::test_guess_without_break
