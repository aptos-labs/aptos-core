/// The `confidential_range_proofs` module provides range proof verification helpers used by the Confidential Asset protocol.
/// Proof enums and their verify/prove functions live in `confidential_asset` (since Move disallows friend
/// modules from constructing/destructuring enum variants).
module aptos_framework::confidential_range_proofs {
    use std::error;
    use std::features;
    use aptos_std::ristretto255::{Self, RistrettoPoint};
    use aptos_std::ristretto255_bulletproofs::{Self as bulletproofs, RangeProof};
    use aptos_framework::confidential_balance;

    friend aptos_framework::confidential_asset;

    //
    // Errors
    //

    const ERANGE_PROOF_VERIFICATION_FAILED: u64 = 2;

    /// DST exceeds 256 bytes.
    const E_DST_TOO_LONG: u64 = 3;

    /// The native functions have not been rolled out yet.
    const E_NATIVE_FUN_NOT_AVAILABLE: u64 = 4;

    //
    // Constants
    //

    const BULLETPROOFS_DST: vector<u8> = b"AptosConfidentialAsset/BulletproofRangeProof";

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
            verify_batch_range_proof(
                &commitments,
                &ristretto255::basepoint(),
                &ristretto255::hash_to_point_base(),
                zkrp,
                confidential_balance::get_chunk_size_bits(),
                BULLETPROOFS_DST
            ),
            error::out_of_range(ERANGE_PROOF_VERIFICATION_FAILED)
        );
    }

    /// Verifies a batch range proof for commitments, ensuring all committed values are in [0, 2^num_bits).
    fun verify_batch_range_proof(
        comms: &vector<RistrettoPoint>,
        val_base: &RistrettoPoint, rand_base: &RistrettoPoint,
        proof: &RangeProof, num_bits: u64, dst: vector<u8>): bool
    {
        assert!(features::bulletproofs_batch_enabled(), error::invalid_state(E_NATIVE_FUN_NOT_AVAILABLE));
        assert!(dst.length() <= 256, error::invalid_argument(E_DST_TOO_LONG));

        let comms = comms.map_ref(|com| ristretto255::point_to_bytes(&ristretto255::point_compress(com)));

        verify_batch_range_proof_internal(
            comms,
            val_base, rand_base,
            bulletproofs::range_proof_to_bytes(proof), num_bits, dst
        )
    }

    //
    // Public view functions
    //

    #[view]
    /// Returns the DST for the range proofs.
    public fun get_bulletproofs_dst(): vector<u8> {
        BULLETPROOFS_DST
    }

    //
    // Native functions
    //

    native fun verify_batch_range_proof_internal(
        comms: vector<vector<u8>>,
        val_base: &RistrettoPoint,
        rand_base: &RistrettoPoint,
        proof: vector<u8>,
        num_bits: u64,
        dst: vector<u8>): bool;

}
