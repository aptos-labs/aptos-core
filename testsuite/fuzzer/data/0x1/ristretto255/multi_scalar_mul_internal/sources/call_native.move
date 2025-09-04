module poc::multi_scalar_mul_internal {
    use velor_std::ristretto255;
    use std::vector;

    public entry fun main(_owner:&signer) {
        let p1 = ristretto255::point_identity();
        let p2 = ristretto255::point_identity(); // Using identity for simplicity
        let s1 = ristretto255::new_scalar_from_u64(1u64);
        let s2 = ristretto255::new_scalar_from_u64(2u64);

        let points = vector::empty<ristretto255::RistrettoPoint>();
        vector::push_back(&mut points, p1);
        vector::push_back(&mut points, p2);

        let scalars = vector::empty<ristretto255::Scalar>();
        vector::push_back(&mut scalars, s1);
        vector::push_back(&mut scalars, s2);

        let _result_point = ristretto255::multi_scalar_mul(&points, &scalars);
    }

    #[test(owner=@0x123)]
    fun a(owner:&signer){
       main(owner);
    }
}
