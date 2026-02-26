/// The `confidential_proof` module provides range proof verification helpers used by the Confidential Asset protocol.
/// Proof enums and their verify/prove functions live in `confidential_asset` (since Move disallows friend
/// modules from constructing/destructuring enum variants).
module aptos_experimental::confidential_proof {
    use std::error;
    use aptos_std::ristretto255::{Self, RistrettoPoint};
    use aptos_std::ristretto255_bulletproofs::{Self as bulletproofs, RangeProof};

    #[test_only]
    use aptos_std::ristretto255::Scalar;

    friend aptos_experimental::confidential_asset;

    //
    // Errors
    //

    const ERANGE_PROOF_VERIFICATION_FAILED: u64 = 2;

    //
    // Constants
    //

    const BULLETPROOFS_DST: vector<u8> = b"AptosConfidentialAsset/BulletproofRangeProof";
    const BULLETPROOFS_NUM_BITS: u64 = 16;

    //
    // Range proof verification helpers
    //

    /// Asserts that the given commitment chunks are each in [0, 2^16) via a range proof.
    public(friend) fun assert_valid_range_proof(
        commitments: &vector<RistrettoPoint>,
        zkrp: &RangeProof
    ) {
        let commitments = commitments.map_ref(|c| c.point_clone());

        assert!(
            bulletproofs::verify_batch_range_proof(
                &commitments,
                &ristretto255::basepoint(),
                &ristretto255::hash_to_point_base(),
                zkrp,
                BULLETPROOFS_NUM_BITS,
                BULLETPROOFS_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );
    }

    //
    // Public view functions
    //

    #[view]
    /// Returns the DST for the range proofs.
    public fun get_bulletproofs_dst(): vector<u8> {
        BULLETPROOFS_DST
    }

    #[view]
    /// Returns the maximum number of bits of the normalized chunk for the range proofs.
    public fun get_bulletproofs_num_bits(): u64 {
        BULLETPROOFS_NUM_BITS
    }

    //
    // Test-only range proof proving helpers
    //

    #[test_only]
    public(friend) fun prove_range(
        amount_chunks: &vector<Scalar>, randomness: &vector<Scalar>
    ): RangeProof {
        let (proof, _) =
            bulletproofs::prove_batch_range_pedersen(
                amount_chunks,
                randomness,
                BULLETPROOFS_NUM_BITS,
                BULLETPROOFS_DST
            );
        proof
    }

}
