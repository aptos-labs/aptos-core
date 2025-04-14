module poc::double_scalar_mul_internal {
    use aptos_std::ristretto255::{Self, RistrettoPoint, Scalar};

    public entry fun main(_owner: &signer) {
        let scalar1: Scalar = ristretto255::new_scalar_from_u64(2u64);
        let scalar2: Scalar = ristretto255::new_scalar_from_u64(3u64);
        let point1: RistrettoPoint = ristretto255::basepoint();
        let point2: RistrettoPoint = ristretto255::point_identity();

        let _result_point: RistrettoPoint = ristretto255::double_scalar_mul(&scalar1, &point1, &scalar2, &point2);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
