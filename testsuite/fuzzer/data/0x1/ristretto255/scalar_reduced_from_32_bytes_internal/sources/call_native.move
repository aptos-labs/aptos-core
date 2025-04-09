module poc::scalar_reduced_from_32_bytes_internal {
    use aptos_std::ristretto255::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let input_bytes = x"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        let maybe_scalar = ristretto255::new_scalar_reduced_from_32_bytes(input_bytes);
        assert!(option::is_some(&maybe_scalar), 1);

        let invalid_bytes = x"0001";
        let maybe_scalar_invalid = ristretto255::new_scalar_reduced_from_32_bytes(invalid_bytes);
        assert!(option::is_none(&maybe_scalar_invalid), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
