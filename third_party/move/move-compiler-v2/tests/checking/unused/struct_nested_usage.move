module 0x42::m {
    // This struct is used as a field in Container
    struct Inner has drop {
        x: u64
    }

    // This struct uses Inner but is itself unused
    struct Container has drop {
        inner: Inner
    }

    // This chain: Level3 -> Level2 -> Level1, but only Level3 is used
    struct Level1 has drop {
        a: u64
    }

    struct Level2 has drop {
        l1: Level1
    }

    struct Level3 has drop {
        l2: Level2
    }

    // Used struct referencing unused Inner indirectly
    struct UsedOuter has drop {
        container: Container
    }

    public fun use_level3(): Level3 {
        Level3 {
            l2: Level2 {
                l1: Level1 { a: 1 }
            }
        }
    }

    public fun use_outer(): UsedOuter {
        UsedOuter {
            container: Container {
                inner: Inner { x: 2 }
            }
        }
    }
}
