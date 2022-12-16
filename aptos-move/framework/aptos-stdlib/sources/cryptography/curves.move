module aptos_std::curves {
    use aptos_std::type_info::type_of;

    // Structs and consts.

    // Fake structs representing group type.
    struct BLS12_381_G1 {}
    struct BLS12_381_G2 {}
    struct BLS12_381_Gt {}

    struct Scalar<phantom Group> has drop {
        handle: u64
    }

    struct Point<phantom Group> has drop {
        handle: u64
    }

    /// Get internal handle. May not be needed.
    public fun get_scalar_handle<Group>(s: &Scalar<Group>): u64 {
        s.handle
    }

    public fun get_point_handle<Group>(p: &Point<Group>): u64 {
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
    public fun scalar_zero<G>(): Scalar<G> {
        Scalar<G> {
            handle: scalar_zero_internal(get_group_id<G>())
        }
    }

    public fun scalar_one<G>(): Scalar<G> {
        Scalar<G> {
            handle: scalar_one_internal(get_group_id<G>())
        }
    }

    public fun scalar_from_u64<G>(value: u64): Scalar<G> {
        Scalar<G> {
            handle: scalar_from_u64_internal(value, get_group_id<G>())
        }
    }

    public fun scalar_neg<T>(_scalar_1: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_neg_internal(get_group_id<T>())
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

    public fun scalar_inv<T>(scalar: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_inv_internal(scalar.handle, get_group_id<T>())
        }
    }

    public fun scalar_eq<T>(scalar_1: &Scalar<T>, scalar_2: &Scalar<T>): bool {
        scalar_eq_internal(scalar_1.handle, scalar_2.handle, get_group_id<T>())
    }

    public fun bytes_into_scalar<T>(bytes: vector<u8>): Scalar<T> {
        Scalar<T> {
            handle: bytes_into_scalar_internal(bytes, get_group_id<T>())
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

    public fun bytes_into_point<T>(bytes: vector<u8>): Point<T> {
        Point<T> {
            handle: bytes_into_point_internal(bytes, get_group_id<T>())
        }
    }

    public fun point_eq<T>(point_1: &Point<T>, point_2: &Point<T>): bool {
        point_eq_internal(point_1.handle, point_2.handle, get_group_id<T>())
    }

    /// Group/bilinear mapping ID assignments.
    /// Move side and rust side share the same ID assignments.
    /// NOTE: If it is possible to retrieve move type info on rust end, we do not need ID assignments at all.
    const GID_UNKNOWN: u64 = 0;
    const GID_BLS12_381_G1: u64 = 1;
    const GID_BLS12_381_G2: u64 = 2;
    const GID_BLS12_381_Gt: u64 = 3;
    const PID_UNKNOWN: u64 = 0;
    const PID_BLS12_381: u64 = 1;

    /// Map a group to its group ID.
    fun get_group_id<G>(): u64 {
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
    public fun get_pairing_id<G1,G2,Gt>(): u64 {
        if (get_group_id<G1>() == GID_BLS12_381_G1 && get_group_id<G2>() == GID_BLS12_381_G2 && get_group_id<Gt>() == GID_BLS12_381_Gt) {
            PID_BLS12_381
        } else {
            PID_UNKNOWN
        }
    }

    // Native functions.

    native fun bytes_into_point_internal(bytes: vector<u8>, gid: u64): u64;
    native fun bytes_into_scalar_internal(bytes: vector<u8>, gid: u64): u64;
    native fun scalar_zero_internal(gid: u64): u64;
    native fun scalar_one_internal(gid: u64): u64;
    native fun scalar_from_u64_internal(value: u64, gid: u64): u64;
    native fun scalar_neg_internal(gid: u64): u64;
    native fun scalar_add_internal(handle_1: u64, handle_2: u64, gid: u64): u64;
    native fun scalar_mul_internal(handle_1: u64, handle_2: u64, gid: u64): u64;
    native fun scalar_inv_internal(handle: u64, gid: u64): u64;
    native fun scalar_eq_internal(handle_1: u64, handle_2: u64, gid: u64): bool;
    native fun scalar_to_bytes_internal(h: u64, gid: u64): vector<u8>;
    native fun pairing_internal(p1_handle: u64, p2_handle: u64, pairing_id: u64): u64;
    native fun point_add_internal(handle_1: u64, handle_2: u64, gid: u64): u64;
    native fun point_eq_internal(handle_1: u64, handle_2: u64, gid: u64): bool;
    native fun point_identity_internal(gid: u64): u64;
    native fun point_generator_internal(gid: u64): u64;
    native fun point_mul_internal(scalar_handle: u64, point_handle: u64, gid: u64): u64;
    native fun point_to_bytes_internal(handle: u64, gid: u64): vector<u8>;

    #[test]
    fun test_scalar_point_arithmatics() {
        let one = scalar_one<BLS12_381_G2>();
        let two = scalar_add(&one, &one);
        let point_p1 = point_generator<BLS12_381_G2>();
        let point_p2 = point_add(&point_p1, &point_p1);
        let point_q = point_mul(&two, &point_p1);
        assert!(point_eq(&point_q, &point_p2), 1);
    }

    #[test]
    fun test_bilinear() {
        let gt_point_1 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &point_mul(&scalar_from_u64(5), &point_generator<BLS12_381_G1>()),
            &point_mul(&scalar_from_u64(7), &point_generator<BLS12_381_G2>()),
        );
        let gt_point_2 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &point_mul(&scalar_one(), &point_generator()),
            &point_mul(&scalar_from_u64(35), &point_generator()),
        );
        let gt_point_3 = pairing<BLS12_381_G1, BLS12_381_G2, BLS12_381_Gt>(
            &point_mul(&scalar_from_u64(35), &point_generator<BLS12_381_G1>()),
            &point_mul(&scalar_one(), &point_generator<BLS12_381_G2>()),
        );
        assert!(point_eq(&gt_point_1, &gt_point_2), 1);
        assert!(point_eq(&gt_point_1, &gt_point_3), 1);
    }
}
