// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{chunky::types::ChunkyTranscriptWithHash, counters, monitor};
use anyhow::{anyhow, ensure, Context};
use aptos_crypto::HashValue;
use aptos_dkg::pvss::traits::transcript::{HasAggregatableSubtranscript, Transcript};
use aptos_types::{
    dkg::chunky_dkg::{ChunkyDKGSession, ChunkyTranscript, DealerPublicKey},
    epoch_state::EpochState,
};
use move_core_types::account_address::AccountAddress;

/// Shared transcript validation pipeline used by both the aggregation producer and the
/// transcript fetcher. Deserializes, cryptographically verifies, and checks dealer ID.
///
/// `transcript.verify()` checks the PVSS proof and dealer signature against the full session
/// parameters; the dealer-ID check ensures the transcript belongs to the expected sender;
/// envelope metadata (epoch, author) is validated by the caller.
pub fn deserialize_chunky_transcript_and_verify(
    sender: AccountAddress,
    transcript_bytes: &[u8],
    dkg_config: &ChunkyDKGSession,
    signing_pubkeys: &[DealerPublicKey],
    epoch_state: &EpochState,
) -> anyhow::Result<ChunkyTranscriptWithHash> {
    // Hash the canonical BCS wire bytes once up front to avoid repeated re-serialization.
    // Safe because BCS is strictly canonical: deserialize(serialize(x)) == x byte-for-byte.
    let hash = monitor!(
        "chunky_validate_transcript_hash",
        HashValue::sha3_256_of(transcript_bytes)
    );

    // Deserialize transcript
    counters::CHUNKY_DKG_OBJECT_SIZE_BYTES
        .with_label_values(&["received_transcript"])
        .observe(transcript_bytes.len() as f64);
    let transcript: ChunkyTranscript = bcs::from_bytes(transcript_bytes)
        .map_err(|e| anyhow!("[ChunkyDKG] Unable to deserialize chunky transcript: {e}"))?;

    let mut rng = rand::thread_rng();
    // Verify the transcript cryptographically.
    monitor!(
        "chunky_validate_transcript_verify",
        transcript.verify(
            &dkg_config.threshold_config,
            &dkg_config.public_parameters,
            signing_pubkeys,
            &dkg_config.eks,
            &dkg_config.session_metadata,
            &mut rng,
        )
    )
    .context("chunky transcript verification failed")?;

    // Ensure the transcript's dealer id matches the sender's validator index.
    // Otherwise a malicious validator could replay another validator's legitimately-signed
    // transcript, causing attribution mismatch between the aggregated subtranscript content
    // and the dealers list built from contributors.
    let sender_index = epoch_state
        .verifier
        .address_to_validator_index()
        .get(&sender)
        .copied()
        .ok_or_else(|| anyhow!("[ChunkyDKG] sender not in validator set"))?;
    let dealers = transcript.get_dealers();
    ensure!(
        dealers.len() == 1,
        "[ChunkyDKG] expected single dealer, got {}",
        dealers.len(),
    );
    ensure!(
        dealers[0].id == sender_index,
        "[ChunkyDKG] transcript dealer id {} does not match sender validator index {}",
        dealers[0].id,
        sender_index,
    );

    Ok(ChunkyTranscriptWithHash::new(transcript, hash))
}
