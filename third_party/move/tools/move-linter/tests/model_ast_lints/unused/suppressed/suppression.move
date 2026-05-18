module 0x42::m {
    // ========================================
    // #[test_only] - suppresses for all items
    // ========================================
    #[test_only]
    fun test_only_func(): u64 { 1 }

    #[test_only]
    const TEST_ONLY_CONST: u64 = 1;

    #[test_only]
    struct TestOnlyStruct has drop { x: u64 }

    // ========================================
    // #[verify_only] - suppresses for all items
    // ========================================
    #[verify_only]
    fun verify_only_func(): u64 { 2 }

    #[verify_only]
    const VERIFY_ONLY_CONST: u64 = 2;

    #[verify_only]
    struct VerifyOnlyStruct has drop { x: u64 }

    // ========================================
    // #[deprecated] - suppresses for all items
    // ========================================
    #[deprecated]
    fun deprecated_func(): u64 { 3 }

    #[deprecated]
    const DEPRECATED_CONST: u64 = 3;

    #[deprecated]
    struct DeprecatedStruct has drop { x: u64 }

    // ========================================
    // #[lint::skip] - per-item suppression
    // ========================================
    #[lint::skip(unused_function)]
    fun lint_skip_func(): u64 { 4 }

    #[lint::skip(unused_constant)]
    const LINT_SKIP_CONST: u64 = 4;

    #[lint::skip(unused_struct)]
    struct LintSkipStruct has drop { x: u64 }

    // ========================================
    // #[persistent] - function-only suppression
    // ========================================
    #[persistent]
    fun persistent_func(): u64 { 5 }

    // ========================================
    // #[view] - function-only suppression
    // ========================================
    #[view]
    fun view_func(): u64 { 6 }

    // ========================================
    // #[resource_group] - struct-only suppression
    // ========================================
    #[resource_group(scope = global)]
    struct ResourceGroupMarker {}

    // ========================================
    // #[resource_group_member] - struct-only suppression
    // ========================================
    #[resource_group_member(group = 0x42::m::ResourceGroupMarker)]
    struct ResourceGroupMemberStruct has key { x: u64 }

    // ========================================
    // Required public function
    // ========================================
    public fun regular(): u64 { 1 }
}
