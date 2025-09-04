module poc::scalar_from_u64_internal {
    use velor_std::ristretto255::{Self, Scalar};

    public entry fun main(_owner: &signer) {
        let input_val = 12345678901234567890u64;
        let _scalar: Scalar = ristretto255::new_scalar_from_u64(input_val);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
