module 0x42::m {
    // This constant is unused - should warn
    const UNUSED_CONST: u64 = 100;

    // This constant is used - should not warn
    const USED_CONST: u64 = 200;

    public fun test(): u64 {
        USED_CONST
    }
}
