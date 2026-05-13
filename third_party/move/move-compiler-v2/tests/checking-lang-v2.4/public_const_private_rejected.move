// Tests that private constants cannot be accessed cross-module at V2.5

module 0x42::M {
    const PRIV: u64 = 10;
}

module 0x42::N {
    use 0x42::M;

    // Should fail: PRIV is private
    public fun use_priv(): u64 {
        M::PRIV
    }
}
