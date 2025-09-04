module poc::double_internal {
    use velor_std::crypto_algebra::{Self};
    use velor_std::bls12381_algebra::{G1};

    public entry fun main(_owner: &signer) {
        let p = crypto_algebra::one<G1>();
        let p_plus_p = crypto_algebra::add(&p, &p);
        let p_doubled = crypto_algebra::double(&p);
        assert!(crypto_algebra::eq(&p_doubled, &p_plus_p), 0);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
