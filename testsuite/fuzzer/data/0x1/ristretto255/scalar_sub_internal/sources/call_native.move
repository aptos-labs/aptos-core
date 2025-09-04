module poc::scalar_sub_internal {
    use velor_std::ristretto255::{Self, Scalar};

    public entry fun main(_owner: &signer) {
        let scalar1: Scalar = ristretto255::new_scalar_from_u64(5u64);
        let scalar2: Scalar = ristretto255::new_scalar_from_u64(3u64);
        let _result_scalar: Scalar = ristretto255::scalar_sub(&scalar1, &scalar2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
