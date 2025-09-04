module poc::scalar_mul_internal {
    use velor_std::crypto_algebra::{Self, Element};
    use velor_std::bls12381_algebra::{G1, Fr};

    public entry fun main(_owner: &signer) {
        let g1_element: Element<G1> = crypto_algebra::one<G1>();
        let fr_scalar: Element<Fr> = crypto_algebra::from_u64<Fr>(2u64);

        let _result_element: Element<G1> = crypto_algebra::scalar_mul<G1, Fr>(&g1_element, &fr_scalar);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
