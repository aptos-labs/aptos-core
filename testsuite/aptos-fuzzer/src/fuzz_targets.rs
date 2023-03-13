// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{FuzzTarget, FuzzTargetImpl};
use anyhow::{format_err, Result};
use once_cell::sync::Lazy;
use std::{collections::BTreeMap, env};

// List fuzz target modules here.
mod consensus;
mod executor;
mod mempool;
mod move_vm;
mod network;
mod proof;
mod safety_rules;
mod secure_storage_vault;
mod storage;
mod transaction;
mod vm;

// TODO(joshlind): add a fuzzer for state sync v2!

static ALL_TARGETS: Lazy<BTreeMap<&'static str, Box<dyn FuzzTargetImpl>>> = Lazy::new(|| {
    // List fuzz targets here in this format:
    let targets: Vec<Box<dyn FuzzTargetImpl>> = vec![
        // Consensus
        Box::<consensus::ConsensusProposal>::default(),
        // Executor
        Box::<executor::ExecuteAndCommitBlocks>::default(),
        // Mempool
        Box::<mempool::MempoolIncomingTransactions>::default(),
        // Move VM
        Box::<move_vm::ValueTarget>::default(),
        // Proof
        Box::<proof::TestAccumulatorProofFuzzer>::default(),
        Box::<proof::SparseMerkleProofFuzzer>::default(),
        Box::<proof::TestAccumulatorRangeProofFuzzer>::default(),
        Box::<proof::TransactionInfoWithProofFuzzer>::default(),
        Box::<proof::TransactionInfoListWithProofFuzzer>::default(),
        // Network
        Box::<network::NetworkNoiseInitiator>::default(),
        Box::<network::NetworkNoiseResponder>::default(),
        Box::<network::NetworkNoiseStream>::default(),
        Box::<network::NetworkHandshakeExchange>::default(),
        Box::<network::NetworkHandshakeNegotiation>::default(),
        Box::<network::PeerNetworkMessagesReceive>::default(),
        // Safety Rules Server (LSR)
        Box::<safety_rules::SafetyRulesConstructAndSignVote>::default(),
        Box::<safety_rules::SafetyRulesInitialize>::default(),
        Box::<safety_rules::SafetyRulesHandleMessage>::default(),
        Box::<safety_rules::SafetyRulesSignProposal>::default(),
        Box::<safety_rules::SafetyRulesSignTimeout>::default(),
        // Secure Storage Vault
        Box::<secure_storage_vault::VaultGenericResponse>::default(),
        Box::<secure_storage_vault::VaultPolicyReadResponse>::default(),
        Box::<secure_storage_vault::VaultPolicyListResponse>::default(),
        Box::<secure_storage_vault::VaultSecretListResponse>::default(),
        Box::<secure_storage_vault::VaultSecretReadResponse>::default(),
        Box::<secure_storage_vault::VaultTokenCreateResponse>::default(),
        Box::<secure_storage_vault::VaultTokenRenewResponse>::default(),
        Box::<secure_storage_vault::VaultTransitCreateResponse>::default(),
        Box::<secure_storage_vault::VaultTransitExportResponse>::default(),
        Box::<secure_storage_vault::VaultTransitListResponse>::default(),
        Box::<secure_storage_vault::VaultTransitReadResponse>::default(),
        Box::<secure_storage_vault::VaultTransitRestoreResponse>::default(),
        Box::<secure_storage_vault::VaultTransitSignResponse>::default(),
        Box::<secure_storage_vault::VaultUnsealedResponse>::default(),
        // Storage
        // Box::new(storage::StorageSaveBlocks::default()),
        Box::<storage::StorageSchemaDecode>::default(),
        //Box::new(storage::JellyfishGetWithProof::default()),
        Box::<storage::JellyfishGetWithProofWithDistinctLastNibble>::default(),
        Box::<storage::JellyfishGetRangeProof>::default(),
        Box::<storage::JellyfishGetLeafCount>::default(),
        Box::<storage::AccumulatorFrozenSubtreeHashes>::default(),
        Box::<storage::AccumulatorProof>::default(),
        Box::<storage::AccumulatorConsistencyProof>::default(),
        Box::<storage::AccumulatorRangeProof>::default(),
        Box::<storage::AccumulatorAppendMany>::default(),
        Box::<storage::AccumulatorAppendEmpty>::default(),
        Box::<storage::SparseMerkleCorrectness>::default(),
        // Transaction
        Box::<transaction::LanguageTransactionExecution>::default(),
        Box::<transaction::SignedTransactionTarget>::default(),
        Box::<transaction::MutatedSignedTransaction>::default(),
        Box::<transaction::TwoSignedTransactions>::default(),
        // VM
        Box::<vm::CompiledModuleTarget>::default(),
    ];
    targets
        .into_iter()
        .map(|target| (target.name(), target))
        .collect()
});

impl FuzzTarget {
    /// The environment variable used for passing fuzz targets to child processes.
    pub(crate) const ENV_VAR: &'static str = "FUZZ_TARGET";

    /// Get the current fuzz target from the environment.
    pub fn from_env() -> Result<Self> {
        let name = env::var(Self::ENV_VAR)?;
        Self::by_name(&name).ok_or_else(|| format_err!("Unknown fuzz target '{}'", name))
    }

    /// Get a fuzz target by name.
    pub fn by_name(name: &str) -> Option<Self> {
        ALL_TARGETS.get(name).map(|target| FuzzTarget(&**target))
    }

    /// A list of all fuzz targets.
    pub fn all_targets() -> impl Iterator<Item = Self> {
        ALL_TARGETS.values().map(|target| FuzzTarget(&**target))
    }
}
