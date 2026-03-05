module 0x42::m {
    // Truly unused private function - should warn
    fun unused_fun(): u64 {
        42
    }

    // Self-recursive only function - should warn (no external caller)
    fun self_recursive(n: u64): u64 {
        if (n <= 1) 1
        else n * self_recursive(n - 1)
    }

    // Truly unused constant - should warn
    const UNUSED_CONST: u64 = 100;

    // Truly unused struct - should warn
    struct UnusedStruct has drop {
        x: u64
    }

    // Used items below - no warnings
    fun used_helper(): u64 {
        USED_CONST
    }

    const USED_CONST: u64 = 1;

    struct UsedStruct has drop {
        value: u64
    }

    public fun public_caller(): UsedStruct {
        UsedStruct { value: used_helper() }
    }
}
