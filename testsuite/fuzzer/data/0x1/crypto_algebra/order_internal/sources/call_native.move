module poc::order_internal {
    use velor_std::crypto_algebra;
    use velor_std::bls12381_algebra::G1;
    use std::vector;

    public entry fun main(_owner: &signer) {
        let order_bytes: vector<u8> = crypto_algebra::order<G1>();
        assert!(vector::length(&order_bytes) > 0, 1);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
