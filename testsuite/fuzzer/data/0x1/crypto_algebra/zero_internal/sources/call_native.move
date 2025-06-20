module poc::zero_internal {
    use aptos_std::crypto_algebra::{Self, Element};
    use aptos_std::bls12381_algebra::{Fr};

    public entry fun main(_owner: &signer) {
        let _fr_zero: Element<Fr> = crypto_algebra::zero<Fr>();
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
