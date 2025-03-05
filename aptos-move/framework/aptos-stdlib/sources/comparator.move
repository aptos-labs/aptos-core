/// Provides a framework for comparing two elements
module aptos_std::comparator {
    use std::bcs;

    const EQUAL: u8 = 0;
    const SMALLER: u8 = 1;
    const GREATER: u8 = 2;

    struct Result has drop {
        inner: u8,
    }

    public fun is_equal(self: &Result): bool {
        self.inner == EQUAL
    }

    public fun is_smaller_than(self: &Result): bool {
        self.inner == SMALLER
    }

    public fun is_greater_than(self: &Result): bool {
        self.inner == GREATER
    }

    // Performs a comparison of two types after BCS serialization.
    // BCS uses little endian encoding for all integer types,
    // so comparison between primitive integer types will not behave as expected.
    // For example, 1(0x1) will be larger than 256(0x100) after BCS serialization.
    public fun compare<T>(left: &T, right: &T): Result {
        let left_bytes = bcs::to_bytes(left);
        let right_bytes = bcs::to_bytes(right);

        compare_u8_vector(left_bytes, right_bytes)
    }

    // Performs a comparison of two vector<u8>s or byte vectors
    public fun compare_u8_vector(left: vector<u8>, right: vector<u8>): Result {
        let left_length = left.length();
        let right_length = right.length();

        let idx = 0;

        while (idx < left_length && idx < right_length) {
            let left_byte = left[idx];
            let right_byte = right[idx];

            if (left_byte < right_byte) {
                return Result { inner: SMALLER }
            } else if (left_byte > right_byte) {
                return Result { inner: GREATER }
            };
            idx += 1;
        };

        if (left_length < right_length) {
            Result { inner: SMALLER }
        } else if (left_length > right_length) {
            Result { inner: GREATER }
        } else {
            Result { inner: EQUAL }
        }
    }

    #[test]
    public fun test_strings() {
        use std::string;

        let value0 = string::utf8(b"alpha");
        let value1 = string::utf8(b"beta");
        let value2 = string::utf8(b"betaa");

        assert!(compare(&value0, &value0).is_equal(), 0);
        assert!(compare(&value1, &value1).is_equal(), 1);
        assert!(compare(&value2, &value2).is_equal(), 2);

        assert!(compare(&value0, &value1).is_greater_than(), 3);
        assert!(compare(&value1, &value0).is_smaller_than(), 4);

        assert!(compare(&value0, &value2).is_smaller_than(), 5);
        assert!(compare(&value2, &value0).is_greater_than(), 6);

        assert!(compare(&value1, &value2).is_smaller_than(), 7);
        assert!(compare(&value2, &value1).is_greater_than(), 8);
    }

    #[test]
    #[expected_failure]
    public fun test_integer() {
        // 1(0x1) will be larger than 256(0x100) after BCS serialization.
        let value0: u128 = 1;
        let value1: u128 = 256;

        assert!(compare(&value0, &value0).is_equal(), 0);
        assert!(compare(&value1, &value1).is_equal(), 1);

        assert!(compare(&value0, &value1).is_smaller_than(), 2);
        assert!(compare(&value1, &value0).is_greater_than(), 3);
    }

    #[test]
    public fun test_u128() {
        let value0: u128 = 5;
        let value1: u128 = 152;
        let value2: u128 = 511; // 0x1ff

        assert!(compare(&value0, &value0).is_equal(), 0);
        assert!(compare(&value1, &value1).is_equal(), 1);
        assert!(compare(&value2, &value2).is_equal(), 2);

        assert!(compare(&value0, &value1).is_smaller_than(), 2);
        assert!(compare(&value1, &value0).is_greater_than(), 3);

        assert!(compare(&value0, &value2).is_smaller_than(), 3);
        assert!(compare(&value2, &value0).is_greater_than(), 4);

        assert!(compare(&value1, &value2).is_smaller_than(), 5);
        assert!(compare(&value2, &value1).is_greater_than(), 6);
    }

    #[test_only]
    struct Complex has drop {
        value0: vector<u128>,
        value1: u8,
        value2: u64,
    }

    #[test]
    public fun test_complex() {
        let value0_0 = vector[10, 9, 5];
        let value0_1 = vector[10, 9, 5, 1];

        let base = Complex {
            value0: value0_0,
            value1: 13,
            value2: 41,
        };

        let other_0 = Complex {
            value0: value0_1,
            value1: 13,
            value2: 41,
        };

        let other_1 = Complex {
            value0: copy value0_0,
            value1: 14,
            value2: 41,
        };

        let other_2 = Complex {
            value0: value0_0,
            value1: 13,
            value2: 42,
        };

        assert!(compare(&base, &base).is_equal(), 0);
        assert!(compare(&base, &other_0).is_smaller_than(), 1);
        assert!(compare(&other_0, &base).is_greater_than(), 2);
        assert!(compare(&base, &other_1).is_smaller_than(), 3);
        assert!(compare(&other_1, &base).is_greater_than(), 4);
        assert!(compare(&base, &other_2).is_smaller_than(), 5);
        assert!(compare(&other_2, &base).is_greater_than(), 6);
    }
}
