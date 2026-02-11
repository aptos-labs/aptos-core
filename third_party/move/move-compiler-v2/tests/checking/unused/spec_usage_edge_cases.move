module 0x42::m {
    // Test 1: Struct used in multiple different spec contexts
    struct MultiSpecStruct has copy, drop, key {
        value: u64
    }

    public fun func_with_spec(x: u64): u64 {
        x
    }
    spec func_with_spec {
        // Used in function spec
        requires exists<MultiSpecStruct>(@0x42);
    }

    public fun another_func(y: u64): u64 {
        y
    }
    spec another_func {
        // Used again in another function spec
        ensures result == y || exists<MultiSpecStruct>(@0x42);
    }

    // Test 2: Constant used in multiple spec contexts
    const MULTI_SPEC_CONST: u64 = 999;

    public fun func_using_const(z: u64): u64 {
        z
    }
    spec func_using_const {
        ensures result <= MULTI_SPEC_CONST;
        aborts_if z > MULTI_SPEC_CONST;
    }

    // Test 3: Struct used ONLY in inline spec (inside function body)
    struct InlineSpecStruct has copy, drop {
        data: u64
    }

    public fun func_with_inline_spec(): u64 {
        spec {
            // Inline spec inside function body
            assert exists<InlineSpecStruct>(@0x42);
        };
        42
    }

    // Test 4: Constant used ONLY in inline spec
    const INLINE_SPEC_CONST: u64 = 777;

    public fun another_inline_spec(): u64 {
        spec {
            assert INLINE_SPEC_CONST > 0;
        };
        100
    }

    // Test 5: Struct used in aborts_if with additional expressions
    struct AbortsStruct has key {
        code: u64
    }

    public fun func_with_aborts(): u64 {
        0
    }
    spec func_with_aborts {
        aborts_if !exists<AbortsStruct>(@0x42) with 1;
    }

    // Test 6: Multiple structs in same spec
    struct FirstStruct has copy, drop {
        a: u64
    }

    struct SecondStruct has copy, drop {
        b: u64
    }

    public fun func_multi_struct(): u64 {
        0
    }
    spec func_multi_struct {
        requires exists<FirstStruct>(@0x1);
        requires exists<SecondStruct>(@0x2);
        ensures result == 0;
    }

    // Test 7: Struct used in modifies clause
    struct ModifiesStruct has key {
        x: u64
    }

    public fun func_with_modifies() {
    }
    spec func_with_modifies {
        modifies global<ModifiesStruct>(@0x42);
    }

    // Test 8: These should be warned as unused
    struct TrulyUnused1 has copy, drop {
        unused: u64
    }

    struct TrulyUnused2 has key {
        also_unused: u64
    }

    const TRULY_UNUSED_CONST: u64 = 123;
}

// Module-level spec using MultiSpecStruct and MULTI_SPEC_CONST
spec 0x42::m {
    invariant update forall addr: address:
        exists<MultiSpecStruct>(addr) ==> global<MultiSpecStruct>(addr).value <= MULTI_SPEC_CONST;
}
