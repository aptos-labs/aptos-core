// Tests that friend constants are accessible from a declared friend module.

module 0x42::M {
    friend 0x42::N;
    friend const FRIEND_CONST: u64 = 42;

    fun call_friend_const(): u64 {
        return FRIEND_CONST
    }
}

// N is a declared friend of M: access must be allowed.
module 0x42::N {
    use 0x42::M;

    public fun use_friend(): u64 {
        M::FRIEND_CONST
    }
}
