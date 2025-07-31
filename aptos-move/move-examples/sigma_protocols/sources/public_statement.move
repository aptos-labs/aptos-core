/// TODO: make more functions public(friend)
module sigma_protocols::public_statement {
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};

    friend sigma_protocols::homomorphism;

    /// A *public statement* consists of a vector `points` of $n_1$ group elements and a vector `scalars` of $n_2$ scalars
    struct PublicStatement has drop {
        points: vector<RistrettoPoint>,
        scalars: vector<Scalar>,
    }

    public fun new_public_statement(points: vector<RistrettoPoint>, scalars: vector<Scalar>): PublicStatement {
        PublicStatement { points, scalars }
    }

    public fun get_point(self: &PublicStatement, i: u64): &RistrettoPoint {
        &self.points[i]
    }

    public fun get_scalars(self: &PublicStatement): &vector<Scalar> {
        &self.scalars
    }

    public(friend) fun get_points(self: &PublicStatement): &vector<RistrettoPoint> {
        &self.points
    }
}
