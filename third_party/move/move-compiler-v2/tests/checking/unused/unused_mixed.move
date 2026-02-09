module 0x42::m {
    // Unused constant
    const UNUSED_A: u64 = 1;

    // Unused struct
    struct UnusedStruct1 {
        x: u64
    }

    // Used constant
    const USED_IN_FUNCTION: u64 = 2;

    // Used struct (via type annotation)
    struct UsedStruct1 has drop {
        y: u64
    }

    // Used struct (via field access)
    struct UsedStruct2 {
        z: u64
    }

    // Another unused struct
    struct UnusedStruct2 has drop {
        w: u64
    }

    public fun test1(): u64 {
        USED_IN_FUNCTION
    }

    public fun test2(): UsedStruct1 {
        UsedStruct1 { y: 10 }
    }

    public fun test3(s: &UsedStruct2): u64 {
        s.z
    }
}
