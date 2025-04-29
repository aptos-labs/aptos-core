module poc::point_sub_internal {
    use aptos_std::ristretto255::{Self, RistrettoPoint};

    public entry fun main(_owner: &signer) {
        let point1: RistrettoPoint = ristretto255::basepoint();
        let point2: RistrettoPoint = ristretto255::basepoint();
        let _result_point: RistrettoPoint = ristretto255::point_sub(&point1, &point2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
