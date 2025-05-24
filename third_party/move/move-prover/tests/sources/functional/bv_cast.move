module 0x42::test {

    struct U256 has copy, drop, store {
        n0: u64,
        n1: u64,
        n2: u64,
        n3: u64
    }

    spec U256 {
      pragma bv=b"0,1,2,3";
    }

    public fun zero(): U256 {
        U256 {
            n0: 0,
            n1: 0,
            n2: 0,
            n3: 0
        }
    }

    spec zero {
        pragma bv_ret = b"0";
        ensures U256_to_u256(result) == 0;
    }
    spec fun U256_to_u256(n: U256): u256 {
        (n.n0 as u256) |
        ((n.n1 as u256) << 64) |
        ((n.n2 as u256) << 128) |
        ((n.n3 as u256) << 192)
    }

    fun test_complement(v: u16): u16 {
        twos_complement(v)
    }

    spec test_complement {
        ensures result == twos_complement(v);
        aborts_if false;
    }

    fun twos_complement(v: u16): u16 {
        if (v == 0) 0
        else (v ^ 0xffff) + 1
    }

    spec twos_complement {
        pragma bv=b"0";
        pragma bv_ret=b"0";
    }

}
