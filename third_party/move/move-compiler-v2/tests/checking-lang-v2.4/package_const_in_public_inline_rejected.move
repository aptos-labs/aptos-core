// Tests that package constants are rejected inside public/friend inline functions.
// A public or friend inline function can be expanded into modules at a different
// address, which would leave an inaccessible `const$NAME` call in the callee's bytecode.

module 0x42::M {
    package const PKG_CONST: u64 = 42;

    // Non-inline: allowed.
    public fun use_pkg(): u64 {
        PKG_CONST
    }
}

module 0x42::N {
    use 0x42::M;

    // Public inline: rejected.
    public inline fun use_pkg_public_inline(): u64 {
        M::PKG_CONST
    }

    // Package inline: allowed (callers are always at the same address).
    package inline fun use_pkg_package_inline(): u64 {
        M::PKG_CONST
    }
}

module 0x42::P {
    friend 0x42::Q;
    use 0x42::M;

    // Friend inline: rejected.
    friend inline fun use_pkg_friend_inline(): u64 {
        M::PKG_CONST
    }
}

module 0x42::Q {}
