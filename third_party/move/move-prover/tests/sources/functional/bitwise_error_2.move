module 0x42::bitset128 {

    spec module {
        global x: num;
    }

    struct BitSet128 has drop {
        s: u128
    }

    public fun insert(s: &mut BitSet128, i: u64) {
        s.s |= (i as u128);
    }

    public fun shift(s: &mut BitSet128, i: u8) {
        s.s <<= i;
    }

    spec shift {
        ensures s.s == (old(s.s) << x); // x needs to be an concrete integer type
    }

}
