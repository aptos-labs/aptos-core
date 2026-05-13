module aptos_framework::sigma_protocol_witness {
    friend aptos_framework::sigma_protocol_proof;
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
    friend aptos_framework::sigma_protocol_proof_tests;
    #[test_only]
    friend aptos_framework::confidential_crypto_test_utils;

    use aptos_std::ristretto255::Scalar;

    /// A *secret witness* consists of a vector $w$ of $k$ scalars
    struct Witness has drop {
        w: vector<Scalar>,
    }

    /// Creates a new secret witness from a vector of scalars.
    public(friend) fun new_secret_witness(w: vector<Scalar>): Witness { Witness { w } }

    /// Returns the length of the witness: i.e., the number of scalars in it.
    public(friend) fun length(self: &Witness): u64 {
        self.w.length()
    }

    /// Returns the `i`th scalar in the witness.
    public(friend) fun get(self: &Witness, i: u64): &Scalar {
        &self.w[i]
    }

    /// Returns the underling vector of witness scalars.
    public(friend) fun get_scalars(self: &Witness): &vector<Scalar> {
        &self.w
    }
}
