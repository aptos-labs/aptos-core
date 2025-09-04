module poc::scalar_is_canonical_internal {
    use velor_std::ristretto255::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let canonical_bytes = x"0100000000000000000000000000000000000000000000000000000000000000";
        let maybe_scalar_ok = ristretto255::new_scalar_from_bytes(canonical_bytes);
        assert!(option::is_some(&maybe_scalar_ok), 1);

        let non_canonical_bytes = x"edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010";
        let maybe_scalar_fail = ristretto255::new_scalar_from_bytes(non_canonical_bytes);
        assert!(option::is_none(&maybe_scalar_fail), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
