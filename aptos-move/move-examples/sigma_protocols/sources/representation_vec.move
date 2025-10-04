module sigma_protocols::representation_vec {
    use aptos_std::ristretto255::Scalar;
    use sigma_protocols::representation::Representation;

    /// A vector of `Representations`.
    /// Used to represent the $\mathbb{G}^m$ output of the transformation function $f$ and the homomorphism $\psi$.
    struct RepresentationVec has drop {
        v: vector<Representation>
    }

    public fun new_representation_vec(v: vector<Representation>): RepresentationVec {
        RepresentationVec {
            v
        }
    }

    /// Returns all the underlying `Representation`'s stored in this vector
    /// (Public due to forced inlining for functions that take lambda arguments.)
    public fun get_representations(self: &RepresentationVec): &vector<Representation> {
        &self.v
    }

    /// Returns the number of representations in the vector.
    public fun length(self: &RepresentationVec): u64 {
        self.v.length()
    }

    /// Iterates through every representation in the vector.
    /// (Forced inlining for functions that take lambda arguments.)
    public inline fun for_each_ref(self: &RepresentationVec, lambda: |&Representation|) {
        self.get_representations().for_each_ref(|repr| lambda(repr))
    }

    /// Multiply all representations by $e$ (i.e., multiply each `self.v[i].scalars` by $e$).
    public fun scale_all(self: &mut RepresentationVec, e: &Scalar) {
        self.v.for_each_mut(|repr| {
            repr.scale(e)
        });
    }

    /// For all $i$, multiply the $i$th representation by `b[i]` (i.e., multiply `self.v[i].scalars` by `b[i]`)
    public fun scale_each(self: &mut RepresentationVec, b: &vector<Scalar>) {
        self.v.enumerate_mut(|i, repr| {
            repr.scale(&b[i])
        });
    }

    #[test_only]
    /// Returns an empty representation vector. Used for testing.
    public fun empty_representation_vec(): RepresentationVec {
        new_representation_vec(vector[])
    }
}
