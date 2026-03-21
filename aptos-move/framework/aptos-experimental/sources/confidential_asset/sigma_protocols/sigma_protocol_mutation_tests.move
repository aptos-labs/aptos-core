#[test_only]
/// Shared test helpers for sigma protocol mutation/soundness tests.
///
/// All helpers mutate in place and return the saved value for O(1) restore,
/// avoiding the O(n) copy-rebuild pattern.
module aptos_experimental::sigma_protocol_mutation_tests {
    use aptos_std::ristretto255::{Scalar, CompressedRistretto, random_point, random_scalar};
    use aptos_experimental::sigma_protocol_proof::Proof;
    use aptos_experimental::sigma_protocol_witness::Witness;
    use aptos_experimental::sigma_protocol_statement::Statement;

    // === Statement tamper helpers (O(1) per call) ===

    /// Replace the point at `idx` with a random point. Returns the saved compressed point for restoration.
    public fun tamper_statement_point<T>(stmt: &mut Statement<T>, idx: u64): CompressedRistretto {
        let prev = stmt.get_compressed_points_mut()[idx];
        let r = random_point();
        stmt.get_compressed_points_mut()[idx] = r.point_compress();
        stmt.get_points_mut()[idx] = r;
        prev
    }

    /// Restore a previously-tampered point.
    public fun restore_statement_point<T>(stmt: &mut Statement<T>, idx: u64, saved: CompressedRistretto) {
        stmt.get_compressed_points_mut()[idx] = saved;
        stmt.get_points_mut()[idx] = saved.point_decompress();
    }

    /// Replace the scalar at `idx` with a random scalar. Returns the saved scalar.
    public fun tamper_statement_scalar<T>(stmt: &mut Statement<T>, idx: u64): Scalar {
        let prev = stmt.get_scalars_mut()[idx];
        stmt.get_scalars_mut()[idx] = random_scalar();
        prev
    }

    /// Restore a previously-tampered scalar.
    public fun restore_statement_scalar<T>(stmt: &mut Statement<T>, idx: u64, saved: Scalar) {
        stmt.get_scalars_mut()[idx] = saved;
    }

    /// Swap points at `idx_a` and `idx_b` in place. Call again to restore.
    public fun swap_statement_points<T>(stmt: &mut Statement<T>, idx_a: u64, idx_b: u64) {
        stmt.get_compressed_points_mut().swap(idx_a, idx_b);
        stmt.get_points_mut().swap(idx_a, idx_b);
    }

    // === Witness tamper helper ===

    /// Increment the scalar at `idx` by 1 in place. Returns the saved scalar.
    public fun tamper_witness(witn: &mut Witness, idx: u64): Scalar {
        let scalars = witn.get_scalars_mut();
        let prev = scalars[idx];
        scalars[idx] = random_scalar();
        prev
    }

    /// Restore a previously-tampered witness scalar.
    public fun restore_witness(witn: &mut Witness, idx: u64, saved: Scalar) {
        witn.get_scalars_mut()[idx] = saved;
    }

    // === Proof tamper helpers (O(1) per call) ===

    /// Replace commitment A[idx] with a random point. Returns the saved compressed commitment.
    public fun tamper_proof_commitment(proof: &mut Proof, idx: u64): CompressedRistretto {
        let prev = proof.get_compressed_commitment_mut()[idx];
        let r = random_point();
        proof.get_compressed_commitment_mut()[idx] = r.point_compress();
        proof.get_commitment_mut()[idx] = r;
        prev
    }

    /// Restore a previously-tampered commitment.
    public fun restore_proof_commitment(proof: &mut Proof, idx: u64, saved: CompressedRistretto) {
        proof.get_compressed_commitment_mut()[idx] = saved;
        proof.get_commitment_mut()[idx] = saved.point_decompress();
    }

    /// Replace response sigma[idx] with a random scalar. Returns the saved scalar.
    public fun tamper_proof_response(proof: &mut Proof, idx: u64): Scalar {
        let prev = proof.get_response_mut()[idx];
        proof.get_response_mut()[idx] = random_scalar();
        prev
    }

    /// Restore a previously-tampered response.
    public fun restore_proof_response(proof: &mut Proof, idx: u64, saved: Scalar) {
        proof.get_response_mut()[idx] = saved;
    }

    /// Append an extra random commitment in place.
    public fun append_to_proof_commitments(proof: &mut Proof) {
        let r = random_point();
        proof.get_compressed_commitment_mut().push_back(r.point_compress());
        proof.get_commitment_mut().push_back(r);
    }

    /// Remove the last commitment in place.
    public fun pop_from_proof_commitments(proof: &mut Proof) {
        proof.get_compressed_commitment_mut().pop_back();
        proof.get_commitment_mut().pop_back();
    }

    /// Append an extra random response in place.
    public fun append_to_proof_responses(proof: &mut Proof) {
        proof.get_response_mut().push_back(random_scalar());
    }

    /// Remove the last response in place.
    public fun pop_from_proof_responses(proof: &mut Proof) {
        proof.get_response_mut().pop_back();
    }
}
