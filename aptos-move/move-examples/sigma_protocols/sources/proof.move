module sigma_protocols::proof {
    use aptos_std::ristretto255::{RistrettoPoint, Scalar};
    use sigma_protocols::secret_witness::{SecretWitness, new_secret_witness};

    /// A sigma protocol *proof* always consists of:
    /// 1. a *commitment* $A \in \mathbb{G}^m$
    /// 2. a *response* $\sigma \in \mathbb{F}^k$
    struct Proof has drop {
        A: vector<RistrettoPoint>,
        sigma: vector<Scalar>,
    }

    /// Creates a new proof consisting of the commitment $A \in \mathbb{G}^m$ and the scalars $\sigma \in \mathbb{F}^k$.
    public fun new(_A: vector<RistrettoPoint>, sigma: vector<Scalar>): Proof {
        Proof {
            A: _A,
            sigma,
        }
    }

    /// Returns a `SecretWitness` with the `w` field is to the proof's $\sigma$ field.
    /// This is needed during proof verification: when calling the homomorphism on the `Proof`'s $\sigma$, it expects a
    /// `SecretWitness` not a `vector<Scalar>`.
    public fun response_to_witness(self: &Proof): SecretWitness {
        new_secret_witness(self.sigma)
    }

    /// Returns $k = |\sigma|$.
    public fun get_response_length(self: &Proof): u64 {
        self.sigma.length()
    }

    /// Returns the commitment component $A$ of the proof.
    public fun get_commitment(self: &Proof): &vector<RistrettoPoint> {
        &self.A
    }

    #[test_only]
    /// Returns an empty proof. Used for testing.
    public fun empty(): Proof {
        Proof {
            A: vector[],
            sigma: vector[]
        }
    }
}
