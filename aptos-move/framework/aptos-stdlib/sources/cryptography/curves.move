module aptos_std::curves {
    use std::option::Option;

    /// This is a phantom type that represents the 1st pairing input group `G1` in BLS12-381 pairing:
    /// TODO: describe the encoding.
    struct BLS12_381_G1 {}

    /// This is a phantom type that represents the 2nd pairing input group `G2` in BLS12-381 pairing.
    /// TODO: describe the encoding.
    struct BLS12_381_G2 {}

    /// This is a phantom type that represents the pairing output group `Gt` in BLS12-381 pairing.
    /// TODO: describe the encoding.
    struct BLS12_381_Gt {}

    /// This struct represents a scalar, usually an integer between 0 and `r-1`,
    /// where `r` is the prime order of a group, where the group is determined by the type argument `G`.
    /// See the comments on the specific `G` for more details about `Scalar<G>`.
    struct Scalar<phantom G> has copy, drop {
        //TODO: handle as u8 temporarily. Upgrade to u64.
        handle: u8
    }

    /// This struct represents a group element, usually a point in an elliptic curve.
    /// The group is determined by the type argument `G`.
    /// See the comments on the specific `G` for more details about `Element<G>`.
    struct Element<phantom G> has copy, drop {
        handle: u8
    }

    /// Perform a bilinear mapping.
    public fun pairing<G1,G2,Gt>(point_1: &Element<G1>, point_2: &Element<G2>): Element<Gt> {
        Element<Gt> {
            handle: pairing_internal<G1,G2,Gt>(point_1.handle, point_2.handle)
        }
    }

    public fun multi_pairing<G1,G2,Gt>(g1_elements: &vector<Element<G1>>, g2_elements: &vector<Element<G2>>): Element<Gt> {
        let num_g1 = std::vector::length(g1_elements);
        let num_g2 = std::vector::length(g2_elements);
        assert!(num_g1 == num_g2, 1);
        let g1_handles = vector[];
        let g2_handles = vector[];
        let i = 0;
        while (i < num_g2) {
            std::vector::push_back(&mut g1_handles, std::vector::borrow(g1_elements, i).handle);
            std::vector::push_back(&mut g2_handles, std::vector::borrow(g2_elements, i).handle);
            i = i + 1;
        };

        Element<Gt> {
            handle: multi_pairing_internal<G1,G2,Gt>(g1_handles, g2_handles)
        }
    }

    public fun scalar_from_u64<G>(value: u64): Scalar<G> {
        Scalar<G> {
            handle: scalar_from_u64_internal<G>(value)
        }
    }

    public fun scalar_neg<G>(scalar_1: &Scalar<G>): Scalar<G> {
        Scalar<G> {
            handle: scalar_neg_internal<G>(scalar_1.handle)
        }
    }

    public fun scalar_add<G>(scalar_1: &Scalar<G>, scalar_2: &Scalar<G>): Scalar<G> {
        Scalar<G> {
            handle: scalar_add_internal<G>(scalar_1.handle, scalar_2.handle)
        }
    }

    public fun scalar_mul<G>(scalar_1: &Scalar<G>, scalar_2: &Scalar<G>): Scalar<G> {
        Scalar<G> {
            handle: scalar_mul_internal<G>(scalar_1.handle, scalar_2.handle)
        }
    }

    public fun scalar_inv<G>(scalar: &Scalar<G>): Option<Scalar<G>> {
        let (succeeded, handle) = scalar_inv_internal<G>(scalar.handle);
        if (succeeded) {
            let scalar = Scalar<G> { handle };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    public fun scalar_eq<G>(scalar_1: &Scalar<G>, scalar_2: &Scalar<G>): bool {
        scalar_eq_internal<G>(scalar_1.handle, scalar_2.handle)
    }

    public fun scalar_from_bytes<G>(bytes: &vector<u8>): Option<Scalar<G>> {
        let (succeeded, handle) = scalar_from_bytes_internal<G>(*bytes);
        if (succeeded) {
            let scalar = Scalar<G> {
                handle
            };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    // Point basics.
    public fun identity<G>(): Element<G> {
        Element<G> {
            handle: identity_internal<G>()
        }
    }

    public fun generator<G>(): Element<G> {
        Element<G> {
            handle: generator_internal<G>()
        }
    }

    public fun element_neg<G>(point: &Element<G>): Element<G> {
        Element<G> {
            handle: element_neg_internal<G>(point.handle)
        }
    }

    public fun element_add<G>(point_1: &Element<G>, point_2: &Element<G>): Element<G> {
        Element<G> {
            handle: element_add_internal<G>(point_1.handle, point_2.handle)
        }
    }

    public fun element_mul<G>(_scalar: &Scalar<G>, _point: &Element<G>): Element<G> {
        Element<G> {
            handle: element_mul_internal<G>(_scalar.handle, _point.handle)
        }
    }

    public fun simul_point_mul<G>(scalars: &vector<Scalar<G>>, points: &vector<Element<G>>): Element<G> {
        //TODO: replace the naive implementation.
        let result = identity<G>();
        let num_points = std::vector::length(points);
        let num_scalars = std::vector::length(scalars);
        assert!(num_points == num_scalars, 1);
        let i = 0;
        while (i < num_points) {
            let scalar = std::vector::borrow(scalars, i);
            let point = std::vector::borrow(points, i);
            result = element_add(&result, &element_mul(scalar, point));
            i = i + 1;
        };
        result
    }

    public fun scalar_to_bytes<G>(scalar: &Scalar<G>): vector<u8> {
        scalar_to_bytes_internal<G>(scalar.handle)
    }

    public fun element_to_bytes<G>(point: &Element<G>): vector<u8> {
        element_to_bytes_internal<G>(point.handle)
    }

    public fun deserialize_element_uncompressed<G>(bytes: vector<u8>): Option<Element<G>> {
        let (succ, handle) = deserialize_element_uncompressed_internal<G>(bytes);
        if (succ) {
            std::option::some(Element<G> { handle })
        } else {
            std::option::none()
        }
    }

    public fun element_eq<G>(point_1: &Element<G>, point_2: &Element<G>): bool {
        element_eq_internal<G>(point_1.handle, point_2.handle)
    }

    // Native functions.
    native fun deserialize_element_uncompressed_internal<G>(bytes: vector<u8>): (bool, u8);
    native fun scalar_from_u64_internal<G>(value: u64): u8;
    native fun scalar_from_bytes_internal<G>(bytes: vector<u8>): (bool, u8);
    native fun scalar_neg_internal<G>(handle: u8): u8;
    native fun scalar_add_internal<G>(handle_1: u8, handle_2: u8): u8;
    native fun scalar_mul_internal<G>(handle_1: u8, handle_2: u8): u8;
    native fun scalar_inv_internal<G>(handle: u8): (bool, u8);
    native fun scalar_eq_internal<G>(handle_1: u8, handle_2: u8): bool;
    native fun scalar_to_bytes_internal<G>(h: u8): vector<u8>;
    native fun element_add_internal<G>(handle_1: u8, handle_2: u8): u8;
    native fun element_eq_internal<G>(handle_1: u8, handle_2: u8): bool;
    native fun identity_internal<G>(): u8;
    native fun generator_internal<G>(): u8;
    native fun element_mul_internal<G>(scalar_handle: u8, point_handle: u8): u8;
    native fun element_neg_internal<G>(handle: u8): u8;
    native fun element_to_bytes_internal<G>(handle: u8): vector<u8>;
    native fun pairing_internal<G1,G2,Gt>(g1_handle: u8, g2_handle: u8): u8;
    native fun multi_pairing_internal<G1,G2,Gt>(g1_handles: vector<u8>, g2_handles: vector<u8>): u8;

    #[test]
    fun test_scalar_mul() {
        let scalar_33 = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"2100000000000000000000000000000000000000000000000000000000000000"));
        let scalar_34 = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"2200000000000000000000000000000000000000000000000000000000000000"));
        let scalar_1122 = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"6204000000000000000000000000000000000000000000000000000000000000"));
        assert!(scalar_eq(&scalar_1122, &scalar_mul(&scalar_33, &scalar_34)), 1);
    }

    #[test]
    fun test_scalar_neg() {
        let scalar_33 = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"2100000000000000000000000000000000000000000000000000000000000000"));
        let scalar_33_neg = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"e0fffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73"));
        assert!(scalar_eq(&scalar_33_neg, &scalar_neg(&scalar_33)), 1);
    }

    #[test]
    fun test_scalar_add() {
        let scalar_33 = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"2100000000000000000000000000000000000000000000000000000000000000"));
        let scalar_32_neg = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"e1fffffffefffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73"));
        assert!(scalar_eq(&scalar_from_u64(1), &scalar_add(&scalar_33, &scalar_32_neg)), 1);
    }

    #[test]
    fun test_scalar_inv() {
        let scalar_33 = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"2100000000000000000000000000000000000000000000000000000000000000"));
        let scalar_33_inv = std::option::extract(&mut scalar_from_bytes<BLS12_381_G1>(&x"01000000e0830f3ed8e4eec16713724981f09c205d5530413e81bfbbad546a70"));
        assert!(scalar_eq(&scalar_33_inv, &std::option::extract(&mut scalar_inv(&scalar_33))), 1);
        assert!(scalar_eq(&scalar_from_u64<BLS12_381_G1>(1), &scalar_mul(&scalar_33,&scalar_33_inv)), 1);
    }

    #[test]
    fun test_bilinear() {
        let gt_point_1 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_mul(&scalar_from_u64(5), &generator<BLS12_381_G1>()),
            &element_mul(&scalar_from_u64(7), &generator<BLS12_381_G2>()),
        );
        let gt_point_2 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_mul(&scalar_from_u64(1), &generator()),
            &element_mul(&scalar_from_u64(35), &generator()),
        );
        let gt_point_3 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &element_mul(&scalar_from_u64(35), &generator<BLS12_381_G1>()),
            &element_mul(&scalar_from_u64(1), &generator<BLS12_381_G2>()),
        );
        assert!(element_eq(&gt_point_1, &gt_point_2), 1);
        assert!(element_eq(&gt_point_1, &gt_point_3), 1);
    }

    #[test]
    fun test_multi_pairing() {
        let g1_point_1 = generator<BLS12_381_G1>();
        let g2_point_1 = generator<BLS12_381_G2>();
        let g1_point_2 = element_mul(&scalar_from_u64<BLS12_381_G1>(5), &g1_point_1);
        let g2_point_2 = element_mul(&scalar_from_u64<BLS12_381_G2>(2), &g2_point_1);
        let g1_point_3 = element_mul(&scalar_from_u64<BLS12_381_G1>(20), &g1_point_1);
        let g2_point_3 = element_mul(&scalar_from_u64<BLS12_381_G2>(5), &g2_point_1);
        let expected = element_mul(&scalar_from_u64<BLS12_381_Gt>(111), &pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point_1, &g2_point_1));
        let actual = multi_pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(&vector[g1_point_1, g1_point_2, g1_point_3], &vector[g2_point_1, g2_point_2, g2_point_3]);
        assert!(element_eq(&expected, &actual), 1);
    }

//    #[test]
//    fun test_bls12_381_g1_basics() {
//        let p = deserialize_element_uncompressed<BLS12_381_G1>(x"");
//        let generator = generator<BLS12_381_G1>();
//        let point_10g = element_mul(&scalar_from_u64<BLS12_381_G1>(10), &generator);
//        let point_2g = element_mul(&scalar_from_u64<BLS12_381_G1>(2), &generator);
//        let point_12g = element_mul(&scalar_from_u64<BLS12_381_G1>(12), &generator);
//        let point_20g = element_mul(&scalar_from_u64<BLS12_381_G1>(20), &generator);
//        let point_8g = element_mul(&scalar_from_u64<BLS12_381_G1>(8), &generator);
//        let point_5g = element_mul(&scalar_from_u64<BLS12_381_G1>(5), &generator);
//    }
}
