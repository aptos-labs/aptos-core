module std::cmp {
    const EQUAL: u8 = 1;
    const LESS_THAN: u8 = 0;
    const GREATER_THAN: u8 = 2;

    enum Ordering has copy, drop {
        /// First value is less than the second value.
        LessThan,
        /// First value is equal to the second value.
        Equal,
        /// First value is greater than the second value.
        GreaterThan,
    }

    public native fun compare<T>(first: &T, second: &T): Ordering;

    public fun is_equal(self: &Ordering): bool {
        self is Ordering::Equal
    }

    public fun is_less_than(self: &Ordering): bool {
        self is Ordering::LessThan
    }

    public fun is_less_or_equal(self: &Ordering): bool {
        !(self is Ordering::GreaterThan)
    }

    public fun is_greater_than(self: &Ordering): bool {
        self is Ordering::GreaterThan
    }

    public fun is__greater_or_equal(self: &Ordering): bool {
        !(self is Ordering::LessThan)
    }

    #[test_only]
    struct SomeStruct has drop {
        field_1: u64,
        field_2: u64,
    }

    #[test_only]
    enum SomeEnum has drop {
        V1 { field_1: u64 },
        V2 { field_2: u64 },
    }

    #[test]
    fun test_compare_numbers() {
        assert!(!is_equal(&compare(&1, &5)), 0);
        assert!(is_less_than(&compare(&1, &5)), 1);
        assert!(is_less_or_equal(&compare(&1, &5)), 2);
        assert!(is_equal(&compare(&5, &5)), 3);
        assert!(!is_less_than(&compare(&5, &5)), 4);
        assert!(is_less_or_equal(&compare(&5, &5)), 5);
        assert!(!is_equal(&compare(&7, &5)), 6);
        assert!(!is_less_than(&compare(&7, &5)), 7);
        assert!(!is_less_or_equal(&compare(&7, &5)), 8);

        assert!(!compare(&1, &5).is_equal(), 0);
        assert!(compare(&1, &5).is_less_than(), 1);
        assert!(compare(&1, &5).is_less_or_equal(), 2);
        assert!(compare(&5, &5).is_equal(), 3);
        assert!(!compare(&5, &5).is_less_than(), 4);
        assert!(compare(&5, &5).is_less_or_equal(), 5);
        assert!(!compare(&7, &5).is_equal(), 6);
        assert!(!compare(&7, &5).is_less_than(), 7);
        assert!(!compare(&7, &5).is_less_or_equal(), 8);
    }

    #[test]
    fun test_compare_structs() {
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 3}) is Ordering::LessThan, 0);
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 1}) is Ordering::GreaterThan, 1);
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 1}) is Ordering::GreaterThan, 2);
    }

    #[test]
    fun test_compare_vector_of_structs() {
        assert!(compare(&vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 3, field_2: 4}], &vector[SomeStruct { field_1: 1, field_2: 3}]) is Ordering::LessThan, 0);
        assert!(compare(&vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 3, field_2: 4}], &vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 1, field_2: 3}]) is Ordering::GreaterThan, 1);
    }

    #[test]
    fun test_compare_enums() {
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V2 { field_2: 1}) is Ordering::LessThan, 0);
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V2 { field_2: 8}) is Ordering::LessThan, 0);
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V1 { field_1: 5}) is Ordering::GreaterThan, 1);
    }

    #[test]
    fun test_compare_vectors() {
        assert!(compare(&vector[1, 2, 3], &vector[5] ) is Ordering::LessThan, 0);
        assert!(compare(&vector[1, 2, 3], &vector[5, 6, 7]) is Ordering::LessThan, 1);
        assert!(compare(&vector[1, 2, 3], &vector[1, 2, 7]) is Ordering::LessThan, 2);
    }
}
