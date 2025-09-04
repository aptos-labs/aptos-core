module poc::point_neg_internal {
    use velor_std::ristretto255::{Self, RistrettoPoint};

    public entry fun main(_owner: &signer) {
        let point: RistrettoPoint = ristretto255::basepoint();
        let _neg_point: RistrettoPoint = ristretto255::point_neg(&point);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
