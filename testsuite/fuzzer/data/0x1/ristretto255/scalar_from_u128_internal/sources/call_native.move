module poc::scalar_from_u128_internal {
    use velor_std::ristretto255::{Self, Scalar};

    public entry fun main(_owner: &signer) {
        let input_val = 123456789012345678901234567890123456789u128;
        let _scalar: Scalar = ristretto255::new_scalar_from_u128(input_val);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
