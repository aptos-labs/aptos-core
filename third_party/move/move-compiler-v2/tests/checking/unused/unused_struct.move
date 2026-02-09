module 0x42::m {
    // This struct is private and unused - should warn
    struct UnusedStruct {
        x: u64
    }

    // This struct is public - should not warn even if unused
    public struct PublicStruct {
        y: u64
    }

    // This struct is private but used - should not warn
    struct UsedStruct {
        z: u64
    }

    public fun test(): UsedStruct {
        UsedStruct { z: 42 }
    }
}
