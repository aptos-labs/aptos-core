// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::{
    bls12381,
    ed25519::Ed25519PrivateKey,
    multi_ed25519::{MultiEd25519PublicKey, MultiEd25519Signature},
    secp256k1_ecdsa, secp256r1_ecdsa, slh_dsa_sha2_128s,
    traits::{SigningKey, Uniform},
    PrivateKey,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_metadata_ext::BlockMetadataExt,
    contract_event, event,
    state_store::{state_key::StateKey, state_value::PersistedStateValueMetadata},
    transaction::{self, block_epilogue::BlockEpiloguePayload},
    validator_txn::ValidatorTransaction,
    write_set,
};
use move_core_types::language_storage;
use rand::{rngs::StdRng, SeedableRng};
use serde::{Deserialize, Serialize};
use serde_reflection::{Registry, Result, Samples, Tracer, TracerConfig};

/// Return a relative path to start tracking changes in commits.
pub fn output_file() -> Option<&'static str> {
    Some("tests/staged/consensus.yaml")
}

/// This aims at signing canonically serializable BCS data
#[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct TestAptosCrypto(String);

/// Record sample values for crypto types used by consensus.
fn trace_crypto_values(tracer: &mut Tracer, samples: &mut Samples) -> Result<()> {
    let message = TestAptosCrypto("Hello, World".to_string());

    let mut rng: StdRng = SeedableRng::from_seed([0; 32]);

    let private_key = Ed25519PrivateKey::generate(&mut rng);
    let public_key = private_key.public_key();
    let signature = private_key.sign(&message).unwrap();

    let bls_private_key = bls12381::PrivateKey::generate(&mut rng);
    let bls_public_key = bls_private_key.public_key();
    let bls_signature = bls_private_key.sign(&message).unwrap();

    tracer.trace_value(samples, &public_key)?;
    tracer.trace_value(samples, &signature)?;
    tracer.trace_value(samples, &bls_public_key)?;
    tracer.trace_value(samples, &bls_signature)?;
    tracer.trace_value::<MultiEd25519PublicKey>(samples, &public_key.clone().into())?;
    tracer.trace_value::<MultiEd25519Signature>(samples, &signature.clone().into())?;

    let secp256k1_private_key = secp256k1_ecdsa::PrivateKey::generate(&mut rng);
    let secp256k1_public_key = aptos_crypto::PrivateKey::public_key(&secp256k1_private_key);
    let secp256k1_signature = secp256k1_private_key.sign(&message).unwrap();
    tracer.trace_value(samples, &secp256k1_private_key)?;
    tracer.trace_value(samples, &secp256k1_public_key)?;
    tracer.trace_value(samples, &secp256k1_signature)?;

    let secp256r1_ecdsa_private_key = secp256r1_ecdsa::PrivateKey::generate(&mut rng);
    let secp256r1_ecdsa_public_key = PrivateKey::public_key(&secp256r1_ecdsa_private_key);
    let secp256r1_ecdsa_signature = secp256r1_ecdsa_private_key.sign(&message).unwrap();
    tracer.trace_value(samples, &secp256r1_ecdsa_private_key)?;
    tracer.trace_value(samples, &secp256r1_ecdsa_public_key)?;
    tracer.trace_value(samples, &secp256r1_ecdsa_signature)?;

    let slh_dsa_sha2_128s_private_key = slh_dsa_sha2_128s::PrivateKey::generate(&mut rng);
    let slh_dsa_sha2_128s_public_key =
        slh_dsa_sha2_128s::PublicKey::from(&slh_dsa_sha2_128s_private_key);
    let slh_dsa_sha2_128s_signature = slh_dsa_sha2_128s_private_key.sign(&message).unwrap();
    tracer.trace_value(samples, &slh_dsa_sha2_128s_private_key)?;
    tracer.trace_value(samples, &slh_dsa_sha2_128s_public_key)?;
    tracer.trace_value(samples, &slh_dsa_sha2_128s_signature)?;

    crate::trace_keyless_structs(tracer, samples, public_key, signature)?;
    crate::trace_encrypted_txn_structs(tracer, samples)?;

    Ok(())
}

