module aptos_std::curves {
    use std::option::Option;

    // Structs and consts.

    // Fake structs representing group type.
    struct BLS12_381_G1 {}
    struct BLS12_381_G2 {}
    struct BLS12_381_Gt {}

    //TODO: handle as u8 temporarily. Upgrade to u64.
    struct Scalar<phantom Group> has copy, drop {
        handle: u8
    }

    struct Point<phantom Group> has copy, drop {
        handle: u8
    }

    /// Perform a bilinear mapping.
    public fun pairing<G1,G2,Gt>(point_1: &Point<G1>, point_2: &Point<G2>): Point<Gt> {
        Point<Gt> {
            handle: pairing_internal<G1,G2,Gt>(point_1.handle, point_2.handle)
        }
    }

    public fun multi_pairing<G1,G2,Gt>(g1_elements: &vector<Point<G1>>, g2_elements: &vector<Point<G2>>): Point<Gt> {
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

        Point<Gt> {
            handle: multi_pairing_internal<G1,G2,Gt>(g1_handles, g2_handles)
        }
    }

    /// Scalar basics.
    public fun scalar_from_u64<G>(value: u64): Scalar<G> {
        Scalar<G> {
            handle: scalar_from_u64_internal<G>(value)
        }
    }

    public fun scalar_neg<G>(_scalar_1: &Scalar<G>): Scalar<G> {
        Scalar<G> {
            handle: scalar_neg_internal<G>(_scalar_1.handle)
        }
    }

    public fun scalar_add<G>(_scalar_1: &Scalar<G>, _scalar_2: &Scalar<G>): Scalar<G> {
        Scalar<G> {
            handle: scalar_add_internal<G>(_scalar_1.handle, _scalar_2.handle)
        }
    }

    public fun scalar_mul<G>(_scalar_1: &Scalar<G>, _scalar_2: &Scalar<G>): Scalar<G> {
        Scalar<G> {
            handle: scalar_mul_internal<G>(_scalar_1.handle, _scalar_2.handle)
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
    public fun point_identity<G>(): Point<G> {
        Point<G> {
            handle: point_identity_internal<G>()
        }
    }

    public fun point_generator<G>(): Point<G> {
        Point<G> {
            handle: point_generator_internal<G>()
        }
    }

    public fun point_add<G>(point_1: &Point<G>, point_2: &Point<G>): Point<G> {
        Point<G> {
            handle: point_add_internal<G>(point_1.handle, point_2.handle)
        }
    }

    public fun point_mul<G>(_scalar: &Scalar<G>, _point: &Point<G>): Point<G> {
        Point<G> {
            handle: point_mul_internal<G>(_scalar.handle, _point.handle)
        }
    }

    public fun simul_point_mul<G>(scalars: &vector<Scalar<G>>, points: &vector<Point<G>>): Point<G> {
        //TODO: replace the naive implementation.
        let result = point_identity<G>();
        let num_points = std::vector::length(points);
        let num_scalars = std::vector::length(scalars);
        assert!(num_points == num_scalars, 1);
        let i = 0;
        while (i < num_points) {
            let scalar = std::vector::borrow(scalars, i);
            let point = std::vector::borrow(points, i);
            result = point_add(&result, &point_mul(scalar, point));
            i = i + 1;
        };
        result
    }

    public fun scalar_to_bytes<G>(scalar: &Scalar<G>): vector<u8> {
        scalar_to_bytes_internal<G>(scalar.handle)
    }

    public fun point_to_bytes<G>(point: &Point<G>): vector<u8> {
        point_to_bytes_internal<G>(point.handle)
    }

    public fun element_from_bytes<G>(bytes: vector<u8>): Point<G> {
        Point<G> {
            handle: element_from_bytes_internal<G>(bytes)
        }
    }

    public fun point_eq<G>(point_1: &Point<G>, point_2: &Point<G>): bool {
        point_eq_internal<G>(point_1.handle, point_2.handle)
    }

    // Native functions.

    native fun element_from_bytes_internal<G>(bytes: vector<u8>): u8;
    native fun scalar_from_u64_internal<G>(value: u64): u8;
    native fun scalar_from_bytes_internal<G>(bytes: vector<u8>): (bool, u8);
    native fun scalar_neg_internal<G>(handle: u8): u8;
    native fun scalar_add_internal<G>(handle_1: u8, handle_2: u8): u8;
    native fun scalar_mul_internal<G>(handle_1: u8, handle_2: u8): u8;
    native fun scalar_inv_internal<G>(handle: u8): (bool, u8);
    native fun scalar_eq_internal<G>(handle_1: u8, handle_2: u8): bool;
    native fun scalar_to_bytes_internal<G>(h: u8): vector<u8>;
    native fun pairing_internal<G1,G2,Gt>(g1_handle: u8, g2_handle: u8): u8;
    native fun multi_pairing_internal<G1,G2,Gt>(g1_handles: vector<u8>, g2_handles: vector<u8>): u8;
    native fun point_add_internal<G>(handle_1: u8, handle_2: u8): u8;
    native fun point_eq_internal<G>(handle_1: u8, handle_2: u8): bool;
    native fun point_identity_internal<G>(): u8;
    native fun point_generator_internal<G>(): u8;
    native fun point_mul_internal<G>(scalar_handle: u8, point_handle: u8): u8;
    native fun point_to_bytes_internal<G>(handle: u8): vector<u8>;

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
            &point_mul(&scalar_from_u64(5), &point_generator<BLS12_381_G1>()),
            &point_mul(&scalar_from_u64(7), &point_generator<BLS12_381_G2>()),
        );
        let gt_point_2 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &point_mul(&scalar_from_u64(1), &point_generator()),
            &point_mul(&scalar_from_u64(35), &point_generator()),
        );
        let gt_point_3 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &point_mul(&scalar_from_u64(35), &point_generator<BLS12_381_G1>()),
            &point_mul(&scalar_from_u64(1), &point_generator<BLS12_381_G2>()),
        );
        assert!(point_eq(&gt_point_1, &gt_point_2), 1);
        assert!(point_eq(&gt_point_1, &gt_point_3), 1);
    }

    #[test]
    fun test_multi_pairing() {
        let g1_point_1 = point_generator<BLS12_381_G1>();
        let g2_point_1 = point_generator<BLS12_381_G2>();
        let g1_point_2 = point_mul(&scalar_from_u64<BLS12_381_G1>(5), &g1_point_1);
        let g2_point_2 = point_mul(&scalar_from_u64<BLS12_381_G2>(2), &g2_point_1);
        let g1_point_3 = point_mul(&scalar_from_u64<BLS12_381_G1>(20), &g1_point_1);
        let g2_point_3 = point_mul(&scalar_from_u64<BLS12_381_G2>(5), &g2_point_1);
        let expected = point_mul(&scalar_from_u64<BLS12_381_Gt>(111), &pairing<BLS12_381_G1,BLS12_381_G2,BLS12_381_Gt>(&g1_point_1, &g2_point_1));
        let actual = multi_pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(&vector[g1_point_1, g1_point_2, g1_point_3], &vector[g2_point_1, g2_point_2, g2_point_3]);
        assert!(point_eq(&expected, &actual), 1);
    }
}
