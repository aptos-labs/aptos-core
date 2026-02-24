//# publish
module 0x42::test {
    use std::option;
    use std::option::Option;

    // Test capturing Option<u64> value in a closure and accessing it
    public entry fun test_capture_some() {
        let opt = option::some(42u64);

        // Create a closure that captures the Option value
        let check_value = || {
            // Access the captured Option value
            option::is_some(&opt) && *option::borrow(&opt) == 42
        };

        // Exercise the closure - it accesses the captured Option
        assert!(check_value(), 1);
    }

    // Test capturing None value
    public entry fun test_capture_none() {
        let opt: Option<u64> = option::none();

        // Create a closure that captures the None value
        let check_none = || {
            option::is_none(&opt)
        };

        // Exercise the closure
        assert!(check_none(), 2);
    }

    // Test capturing and extracting value from Option in closure
    public entry fun test_capture_and_extract() {
        let opt = option::some(100u64);

        // Create a closure that captures and extracts the value
        let get_double = || {
            if (option::is_some(&opt)) {
                *option::borrow(&opt) * 2
            } else {
                0
            }
        };

        // Exercise the closure
        let result = get_double();
        assert!(result == 200, 3);
    }

    // Test capturing Option with different values and comparing
    public entry fun test_capture_and_compare() {
        let opt1 = option::some(10u64);
        let opt2 = option::some(20u64);

        // Create closure that captures both Options and compares them
        let compare = || {
            if (option::is_some(&opt1) && option::is_some(&opt2)) {
                *option::borrow(&opt1) < *option::borrow(&opt2)
            } else {
                false
            }
        };

        assert!(compare(), 5);
    }

    // Test with generic Option capturing
    struct Data has drop, copy {
        value: u64
    }

    public entry fun test_capture_generic_option() {
        let opt = option::some(Data { value: 123 });

        // Closure captures Option<Data>
        let check_data = || {
            if (option::is_some(&opt)) {
                let data = option::borrow(&opt);
                data.value == 123
            } else {
                false
            }
        };

        assert!(check_data(), 6);
    }

    // Test passing closure with captured Option to another function
    fun apply_validator(validator: || bool): bool {
        validator()
    }

    public entry fun test_pass_closure_with_captured_option() {
        let opt = option::some(99u64);

        // Create closure that captures Option
        let check = || {
            option::is_some(&opt) && *option::borrow(&opt) == 99
        };

        // Pass the closure to another function and exercise it
        assert!(apply_validator(check), 7);
    }

    // Test multiple closures capturing the same Option
    public entry fun test_multiple_closures_same_option() {
        let opt = option::some(50u64);

        // Create multiple closures capturing the same Option
        let is_some = || option::is_some(&opt);
        let get_value = || {
            if (option::is_some(&opt)) {
                *option::borrow(&opt)
            } else {
                0
            }
        };
        let is_fifty = || {
            if (option::is_some(&opt)) {
                *option::borrow(&opt) == 50
            } else {
                false
            }
        };

        // Exercise all closures
        assert!(is_some(), 8);
        assert!(get_value() == 50, 9);
        assert!(is_fifty(), 10);
    }

    // Resource struct (has `key` ability) containing a closure that captures an Option
    struct Validator has key {
        // Closure field that captures an Option value
        // The closure needs `store` ability to be in a resource
        check: |u64|bool has copy+drop+store
    }

    #[persistent] fun foo(value: u64, threshold_opt: Option<u64>): bool {
        if (option::is_some(&threshold_opt)) {
                value >= *option::borrow(&threshold_opt)
        } else {
                false  // If no threshold, reject all
        }
    }

    public entry fun test_resource_with_captured_option(account: &signer) {
        let threshold_opt = option::some(100u64);

        let f: |u64|bool has copy+drop+store = |y| foo(y, threshold_opt);

        // Store the closure in a resource (struct with `key` ability)
        move_to(account, Validator { check: f });
    }

    public entry fun test_use_resource_with_captured_option(account: &signer) acquires Validator {
        // First, create and store the validator resource
        test_resource_with_captured_option(account);

        // Now use the stored closure from the resource
        let validator = borrow_global<Validator>(@0x42);

        // Exercise the closure - it should access the captured Option value
        assert!((validator.check)(100), 11);  // 100 >= 100, should pass
        assert!((validator.check)(200), 12);  // 200 >= 100, should pass
        assert!(!(validator.check)(50), 13);  // 50 < 100, should fail
    }
}

//# run 0x42::test::test_capture_some

//# run 0x42::test::test_capture_none

//# run 0x42::test::test_capture_and_extract

//# run 0x42::test::test_capture_and_compare

//# run 0x42::test::test_capture_generic_option

//# run 0x42::test::test_pass_closure_with_captured_option

//# run 0x42::test::test_multiple_closures_same_option

//# run --signers 0x42 -- 0x42::test::test_use_resource_with_captured_option
