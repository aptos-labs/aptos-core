module sigma_protocols::utils {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, point_add, point_equals, point_clone, CompressedRistretto,
        point_compress, Scalar, point_mul, scalar_add, scalar_mul, scalar_neg
    };

    friend sigma_protocols::homomorphism;

    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 0;

    /// Adds up two vectors of Ristretto255 points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, pt| {
            r.push_back(point_add(pt, &b[i]));
        });

        r
    }

    /// Given a vector of Ristretto255 points `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_points(a: &vector<RistrettoPoint>, e: &Scalar): vector<RistrettoPoint> {
        let r = vector[];
        a.for_each_ref(|pt| {
            r.push_back(point_mul(pt, e));
        });

        r
    }

    /// Ensures two vectors of Ristretto255 points are equal.
    public fun equal_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): bool {
        let m = a.length();
        assert!(m == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let i = 0;
        while (i < m) {
            if (!point_equals(&a[i], &b[i])) {
                return false
            };

            i += 1;
        };

        true
    }

    /// Clones a vector of Ristretto255 points
    public fun points_clone(a: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        let cloned = vector[];

        a.for_each_ref(|p| {
            // TODO(Perf): Annoying limitation of our Ristretto255 module. (Should we "fix" it as per `crypto_algebra`?)
            cloned.push_back(point_clone(p));
        });

        cloned
    }

    /// Needed for Fiat-Shamir hashing
    public(friend) fun compress_points(points: &vector<RistrettoPoint>): vector<CompressedRistretto> {
        let compressed = vector[];

        let i = 0;
        let len = points.length();
        while (i < len) {
            compressed.push_back(point_compress(&points[i]));
            i += 1;
        };

        compressed
    }

    /// Adds up two vectors of scalar points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_scalars(a: &vector<Scalar>, b: &vector<Scalar>): vector<Scalar> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, a_i| {
            r.push_back(scalar_add(a_i, &b[i]));
        });

        r
    }

    /// Given a vector of scalars `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_scalars(a: &vector<Scalar>, e: &Scalar): vector<Scalar> {
        let r = vector[];
        a.for_each_ref(|s| {
            r.push_back(scalar_mul(s, e));
        });

        r
    }

    /// Given a vector of scalars `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun neg_scalars(a: &vector<Scalar>): vector<Scalar> {
        let r = vector[];
        a.for_each_ref(|s| {
            r.push_back(scalar_neg(s));
        });

        r
    }
}
