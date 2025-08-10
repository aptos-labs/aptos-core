/// TODO: make more functions public(friend)
module sigma_protocols::public_statement {
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};

    friend sigma_protocols::homomorphism;

    /// A *public statement* consists of:
    /// - a `points` vector of $n_1$ group elements
    /// - a `scalars` vector of $n_2$ scalars
    struct PublicStatement has drop {
        points: vector<RistrettoPoint>,
        scalars: vector<Scalar>,
    }

    public fun new_public_statement(points: vector<RistrettoPoint>, scalars: vector<Scalar>): PublicStatement {
        PublicStatement { points, scalars }
    }

    /// Returns the $i$th elliptic curve point in the public statement.
    public fun get_point(self: &PublicStatement, i: u64): &RistrettoPoint {
        &self.points[i]
    }

    /// Returns all the scalars in the statement.
    /// (Needed to feed in the statement in the Fiat-Shamir transform.)
    public fun get_scalars(self: &PublicStatement): &vector<Scalar> {
        &self.scalars
    }

    /// Returns all the elliptic curve points in the statement.
    /// (Needed to feed in the statement in the Fiat-Shamir transform.)
    public fun get_points(self: &PublicStatement): &vector<RistrettoPoint> {
        &self.points
    }
}
