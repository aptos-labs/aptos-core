// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    epoch_manager::LivenessStorageData,
    persistent_liveness_storage::{
        LedgerRecoveryData, PersistentLivenessStorage, RecoveryData, RootMetadata,
    },
};
use anyhow::Result;
use aptos_consensus_types::{
    block::Block, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate, vote::Vote,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_storage_interface::DbReader;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    epoch_change::EpochChangeProof,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ValidatorSet,
};
use std::{collections::HashMap, sync::Arc};

pub struct MockSharedStorage {
    // Safety state
    pub block: Mutex<HashMap<HashValue, Block>>,
    pub qc: Mutex<HashMap<HashValue, QuorumCert>>,
    pub lis: Mutex<HashMap<u64, LedgerInfoWithSignatures>>,
    pub last_vote: Mutex<Option<Vote>>,

    // Liveness state
    pub highest_2chain_timeout_certificate: Mutex<Option<TwoChainTimeoutCertificate>>,
    pub validator_set: ValidatorSet,
}

impl MockSharedStorage {
    pub fn new(validator_set: ValidatorSet) -> Self {
        MockSharedStorage {
            block: Mutex::new(HashMap::new()),
            qc: Mutex::new(HashMap::new()),
            lis: Mutex::new(HashMap::new()),
            last_vote: Mutex::new(None),
            highest_2chain_timeout_certificate: Mutex::new(None),
            validator_set,
        }
    }
}

/// A storage that simulates the operations in-memory, used in the tests that cares about storage
/// consistency.
pub struct MockStorage {
    pub shared_storage: Arc<MockSharedStorage>,
    storage_ledger: Mutex<LedgerInfo>,
}

impl MockStorage {
    pub fn new_with_ledger_info(
        shared_storage: Arc<MockSharedStorage>,
        ledger_info: LedgerInfo,
    ) -> Self {
        let li = if ledger_info.ends_epoch() {
            ledger_info.clone()
        } else {
            let validator_set = Some(shared_storage.validator_set.clone());
            LedgerInfo::mock_genesis(validator_set)
        };
        let lis = LedgerInfoWithSignatures::new(li, AggregateSignature::empty());
        shared_storage
            .lis
            .lock()
            .insert(lis.ledger_info().version(), lis);
        MockStorage {
            shared_storage,
            storage_ledger: Mutex::new(ledger_info),
        }
    }

    pub fn get_ledger_info(&self) -> LedgerInfo {
        self.storage_ledger.lock().clone()
    }

    pub fn commit_to_storage(&self, ledger: LedgerInfo) {
        *self.storage_ledger.lock() = ledger;

        if let Err(e) = self.verify_consistency() {
            panic!("invalid db after commit: {}", e);
        }
    }

    pub fn get_validator_set(&self) -> &ValidatorSet {
        &self.shared_storage.validator_set
    }

    pub fn get_ledger_recovery_data(&self) -> LedgerRecoveryData {
        LedgerRecoveryData::new(LedgerInfoWithSignatures::new(
            self.storage_ledger.lock().clone(),
            AggregateSignature::empty(),
        ))
    }

    pub fn try_start(&self, order_vote_enabled: bool, window_size: usize) -> Result<RecoveryData> {
        let ledger_recovery_data = self.get_ledger_recovery_data();
        let mut blocks: Vec<_> = self
            .shared_storage
            .block
            .lock()
            .clone()
            .into_values()
            .collect();
        let quorum_certs = self
            .shared_storage
            .qc
            .lock()
            .clone()
            .into_values()
            .collect();
        blocks.sort_by_key(Block::round);
        let last_vote = self.shared_storage.last_vote.lock().clone();
        let qc = self
            .shared_storage
            .highest_2chain_timeout_certificate
            .lock()
            .clone();
        RecoveryData::new(
            last_vote,
            ledger_recovery_data,
            blocks,
            RootMetadata::new_empty(),
            quorum_certs,
            qc,
            order_vote_enabled,
            window_size,
        )
    }

    pub fn verify_consistency(&self) -> Result<()> {
        // TODO: Also test by setting order_vote_enabled to true
        self.try_start(false, 1).map(|_| ())
    }

    pub fn start_for_testing(validator_set: ValidatorSet) -> (RecoveryData, Arc<Self>) {
        let shared_storage = Arc::new(MockSharedStorage::new(validator_set.clone()));
        let genesis_li = LedgerInfo::mock_genesis(Some(validator_set));
        let storage = Self::new_with_ledger_info(shared_storage, genesis_li);
        let recovery_data = match storage.start(false, 1) {
            LivenessStorageData::FullRecoveryData(recovery_data) => recovery_data,
            _ => panic!("Mock storage should never fail constructing recovery data"),
        };

        (recovery_data, Arc::new(storage))
    }
}