/// Create a registry for consensus types.
pub fn get_registry() -> Result<Registry> {
    let mut tracer = Tracer::new(
        TracerConfig::default()
            .record_samples_for_structs(true)
            .is_human_readable(bcs::is_human_readable()),
    );
    let mut samples = Samples::new();

    // 1. Record samples for types with custom deserializers.
    trace_crypto_values(&mut tracer, &mut samples)?;
    tracer.trace_value(&mut samples, &event::EventKey::random())?;
    tracer.trace_value(&mut samples, &write_set::WriteOp::legacy_deletion())?;

    // 2. Trace the main entry point(s) + every enum separately.
    tracer.trace_type::<AggregateSignature>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::block::Block>(&samples)?;
    tracer.trace_type::<contract_event::ContractEvent>(&samples)?;
    tracer.trace_type::<language_storage::FunctionParamOrReturnTag>(&samples)?;
    tracer.trace_type::<language_storage::TypeTag>(&samples)?;
    tracer.trace_type::<ValidatorTransaction>(&samples)?;
    tracer.trace_type::<BlockMetadataExt>(&samples)?;
    tracer.trace_type::<BlockEpiloguePayload>(&samples)?;
    tracer.trace_type::<transaction::Transaction>(&samples)?;
    tracer.trace_type::<transaction::TransactionArgument>(&samples)?;
    tracer.trace_type::<transaction::TransactionPayload>(&samples)?;
    tracer.trace_type::<transaction::TransactionPayloadInner>(&samples)?;
    tracer.trace_type::<transaction::encrypted_payload::EncryptedPayload>(&samples)?;
    tracer.trace_type::<transaction::TransactionExecutable>(&samples)?;
    tracer.trace_type::<transaction::TransactionExtraConfig>(&samples)?;
    tracer.trace_type::<transaction::WriteSetPayload>(&samples)?;
    tracer.trace_type::<transaction::BlockEpiloguePayload>(&samples)?;
    tracer.trace_type::<transaction::authenticator::AccountAuthenticator>(&samples)?;
    tracer.trace_type::<transaction::authenticator::AbstractAuthenticationData>(&samples)?;
    tracer.trace_type::<transaction::authenticator::AbstractAuthenticator>(&samples)?;
    tracer.trace_type::<transaction::authenticator::TransactionAuthenticator>(&samples)?;
    tracer.trace_type::<transaction::authenticator::AnyPublicKey>(&samples)?;
    tracer.trace_type::<transaction::authenticator::AnySignature>(&samples)?;
    tracer.trace_type::<transaction::webauthn::AssertionSignature>(&samples)?;
    tracer.trace_type::<aptos_types::keyless::EphemeralCertificate>(&samples)?;
    tracer.trace_type::<write_set::WriteOp>(&samples)?;
    tracer.trace_type::<PersistedStateValueMetadata>(&samples)?;

    tracer.trace_type::<StateKey>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proof_of_store::BatchInfoExt>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::round_timeout::RoundTimeoutReason>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proof_of_store::BatchKind>(&samples)?;
    tracer.trace_type::<aptos_consensus::quorum_store::types::BatchResponse>(&samples)?;
    tracer.trace_type::<aptos_consensus::network_interface::ConsensusMsg>(&samples)?;
    tracer.trace_type::<aptos_consensus::network_interface::CommitMessage>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::block_data::BlockType>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::block_retrieval::BlockRetrievalStatus>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::payload::PayloadExecutionLimit>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::payload::OptQuorumStorePayload>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::common::Payload>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::block_retrieval::BlockRetrievalRequest>(&samples)?;

    // Proxy consensus types
    tracer.trace_type::<aptos_consensus_types::proxy_sync_info::ProxySyncInfo>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_messages::ProxyProposalMsg>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_messages::OptProxyProposalMsg>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_messages::ProxyVoteMsg>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_messages::ProxyOrderVoteMsg>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_messages::OrderedProxyBlocksMsg>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_block_data::OptProxyBlockData>(&samples)?;
    tracer.trace_type::<aptos_consensus_types::proxy_block_data::OptProxyBlockBody>(&samples)?;

    // aliases within StructTag
    tracer.ignore_aliases("StructTag", &["type_params"])?;

    tracer.registry()
}
