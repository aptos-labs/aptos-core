module std::cmp {
    enum Ordering has copy, drop {
        /// First value is less than the second value.
        Less,
        /// First value is equal to the second value.
        Equal,
        /// First value is greater than the second value.
        Greater,
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
        pragma intrinsic;
    }

    spec Ordering {
        pragma intrinsic;
    }

    spec is_eq {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }

    spec is_ne {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }

    spec is_lt {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }

    spec is_le {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }

    spec is_gt {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }

    spec is_ge {
        pragma intrinsic;
        pragma opaque;
        pragma verify = false;
    }

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

    #[verify_only]
    fun test_verify_compare_preliminary_types() {
        spec {
            assert compare(1, 5).is_ne();
            assert !compare(1, 5).is_eq();
            assert compare(1, 5).is_lt();
            assert compare(1, 5).is_le();
            assert compare(5, 5).is_eq();
            assert !compare(5, 5).is_ne();
            assert !compare(5, 5).is_lt();
            assert compare(5, 5).is_le();
            assert !compare(7, 5).is_eq();
            assert compare(7, 5).is_ne();
            assert !compare(7, 5).is_lt();
            assert !compare(7, 5).is_le();
            assert compare(false, true).is_ne();
            assert compare(false, true).is_lt();
            assert compare(true, false).is_ge();
            assert compare(true, true).is_eq();
        };
    }

    #[verify_only]
    fun test_verify_compare_vectors() {
        let empty: vector<u64> = vector[];
        let v1 = vector[1 as u64];
        let v8 = vector[1 as u8, 2];
        let v32_1 = vector[1 as u32, 2, 3];
        let v32_2 = vector[5 as u32];
        spec {
            assert compare(empty, v1) == Ordering::Less;
            assert compare(empty, empty) == Ordering::Equal;
            assert compare(v1, empty) == Ordering::Greater;
            assert compare(v8, v8) == Ordering::Equal;
            assert compare(v32_1, v32_2) is Ordering::Less;
            assert compare(v32_2, v32_1) == Ordering::Greater;
        };
    }

    #[verify_only]
    struct SomeStruct has drop {
        field_1: u64,
        field_2: u64,
    }

    #[verify_only]
    fun test_verify_compare_structs() {
        let s1 = SomeStruct { field_1: 1, field_2: 2};
        let s2 = SomeStruct { field_1: 1, field_2: 3};
        let s3 = SomeStruct { field_1: 1, field_2: 1};
        let s4 = SomeStruct { field_1: 2, field_2: 1};
        spec {
            assert compare(s1, s1) == Ordering::Equal;
            assert compare(s1, s2) == Ordering::Less;
            assert compare(s1, s3) == Ordering::Greater;
            assert compare(s4, s1) == Ordering::Greater;
        };
    }

    #[verify_only]
    fun test_verify_compare_vector_of_structs() {
        let v1 = vector[SomeStruct { field_1: 1, field_2: 2}];
        let v2 = vector[SomeStruct { field_1: 1, field_2: 3}];
        spec {
            assert compare(v1, v2) == Ordering::Less;
            assert compare(v1, v1) == Ordering::Equal;
        };
    }

    #[verify_only]
    enum SomeEnum has drop {
        V1 { field_1: u64 },
        V2 { field_2: u64 },
        V3 { field_3: SomeStruct },
        V4 { field_4: vector<u64> },
        V5 { field_5: SimpleEnum },
    }

    #[verify_only]
    enum SimpleEnum has drop {
        V { field: u64 },
    }

    #[verify_only]
    fun test_verify_compare_enums() {
        let e1 = SomeEnum::V1 { field_1: 6};
        let e2 = SomeEnum::V2 { field_2: 1};
        let e3 = SomeEnum::V3 { field_3: SomeStruct { field_1: 1, field_2: 2}};
        let e4 = SomeEnum::V4 { field_4: vector[1, 2]};
        let e5 = SomeEnum::V5 { field_5: SimpleEnum::V { field: 3}};
        spec {
            assert compare(e1, e1) == Ordering::Equal;
            assert compare(e1, e2) == Ordering::Less;
            assert compare(e2, e1) == Ordering::Greater;
            assert compare(e3, e4) == Ordering::Less;
            assert compare(e5, e4) == Ordering::Greater;
        };
    }

    #[verify_only]
    struct SomeStruct_BV has copy,drop {
        field: u64
    }

    spec SomeStruct_BV  {
        pragma bv=b"0";
    }

    #[verify_only]
    fun test_compare_bv() {
        let a = 1;
        let b = 5;
        let se_a = SomeStruct_BV { field: a};
        let se_b = SomeStruct_BV { field: b};
        let v_a = vector[a];
        let v_b = vector[b];
        spec {
            assert compare(a, b) == Ordering::Less;
            assert compare(se_a, se_b) == Ordering::Less;
            assert compare(v_a, v_b) == Ordering::Less;
        };
    }

}
