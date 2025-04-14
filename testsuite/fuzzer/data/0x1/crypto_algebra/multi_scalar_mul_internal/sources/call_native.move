module poc::multi_scalar_mul_internal {
    use std::vector;
    use aptos_std::crypto_algebra::{Element, one, from_u64, scalar_mul, multi_scalar_mul, add, eq, zero};
    use aptos_std::bls12381_algebra::{G1, Fr};

    public entry fun main(_owner: &signer) {
        let p = one<G1>();
        let a = from_u64<Fr>(2);
        let b = from_u64<Fr>(3);
        let p1 = scalar_mul(&p, &a);
        let p2 = scalar_mul(&p, &b);
        let expected = add(&p1, &p2);
        let elements = vector::empty<Element<G1>>();
        vector::push_back(&mut elements, p);
        vector::push_back(&mut elements, p);
        let scalars = vector::empty<Element<Fr>>();
        vector::push_back(&mut scalars, a);
        vector::push_back(&mut scalars, b);
        let result = multi_scalar_mul<G1, Fr>(&elements, &scalars);
        assert!(eq(&result, &expected), 0);
        let empty_elements = vector::empty<Element<G1>>();
        let empty_scalars = vector::empty<Element<Fr>>();
        let empty_result = multi_scalar_mul<G1, Fr>(&empty_elements, &empty_scalars);
        assert!(eq(&empty_result, &zero<G1>()), 1);
    }

    #[test(owner=@0xcaffe)]
    fun a(owner: &signer){
        main(owner);
    }
}
