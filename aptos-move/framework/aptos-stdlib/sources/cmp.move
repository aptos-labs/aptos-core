module std::cmp {
    const EQUAL: u8 = 1;
    const LESS_THAN: u8 = 0;
    const GREATER_THAN: u8 = 2;

    /// As there are no signed values in move, all values are shifted by 1 up.
    /// An int value:
    /// 1   iff both values are the same
    /// 0   iff first value is smaller than the second
    /// 2   iff first value is larger than the second
    native fun compare_impl<T>(first: &T, second: &T): u8;

    struct Ordering has copy, drop {
        value: u8,
    }

    public fun compare<T>(first: &T, second: &T): Ordering {
        Ordering {
            value: compare_impl(first, second),
        }
    }

    public fun is_equal(self: &Ordering): bool {
        self.value == EQUAL
    }

    public fun is_less_than(self: &Ordering): bool {
        self.value == LESS_THAN
    }

    public fun is_less_or_equal(self: &Ordering): bool {
        self.value != GREATER_THAN
    }

    public fun is_greater_than(self: &Ordering): bool {
        self.value == GREATER_THAN
    }

    public fun is__greater_or_equal(self: &Ordering): bool {
        self.value != LESS_THAN
    }

    #[test_only]
    struct SomeStruct has drop {
        field_1: u64,
        field_2: u64,
    }

    // #[test_only]
    // enum SomeEnum has drop {
    //     V1 { field_1: u64 },
    //     V2 { field_2: u64 },
    // }

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


        // assert!(!compare(&1, &5).is_equal(), 0);
        // assert!(compare(&1, &5).is_less_than(), 1);
        // assert!(compare(&1, &5).is_less_or_equal(), 2);
        // assert!(compare(&5, &5).is_equal(), 3);
        // assert!(!compare(&5, &5).is_less_than(), 4);
        // assert!(compare(&5, &5).is_less_or_equal(), 5);
        // assert!(!compare(&7, &5).is_equal(), 6);
        // assert!(!compare(&7, &5).is_less_than(), 7);
        // assert!(!compare(&7, &5).is_less_or_equal(), 8);
    }

    #[test]
    fun test_compare_structs() {
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 3}).value == LESS_THAN, 0);
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 1}).value == GREATER_THAN, 1);
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 1}).value == GREATER_THAN, 2);
    }

    #[test]
    fun test_compare_vector_of_structs() {
        assert!(compare(&vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 3, field_2: 4}], &vector[SomeStruct { field_1: 1, field_2: 3}]).value == LESS_THAN, 0);
        assert!(compare(&vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 3, field_2: 4}], &vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 1, field_2: 3}]).value == GREATER_THAN, 1);
    }

    // #[test]
    // fun test_compare_enums() {
    //     assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V2 { field_2: 1}).value == LESS_THAN, 0);
    //     assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V2 { field_2: 8}).value == LESS_THAN, 0);
    //     assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V1 { field_1: 5}).value == GREATER_THAN, 1);
    // }

    #[test]
    fun test_compare_vectors() {
        assert!(compare(&vector[1, 2, 3], &vector[5] ).value == LESS_THAN, 0);
        assert!(compare(&vector[1, 2, 3], &vector[5, 6, 7]).value == LESS_THAN, 1);
        assert!(compare(&vector[1, 2, 3], &vector[1, 2, 7]).value == LESS_THAN, 2);
    }
}
