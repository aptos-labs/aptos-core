//#publish
module 0xcafe::vectors {
    use std::vector;

    // break
    public entry fun guess_flips_break(flips: vector<u8>) {
        let i = 0;
        let flipsref1 = &flips;
        while (i < vector::length(flipsref1)) {
            if (*vector::borrow(flipsref1, i) != 0) {
                break
            };
            i = i + 1;
        };
        let _v = copy flips; // this is ok
        // this used to fail with an UNKNOWN_INVARIANT_VIOLATION_ERROR (code 2000)
        let _v2 =  flips;
    }

    // assert
    public entry fun guess_flips_abort(flips: vector<u8>) {
        let i = 0;
        let flipsref2 = &flips;
        while (i < vector::length(flipsref2)) {
            assert!(*vector::borrow(flipsref2, i) == 0, 3);
            i = i + 1;
        };
        let _v = copy flips; // this is ok
        let _v2 = flips; // this is ok
    }

    // no check
    public entry fun guess_flips_nocheck(flips: vector<u8>) {
        let i = 0;
        let flipsref3 = &flips;
        while (i < vector::length(flipsref3)) {
            i = i + 1;
        };
        let _v = copy flips; // this is ok
        let _v2 = flips; // this is ok
    }

    // continue
    public entry fun guess_flips_continue(flips: vector<u8>) {
        let i = 0;
        let flipsref4 = &flips;
        while (i < vector::length(flipsref4)) {
            if (*vector::borrow(flipsref4, i) != 0) {
                continue
            };
            i = i + 1;
        };
        let _v = copy flips; // this is ok
        let _v2 = flips; // this is ok
    }

    // multi-break
    public entry fun guess_flips_break2(flips: vector<u8>) {
        let i = 0;
        let flipsref5 = &flips;
        while (i < vector::length(flipsref5)) {
            if (*vector::borrow(flipsref5, i) != 0) {
                break
            };
            i = i + 1;
            if (*vector::borrow(flipsref5, i) == 5) {
                break
            };
        };
        let _v = copy flips; // this is ok
        // this used to fail with an UNKNOWN_INVARIANT_VIOLATION_ERROR (code 2000)
        let _v2 =  flips;
    }

    // #[test]
    fun test_guess_flips_break() {
        let flips = vector[0, 0, 0,0];
        guess_flips_break(flips);
    }

    // #[test]
    fun test_guess_flips_abort() {
        let flips = vector[0, 0, 0,0];
        guess_flips_abort(flips);
    }

    // #[test]
    fun test_guess_flips_nocheck() {
        let flips = vector[0, 0, 0,0];
        guess_flips_nocheck(flips);
    }

    // #[test]
    fun test_guess_flips_continue() {
        let flips = vector[0, 0, 0,0];
        guess_flips_continue(flips);
    }

    // #[test]
    fun test_guess_flips_break2() {
        let flips = vector[0, 0, 0,0];
        guess_flips_break2(flips);
    }

    // direct entry versions of test cases for debugging
    public entry fun entry_test_guess_flips_break() {
        let flips = vector[0, 0, 0,0];
        guess_flips_break(flips);
    }

    public entry fun entry_test_guess_flips_abort() {
        let flips = vector[0, 0, 0,0];
        guess_flips_abort(flips);
    }

    public entry fun entry_test_guess_flips_nocheck() {
        let flips = vector[0, 0, 0,0];
        guess_flips_nocheck(flips);
    }

    public entry fun entry_test_guess_flips_continue() {
        let flips = vector[0, 0, 0,0];
        guess_flips_continue(flips);
    }

    public entry fun entry_test_guess_flips_break2() {
        let flips = vector[0, 0, 0,0];
        guess_flips_break2(flips);
    }
}

//#run 0xcafe::vectors::test_guess_flips_break

//#run 0xcafe::vectors::test_guess_flips_abort

//#run 0xcafe::vectors::test_guess_flips_nocheck

//#run 0xcafe::vectors::test_guess_flips_continue
