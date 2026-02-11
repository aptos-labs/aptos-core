module 0x42::m {
    // Struct used ONLY in function spec - should NOT be warned as unused
    struct SpecOnlyStruct has copy, drop {
        value: u64
    }

    // Struct used in function body - should NOT be warned
    struct UsedInBody has copy, drop {
        x: u64
    }

    // Struct completely unused - SHOULD be warned
    struct CompletelyUnused has copy, drop {
        y: u64
    }

    // Constant used ONLY in spec - should NOT be warned as unused
    const SPEC_ONLY_CONST: u64 = 100;

    // Constant used in function body - should NOT be warned
    const BODY_CONST: u64 = 200;

    // Constant completely unused - SHOULD be warned
    const UNUSED_CONST: u64 = 300;

    // Function with spec that uses SpecOnlyStruct
    public fun test_function_spec(x: u64): u64
    {
        x + BODY_CONST
    }
    spec test_function_spec {
        // Use SpecOnlyStruct in requires
        requires exists<SpecOnlyStruct>(@0x42);
        // Use SPEC_ONLY_CONST in ensures
        ensures result == x + SPEC_ONLY_CONST;
    }

    // Function that uses UsedInBody
    public fun use_body_struct(): UsedInBody {
        UsedInBody { x: 42 }
    }

    // Test aborts_if with struct
    public fun test_aborts(x: u64): u64
    {
        x
    }
    spec test_aborts {
        aborts_if !exists<SpecOnlyStruct>(@0x42);
    }

    // Test ensures with constant
    public fun test_ensures(x: u64): u64
    {
        x
    }
    spec test_ensures {
        ensures result <= SPEC_ONLY_CONST;
    }

    // Constant used in struct spec
    const STRUCT_SPEC_CONST: u64 = 500;

    struct StructUsingConst has key {
        amount: u64
    }
    spec StructUsingConst {
        invariant amount <= STRUCT_SPEC_CONST;
    }
}

// Module-level spec using struct and constant
spec 0x42::m {
    // Use SpecOnlyStruct in global invariant
    invariant update [global] forall addr: address:
        exists<SpecOnlyStruct>(addr) ==> global<SpecOnlyStruct>(addr).value <= SPEC_ONLY_CONST;
}
