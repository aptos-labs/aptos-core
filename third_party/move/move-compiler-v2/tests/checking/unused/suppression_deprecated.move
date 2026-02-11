module 0x42::m {
    // Deprecated struct - should NOT warn
    #[deprecated]
    struct DeprecatedStruct {
        x: u64
    }

    // Regular unused struct - SHOULD warn
    struct UnusedStruct {
        y: u64
    }

    // Used struct - should NOT warn
    struct UsedStruct {
        z: u64
    }

    public fun use_struct(): UsedStruct {
        UsedStruct { z: 42 }
    }
}
