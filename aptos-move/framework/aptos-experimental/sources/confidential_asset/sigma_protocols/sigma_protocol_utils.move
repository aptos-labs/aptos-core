module aptos_experimental::sigma_protocol_utils {
    friend aptos_experimental::sigma_protocol_proof;

    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto,
        new_point_and_compressed_from_bytes, new_compressed_point_from_bytes,
        new_scalar_from_bytes
    };

    /// Clones a vector of Ristretto255 points
    // TODO(Perf): Annoying limitation of our Ristretto255 module. (Should we "fix" it as per `crypto_algebra`?)
    public fun points_clone(a: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        a.map_ref(|p| p.point_clone())
    }

    /// Deserializes a vector of point bytes to a vector of RistrettoPoints and a vector of their compressed counterparts.
    public(friend) fun deserialize_points(points_bytes: vector<vector<u8>>): (vector<RistrettoPoint>, vector<CompressedRistretto>) {
        let points = vector[];
        let compressed_points = vector[];
        points_bytes.for_each(|point_bytes| {
            let (point, compressed_point) = new_point_and_compressed_from_bytes(point_bytes);
            points.push_back(point);
            compressed_points.push_back(compressed_point);
        });

        (points, compressed_points)
    }

    /// Deserializes a vector of point bytes to a vector of CompressedRistretto's (without decompressing to RistrettoPoint).
    public fun deserialize_compressed_points(points_bytes: vector<vector<u8>>): vector<CompressedRistretto> {
        points_bytes.map(|bytes| new_compressed_point_from_bytes(bytes).extract())
    }

    public(friend) fun deserialize_scalars(scalars_bytes: vector<vector<u8>>): vector<Scalar> {
        scalars_bytes.map(|scalar_bytes| new_scalar_from_bytes(scalar_bytes).extract())
    }

    // === Shared error codes for sigma protocol proof modules ===

    use std::error;

    public fun e_wrong_num_points(): u64 { error::invalid_argument(1) }
    public fun e_wrong_num_scalars(): u64 { error::invalid_argument(2) }
    public fun e_wrong_witness_len(): u64 { error::invalid_argument(3) }
    public fun e_wrong_output_len(): u64 { error::invalid_argument(4) }

    /// Negates a vector of scalars `a`, returns a new vector `c` where `c[i] = -a[i]`.
    public fun neg_scalars(a: &vector<Scalar>): vector<Scalar> {
        a.map_ref(|s| s.scalar_neg())
    }

    #[test_only]
    use aptos_std::ristretto255::point_identity_compressed;

    #[test_only]
    public fun decompress_points(compressed: &vector<CompressedRistretto>): vector<RistrettoPoint> {
        compressed.map_ref(|p| p.point_decompress())
    }

    #[test_only]
    public fun compress_points(points: &vector<RistrettoPoint>): vector<CompressedRistretto> {
        points.map_ref(|p| p.point_compress())
    }

    #[test_only]
    const E_INTERNAL_INVARIANT_FAILED: u64 = 1;

    #[test_only]
    /// Returns a vector of `n` compressed identity (zero) points.
    public fun compressed_identity_points(n: u64): vector<CompressedRistretto> {
        std::vector::range(0, n).map(|_| point_identity_compressed())
    }

    #[test_only]
    /// Adds up two vectors of points `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): vector<RistrettoPoint> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, pt| {
            r.push_back(pt.point_add(&b[i]));
        });

        r
    }

    #[test_only]
    /// Given a vector of Ristretto255 points `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_points(a: &vector<RistrettoPoint>, e: &Scalar): vector<RistrettoPoint> {
        a.map_ref(|pt| pt.point_mul(e))
    }

    #[test_only]
    /// Ensures two vectors of Ristretto255 points are equal.
    public fun equal_vec_points(a: &vector<RistrettoPoint>, b: &vector<RistrettoPoint>): bool {
        let m = a.length();
        assert!(m == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        std::vector::range(0, m).all(|i| a[*i].point_equals(&b[*i]))
    }

    #[test_only]
    /// Adds up two vectors of scalars `a` and `b` returning a new vector `c` where `c[i] = a[i] + b[i]`.
    public fun add_vec_scalars(a: &vector<Scalar>, b: &vector<Scalar>): vector<Scalar> {
        assert!(a.length() == b.length(), error::internal(E_INTERNAL_INVARIANT_FAILED));

        let r = vector[];
        a.enumerate_ref(|i, a_i| {
            r.push_back(a_i.scalar_add(&b[i]));
        });

        r
    }

    #[test_only]
    /// Given a vector of scalars `a` and a scalar `e`, returns a new vector `c` where `c[i] = e * a[i]`.
    public fun mul_scalars(a: &vector<Scalar>, e: &Scalar): vector<Scalar> {
        a.map_ref(|s| s.scalar_mul(e))
    }
}
