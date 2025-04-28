module poc::pairing_internal {
    use aptos_std::crypto_algebra::{Self, Element};
    use aptos_std::bls12381_algebra::{G1, G2, Gt};

    public entry fun main(_owner: &signer) {
        let g1_element: Element<G1> = crypto_algebra::one<G1>();
        let g2_element: Element<G2> = crypto_algebra::one<G2>();

        let _gt_result: Element<Gt> = crypto_algebra::pairing<G1, G2, Gt>(&g1_element, &g2_element);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
