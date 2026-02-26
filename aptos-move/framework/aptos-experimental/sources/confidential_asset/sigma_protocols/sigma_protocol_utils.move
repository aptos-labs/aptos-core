module aptos_experimental::sigma_protocol_utils {
    use std::error;
    use aptos_std::ristretto255;
    use aptos_std::ristretto255::{RistrettoPoint,
        Scalar, CompressedRistretto
    };

    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 1;

    /// Adds up two vectors of Ristretto255 points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, pt| {
            r.push_back(pt.point_add(&b[i]));
        });

        r
    }

    /// Given a vector of Ristretto255 points `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_points(a: &vector<RistrettoPoint>, e: &Scalar): vector<RistrettoPoint> {
        a.map_ref(|pt| pt.point_mul(e))
    }

    /// Ensures two vectors of Ristretto255 points are equal.
    public fun equal_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): bool {
        let m = a.length();
        assert!(m == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let i = 0;
        while (i < m) {
            if (!a[i].point_equals(&b[i])) {
                return false
            };

            i += 1;
        };

        true
    }

    /// Clones a vector of Ristretto255 points
    // TODO(Perf): Annoying limitation of our Ristretto255 module. (Should we "fix" it as per `crypto_algebra`?)
    public fun points_clone(a: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        a.map_ref(|p| p.point_clone())
    }

    /// Deserializes a vector of point bytes to a vector of RistrettoPoints and a vector of their compressed counterparts.
    public fun deserialize_points(points_bytes: vector<vector<u8>>): (vector<RistrettoPoint>, vector<CompressedRistretto>) {
        let points = vector[];
        let compressed_points = vector[];
        points_bytes.for_each(|point_bytes| {
            let (point, compressed_point) = ristretto255::new_point_and_compressed_from_bytes(point_bytes);

            points.push_back(point);
            compressed_points.push_back(compressed_point);
        });

        assert!(points.length() == points_bytes.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));
        assert!(points.length() == compressed_points.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        (points, compressed_points)
    }

    /// Deserializes a vector of scalar bytes to a vector of Scalar's
    public fun deserialize_scalars(scalars_bytes: vector<vector<u8>>): vector<Scalar> {
        scalars_bytes.map(|scalar_bytes| {
            ristretto255::new_scalar_from_bytes(scalar_bytes).extract()

        })
    }

    /// Decmpresses a vector of CompressedRistretto's
    public fun decompress_points(compressed: &vector<CompressedRistretto>): vector<RistrettoPoint> {
        compressed.map_ref(|p| {
            p.point_decompress()
        })
    }

    /// Compresses a vector of Ristretto255 points.
    public fun compress_points(points: &vector<RistrettoPoint>): vector<CompressedRistretto> {
        points.map_ref(|p| p.point_compress())
    }

    /// Adds up two vectors of scalar points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_scalars(a: &vector<Scalar>, b: &vector<Scalar>): vector<Scalar> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, a_i| {
            r.push_back(a_i.scalar_add(&b[i]));
        });

        r
    }

    /// Given a vector of scalars `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_scalars(a: &vector<Scalar>, e: &Scalar): vector<Scalar> {
        a.map_ref(|s| s.scalar_mul(e))
    }

    /// Negates a vector of scalars `a`, returns a new vector `c` where `c[i] = -a[i]`.
    public fun neg_scalars(a: &vector<Scalar>): vector<Scalar> {
        a.map_ref(|s| s.scalar_neg())
    }
}
