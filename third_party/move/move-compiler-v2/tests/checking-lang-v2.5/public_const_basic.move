// Tests that public and package constants are accepted in V2.5

module 0x42::M {
    // Private constant (default visibility) - always allowed
    const PRIV: u64 = 10;
    // Public constant - new in V2.5
    public const PUB: u64 = 20;
    // Package constant - new in V2.5
    package const PKG: u64 = 30;

    public fun use_all(): u64 {
        PRIV + PUB + PKG
    }
}

module 0x42::N {
    use 0x42::M;

    // Can access public constant from another module at V2.5
    public fun use_pub(): u64 {
        M::PUB
    }
}