// A impl that always start from genesis.
impl PersistentLivenessStorage for MockStorage {
    fn save_tree(&self, blocks: Vec<Block>, quorum_certs: Vec<QuorumCert>) -> Result<()> {
        // When the shared storage is empty, we are expected to not able to construct an block tree
        // from it. During test we will intentionally clear shared_storage to simulate the situation
        // of restarting from an empty consensusDB
        // info!("step 1.3.4.2.3.1");
        let should_check_for_consistency = !(self.shared_storage.block.lock().is_empty()
            && self.shared_storage.qc.lock().is_empty());
        for block in blocks {
            self.shared_storage.block.lock().insert(block.id(), block);
        }
        // info!("step 1.3.4.2.3.2");
        for qc in quorum_certs {
            self.shared_storage
                .qc
                .lock()
                .insert(qc.certified_block().id(), qc);
        }
        // info!("step 1.3.4.2.3.3");
        if should_check_for_consistency {
            if let Err(e) = self.verify_consistency() {
                panic!("invalid db after save tree: {}", e);
            }
        }
        Ok(())
    }

    fn prune_tree(&self, block_id: Vec<HashValue>) -> Result<()> {
        for id in block_id {
            self.shared_storage.block.lock().remove(&id);
            self.shared_storage.qc.lock().remove(&id);
        }
        if let Err(e) = self.verify_consistency() {
            panic!("invalid db after prune tree: {}", e);
        }
        Ok(())
    }

    fn save_vote(&self, last_vote: &Vote) -> Result<()> {
        self.shared_storage
            .last_vote
            .lock()
            .replace(last_vote.clone());
        Ok(())
    }

    fn recover_from_ledger(&self) -> LedgerRecoveryData {
        self.get_ledger_recovery_data()
    }

    fn start(&self, order_vote_enabled: bool, window_size: usize) -> LivenessStorageData {
        match self.try_start(order_vote_enabled, window_size) {
            Ok(recovery_data) => LivenessStorageData::FullRecoveryData(recovery_data),
            Err(_) => LivenessStorageData::PartialRecoveryData(self.recover_from_ledger()),
        }
    }

    fn save_highest_2chain_timeout_cert(
        &self,
        highest_timeout_certificate: &TwoChainTimeoutCertificate,
    ) -> Result<()> {
        self.shared_storage
            .highest_2chain_timeout_certificate
            .lock()
            .replace(highest_timeout_certificate.clone());
        Ok(())
    }

    fn retrieve_epoch_change_proof(&self, version: u64) -> Result<EpochChangeProof> {
        let lis = self
            .shared_storage
            .lis
            .lock()
            .get(&version)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("LedgerInfo for version not found"))?;
        Ok(EpochChangeProof::new(vec![lis], false))
    }

    fn aptos_db(&self) -> Arc<dyn DbReader> {
        unimplemented!()
    }

    fn consensus_db(&self) -> Arc<crate::consensusdb::ConsensusDB> {
        unimplemented!()
    }
}

/// A storage that ignores any requests, used in the tests that don't care about the storage.
pub struct EmptyStorage;

impl EmptyStorage {
    pub fn new() -> Self {
        Self
    }

    pub fn start_for_testing() -> (RecoveryData, Arc<Self>) {
        let storage = Arc::new(EmptyStorage::new());
        let recovery_data = match storage.start(false, 1) {
            LivenessStorageData::FullRecoveryData(recovery_data) => recovery_data,
            _ => panic!("Mock storage should never fail constructing recovery data"),
        };
        (recovery_data, storage)
    }
}

impl PersistentLivenessStorage for EmptyStorage {
    fn save_tree(&self, _: Vec<Block>, _: Vec<QuorumCert>) -> Result<()> {
        Ok(())
    }

    fn prune_tree(&self, _: Vec<HashValue>) -> Result<()> {
        Ok(())
    }

    fn save_vote(&self, _: &Vote) -> Result<()> {
        Ok(())
    }

    fn recover_from_ledger(&self) -> LedgerRecoveryData {
        LedgerRecoveryData::new(LedgerInfoWithSignatures::new(
            LedgerInfo::mock_genesis(None),
            AggregateSignature::empty(),
        ))
    }

    fn start(&self, order_vote_enabled: bool, window_size: usize) -> LivenessStorageData {
        match RecoveryData::new(
            None,
            self.recover_from_ledger(),
            vec![],
            RootMetadata::new_empty(),
            vec![],
            None,
            order_vote_enabled,
            window_size,
        ) {
            Ok(recovery_data) => LivenessStorageData::FullRecoveryData(recovery_data),
            Err(e) => {
                eprintln!("{}", e);
                panic!("Construct recovery data during genesis should never fail");
            },
        }
    }

    fn save_highest_2chain_timeout_cert(&self, _: &TwoChainTimeoutCertificate) -> Result<()> {
        Ok(())
    }

    fn retrieve_epoch_change_proof(&self, _version: u64) -> Result<EpochChangeProof> {
        Ok(EpochChangeProof::new(vec![], false))
    }

    fn aptos_db(&self) -> Arc<dyn DbReader> {
        unimplemented!()
    }

    fn consensus_db(&self) -> Arc<crate::consensusdb::ConsensusDB> {
        unimplemented!()
    }
}
