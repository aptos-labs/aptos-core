// Tests for deprecated_usage lint

// --- Test modules with deprecated items ---

#[deprecated]
module 0xc0ffee::deprecated_module {
    public fun foo(): u64 { 42 }
    struct DepModStruct has drop { value: u64 }
}

module 0xc0ffee::other {
    #[deprecated]
    public fun cross_deprecated_fun(): u64 { 0 }

    #[deprecated]
    struct CrossDeprecatedStruct has drop {
        value: u64,
    }

    public fun cross_good_fun(): u64 { 1 }
}

// Module where both the module and individual members are deprecated.
#[deprecated]
module 0xc0ffee::all_deprecated {
    #[deprecated]
    public fun doubly_deprecated_fun(): u64 { 0 }

    #[deprecated]
    struct DoublyDeprecatedStruct has drop { value: u64 }

    public fun inherited_deprecated_fun(): u64 { 1 }
}

// --- Main test module ---

module 0xc0ffee::m {
    #[deprecated]
    public fun deprecated_fun(): u64 { 0 }

    #[deprecated]
    const DEPRECATED_CONST: u64 = 99;

    const GOOD_CONST: u64 = 1;

    #[deprecated]
    struct DeprecatedStruct has key, drop {
        value: u64,
    }

    #[deprecated]
    enum DeprecatedEnum has drop {
        A { value: u64 },
        B { value: u64 },
    }

    public fun good_fun(): u64 { 1 }

    // --- StructChecker: field types ---

    // Non-deprecated struct holding a deprecated struct field -> warn
    struct HoldsDeprecated has drop {
        inner: DeprecatedStruct,
    }

    // Deprecated struct holding another deprecated struct -> NO warn (suppressed)
    #[deprecated]
    struct DeprecatedHolder has drop {
        inner: DeprecatedStruct,
    }

    // Non-deprecated enum with deprecated struct in variant field -> warn
    enum HoldsDeprecatedEnum has drop {
        Good { value: u64 },
        Bad { inner: DeprecatedStruct },
    }

    // Cross-module deprecated struct as a field type -> warn
    struct HoldsCrossDeprecated has drop {
        inner: 0xc0ffee::other::CrossDeprecatedStruct,
    }

    // --- ExpChecker: function calls ---

    // Calling a deprecated function -> warn
    public fun test_call_deprecated_fun(): u64 {
        deprecated_fun()
    }

    // Calling non-deprecated function from deprecated module -> warn
    public fun test_call_from_deprecated_module(): u64 {
        0xc0ffee::deprecated_module::foo()
    }

    // Cross-module: calling a deprecated function -> warn
    public fun test_cross_module_fun(): u64 {
        0xc0ffee::other::cross_deprecated_fun()
    }

    // Cross-module: calling a non-deprecated function -> NO warn
    public fun test_cross_module_good(): u64 {
        0xc0ffee::other::cross_good_fun()
    }

    // Closure over a deprecated function -> warn
    public fun test_closure_deprecated(): ||u64 {
        deprecated_fun
    }

    // Module + member both #[deprecated], calling function -> single warn (not two)
    public fun test_doubly_deprecated_fun(): u64 {
        0xc0ffee::all_deprecated::doubly_deprecated_fun()
    }

    // Non-deprecated member inheriting deprecation from module -> warn
    public fun test_inherited_deprecated_fun(): u64 {
        0xc0ffee::all_deprecated::inherited_deprecated_fun()
    }

    // --- ExpChecker: struct operations ---

    // Constructing a deprecated struct (Pack) -> warn
    public fun test_pack_deprecated_struct(): DeprecatedStruct {
        DeprecatedStruct { value: 42 }
    }

    // Accessing a field of a deprecated struct (Select) -> warn
    public fun test_select_deprecated_struct(s: &DeprecatedStruct): u64 {
        s.value
    }

    // Accessing a common field on a deprecated enum (SelectVariants) -> warn
    public fun test_select_variants_deprecated(e: &DeprecatedEnum): u64 {
        e.value
    }

    // Pattern matching on a deprecated enum (TestVariants) -> warn
    public fun test_test_variants_deprecated(e: DeprecatedEnum): u64 {
        match (e) {
            DeprecatedEnum::A { value } => value,
            DeprecatedEnum::B { value } => value,
        }
    }

    // --- ExpChecker: global storage operations ---

    // exists<DeprecatedStruct> -> warn
    public fun test_exists_deprecated_struct(addr: address): bool {
        exists<DeprecatedStruct>(addr)
    }

    // borrow_global<DeprecatedStruct> -> warn
    public fun test_borrow_global_deprecated_struct(addr: address): u64 acquires DeprecatedStruct {
        DeprecatedStruct[addr].value
    }

    // move_to with a deprecated struct -> warn
    public fun test_move_to_deprecated_struct(s: &signer) {
        move_to(s, DeprecatedStruct { value: 0 });
    }

    // move_from a deprecated struct -> warn
    public fun test_move_from_deprecated_struct(addr: address): DeprecatedStruct acquires DeprecatedStruct {
        move_from<DeprecatedStruct>(addr)
    }

    // --- ExpChecker: pattern matching ---

    // Let-destructuring a deprecated struct -> warn
    public fun test_let_destructure(s: DeprecatedStruct): u64 {
        let DeprecatedStruct { value } = s;
        value
    }

    // --- ExpChecker: suppression ---

    // Deprecated function calling another deprecated item -> NO warn
    #[deprecated]
    public fun test_deprecated_calling_deprecated(): u64 {
        deprecated_fun()
    }

    // #[lint::skip(deprecated_usage)] suppresses expression warnings -> NO warn
    #[lint::skip(deprecated_usage)]
    public fun test_skip_deprecated(): u64 {
        deprecated_fun()
    }

    // #[lint::skip(deprecated_usage, unused_struct)] with multiple skips -> NO warn, no spurious errors
    #[lint::skip(deprecated_usage, unused_struct)]
    public fun test_skip_deprecated_multi(): u64 {
        deprecated_fun()
    }

    // Non-deprecated usage -> NO warn
    public fun test_no_warn(): u64 {
        good_fun()
    }

    // --- FunctionChecker: signature types ---

    // Deprecated struct in function parameter type -> warn
    public fun test_param_deprecated(_s: DeprecatedStruct): u64 {
        0
    }

    // Deprecated struct in function return type -> warn
    public fun test_return_deprecated(): DeprecatedStruct {
        DeprecatedStruct { value: 0 }
    }

    // Cross-module: deprecated struct type in signature -> warn
    public fun test_cross_module_struct(_s: 0xc0ffee::other::CrossDeprecatedStruct): u64 {
        0
    }

    // Non-deprecated struct from deprecated module in param type -> single warn (not two)
    public fun test_struct_from_deprecated_module(_s: 0xc0ffee::deprecated_module::DepModStruct): u64 {
        0
    }

    // Module + member both #[deprecated], struct in signature -> single warn (not two)
    public fun test_doubly_deprecated_struct(_s: 0xc0ffee::all_deprecated::DoublyDeprecatedStruct): u64 {
        0
    }

    // Deprecated function with deprecated param/return -> NO warn (suppressed)
    #[deprecated]
    public fun test_deprecated_sig(s: DeprecatedStruct): DeprecatedStruct {
        s
    }

    // #[lint::skip(deprecated_usage)] suppresses signature warnings -> NO warn
    #[lint::skip(deprecated_usage)]
    public fun test_skip_sig(s: DeprecatedStruct): DeprecatedStruct {
        s
    }

    // Non-deprecated struct in param/return -> NO warn
    public fun test_good_sig(s: HoldsDeprecated): HoldsDeprecated {
        s
    }

    // --- ConstantChecker ---

    // Using a deprecated constant -> warn
    public fun test_use_deprecated_const(): u64 {
        DEPRECATED_CONST
    }

    // Deprecated function using a deprecated constant -> NO warn (suppressed)
    #[deprecated]
    public fun test_deprecated_using_deprecated_const(): u64 {
        DEPRECATED_CONST
    }

    // #[lint::skip(deprecated_usage)] suppresses constant usage warnings -> NO warn
    #[lint::skip(deprecated_usage)]
    public fun test_skip_deprecated_const(): u64 {
        DEPRECATED_CONST
    }

    // Using a non-deprecated constant -> NO warn
    public fun test_use_good_const(): u64 {
        GOOD_CONST
    }
}
