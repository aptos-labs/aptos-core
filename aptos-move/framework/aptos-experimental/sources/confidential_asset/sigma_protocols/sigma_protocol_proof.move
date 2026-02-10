module aptos_experimental::sigma_protocol_proof {
    use std::error;
    use aptos_std::ristretto255::{RistrettoPoint, Scalar, CompressedRistretto};
    use aptos_experimental::sigma_protocol_utils;
    use aptos_experimental::sigma_protocol_witness::{Witness, new_secret_witness};

    /// When creating a `Proof`, the # of commitment points must match the # of compressed commitment points.
    const E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS : u64 = 1;

    /// A sigma protocol *proof* always consists of:
    /// 1. a *commitment* $A \in \mathbb{G}^m$
    /// 2. a *compressed commitment* (redundant, for faster Fiat-Shamir)
    /// 3. a *response* $\sigma \in \mathbb{F}^k$
    struct Proof has drop {
        comm_A: vector<RistrettoPoint>,
        compressed_comm_A: vector<CompressedRistretto>,
        resp_sigma: vector<Scalar>,
    }

    /// Creates a new proof consisting of the commitment $A \in \mathbb{G}^m$ and the scalars $\sigma \in \mathbb{F}^k$.
    public fun new_proof(
        _A: vector<RistrettoPoint>,
        compressed_A: vector<CompressedRistretto>,
        sigma: vector<Scalar>
    ): Proof {
        assert!(_A.length() == compressed_A.length(), error::invalid_argument(E_MISMATCHED_NUMBER_OF_COMPRESSED_POINTS));

        Proof {
            comm_A: _A,
            compressed_comm_A: compressed_A,
            resp_sigma: sigma,
        }
    }

    /// Deserializes the elliptic curve points and scalars and then calls `new_proof`.
    public fun new_proof_from_bytes(
        _A_bytes: vector<vector<u8>>,
        sigma_bytes: vector<vector<u8>>
    ): Proof {
        let (_A, compressed_A) = sigma_protocol_utils::deserialize_points(_A_bytes);

        new_proof(_A, compressed_A, sigma_protocol_utils::deserialize_scalars(sigma_bytes))
    }

    /// Returns a `Witness` with the `w` field set to the proof's $\sigma$ field.
    /// This is needed during proof verification: when calling the homomorphism on the `Proof`'s $\sigma$, it expects a
    /// `Witness` not a `vector<Scalar>`.
    public fun response_to_witness(self: &Proof): Witness {
        new_secret_witness(self.resp_sigma)
    }

    public fun get_commitment(self: &Proof): &vector<RistrettoPoint> {
        &self.comm_A
    }

    public fun get_compressed_commitment(self: &Proof): &vector<CompressedRistretto> {
        &self.compressed_comm_A
    }

    public fun get_response_length(self: &Proof): u64 {
        self.resp_sigma.length()
    }

    public fun get_response(self: &Proof): &vector<Scalar> {
        &self.resp_sigma
    }

    #[test_only]
    /// Returns an empty proof. Used for testing.
    public fun empty(): Proof {
        Proof {
            comm_A: vector[],
            compressed_comm_A: vector[],
            resp_sigma: vector[]
        }
    }
}
