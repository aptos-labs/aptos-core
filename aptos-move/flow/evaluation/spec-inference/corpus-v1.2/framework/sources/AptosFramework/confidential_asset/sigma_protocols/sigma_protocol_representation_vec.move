module aptos_framework::sigma_protocol_representation_vec {
    friend aptos_framework::sigma_protocol_homomorphism;
    friend aptos_framework::sigma_protocol;
    friend aptos_framework::sigma_protocol_registration;
    friend aptos_framework::sigma_protocol_withdraw;
    friend aptos_framework::sigma_protocol_transfer;
    friend aptos_framework::sigma_protocol_key_rotation;
    #[test_only]
    friend aptos_framework::sigma_protocol_pedeq_example;
    #[test_only]
    friend aptos_framework::sigma_protocol_schnorr_example;
    #[test_only]
    friend aptos_framework::confidential_crypto_test_utils;

    use aptos_std::ristretto255::Scalar;
    use aptos_framework::sigma_protocol_representation::Representation;

    /// A vector of `Representations`.
    /// Used to represent the output of the transformation function $f$ and the homomorphism $\psi$
    /// (i.e., a vector in $\mathbb{G}^m$).
    struct RepresentationVec has drop {
        v: vector<Representation>
    }

    public(friend) fun new_representation_vec(v: vector<Representation>): RepresentationVec { RepresentationVec { v } }

    /// Returns all the underlying `Representation`'s stored in this vector
    /// (Public due to forced inlining for functions that take lambda arguments.)
    public(friend) fun get_representations(self: &RepresentationVec): &vector<Representation> {
        &self.v
    }

    /// Returns the number of representations in the vector.
    public(friend) fun length(self: &RepresentationVec): u64 {
        self.v.length()
    }

    /// Iterates through every representation in the vector.
    /// (Forced inlining for functions that take lambda arguments.)
    public(friend) inline fun for_each_ref(self: &RepresentationVec, lambda: |&Representation|) {
        self.get_representations().for_each_ref(|repr| lambda(repr))
    }

    /// Maps each representation in the vector to a value of type `T`.
    public(friend) inline fun map_ref<T>(self: &RepresentationVec, lambda: |&Representation| T): vector<T> {
        self.get_representations().map_ref(|repr| lambda(repr))
    }

    /// Multiply all representations by $e$ (i.e., multiply each `self.v[i].scalars` by $e$).
    public(friend) fun scale_all(self: &mut RepresentationVec, e: &Scalar) {
        self.v.for_each_mut(|repr| repr.scale(e));
    }

    /// For all $i$, multiply the $i$th representation by `b[i]` (i.e., multiply `self.v[i].scalars` by `b[i]`)
    public(friend) fun scale_each(self: &mut RepresentationVec, b: &vector<Scalar>) {
        self.v.enumerate_mut(|i, repr| {
            repr.scale(&b[i])
        });
    }
}
