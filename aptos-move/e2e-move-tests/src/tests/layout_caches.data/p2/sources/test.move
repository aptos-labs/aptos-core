module 0xcafe::m2 {

    struct M2 has store, drop {
        m1: 0xcafe::m1::M1,
    }

    public fun m2(): M2 {
        M2 { m1: 0xcafe::m1::m1() }
    }
}
