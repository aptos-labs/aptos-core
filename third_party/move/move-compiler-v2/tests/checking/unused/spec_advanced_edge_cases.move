module 0x42::m {
    // Test 1: Native spec function (declared without body)
    // Struct used ONLY in native spec function signature - should NOT be warned
    struct UsedInNativeSpecFun has drop {
        value: u64
    }

    spec fun native_helper(s: UsedInNativeSpecFun): u64;

    public fun test_native_spec(): u64 {
        42
    }
    spec test_native_spec {
        ensures result == native_helper(UsedInNativeSpecFun { value: 10 });
    }

    // Test 2: Spec schema (should NOT track usage - schemas are reusable templates)
    // This struct should be warned as unused because schema usage doesn't count
    struct OnlyInSchema has drop {
        x: u64
    }

    spec schema TestSchema {
        requires exists<OnlyInSchema>(@0x1);
    }

    // Test 3: Struct used in aborts_if condition
    struct UsedInAbortsIfCondition has key {
        error_code: u64
    }

    public fun test_aborts_condition(): u64 {
        0
    }
    spec test_aborts_condition {
        aborts_if !exists<UsedInAbortsIfCondition>(@0x42);
    }

    // Test 4: Struct used in assume
    struct UsedInAssume has drop {
        val: u64
    }

    public fun test_assume(): u64 {
        spec {
            assume exists<UsedInAssume>(@0x1);
        };
        100
    }

    // Test 5: Generic struct used in spec
    struct GenericStruct<T> has drop {
        item: T
    }

    public fun test_generic(): u64 {
        0
    }
    spec test_generic {
        requires exists<GenericStruct<u64>>(@0x1);
        ensures result == 0;
    }

    // Test 6: Struct with vector type field used in spec
    struct VectorStruct has drop {
        items: vector<u64>
    }

    public fun test_vector(): u64 {
        0
    }
    spec test_vector {
        ensures exists<VectorStruct>(@0x1);
    }

    // Test 7: Native spec function with multiple parameter types
    struct FirstParam has drop { a: u64 }
    struct SecondParam has drop { b: u64 }
    struct ReturnType has drop { c: u64 }

    spec fun complex_native_spec(
        p1: FirstParam,
        p2: SecondParam
    ): ReturnType;

    public fun test_complex_native(): u64 {
        0
    }
    spec test_complex_native {
        ensures result == complex_native_spec(
            FirstParam { a: 1 },
            SecondParam { b: 2 }
        ).c;
    }

    // Test 8: Schema that is applied with 'include'
    // When a schema is applied, structs used in it DO count as used
    struct OnlyInAppliedSchema has drop {
        y: u64
    }

    spec schema AppliedSchema {
        requires exists<OnlyInAppliedSchema>(@0x2);
    }

    spec test_complex_native {
        include AppliedSchema;
    }

    // Test 9: Truly unused entities that should be warned
    struct CompletelyUnused has drop {
        unused_field: u64
    }

    const UNUSED_CONST: u64 = 999;

    spec fun unused_spec_fun(x: u64): u64 {
        x + 1
    }
}
