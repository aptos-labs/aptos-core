// Package function called from another module in the same package - no warning
module 0x42::provider {
    package fun pkg_helper(): u64 { 42 }
}

module 0x42::consumer {
    use 0x42::provider;

    public fun call_pkg(): u64 {
        provider::pkg_helper()
    }
}

// Friend function called from a declared friend module - no warning
module 0x42::with_friend {
    friend 0x42::friend_caller;

    friend fun friend_helper(): u64 { 99 }
}

module 0x42::friend_caller {
    use 0x42::with_friend;

    public fun call_friend(): u64 {
        with_friend::friend_helper()
    }
}

// Friend function called from both same module and a friend module - no warning
module 0x42::self_caller {
    friend 0x42::someone;

    friend fun friend_used_internally(): u64 { 7 }

    public fun caller(): u64 {
        friend_used_internally()
    }
}

module 0x42::someone {
    use 0x42::self_caller;

    public fun call_friend(): u64 {
        self_caller::friend_used_internally()
    }
}
