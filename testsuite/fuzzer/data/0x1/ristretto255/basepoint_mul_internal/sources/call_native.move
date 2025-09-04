module poc::basepoint_mul_internal {
    use velor_std::ristretto255::{Self, RistrettoPoint, Scalar};

    public entry fun main(_owner: &signer) {
        let scalar: Scalar = ristretto255::new_scalar_from_u64(3u64);
        let _result_point: RistrettoPoint = ristretto255::basepoint_mul(&scalar);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
        main(owner);
    }
}
