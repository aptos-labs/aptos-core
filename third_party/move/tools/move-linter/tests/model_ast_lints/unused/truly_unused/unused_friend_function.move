// Friend function with friends declared but no callers - should warn (unused_function)
module 0x42::m {
    friend 0x42::n;

    // Not called at all - should warn (unused_function)
    friend fun unused_friend_func(): u64 { 1 }

    // Called from friend module - should NOT warn
    friend fun used_friend_func(): u64 { 2 }

    // Called only from same module - should warn (needless_visibility)
    friend fun same_module_only_friend(): u64 { 3 }

    public fun caller(): u64 {
        same_module_only_friend()
    }
}

module 0x42::n {
    use 0x42::m;

    public fun call_friend(): u64 {
        m::used_friend_func()
    }
}

// Friend function in module with no friends - should warn (needless_visibility)
module 0x42::no_friends {
    // No callers and no friends - warns from both unused_function and needless_visibility
    friend fun unreachable_friend(): u64 { 1 }

    // Has same-module callers but no friends - warns from needless_visibility only
    friend fun internally_used(): u64 { 2 }

    public fun caller(): u64 {
        internally_used()
    }
}
