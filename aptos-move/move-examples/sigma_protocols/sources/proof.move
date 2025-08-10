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
    public fun new_proof(_A: vector<RistrettoPoint>, sigma: vector<Scalar>): Proof {
        Proof {
            A: _A,
            sigma,
        }
    }

    // TODO: implement a deserialize_proof!

    /// Converts the proof's response $\sigma$ into a `SecretWitness` by setting the `w` field to $\sigma$.
    /// This is needed during proof verification, when calling the homomorphism on the `Proof`'s $\sigma$, but the
    /// homomorphism expects a `SecretWitness`.
    public fun response_to_witness(self: &Proof): SecretWitness {
        new_secret_witness(self.sigma)
    }

    /// Returns the commitment component $A$ of the proof.
    public fun get_commitment(self: &Proof): &vector<RistrettoPoint> {
        &self.A
    }

    #[test_only]
    public fun empty_proof(): Proof {
        Proof {
            A: vector[],
            sigma: vector[]
        }
    }
}
