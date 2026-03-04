// Tests that package constants cannot be accessed from a different package (address).

module 0x42::M {
    package const PKG: u64 = 10;
}

// Different address = different package: access must be rejected.
module 0x43::N {
    use 0x42::M;

    public fun use_pkg(): u64 {
        M::PKG
    }
}
