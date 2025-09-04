module poc::point_mul_internal {
    use velor_std::ristretto255::{Self, RistrettoPoint, Scalar};

    public entry fun main(_owner: &signer) {
        let point: RistrettoPoint = ristretto255::basepoint();
        let scalar: Scalar = ristretto255::new_scalar_from_u64(2u64);
        let _result_point: RistrettoPoint = ristretto255::point_mul(&point, &scalar);
    }

    #[test(owner=@0x123)]
    fun a(owner: &signer){
        main(owner);
    }
}
