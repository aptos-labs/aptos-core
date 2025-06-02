module poc::point_identity_internal {
    use aptos_std::ristretto255::{Self, RistrettoPoint};

    public entry fun main(_owner: &signer) {
        let _identity_point: RistrettoPoint = ristretto255::point_identity();
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
