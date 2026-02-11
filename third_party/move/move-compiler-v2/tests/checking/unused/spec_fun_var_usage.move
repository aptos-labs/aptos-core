module 0x42::m {
    // This struct is used in spec function and should NOT be warned as unused
    struct UsedInSpecFun has drop { value: u64 }

    // This struct is used in spec variable and should NOT be warned as unused
    struct UsedInSpecVar has drop { count: u64 }

    // This struct is NOT used anywhere and SHOULD be warned as unused
    struct ReallyUnused has drop { data: u64 }

    // Regular function that will be checked
    public fun test_function(x: u64): u64 {
        x + 1
    }

    spec test_function {
        // Use the spec function in a condition
        ensures result == helper_spec_fun(x);
    }

    // Spec function that uses UsedInSpecFun
    spec fun helper_spec_fun(val: u64): u64 {
        let s = UsedInSpecFun { value: val };
        s.value + 1
    }

    // Spec variable that uses UsedInSpecVar
    spec module {
        global counter: UsedInSpecVar;
    }

    // Another spec function to test signature type tracking
    spec fun process_data(data: UsedInSpecVar): u64 {
        data.count
    }

    // Spec variable with initializer
    spec module {
        global initialized: UsedInSpecFun = UsedInSpecFun { value: 0 };
    }
}
