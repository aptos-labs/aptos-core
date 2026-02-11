module 0x42::m {
    // Resource group container - should NOT warn (intentionally empty)
    #[resource_group(scope = global)]
    struct MyGroup {}

    // Resource group member - should NOT warn
    #[resource_group_member(group = 0x42::m::MyGroup)]
    struct MyResource has key {
        value: u64
    }

    // Regular unused struct - SHOULD warn
    struct UnusedStruct {
        x: u64
    }

    // Used struct - should NOT warn
    struct UsedStruct {
        y: u64
    }

    public fun use_struct(): UsedStruct {
        UsedStruct { y: 42 }
    }
}
