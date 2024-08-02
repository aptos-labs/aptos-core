//# publish
module 0x42::power_function {
    public fun pow(base: u64, exponent: u64): u64 {
        if (exponent == 0) {
            return 1
        } else {
            let result = base;
            let i = 1;
            while (i < exponent) {
                result = result * base;
                i = i + 1;
            };
            result
        }
    }

    public fun test_pow() {
        assert!(pow(2, 0) == 1, 0);
        assert!(pow(2, 1) == 2, 1);
        assert!(pow(2, 2) == 4, 2);
        assert!(pow(2, 3) == 8, 3);
        assert!(pow(3, 3) == 27, 4);
        assert!(pow(5, 4) == 625, 5);
    }
}

//# run 0x42::power_function::test_pow
