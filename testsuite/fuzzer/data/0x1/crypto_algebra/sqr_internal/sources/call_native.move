module poc::sqr_internal {
    use velor_std::crypto_algebra::{Self, Element};
    use velor_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let fr_element: Element<Fr> = crypto_algebra::from_u64<Fr>(3u64);
        let _squared_element: Element<Fr> = crypto_algebra::sqr<Fr>(&fr_element);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
