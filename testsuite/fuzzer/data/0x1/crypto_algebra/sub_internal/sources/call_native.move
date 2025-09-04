module poc::sub_internal {
    use velor_std::crypto_algebra::{Self, Element};
    use velor_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let fr_element1: Element<Fr> = crypto_algebra::from_u64<Fr>(5u64);
        let fr_element2: Element<Fr> = crypto_algebra::from_u64<Fr>(3u64);
        let _result_element: Element<Fr> = crypto_algebra::sub<Fr>(&fr_element1, &fr_element2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
