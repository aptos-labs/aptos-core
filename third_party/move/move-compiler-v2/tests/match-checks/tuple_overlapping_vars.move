module 0xc0ffee::m {
    enum Data {
        V1 { x: u64 },
        V2 { x: u64 },
    }

    // Overlapping variable name: enum field `x` and tuple position `x`.
    fun overlap_enum_and_prim(d: Data, n: u64): u64 {
        match ((d, n)) {
            (Data::V1 { x }, x) => x,
            _ => 0,
        }
    }
}
