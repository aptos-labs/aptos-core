module 0xc0ffee::o {
    const ERROR_CODE_2: u64 = 41;
    // inlined in n
    // not inlined in m
    public fun not_inlined_o(x: &mut u64) {
        if (* x > 5) {
            assert!(*x > 5, ERROR_CODE_2);
            *x = 42;
        } else {
            *x = *x + 1;
        }
    }
}


module 0xc0ffee::n {
    const ERROR_CODE_2: u64 = 42;
    const ERROR_CODE_3: u64 = 43;
    const ERROR_CODE_4: u64 = 44;
    const ERROR_CODE_5: u64 = 45;

    // not inlined in m
    public fun not_inlined(x: &mut u64) {
        if (* x > 5) {
            assert!(*x > 5, ERROR_CODE_2);
            *x = 42;
        } else {
            *x = *x + 1;
        }
    }

    // inlined in m
    public fun inlined(x: &mut u64) {
        if (* x > 5) {
            assert!(*x > 5, ERROR_CODE_3);
            *x = 42;
        } else {
            assert!(*x > 5, ERROR_CODE_4);
            *x = *x + 1;
        }
    }

    // inlined in m
    public fun indirect_inlined(x: &mut u64) {
        use 0xc0ffee::o;
        // not inlined in m
        o::not_inlined_o(x);
    }

    public fun not_inlined_non_constant_error_code(x: &mut u64) {
        use std::error;
        assert!(*x > 5, error::invalid_argument(ERROR_CODE_2));
        *x = 42;
    }

    public fun not_inlined_non_constant_error_code_2(x: &mut u64) {
        use std::error;
        assert!(*x > 5, error::canonical(0xD, ERROR_CODE_2));
        *x = 42;
    }

    public fun not_inlined_non_constant_error_code_3(x: &mut u64) {
        use std::error;
        assert!(*x > 5, error::canonical(0xC, ERROR_CODE_5));
        *x = 42;
    }
}


module 0xc0ffee::m {
    const ERROR_CODE: u64 = 42;
    const ERROR_CODE_2: u64 = 41;
    const N_ERROR_CODE_2: u64 = 43;
    const E_CANONICAL_ERROR_CODE: u64 = 0xD00045;

    fun one(x: &mut u64) {
        if (* x > 5) {
            assert!(*x > 5, ERROR_CODE);
            *x = 42;
        } else {
            *x = *x + N_ERROR_CODE_2;
        }
    }

    public fun two(): u64 {
        2
    }

    public fun compute(): u64 {
        use 0xc0ffee::n;
        let x = 3;
        // not inlined
        n::not_inlined(&mut x);
        // inlined
        n::inlined(&mut x);
        // inlined
        n::indirect_inlined(&mut x);
        // inlined
        one(&mut x);
        // not inlined
        n::not_inlined_non_constant_error_code(&mut x);
        // not inlined
        n::not_inlined_non_constant_error_code_2(&mut x);
        // not inlined
        n::not_inlined_non_constant_error_code_3(&mut x);
        x + two()
    }
}
