module 0x42::m {
    // Truly unused private function - should warn
    fun unused_fun(): u64 {
        42
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
