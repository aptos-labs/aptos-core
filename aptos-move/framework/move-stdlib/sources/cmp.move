module std::cmp {
    enum Ordering has copy, drop {
        /// First value is less than the second value.
        Less,
        /// First value is equal to the second value.
        Equal,
        /// First value is greater than the second value.
        Greater,
    }

    spec Ordering {
        pragma intrinsic;
    }

    spec is_eq {
        pragma intrinsic;
    }

    spec is_ne {
        pragma intrinsic;
    }

    spec is_lt {
        pragma intrinsic;
    }

    spec is_le {
        pragma intrinsic;
    }

    spec is_gt {
        pragma intrinsic;
    }

    spec is_ge {
        pragma intrinsic;
    }
    /// Compares two values with the natural ordering:
    /// - native types are compared identically to `<` and other operators
    /// - complex types
    ///   - Structs and vectors - are compared lexicographically - first field/element is compared first,
    ///     and if equal we proceed to the next.
    ///   - enum's are compared first by their variant, and if equal - they are compared as structs are.
    native public fun compare<T>(first: &T, second: &T): Ordering;

    public fun is_eq(self: &Ordering): bool {
        self is Ordering::Equal
    }

    public fun is_ne(self: &Ordering): bool {
        !(self is Ordering::Equal)
    }

    public fun is_lt(self: &Ordering): bool {
        self is Ordering::Less
    }

    public fun is_le(self: &Ordering): bool {
        !(self is Ordering::Greater)
    }

    public fun is_gt(self: &Ordering): bool {
        self is Ordering::Greater
    }

    public fun is_ge(self: &Ordering): bool {
        !(self is Ordering::Less)
    }

    spec compare {
        // TODO: temporary mockup.
        pragma intrinsic;
    }

    fun test_compare_preliminary_types(): Ordering {
        let a = 1;
        let b = 5;
        spec {
            assert compare(a, b) == Ordering::Less;
        };
        let x = true;
        let y = false;
        spec {
            assert compare(x, y) == Ordering::Greater;
        };
        let addr_1 = @0x1;
        let addr_2 = @0x2;
        spec {
            assert compare(addr_1, addr_1) == Ordering::Equal;
        };
        compare(&x, &y);
        compare(&a, &b)
    }

    // fun test_compare_signer(s: &signer) {

    // }

    // spec test_compare_signer {
    //     ensures compare(s, s) == Ordering::Equal;
    // }

    fun test_compare_vec(v2: vector<u64>) {
        let v1 = vector[1, 2, 3];
        let v1_1 = vector[1, 2, 3];
        let v2 = vector[1, 2];
        let v3 = vector[1, 2, 4];
        let v4 = vector[1, 2, 3, 4];
        let v5 = vector[5];
        let v6 = vector[vector[1, 2, 3]];
        spec {
            assert compare(v1, v1_1) == Ordering::Equal;
            assert compare(v1, v3) == Ordering::Less;
            assert compare(v1, v2) == Ordering::Greater;
            assert compare(v3, v1) == Ordering::Greater;
            assert compare(v4, v1) == Ordering::Greater;
            assert compare(v1, v4) == Ordering::Less;
            assert compare(v5, v1) == Ordering::Greater;
            assert compare(v1, v5) == Ordering::Less;
            assert compare(v6, v6) == Ordering::Equal;
        };
    }

    // fun test_compare_preliminary_types(): Ordering {
    //     let a = 1;
    //     let b = 5;
    //     spec {
    //         assert compare(a, b) == Ordering::Less;
    //     };
    //     let x = true;
    //     let y = false;
    //     spec {
    //         assert compare(x, y) == Ordering::Greater;
    //     };
    //     compare(&x, &y);
    //     compare(&a, &b)
    // }

    #[test_only]
    struct SomeStruct has drop {
        field_1: u64,
        field_2: u64,
    }

    #[test_only]
    enum SimpleEnum has drop {
        V { field: u64 },
    }

    #[test_only]
    enum SomeEnum has drop {
        V1 { field_1: u64 },
        V2 { field_2: u64 },
        V3 { field_3: SomeStruct },
        V4 { field_4: vector<u64> },
        V5 { field_5: SimpleEnum },
    }

    #[test]
    fun test_compare_numbers() {
        assert!(compare(&1, &5).is_ne(), 0);
        assert!(!compare(&1, &5).is_eq(), 0);
        assert!(compare(&1, &5).is_lt(), 1);
        assert!(compare(&1, &5).is_le(), 2);
        assert!(compare(&5, &5).is_eq(), 3);
        assert!(!compare(&5, &5).is_ne(), 3);
        assert!(!compare(&5, &5).is_lt(), 4);
        assert!(compare(&5, &5).is_le(), 5);
        assert!(!compare(&7, &5).is_eq(), 6);
        assert!(compare(&7, &5).is_ne(), 6);
        assert!(!compare(&7, &5).is_lt(), 7);
        assert!(!compare(&7, &5).is_le(), 8);

        assert!(!compare(&1, &5).is_eq(), 0);
        assert!(compare(&1, &5).is_ne(), 0);
        assert!(compare(&1, &5).is_lt(), 1);
        assert!(compare(&1, &5).is_le(), 2);
        assert!(!compare(&1, &5).is_gt(), 1);
        assert!(!compare(&1, &5).is_ge(), 1);
        assert!(compare(&5, &5).is_eq(), 3);
        assert!(!compare(&5, &5).is_ne(), 3);
        assert!(!compare(&5, &5).is_lt(), 4);
        assert!(compare(&5, &5).is_le(), 5);
        assert!(!compare(&5, &5).is_gt(), 5);
        assert!(compare(&5, &5).is_ge(), 5);
        assert!(!compare(&7, &5).is_eq(), 6);
        assert!(compare(&7, &5).is_ne(), 6);
        assert!(!compare(&7, &5).is_lt(), 7);
        assert!(!compare(&7, &5).is_le(), 8);
        assert!(compare(&7, &5).is_gt(), 7);
        assert!(compare(&7, &5).is_ge(), 8);
    }

    #[test]
    fun test_compare_vectors() {
        let empty = vector[]; // here for typing, for the second line
        assert!(compare(&empty, &vector[1] ) is Ordering::Less, 0);
        assert!(compare(&empty, &vector[] ) is Ordering::Equal, 1);
        assert!(compare(&vector[1], &vector[] ) is Ordering::Greater, 2);
        assert!(compare(&vector[1, 2], &vector[1, 2] ) is Ordering::Equal, 3);
        assert!(compare(&vector[1, 2, 3], &vector[5] ) is Ordering::Less, 4);
        assert!(compare(&vector[1, 2, 3], &vector[5, 6, 7]) is Ordering::Less, 5);
        assert!(compare(&vector[1, 2, 3], &vector[1, 2, 7]) is Ordering::Less, 6);
    }

    #[test]
    fun test_compare_structs() {
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 2}) is Ordering::Equal, 0);
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 3}) is Ordering::Less, 1);
        assert!(compare(&SomeStruct { field_1: 1, field_2: 2}, &SomeStruct { field_1: 1, field_2: 1}) is Ordering::Greater, 2);
        assert!(compare(&SomeStruct { field_1: 2, field_2: 1}, &SomeStruct { field_1: 1, field_2: 2}) is Ordering::Greater, 3);
    }

    #[test]
    fun test_compare_vector_of_structs() {
        assert!(compare(&vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 3, field_2: 4}], &vector[SomeStruct { field_1: 1, field_2: 3}]) is Ordering::Less, 0);
        assert!(compare(&vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 3, field_2: 4}], &vector[SomeStruct { field_1: 1, field_2: 2}, SomeStruct { field_1: 1, field_2: 3}]) is Ordering::Greater, 1);
    }

    #[test]
    fun test_compare_enums() {
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V1 { field_1: 6}) is Ordering::Equal, 0);
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V2 { field_2: 1}) is Ordering::Less, 1);
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V2 { field_2: 8}) is Ordering::Less, 2);
        assert!(compare(&SomeEnum::V1 { field_1: 6}, &SomeEnum::V1 { field_1: 5}) is Ordering::Greater, 3);

        assert!(compare(&SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 2}}, &SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 2}}) is Ordering::Equal, 4);
        assert!(compare(&SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 2}}, &SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 3}}) is Ordering::Less, 5);
        assert!(compare(&SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 2}}, &SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 1}}) is Ordering::Greater, 6);
        assert!(compare(&SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 2}}, &SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 1}}) is Ordering::Greater, 7);

        assert!(compare(&SomeEnum::V4 { field_4: vector[1, 2]}, &SomeEnum::V4 { field_4: vector[1, 2]}) is Ordering::Equal, 8);
        assert!(compare(&SomeEnum::V4 { field_4: vector[1, 2, 3]}, &SomeEnum::V4 { field_4: vector[5]}) is Ordering::Less, 9);
        assert!(compare(&SomeEnum::V4 { field_4: vector[1, 2, 3]}, &SomeEnum::V4 { field_4: vector[5, 6, 7]}) is Ordering::Less, 10);
        assert!(compare(&SomeEnum::V4 { field_4: vector[1, 2, 3]}, &SomeEnum::V4 { field_4: vector[1, 2, 7]}) is Ordering::Less, 11);

        assert!(compare(&SomeEnum::V5 { field_5: SimpleEnum::V { field: 3}}, &SomeEnum::V5 { field_5: SimpleEnum::V { field: 3}}) is Ordering::Equal, 12);
        assert!(compare(&SomeEnum::V5 { field_5: SimpleEnum::V { field: 5}}, &SomeEnum::V5 { field_5: SimpleEnum::V { field: 3}}) is Ordering::Greater, 13);
        assert!(compare(&SomeEnum::V5 { field_5: SimpleEnum::V { field: 3}}, &SomeEnum::V5 { field_5: SimpleEnum::V { field: 5}}) is Ordering::Less, 14);
    }
}
