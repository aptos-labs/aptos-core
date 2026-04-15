//# publish
module 0xc0ffee::m {
    // Range with immutable reference to primitive
    public fun test_ref_range(x: u64): u64 {
        match (&x) {
            1..10 => 1,
            _ => 0,
        }
    }

    enum E has drop, copy {
        V1(u64),
        V2,
    }

    // Range in enum field via reference
    fun ref_enum_range(e: &E): u64 {
        match (e) {
            E::V1(0..5) => 1,
            E::V1(_) => 2,
            E::V2 => 3,
        }
    }

    public fun test_ref_enum_in_range(): u64 {
        let e = E::V1(3);
        ref_enum_range(&e)
    }

    public fun test_ref_enum_out_range(): u64 {
        let e = E::V1(10);
        ref_enum_range(&e)
    }

    public fun test_ref_enum_v2(): u64 {
        let e = E::V2;
        ref_enum_range(&e)
    }

    // Mutable reference with range
    public fun test_mut_ref_range(x: u64): u64 {
        let mut_x = x;
        match (&mut mut_x) {
            1..10 => 1,
            _ => 0,
        }
    }
}

//# run 0xc0ffee::m::test_ref_range --args 0u64

//# run 0xc0ffee::m::test_ref_range --args 5u64

//# run 0xc0ffee::m::test_ref_range --args 10u64

//# run 0xc0ffee::m::test_ref_enum_in_range

//# run 0xc0ffee::m::test_ref_enum_out_range

//# run 0xc0ffee::m::test_ref_enum_v2

//# run 0xc0ffee::m::test_mut_ref_range --args 0u64

//# run 0xc0ffee::m::test_mut_ref_range --args 5u64

//# run 0xc0ffee::m::test_mut_ref_range --args 10u64
