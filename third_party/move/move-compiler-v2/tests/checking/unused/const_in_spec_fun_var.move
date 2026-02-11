module 0x42::m {
    // Constants used in spec functions should NOT be warned as unused
    const SPEC_FUN_CONST: u64 = 100;

    // Constants used in spec variable initializers should NOT be warned as unused
    const SPEC_VAR_CONST: u64 = 200;

    // This constant is truly unused and SHOULD be warned
    const TRULY_UNUSED_CONST: u64 = 300;

    public fun test_function(): u64 {
        42
    }

    // Spec function that uses SPEC_FUN_CONST
    spec fun helper_spec_fun(): u64 {
        SPEC_FUN_CONST + 10
    }

    spec test_function {
        ensures result == helper_spec_fun();
    }

    // Spec variable with initializer that uses SPEC_VAR_CONST
    spec module {
        global initialized_value: u64 = SPEC_VAR_CONST;
    }
}
