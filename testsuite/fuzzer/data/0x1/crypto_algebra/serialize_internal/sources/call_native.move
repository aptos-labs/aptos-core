module poc::serialize_internal {
    use velor_std::crypto_algebra::{Self, Element};
    use velor_std::bls12381_algebra::{G1, FormatG1Compr};
    use std::vector;

    public entry fun main(_owner: &signer) {
        let g1_element: Element<G1> = crypto_algebra::one<G1>();
        let serialized_bytes: vector<u8> = crypto_algebra::serialize<G1, FormatG1Compr>(&g1_element);
        assert!(vector::length(&serialized_bytes) == 48, 1);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
