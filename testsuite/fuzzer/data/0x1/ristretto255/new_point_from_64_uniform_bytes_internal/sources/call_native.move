module poc::new_point_from_64_uniform_bytes_internal {
    use aptos_std::ristretto255::{Self};
    use std::option::{Self};

    public entry fun main(_owner: &signer) {
        let input_bytes = x"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f202122232425262728292a2b2c2d2e2f303132333435363738393a3b3c3d3e3f";
        let maybe_point = ristretto255::new_point_from_64_uniform_bytes(input_bytes);
        assert!(option::is_some(&maybe_point), 1);

        let invalid_input_bytes = x"0001";
        let maybe_point_invalid = ristretto255::new_point_from_64_uniform_bytes(invalid_input_bytes);
        assert!(option::is_none(&maybe_point_invalid), 2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
