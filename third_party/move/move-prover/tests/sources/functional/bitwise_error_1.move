module 0x42::bitset128 {

    struct BitSet128 has drop {
        s: u128
    }

    public fun insert(s: &mut BitSet128, i: u64) {
        s.s |= (i as u128);
    }

    spec insert {
        ensures s.s == old(s.s) | i; // i needs explicit type cast
    }

}
