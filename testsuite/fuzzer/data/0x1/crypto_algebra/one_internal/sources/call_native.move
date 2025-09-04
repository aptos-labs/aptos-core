module poc::one_internal {
    use velor_std::crypto_algebra::{from_u64, one, eq, mul, scalar_mul};
    use velor_std::bls12381_algebra::{Fr, G1};

    public entry fun main(_owner: &signer) {
        let o_fr = one<Fr>();
        let o_fr_manual = from_u64<Fr>(1);
        assert!(eq(&o_fr, &o_fr_manual), 0);
        assert!(eq(&mul(&o_fr, &o_fr), &o_fr), 1);
        let g1 = one<G1>();
        let s1 = one<Fr>();
        let g1_mul_s1 = scalar_mul(&g1, &s1);
        assert!(eq(&g1, &g1_mul_s1), 2);
    }

    #[test(owner=@0xcaffe)]
    fun a(owner: &signer){
        main(owner);
    }
}
