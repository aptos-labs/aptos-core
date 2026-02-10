/// TODO: make more functions public(friend)
module aptos_experimental::sigma_protocol_statement {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto};

    friend aptos_experimental::sigma_protocol_homomorphism;

    /// When creating a `Statement`, the # of points must match the # of compressed points.
    const E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS : u64 = 1;

    /// A *public statement* consists of:
    /// - a `points` vector of $n_1$ group elements
    /// - a `compressed_points` vector of $n_1$ compressed group elements (redundant, for faster Fiat-Shamir)
    /// - a `scalars` vector of $n_2$ scalars
    struct Statement has drop {
        points: vector<RistrettoPoint>,
        compressed_points: vector<CompressedRistretto>,
        scalars: vector<Scalar>,
    }

    /// TODO: Maybe make this a public(friend) and only let the proofs/*.move files call it, to make sure no one accidentally
    ///  calls it in confidential_asset.move
    public fun new_statement(
        points: vector<RistrettoPoint>,
        compressed_points: vector<CompressedRistretto>,
        scalars: vector<Scalar>
    ): Statement {
        assert!(points.length() == compressed_points.length(), error::invalid_argument(E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS));
        Statement { points, compressed_points, scalars }
    }

    /// Returns the $i$th elliptic curve point in the public statement.
    public fun get_point(self: &Statement, i: u64): &RistrettoPoint {
        &self.points[i]
    }

    /// Returns all the scalars in the statement.
    /// (Needed to feed in the statement in the Fiat-Shamir transform.)
    public fun get_scalars(self: &Statement): &vector<Scalar> {
        &self.scalars
    }

    /// Returns all the elliptic curve points in the statement.
    public fun get_points(self: &Statement): &vector<RistrettoPoint> {
        &self.points
    }

    /// Returns all the compressed elliptic curve points in the statement.
    /// (Needed to feed in the statement in the Fiat-Shamir transform.)
    public fun get_compressed_points(self: &Statement): &vector<CompressedRistretto> {
        &self.compressed_points
    }
}
