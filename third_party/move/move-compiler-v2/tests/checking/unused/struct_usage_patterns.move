module 0x42::m {
    // Used via constructor
    struct UsedViaConstructor has drop {
        x: u64
    }

    // Used via destructuring
    struct UsedViaDestructuring has drop {
        y: u64
    }

    // Used via borrow
    struct UsedViaBorrow {
        z: u64
    }

    // Used in nested type
    struct UsedInVector has drop {
        w: u64
    }

    // Unused
    struct ReallyUnused {
        a: u64
    }

    public fun test_constructor(): UsedViaConstructor {
        UsedViaConstructor { x: 1 }
    }

    public fun test_destructuring(s: UsedViaDestructuring): u64 {
        let UsedViaDestructuring { y } = s;
        y
    }

    public fun test_borrow(s: &UsedViaBorrow): u64 {
        s.z
    }

    public fun test_vector(): vector<UsedInVector> {
        vector[]
    }
}
