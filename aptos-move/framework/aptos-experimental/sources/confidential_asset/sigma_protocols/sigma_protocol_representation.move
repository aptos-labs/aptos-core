// TODO: Consider renaming to MSM / MSMs
module aptos_experimental::sigma_protocol_representation {
    use std::error;
    use aptos_std::ristretto255::{Scalar, RistrettoPoint};
    use aptos_experimental::sigma_protocol_statement::Statement;

    /// The number of points and scalars in a Representation needs to be the same.
    const E_MISMATCHED_LENGTHS: u64 = 1;

    /// A *representation* of a group element $G$ is a list of group elements $G_i$ and scalars $a_i$ such that:
    ///   $G = \sum_{i \in [n_1]} a_i G_i$
    /// The actual group elements are large, so to indicate that $G_i$ is the $j$th entry from the
    /// `Statement::points` vector, we set `Representation::points_idxs[i]` to $j$. (Note that $j \in [0, n_1)$.)
    ///
    /// Note: Instead of returning $m$ group elements, the Move implementation of a transformation function $f$ (and/or
    /// a homomorphism $\psi$) will return $m$ representations. This makes it possible to implement a faster verifier
    /// (and prover too) that uses multi-scalar multiplications!
    struct Representation has copy, drop {
        point_idxs: vector<u64>,
        scalars: vector<Scalar>,
    }

    public fun new_representation(points: vector<u64>, scalars: vector<Scalar>): Representation {
        assert!(points.length() == scalars.length(), error::invalid_argument(E_MISMATCHED_LENGTHS));
        Representation {
            point_idxs: points, scalars
        }
    }

    /// Given a representation, which only stores locations of group elements within a public statement, returns the
    /// actual vector of group elements by "looking up" these elements in the public statement.
    public fun to_points(self: &Representation, stmt: &Statement): vector<RistrettoPoint> {
        self.point_idxs.map(|idx| stmt.get_point(idx).point_clone())
    }

    /// Returns the scalars in the representation.
    public fun get_scalars(self: &Representation): &vector<Scalar> {
        &self.scalars
    }

    /// Multiplies all the scalars in the representation by $e$.
    public fun scale(self: &mut Representation, e: &Scalar) {
        self.scalars.for_each_mut(|scalar| {
            scalar.scalar_mul_assign(e);
        });
    }
}
