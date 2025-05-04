//# publish
module 0xc0ffee::n {
    /// Returns a function that returns a function that adds a captured value
    fun create_adder(base: u64): |u64|(|u64|u64) has copy {
        |x| |y| base + x + y
    }

    /// Returns a function that returns a function that multiplies by a captured value
    fun create_multiplier(factor: u64): |u64|(|u64|u64) has copy {
        |x| |y| factor * x * y
    }

    /// Test nested function values with variable capture
    fun test_nested_functions() {
        // Create a function that adds 10 to its input
        let add_10 = create_adder(10);
        let add_10_5 = add_10(5); // Now adds 10 + 5 + input
        assert!(add_10_5(3) == 18, 0); // 10 + 5 + 3 = 18

        // Create a function that multiplies by 2
        let multiply_by_2 = create_multiplier(2);
        let multiply_by_2_3 = multiply_by_2(3); // Now multiplies by 2 * 3 * input
        assert!(multiply_by_2_3(4) == 24, 1); // 2 * 3 * 4 = 24

        // Test nested captures
        let outer = 5;
        let middle = 3;
        let inner = |x| |y| outer * middle * x * y;
        let inner_func = inner(2);
        assert!(inner_func(4) == 120, 2); // 5 * 3 * 2 * 4 = 120
    }

    /// Test function value composition with captures
    fun test_composed_captures() {
        let base = 2;
        let multiplier = |x| |y| base * x * y;
        let adder = |x| |y| base + x + y;

        // Compose multiplier and adder
        let composed = |x| |y| multiplier(x)(adder(x)(y));
        let result = composed(3)(4);
        assert!(result == 54, 0); // 2 * 3 * (2 + 3 + 4) = 54
    }
}

//# run 0xc0ffee::n::test_nested_functions

//# run 0xc0ffee::n::test_composed_captures
