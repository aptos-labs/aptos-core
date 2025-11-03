module 0xcafe::m1 {
    struct M1 has store, drop {
        x: u64,
    }

    public fun m1(): M1 {
        M1 { x: 0 }
    }
}

module 0xcafe::d1 {
    struct D1 has store, drop {
        d2: 0xcafe::d2::D2,
    }
}

module 0xcafe::d2 {
    struct D2 has store, drop {
        d3: 0xcafe::d3::D3,
    }
}

module 0xcafe::d3 {
    struct D3 has store, drop {
        d4: 0xcafe::d4::D4,
    }
}

module 0xcafe::d4 {
    struct D4 has store, drop {
        d5: 0xcafe::d5::D5,
    }
}

module 0xcafe::d5 {
    struct D5 has store, drop {
        d6: 0xcafe::d6::D6,
    }
}

module 0xcafe::d6 {
    struct D6 has store, drop {
        d7: 0xcafe::d7::D7,
    }
}

module 0xcafe::d7 {
    struct D7 has store, drop {
        x: bool
    }
}
