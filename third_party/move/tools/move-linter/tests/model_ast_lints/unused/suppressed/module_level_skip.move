// Test that module-level #[lint::skip] suppresses unused warnings for all items in the module.
#[lint::skip(unused_function, unused_struct, unused_constant)]
module 0x42::skip_all {
    fun unused_func(): u64 { 1 }
    const UNUSED_CONST: u64 = 1;
    struct UnusedStruct has drop { x: u64 }

    public fun regular(): u64 { 1 }
}

// Test that module-level skip only suppresses the specified checker.
#[lint::skip(unused_function)]
module 0x42::skip_function_only {
    fun unused_func(): u64 { 1 }
    const UNUSED_CONST: u64 = 1;
    struct UnusedStruct has drop { x: u64 }

    public fun regular(): u64 { 1 }
}
