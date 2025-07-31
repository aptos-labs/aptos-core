module sigma_protocols::representation {
    use std::error;
    use std::vector::range;
    use aptos_std::ristretto255::{Scalar, scalar_mul_assign, RistrettoPoint};
    use sigma_protocols::public_statement::PublicStatement;


    /// The number of points and scalars in a Representation needs to be the same.
    const E_MISMATCHED_LENGTHS: u64 = 1;

    /// A *representation* of a group element $G$ is a list of group elements $G_i$ and scalars $a_i$ such that:
    ///   $G = \sum_{i \in [n]} a_i G_i$
    /// The actual group elements are large, so we store their position in `points`. i.e., to indicate that $G_i$ is the
    /// $j$th entry from the `PublicStatement::points` vector, we set `points[i] = j`. (Note that
    /// $j \in [0, n_1)$.)
    ///
    /// The output of the transformation function $f$ and the homomorphism $\psi$ consists of $m$ group elements, each
    /// of which will have a `Representation` (e.g., for $\psi$ such a representation is will be in terms of the
    /// `PublicStatement` points and the `SecretWitness` scalars).
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

    public fun length(self: &Representation): u64 {
        self.points.length()
    }

    public fun get_point_idx(self: &Representation, i: u64): u64 {
        self.points[i]
    }

    public fun get_scalar(self: &Representation, i: u64): &Scalar {
        &self.scalars[i]
    }

    /// Multiplies all the scalars in the representation by $e$.
    public fun scale(self: &mut Representation, e: &Scalar) {
        self.scalars.for_each_mut(|scalar| {
            scalar_mul_assign(scalar, e);
        });
    }

    /// Iterates through each Ristretto255 point and its scalar in the `Representation`. Recall that a `Representation`
    /// only stores the index of the Ristretto255 points w.r.t. the `PublicStatement::points` vector, which is given as
    /// input here.
    public inline fun for_each(self: &Representation, stmt: &PublicStatement, lambda: |&RistrettoPoint, &Scalar|) {
        let len = self.length();
        range(0, len).for_each(|i| {
            lambda(
                stmt.get_point(self.get_point_idx(i)),
                self.get_scalar(i)
            )
        });
    }

}
