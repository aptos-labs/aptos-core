// Tests that package constants are accessible within the same package (same address).

module 0x42::M {
    package const PKG: u64 = 10;
}

// Same address = same package: access must be allowed.
module 0x42::N {
    use 0x42::M;

    public fun use_pkg(): u64 {
        M::PKG
    }
}
