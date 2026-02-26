module aptos_experimental::sigma_protocol_witness {
    use aptos_std::ristretto255::Scalar;
    #[test_only]
    use std::vector::range;
    #[test_only]
    use aptos_std::ristretto255::random_scalar;

    /// A *secret witness* consists of a vector $w$ of $k$ scalars
    struct Witness has drop {
        w: vector<Scalar>,
    }

    /// Creates a new secret witness from a vector of scalars.
    public fun new_secret_witness(w: vector<Scalar>): Witness {
        Witness {
            w
        }
    }

    /// Returns the length of the witness: i.e., the number of scalars in it.
    public fun length(self: &Witness): u64 {
        self.w.length()
    }

    /// Returns the `i`th scalar in the witness.
    public fun get(self: &Witness, i: u64): &Scalar {
        // debug::print(&string_utils::format2(&b"len = {}, i = {}", self.length(), i));
        &self.w[i]
    }

    /// Returns the underling vector of witness scalars.
    public fun get_scalars(self: &Witness): &vector<Scalar> {
        &self.w
    }

    #[test_only]
    /// Returns a size-$k$ random witness. Useful when creating a ZKP during testing.
    public fun random(k: u64): Witness {
        new_secret_witness(range(0, k).map(|_| random_scalar()))
    }
}
