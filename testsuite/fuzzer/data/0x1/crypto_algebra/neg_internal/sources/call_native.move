module poc::neg_internal {
    use velor_std::crypto_algebra::{from_u64, neg, add, eq, zero};
    use velor_std::bls12381_algebra::{Fr};

    public entry fun main(_owner:&signer) {
        let z = zero<Fr>();
        let x = from_u64<Fr>(5);
        let neg_x = neg(&x);
        let sum = add(&x, &neg_x);
        assert!(eq(&sum, &z), 0);
        let neg_z = neg(&z);
        assert!(eq(&neg_z, &z), 1);
    }

    #[test(owner=@0xcaffe)]
    fun a(owner: &signer){
        main(owner);
    }
}
