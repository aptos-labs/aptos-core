module poc::multi_pairing_internal {
    use std::vector;
    use aptos_std::crypto_algebra::{
        Element,
        one, from_u64, scalar_mul, pairing, multi_pairing, add, eq, zero
    };
    use aptos_std::bls12381_algebra::{G1, G2, Gt, Fr};

    public entry fun main(_owner: &signer) {
        let p = one<G1>();
        let q = one<G2>();
        let a = from_u64<Fr>(2);
        let b = from_u64<Fr>(3);
        let p1 = scalar_mul(&p, &a);
        let q1 = scalar_mul(&q, &b);
        let p2 = scalar_mul(&p, &b);
        let q2 = scalar_mul(&q, &a);
        let pairing1 = pairing<G1, G2, Gt>(&p1, &q1);
        let pairing2 = pairing<G1, G2, Gt>(&p2, &q2);
        let expected = add(&pairing1, &pairing2);
        let g1_elements = vector::empty<Element<G1>>();
        vector::push_back(&mut g1_elements, p1);
        vector::push_back(&mut g1_elements, p2);
        let g2_elements = vector::empty<Element<G2>>();
        vector::push_back(&mut g2_elements, q1);
        vector::push_back(&mut g2_elements, q2);
        let result = multi_pairing<G1, G2, Gt>(&g1_elements, &g2_elements);
        assert!(eq(&result, &expected), 0);
        let empty_g1 = vector::empty<Element<G1>>();
        let empty_g2 = vector::empty<Element<G2>>();
        let empty_result = multi_pairing<G1, G2, Gt>(&empty_g1, &empty_g2);
        assert!(eq(&empty_result, &zero<Gt>()), 1);
    }

    #[test(owner=@0xcaffe)]
    fun a(owner: &signer){
        main(owner);
    }
}
