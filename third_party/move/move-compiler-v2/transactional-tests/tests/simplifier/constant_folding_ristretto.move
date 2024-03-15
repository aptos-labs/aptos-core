//# publish --print-bytecode
module 0xcafe::Ristretto {
    public fun test() {
        let non_canonical_highbit = vector[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128];
        let non_canonical_highbit_hex = x"0000000000000000000000000000000000000000000000000000000000000080";
        assert!(non_canonical_highbit == non_canonical_highbit_hex, 1);
    }
}
