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
    // #[lint::skip(unused)] - function only
    // ========================================
    #[lint::skip(unused)]
    fun lint_skip_func(): u64 { 4 }

    // ========================================
    // #[persistent] - function only
    // ========================================
    #[persistent]
    fun persistent_func(): u64 { 5 }

    // ========================================
    // Required public function
    // ========================================
    public fun regular(): u64 { 1 }
}
