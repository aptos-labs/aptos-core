module aptos_std::algebra {
    use std::option::Option;

    struct BLS12_381_Fr has copy, drop { handle: u64 }
    struct BLS12_381_Fq has copy, drop { handle: u64 }
    struct BLS12_381_Fq2 has copy, drop { handle: u64 }
    struct BLS12_381_Fq6 has copy, drop { handle: u64 }
    struct BLS12_381_Fq12 has copy, drop { handle: u64 }
    struct BLS12_381_G1 has copy, drop { handle: u64 }
    struct BLS12_381_G2 has copy, drop { handle: u64 }
    struct BLS12_381_Gt has copy, drop { handle: u64 }

    native public fun deserialize_compressed_checked<S>(bytes: &vector<u8>): Option<S>;
    native public fun deserialize_compressed_unchecked<S>(bytes: &vector<u8>): Option<S>;
    native public fun deserialize_uncompressed_checked<S>(bytes: &vector<u8>): Option<S>;
    native public fun deserialize_uncompressed_unchecked<S>(bytes: &vector<u8>): Option<S>;
    native public fun serialize_compressed<S>(element: &S): vector<u8>;
    native public fun serialize_uncompressed<S>(element: &S): vector<u8>;
    native public fun hash_to<S>(bytes: &vector<u8>): S;
    native public fun validate<S>(element: &S): bool;

    // Field operations.
    native public fun field_add<F>(element_0: &F, element_1: &F): F;
    native public fun field_add_identity<F>(): F;
    native public fun field_div<F>(element_0: &F, element_1: &F): F;
    native public fun field_eq<F>(element_0: &F, element_1: &F): bool;
    native public fun field_inv<F>(element: &F): Option<F>;
    native public fun field_mul<F>(element_0: &F, element_1: &F): F;
    native public fun field_mul_identity<F>(): F;
    native public fun field_neg<F>(element: &F): F;
    native public fun field_sub<F>(element_0: &F, element_1: &F): F;

    native public fun field_element_from_u64<F>(val: u64): F;
    native public fun scalar_from_field_element<F>(e: &F): vector<u8>;

    // Group operations.
    native public fun group_add<G>(element_1: &G, element_2: &G): G;
    native public fun group_equal<G>(element_1: &G, element_2: &G): bool;
    native public fun group_generator<G>(): G;
    native public fun group_identity<G>(): G;
    native public fun group_multi_scalar_mul<G>(element: &vector<G>, scalar: &vector<vector<u8>>): G;
    native public fun group_neg<G>(): G;
    native public fun group_scalar_mul<G>(element: &G, scalar: &vector<u8>): G;

    native public fun pairing<G1,G2,Gt>(element_1: &G1, element_2: &G2): Gt;
    native public fun pairing_product<G1,G2,Gt>(g1_elements: &vector<G1>, g2_elements: &vector<G2>): Gt;

    #[test]
    fun test_BLS12_381_G1() {
        let g1_gen = group_generator<BLS12_381_G1>();
        let g2_gen = group_generator<BLS12_381_G1>();
        let fr_5 = field_element_from_u64<BLS12_381_Fr>(5);
        let fr_7 = field_element_from_u64<BLS12_381_Fr>(7);
        let scalar_5 = scalar_from_field_element<BLS12_381_Fr>(&fr_5);
        let scalar_7 = scalar_from_field_element<BLS12_381_Fr>(&fr_7);
        let g1_5 = group_scalar_mul(&g1_gen, &scalar_5);
        let g1_7 = group_scalar_mul(&g1_gen, &scalar_7);
        let g2_5 = group_scalar_mul(&g2_gen, &scalar_5);
        let g2_7 = group_scalar_mul(&g2_gen, &scalar_7);
        let gt_35 = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_5, &g2_7);
        let gt_35_another = pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_7, &g2_5);
        assert!(group_equal(&gt_35, &gt_35_another));

        //        let g2_generator = group_generator<BLS12_381_G2>();
        //        let scalar_1 = scalar_from_u64<BLS12_381_Scalar>(1);
        //        let scalar_2 = scalar_from_u64<BLS12_381_Scalar>(2);
        //        let scalar_3 = scalar_from_u64<BLS12_381_Scalar>(3);
        //        let paired = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Fq12>(&g1_generator, &g2_generator);
    }
}
