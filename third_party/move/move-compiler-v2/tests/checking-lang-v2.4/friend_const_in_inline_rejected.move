// Tests that friend constants are rejected inside non-private inline functions.
// A non-private inline function can be expanded into modules that are not friends
// of the defining module, leaving an inaccessible const$NAME call in the callee's bytecode.
// Private inline functions are safe: they can only be called within the same module.

module 0x42::M {
    friend 0x42::N;
    friend const FRIEND_CONST: u64 = 42;
}

module 0x42::N {
    use 0x42::M;

    // Non-inline: allowed.
    public fun use_friend(): u64 {
        M::FRIEND_CONST
    }

    // Public inline: rejected.
    public inline fun use_friend_inline(): u64 {
        M::FRIEND_CONST
    }

    // Private inline: allowed (can only be called within N, which is a declared friend of M).
    inline fun use_friend_private_inline(): u64 {
        M::FRIEND_CONST
    }
}
