// Tests that public constants with signed integer types are accepted in V2.5

module 0x42::M {
    public const NEG: i64 = -1;
    public const POS: i64 = 100;

    public fun use_all(): i64 {
        NEG + POS
    }
}

module 0x42::N {
    use 0x42::M;

    // Can access public signed integer constant from another module at V2.5
    public fun use_neg(): i64 {
        M::NEG
    }
}
