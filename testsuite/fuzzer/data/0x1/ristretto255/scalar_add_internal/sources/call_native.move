module poc::scalar_add_internal {
    use aptos_std::ristretto255::{Self, Scalar};

    public entry fun main(_owner: &signer) {
        let scalar1: Scalar = ristretto255::new_scalar_from_u64(2u64);
        let scalar2: Scalar = ristretto255::new_scalar_from_u64(3u64);
        let _result_scalar: Scalar = ristretto255::scalar_add(&scalar1, &scalar2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
