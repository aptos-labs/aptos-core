/// Provides a framework for comparing two elements
module AptosFramework::Comparator {
    use Std::BCS;
    use Std::Vector;

    const EQUAL: u8 = 0;
    const SMALLER: u8 = 1;
    const GREATER: u8 = 2;

    struct Result has drop {
        inner: u8,
    }

    public fun is_equal(result: &Result): bool {
        return result.inner == EQUAL
    }

    public fun is_smaller_than(result: &Result): bool {
        return result.inner == SMALLER
    }

    public fun is_greater_than(result: &Result): bool {
        result.inner == GREATER
    }

    // Performs a comparison of two types after BCS serialization.
    public fun compare<T>(left: &T, right: &T): Result {
        let left_bytes = BCS::to_bytes(left);
        let right_bytes = BCS::to_bytes(right);

        compare_u8_vector(left_bytes, right_bytes)
    }

    // Performs a comparison of two vector<u8>s or byte vectors
    public fun compare_u8_vector(left: vector<u8>, right: vector<u8>): Result {
        let left_length = Vector::length(&left);
        let right_length = Vector::length(&right);

        let idx = 0;

        while (idx < left_length && idx < right_length) {
            let left_byte = *Vector::borrow(&left, idx);
            let right_byte = *Vector::borrow(&right, idx);

            if (left_byte < right_byte) {
                return Result { inner: SMALLER }
            } else if (left_byte > right_byte) {
                return Result { inner: GREATER }
            };
            idx = idx + 1;
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
        use Std::ASCII;

        let value0 = ASCII::string(b"alpha");
        let value1 = ASCII::string(b"beta");
        let value2 = ASCII::string(b"betaa");

        assert!(is_equal(&compare(&value0, &value0)), 0);
        assert!(is_equal(&compare(&value1, &value1)), 1);
        assert!(is_equal(&compare(&value2, &value2)), 2);

        assert!(is_greater_than(&compare(&value0, &value1)), 3);
        assert!(is_smaller_than(&compare(&value1, &value0)), 4);

        assert!(is_smaller_than(&compare(&value0, &value2)), 5);
        assert!(is_greater_than(&compare(&value2, &value0)), 6);

        assert!(is_smaller_than(&compare(&value1, &value2)), 7);
        assert!(is_greater_than(&compare(&value2, &value1)), 8);
    }

    #[test]
    public fun test_u128() {
        let value0: u128 = 5;
        let value1: u128 = 152;
        let value2: u128 = 511; // 0x1ff

        assert!(is_equal(&compare(&value0, &value0)), 0);
        assert!(is_equal(&compare(&value1, &value1)), 1);
        assert!(is_equal(&compare(&value2, &value2)), 2);

        assert!(is_smaller_than(&compare(&value0, &value1)), 2);
        assert!(is_greater_than(&compare(&value1, &value0)), 3);

        assert!(is_smaller_than(&compare(&value0, &value2)), 3);
        assert!(is_greater_than(&compare(&value2, &value0)), 4);

        assert!(is_smaller_than(&compare(&value1, &value2)), 5);
        assert!(is_greater_than(&compare(&value2, &value1)), 6);
    }

    #[test_only]
    struct Complex has drop {
        value0: vector<u128>,
        value1: u8,
        value2: u64,
    }

    #[test]
    public fun test_complex() {
        let value0_0 = Vector::empty();
        Vector::push_back(&mut value0_0, 10);
        Vector::push_back(&mut value0_0, 9);
        Vector::push_back(&mut value0_0, 5);

        let value0_1 = Vector::empty();
        Vector::push_back(&mut value0_1, 10);
        Vector::push_back(&mut value0_1, 9);
        Vector::push_back(&mut value0_1, 5);
        Vector::push_back(&mut value0_1, 1);

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

        assert!(is_equal(&compare(&base, &base)), 0);
        assert!(is_smaller_than(&compare(&base, &other_0)), 1);
        assert!(is_greater_than(&compare(&other_0, &base)), 2);
        assert!(is_smaller_than(&compare(&base, &other_1)), 3);
        assert!(is_greater_than(&compare(&other_1, &base)), 4);
        assert!(is_smaller_than(&compare(&base, &other_2)), 5);
        assert!(is_greater_than(&compare(&other_2, &base)), 6);
    }
}
