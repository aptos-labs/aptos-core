module poc::scalar_invert_internal {
    use velor_std::ristretto255::{Self, Scalar};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let scalar_val: Scalar = ristretto255::new_scalar_from_u64(5u64);
        let maybe_inverted = ristretto255::scalar_invert(&scalar_val);
        assert!(option::is_some(&maybe_inverted), 1);

        let zero_scalar = ristretto255::scalar_zero();
        let maybe_inverted_zero = ristretto255::scalar_invert(&zero_scalar);
        assert!(option::is_none(&maybe_inverted_zero), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
