module 0xc0ffee::Example {
    use std::option::{Option};

    const E_CONDITION_A: u64 = 1;
    const E_CONDITION_B: u64 = 2;
    const E_CONDITION_C: u64 = 3;
    const E_CONDITION_D: u64 = 4;

    fun validate_parameters(
        param_a: Option<u64>, // Optional first parameter to validate
        param_b: Option<u64>, // Optional second parameter to validate
        reference_value: u64, // Reference value for comparisons
        threshold_value: u64, // Threshold for additional validation
        flag: bool // Indicates a positive or negative condition
    ) {
        // Determine the context based on the flag
        let is_positive = flag;

        // Check and validate param_a if it exists
        if (option::is_some(&param_a)) {
            // Extract the value from the Option
            let value_a = option::extract(&mut param_a);

            // Ensure the value is non-zero
            assert!(value_a > 0, E_CONDITION_A);

            // Validate based on the condition (is_positive)
            let is_valid_a =
                if (is_positive) {
                    value_a > reference_value // For positive condition, value_a must be greater than reference_value
                } else {
                    value_a < reference_value // For negative condition, value_a must be less than reference_value
                };

            // Assert that the validation passed
            assert!(
                is_valid_a,
                if (is_positive) E_CONDITION_B else E_CONDITION_C
            );
        };

        // Check and validate param_b if it exists
        if (option::is_some(&param_b)) {
            // Extract the value from the Option
            let value_b = option::extract(&mut param_b);

            // Ensure the value is non-zero
            assert!(value_b > 0, E_CONDITION_A);

            // Validate based on the condition (is_positive)
            let is_valid_b =
                if (is_positive) {
                    value_b < reference_value // For positive condition, value_b must be less than reference_value
                } else {
                    value_b > reference_value // For negative condition, value_b must be greater than reference_value
                };

            // Assert that the validation passed
            assert!(
                is_valid_b,
                if (is_positive) E_CONDITION_C else E_CONDITION_D
            );

            // Additional validation against the threshold value if it exists
            if (threshold_value > 0) {
                let is_valid_threshold =
                    if (is_positive) {
                        value_b > threshold_value // For positive condition, value_b must be greater than threshold_value
                    } else {
                        value_b < threshold_value // For negative condition, value_b must be less than threshold_value
                    };

                // Assert that the threshold validation passed
                assert!(is_valid_threshold, E_CONDITION_A);
            }
        };
    }

    #[test_only]
    use std::option;

    #[test]
    fun test_validate_parameters() {
        // Passing Invalid param_a for positive condition
        // This should throw E_CONDITION_B error
        validate_parameters(
            option::some(40), // Less than reference_value (60)
            option::some(50),
            60,
            30,
            true
        );
    }
}
