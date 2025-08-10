module sigma_protocols::secret_witness {
    use aptos_std::ristretto255::Scalar;
    #[test_only]
    use std::error;
    #[test_only]
    use std::vector::range;
    #[test_only]
    use aptos_std::ristretto255::random_scalar;

    /// One of our internal invariants was broken. There is likely a logical error in the code.
    const E_INTERNAL_INVARIANT_FAILED: u64 = 0;

    /// A *secret witness* consists of a vector $w$ of $k$ scalars
    struct SecretWitness has drop {
        w: vector<Scalar>,
    }

    /// Creates a new secret witness from a vector of scalars.
    public fun new_secret_witness(w: vector<Scalar>): SecretWitness {
        SecretWitness {
            w
        }
    }

    /// Returns the length of the witness: i.e., the number of scalars in it.
    public fun length(self: &SecretWitness): u64 {
        self.w.length()
    }

    /// Returns the `i`th scalar in the witness.
    public fun get(self: &SecretWitness, i: u64): &Scalar {
        // debug::print(&string_utils::format2(&b"len = {}, i = {}", self.length(), i));
        &self.w[i]
    }

    /// Returns the underling vector of witness scalars.
    public fun get_scalars(self: &SecretWitness): &vector<Scalar> {
        &self.w
    }

    #[test_only]
    /// Returns a size-$k$ random witness. Useful when creating a ZKP during testing.
    public fun random_witness(k: u64): SecretWitness {
        let w = vector[];

        range(0, k).for_each(|_|
            w.push_back(random_scalar())
        );

        assert!(w.length() == k, error::internal(E_INTERNAL_INVARIANT_FAILED));

        new_secret_witness(
            w
        )
    }
}
