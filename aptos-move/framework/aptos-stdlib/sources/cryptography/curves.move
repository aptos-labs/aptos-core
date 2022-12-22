module aptos_std::curves {
    use aptos_std::type_info::type_of;
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

    /// Get internal handle for a Scalar. Currently needed by groth16 module.
    /// TODO: can this be avoided?
    public fun get_scalar_handle<Group>(s: &Scalar<Group>): u8 {
        s.handle
    }

    /// Get internal handle for a point. Currently needed by groth16 module.
    /// TODO: can this be avoided?
    public fun get_point_handle<Group>(p: &Point<Group>): u8 {
        p.handle
    }

    /// Perform a bilinear mapping.
    /// TODO: is it possible to have 2+ mappings between same (G1,G2,Gt)? If so we need a parameter for `mapping_id`?
    public fun pairing<G1,G2,Gt>(point_1: &Point<G1>, point_2: &Point<G2>): Point<Gt> {
        Point<Gt> {
            handle: pairing_internal(point_1.handle, point_2.handle, get_pairing_id<G1,G2,Gt>())
        }
    }

    /// Scalar basics.
    public fun scalar_from_u64<G>(value: u64): Scalar<G> {
        Scalar<G> {
            handle: scalar_from_u64_internal(value, get_group_id<G>())
        }
    }

    public fun scalar_neg<T>(_scalar_1: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_neg_internal(_scalar_1.handle, get_group_id<T>())
        }
    }

    public fun scalar_add<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_add_internal(_scalar_1.handle, _scalar_2.handle, get_group_id<T>())
        }
    }

    public fun scalar_mul<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_mul_internal(_scalar_1.handle, _scalar_2.handle, get_group_id<T>())
        }
    }

    public fun scalar_inv<T>(scalar: &Scalar<T>): Option<Scalar<T>> {
        let (succeeded, handle) = scalar_inv_internal(scalar.handle, get_group_id<T>());
        if (succeeded) {
            let scalar = Scalar<T> { handle };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    public fun scalar_eq<T>(scalar_1: &Scalar<T>, scalar_2: &Scalar<T>): bool {
        scalar_eq_internal(scalar_1.handle, scalar_2.handle, get_group_id<T>())
    }

    public fun scalar_from_bytes<T>(bytes: &vector<u8>): Option<Scalar<T>> {
        let (succeeded, handle) = scalar_from_bytes_internal(*bytes, get_group_id<T>());
        if (succeeded) {
            let scalar = Scalar<T> {
                handle
            };
            std::option::some(scalar)
        } else {
            std::option::none()
        }
    }

    // Point basics.
    public fun point_identity<T>(): Point<T> {
        Point<T> {
            handle: point_identity_internal(get_group_id<T>())
        }
    }

    public fun point_generator<T>(): Point<T> {
        Point<T> {
            handle: point_generator_internal(get_group_id<T>())
        }
    }

    public fun point_add<T>(point_1: &Point<T>, point_2: &Point<T>): Point<T> {
        Point<T> {
            handle: point_add_internal(point_1.handle, point_2.handle, get_group_id<T>())
        }
    }

    public fun point_mul<G>(_scalar: &Scalar<G>, _point: &Point<G>): Point<G> {
        Point<G> {
            handle: point_mul_internal(_scalar.handle, _point.handle, get_group_id<G>())
        }
    }

    public fun scalar_to_bytes<T>(scalar: &Scalar<T>): vector<u8> {
        scalar_to_bytes_internal(scalar.handle, get_group_id<T>())
    }

    public fun point_to_bytes<T>(point: &Point<T>): vector<u8> {
        point_to_bytes_internal(point.handle, get_group_id<T>())
    }

    public fun element_from_bytes<T>(bytes: vector<u8>): Point<T> {
        Point<T> {
            handle: element_from_bytes_internal(bytes, get_group_id<T>())
        }
    }

    public fun point_eq<T>(point_1: &Point<T>, point_2: &Point<T>): bool {
        point_eq_internal(point_1.handle, point_2.handle, get_group_id<T>())
    }

    /// Group/bilinear mapping ID assignments.
    /// The assignment here should match what is in `/aptos-move/framework/src/natives/cryptography/curves.rs`.
    /// TODO: it is possible to retrieve move type info on rust end, so we do not need these ID assignments at all?
    const GID_UNKNOWN: u8 = 0;
    const GID_BLS12_381_G1: u8 = 1;
    const GID_BLS12_381_G2: u8 = 2;
    const GID_BLS12_381_Gt: u8 = 3;
    const PID_UNKNOWN: u8 = 0;
    const PID_BLS12_381: u8 = 1;

    /// Map a group to its group ID.
    fun get_group_id<G>(): u8 {
        let typ = type_of<G>();
        if (typ == type_of<BLS12_381_G1>()) {
            GID_BLS12_381_G1
        } else if (typ == type_of<BLS12_381_G2>()) {
            GID_BLS12_381_G2
        } else if (typ == type_of<BLS12_381_Gt>()) {
            GID_BLS12_381_Gt
        } else {
            GID_UNKNOWN
        }
    }

    /// Map a pairing group set to its bilinear mapping ID.
    public fun get_pairing_id<G1,G2,Gt>(): u8 {
        if (get_group_id<G1>() == GID_BLS12_381_G1 && get_group_id<G2>() == GID_BLS12_381_G2 && get_group_id<Gt>() == GID_BLS12_381_Gt) {
            PID_BLS12_381
        } else {
            PID_UNKNOWN
        }
    }

    // Native functions.

    native fun element_from_bytes_internal(bytes: vector<u8>, gid: u8): u8;
    native fun scalar_from_u64_internal(value: u64, gid: u8): u8;
    native fun scalar_from_bytes_internal(bytes: vector<u8>, gid: u8): (bool, u8);
    native fun scalar_neg_internal(handle: u8, gid: u8): u8;
    native fun scalar_add_internal(handle_1: u8, handle_2: u8, gid: u8): u8;
    native fun scalar_mul_internal(handle_1: u8, handle_2: u8, gid: u8): u8;
    native fun scalar_inv_internal(handle: u8, gid: u8): (bool, u8);
    native fun scalar_eq_internal(handle_1: u8, handle_2: u8, gid: u8): bool;
    native fun scalar_to_bytes_internal(h: u8, gid: u8): vector<u8>;
    native fun pairing_internal(p1_handle: u8, p2_handle: u8, pairing_id: u8): u8;
    native fun point_add_internal(handle_1: u8, handle_2: u8, gid: u8): u8;
    native fun point_eq_internal(handle_1: u8, handle_2: u8, gid: u8): bool;
    native fun point_identity_internal(gid: u8): u8;
    native fun point_generator_internal(gid: u8): u8;
    native fun point_mul_internal(scalar_handle: u8, point_handle: u8, gid: u8): u8;
    native fun point_to_bytes_internal(handle: u8, gid: u8): vector<u8>;

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
}
