module poc::upcast_internal {
    use velor_std::crypto_algebra::{Self, Element};
    use velor_std::bls12381_algebra::{Gt, Fq12};

    public entry fun main(_owner:&signer) {
        let gt_element: Element<Gt> = crypto_algebra::one<Gt>();
        let _fq12_element: Element<Fq12> = crypto_algebra::upcast<Gt, Fq12>(&gt_element);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
