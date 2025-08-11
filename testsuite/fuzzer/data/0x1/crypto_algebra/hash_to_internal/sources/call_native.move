module poc::hash_to_internal {
    use aptos_std::crypto_algebra::{Self};
    use aptos_std::bls12381_algebra::{G1, HashG1XmdSha256SswuRo};

    public entry fun main(_owner: &signer) {
        let msg1 = b"hello";
        let msg2 = b"world";
        let dst1 = b"MY_DST_1";
        let dst2 = b"MY_DST_2";

        let hash1_a = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&dst1, &msg1);
        let hash1_b = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&dst1, &msg1);
        let hash2 = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&dst1, &msg2);
        let hash3 = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&dst2, &msg1);

        assert!(crypto_algebra::eq(&hash1_a, &hash1_b), 0);
        assert!(!crypto_algebra::eq(&hash1_a, &hash2), 1);
        assert!(!crypto_algebra::eq(&hash1_a, &hash3), 2);
    }

    #[test(owner = @0x123)]
    fun a(owner: &signer) {
        main(owner);
    }
}
