// Tests that friend constants are not accessible from a non-friend module.

module 0x42::M {
    friend 0x42::N;
    friend const FRIEND_CONST: u64 = 42;
}

// O is NOT a declared friend of M: access must be rejected.
module 0x42::O {
    use 0x42::M;

    public fun use_friend(): u64 {
        M::FRIEND_CONST
    }
}
