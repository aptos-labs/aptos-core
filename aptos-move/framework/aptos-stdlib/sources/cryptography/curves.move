module aptos_std::curves {
    struct Scalar<phantom T> has drop {
        bytes: vector<u8>
    }
    struct Point<phantom T> has drop {
        bytes: vector<u8>
    }

    struct BLS12_381_G1 {}
    struct BLS12_381_G2 {}
    struct BLS12_381_Gt {}

    //...
    public fun pairing<G1,G2,Gt>(_p1: &Point<G1>, _p2: &Point<G2>): Point<Gt> {
        Point<Gt> {
            bytes: b"asdf"
        }
    }

    /// Scalar basics.

    public fun scalar_zero<G>(): Scalar<G> {
        Scalar<G> {
            bytes: b"asdf"
        }
    }

    public native fun scalar_one<G>(): Scalar<G>;

    public fun scalar_neg<T>(_scalar_1: &Scalar<T>): Scalar<T> {
        //todo
        Scalar<T> {
            bytes: b"asdf"
        }
    }

    public fun scalar_add<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): Scalar<T> {
        //todo
        Scalar<T> {
            bytes: b"asdf"
        }
    }

    public fun scalar_mul<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): Scalar<T> {
        //todo
        Scalar<T> {
            bytes: b"asdf"
        }
    }

    public fun scalar_inv<T>(_scalar: &Scalar<T>): Scalar<T> {
        //todo
        Scalar<T> {
            bytes: b"asdf"
        }
    }

    public fun scalar_eq<T>(_scalar_1: &Scalar<T>, _scalar_2: &Scalar<T>): bool {
        //todo
        false
    }

    // Point basics.
    public fun point_zero<T>(): Point<T> {
        Point<T> {
            bytes: b"adf"
        }
    }

    public fun point_identity<T>(): Point<T> {
        Point<T> {
            bytes: b"adf"
        }
    }

    public fun point_add<T>(_point_1: &Point<T>, _point_2: &Point<T>): Point<T> {
        Point<T> {
            bytes: b"adf"
        }
    }

    public fun point_mul<G>(_scalar: &Scalar<G>, _point: &Point<G>): Point<G> {
        Point<G> {
            bytes: b"asdf"
        }
    }

    public fun scalar_to_bytes<T>(_scalar :&Scalar<T>): vector<u8> {
        b"adfd"
    }

    public fun point_to_bytes<T>(_element :&Point<T>): vector<u8> {
        b"asdf"
    }

    public fun scalar_from_bytes<T>(bytes: &vector<u8>): Scalar<T> {
        Scalar<T> {
            bytes: *bytes
        }
    }

    public fun point_from_bytes<T>(bytes: &vector<u8>): Point<T> {
        Point<T> {
            bytes: *bytes
        }
    }

    public fun point_eq<T>(_point_1: &Point<T>, _point_2: &Point<T>): bool {
        //todo
        false
    }

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
