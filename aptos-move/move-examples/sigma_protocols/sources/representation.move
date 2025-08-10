module sigma_protocols::representation {
    use std::error;
    use aptos_std::ristretto255::{Scalar, scalar_mul_assign, RistrettoPoint, point_clone};
    use sigma_protocols::public_statement::PublicStatement;

    /// The number of points and scalars in a Representation needs to be the same.
    const E_MISMATCHED_LENGTHS: u64 = 1;

    /// A *representation* of a group element $G$ is a list of group elements $G_i$ and scalars $a_i$ such that:
    ///   $G = \sum_{i \in [n_1]} a_i G_i$
    /// The actual group elements are large, so to indicate that $G_i$ is the $j$th entry from the
    /// `PublicStatement::points` vector, we set `Representation::points[i]` to $j$. (Note that $j \in [0, n_1)$.)
    ///
    /// Note: Instead of returning $m$ group elements, the Move implementation of a transformation function $f$ (and/or
    /// a homomorphism $\psi$) will return $m$ representations. This makes it possible to implement a faster verifier
    /// (and prover too) that uses multi-scalar multiplications!
    struct Representation has copy, drop {
        points: vector<u64>,
        scalars: vector<Scalar>,
    }

    public fun new_representation(points: vector<u64>, scalars: vector<Scalar>): Representation {
        assert!(points.length() == scalars.length(), error::invalid_argument(E_MISMATCHED_LENGTHS));
        Representation {
            points, scalars
        }
    }

    /// Given a representation, which only stores locations of group elements within a public statement, returns the
    /// actual vector of group elements by "looking up" these elements in the public statement.
    public fun to_points(self: &Representation, stmt: &PublicStatement): vector<RistrettoPoint> {
        let bases = vector[];

        self.points.for_each(|idx| {
            bases.push_back(point_clone(stmt.get_point(idx)));
        });

        bases
    }

    /// Returns the scalars in the representation.
    public fun get_scalars(self: &Representation): &vector<Scalar> {
        &self.scalars
    }

    /// Multiplies all the scalars in the representation by $e$.
    public fun scale(self: &mut Representation, e: &Scalar) {
        self.scalars.for_each_mut(|scalar| {
            scalar_mul_assign(scalar, e);
        });
    }
}

// Prize-winning stuff!

    // public fun length(self: &Representation): u64 {
    //     self.points.length()
    // }

    // public fun get_scalar(self: &Representation, i: u64): &Scalar {
    //     &self.scalars[i]
    // }

    // public fun get_point_idx(self: &Representation, i: u64): u64 {
    //     self.points[i]
    // }

    // Iterates through each Ristretto255 point and its scalar in the `Representation`. Recall that a `Representation`
    // only stores the index of the Ristretto255 points w.r.t. the `PublicStatement::points` vector, which is given as
    // input here.
    // public inline fun for_each(self: &Representation, stmt: &PublicStatement, lambda: |&RistrettoPoint, &Scalar|) {
    //     let len = self.length();
    //     range(0, len).for_each(|i| {
    //         lambda(
    //             stmt.get_point(self.get_point_idx(i)),
    //             self.get_scalar(i)
    //         )
    //     });
    // }

//}
