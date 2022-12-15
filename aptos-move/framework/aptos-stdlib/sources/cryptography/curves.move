module aptos_std::curves {
    struct Scalar<phantom T> has drop {
        handle: u64
    }
    struct Point<phantom T> has drop {
        handle: u64
    }

    struct BLS12_381_G1 {}
    struct BLS12_381_G2 {}
    struct BLS12_381_Gt {}

    //...
    public fun pairing<G1,G2,Gt>(_p1: &Point<G1>, _p2: &Point<G2>): Point<Gt> {
        Point<Gt> {
            handle: pairing_internal<G1,G2,Gt>(_p1.handle, _p2.handle)
        }
    }


    /// Scalar basics.
    public fun scalar_zero<G>(): Scalar<G> {
        Scalar<G> {
            handle: scalar_zero_internal<G>()
        }
    }

    public fun scalar_one<G>(): Scalar<G> {
        Scalar<G> {
            handle: scalar_one_internal<G>()
        }
    }

    public fun scalar_neg<T>(_scalar_1: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_neg_internal<T>()
        }
    }

    public fun scalar_add<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): Scalar<T> {
        Scalar<T> {
            handle: scalar_add_internal<T>(_scalar_1.handle, _scalar_2.handle)
        }
    }

    public fun scalar_mul<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): Scalar<T> {
        //todo
        Scalar<T> {
            handle: scalar_mul_internal<T>(_scalar_1.handle, _scalar_2.handle)
        }
    }

    public fun scalar_inv<T>(_scalar: &Scalar<T>): Scalar<T> {
        //todo
        Scalar<T> {
            handle: scalar_inv_internal<T>(_scalar.handle)
        }
    }

    public fun scalar_eq<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): bool {
        scalar_eq_internal<T>(_scalar_1.handle, _scalar_2.handle)
    }

    public fun bytes_into_scalar<T>(bytes: vector<u8>): Scalar<T> {
        Scalar<T> {
            handle: bytes_into_scalar_internal<T>(bytes)
        }
    }

    // Point basics.
    public fun point_zero<T>(): Point<T> {
        Point<T> {
            handle: point_zero_internal<T>()
        }
    }

    public fun point_identity<T>(): Point<T> {
        Point<T> {
            handle: point_identity_internal<T>()
        }
    }

    public fun point_add<T>(_point_1: &Point<T>, _point_2: &Point<T>): Point<T> {
        Point<T> {
            handle: point_add_internal<T>(_point_1.handle, _point_2.handle)
        }
    }

    public fun point_mul<G>(_scalar: &Scalar<G>, _point: &Point<G>): Point<G> {
        Point<G> {
            handle: point_mul_internal<G>(_scalar.handle, _point.handle)
        }
    }

    public fun scalar_to_bytes<T>(scalar: &Scalar<T>): vector<u8> {
        scalar_to_bytes_internal<T>(scalar.handle)
    }

    public fun point_to_bytes<T>(point: &Point<T>): vector<u8> {
        point_to_bytes_internal<T>(point.handle)
    }

    public fun bytes_into_point<T>(bytes: vector<u8>): Point<T> {
        Point<T> {
            handle: bytes_into_point_internal<T>(bytes)
        }
    }

    public fun point_eq<T>(_point_1: &Point<T>, _point_2: &Point<T>): bool {
        point_eq_internal<T>(_point_1.handle, _point_2.handle)
    }

    native fun bytes_into_point_internal<T>(bytes: vector<u8>): u64;
    native fun bytes_into_scalar_internal<T>(bytes: vector<u8>): u64;
    native fun scalar_zero_internal<G>(): u64;
    native fun scalar_one_internal<G>(): u64;
    native fun scalar_neg_internal<G>(): u64;
    native fun scalar_add_internal<G>(handle_1: u64, handle_2: u64): u64;
    native fun scalar_mul_internal<G>(h1: u64, h2: u64): u64;
    native fun scalar_inv_internal<G>(handle: u64): u64;
    native fun scalar_eq_internal<T>(h1: u64, h2: u64): bool;
    native fun scalar_to_bytes_internal<T>(h: u64): vector<u8>;
    native fun pairing_internal<G1,G2,Gt>(p1_handle: u64, p2_handle: u64): u64;
    native fun point_add_internal<T>(h1: u64, h2: u64): u64;
    native fun point_eq_internal<T>(h1: u64, h2: u64): bool;
    native fun point_identity_internal<G>(): u64;
    native fun point_mul_internal<G>(scalar_handle: u64, point_handle: u64): u64;
    native fun point_to_bytes_internal<G>(h: u64): vector<u8>;
    native fun point_zero_internal<T>(): u64;

    #[test]
    fun test_scalar_point_arithmatics() {
        let one = scalar_one<BLS12_381_G1>();
        let two = scalar_add(&one, &one);
        let point_p1 = point_identity<BLS12_381_G1>();
        let point_p2 = point_add(&point_p1, &point_p1);
        let point_q = point_mul(&two, &point_p1);
        assert!(point_eq(&point_q, &point_p2), 1);
    }
}
