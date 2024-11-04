// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters,
    logging::{self, LogEntry, LogEvent},
    Error,
};
use aptos_consensus_types::{common::Author, safety_data::SafetyData};
use aptos_crypto::{bls12381, PrivateKey};
use aptos_global_constants::{CONSENSUS_KEY, OWNER_ACCOUNT, SAFETY_DATA, WAYPOINT};
use aptos_logger::prelude::*;
use aptos_secure_storage::{KVStorage, Storage};
use aptos_types::waypoint::Waypoint;

/// SafetyRules needs an abstract storage interface to act as a common utility for storing
/// persistent data to local disk, cloud, secrets managers, or even memory (for tests)
/// Any set function is expected to sync to the remote system before returning.
///
/// Note: cached_safety_data is a local in-memory copy of SafetyData. As SafetyData should
/// only ever be used by safety rules, we maintain an in-memory copy to avoid issuing reads
/// to the internal storage if the SafetyData hasn't changed. On writes, we update the
/// cache and internal storage.
pub struct PersistentSafetyStorage {
    enable_cached_safety_data: bool,
    cached_safety_data: Option<SafetyData>,
    internal_store: Storage,
}

impl PersistentSafetyStorage {
    /// Use this to instantiate a PersistentStorage for a new data store, one that has no
    /// SafetyRules values set.
    pub fn initialize(
        mut internal_store: Storage,
        author: Author,
        consensus_private_key: bls12381::PrivateKey,
        waypoint: Waypoint,
        enable_cached_safety_data: bool,
    ) -> Self {
        // Initialize the keys and accounts
        Self::initialize_keys_and_accounts(&mut internal_store, author, consensus_private_key)
            .expect("Unable to initialize keys and accounts in storage");

        // Create the new persistent safety storage
        let safety_data = SafetyData::new(1, 0, 0, 0, None, 0);
        let mut persisent_safety_storage = Self {
            enable_cached_safety_data,
            cached_safety_data: Some(safety_data.clone()),
            internal_store,
        };

        // Initialize the safety data and waypoint
        persisent_safety_storage
            .set_safety_data(safety_data)
            .expect("Unable to initialize safety data");
        persisent_safety_storage
            .set_waypoint(&waypoint)
            .expect("Unable to initialize waypoint");

        persisent_safety_storage
    }

    fn initialize_keys_and_accounts(
        internal_store: &mut Storage,
        author: Author,
        consensus_private_key: bls12381::PrivateKey,
    ) -> Result<(), Error> {
        let result = internal_store.set(CONSENSUS_KEY, consensus_private_key);
        // Attempting to re-initialize existing storage. This can happen in environments like
        // forge. Rather than be rigid here, leave it up to the developer to detect
        // inconsistencies or why they did not reset storage between rounds. Do not repeat the
        // checks again below, because it is just too strange to have a partially configured
        // storage.
        if let Err(aptos_secure_storage::Error::KeyAlreadyExists(_)) = result {
            warn!("Attempted to re-initialize existing storage");
            return Ok(());
        }

        internal_store.set(OWNER_ACCOUNT, author)?;
        Ok(())
    }

    /// Use this to instantiate a PersistentStorage with an existing data store. This is intended
    /// for constructed environments.
    pub fn new(internal_store: Storage, enable_cached_safety_data: bool) -> Self {
        Self {
            enable_cached_safety_data,
            cached_safety_data: None,
            internal_store,
        }
    }

    pub fn author(&self) -> Result<Author, Error> {
        let _timer = counters::start_timer("get", OWNER_ACCOUNT);
        Ok(self.internal_store.get(OWNER_ACCOUNT).map(|v| v.value)?)
    }

    pub fn consensus_sk_by_pk(
        &self,
        pk: bls12381::PublicKey,
    ) -> Result<bls12381::PrivateKey, Error> {
        let _timer = counters::start_timer("get", CONSENSUS_KEY);
        let pk_hex = hex::encode(pk.to_bytes());
        let explicit_storage_key = format!("{}_{}", CONSENSUS_KEY, pk_hex);
        let explicit_sk = self
            .internal_store
            .get::<bls12381::PrivateKey>(explicit_storage_key.as_str())
            .map(|v| v.value);
        let default_sk = self
            .internal_store
            .get::<bls12381::PrivateKey>(CONSENSUS_KEY)
            .map(|v| v.value);
        let key = match (explicit_sk, default_sk) {
            (Ok(sk_0), _) => sk_0,
            (Err(_), Ok(sk_1)) => sk_1,
            (Err(_), Err(_)) => {
                return Err(Error::ValidatorKeyNotFound("not found!".to_string()));
            },
        };
        if key.public_key() != pk {
            return Err(Error::SecureStorageMissingDataError(format!(
                "Incorrect sk saved for {:?} the expected pk",
                pk
            )));
        }
        Ok(key)
    }

    pub fn safety_data(&mut self) -> Result<SafetyData, Error> {
        if !self.enable_cached_safety_data {
            let _timer = counters::start_timer("get", SAFETY_DATA);
            return self.internal_store.get(SAFETY_DATA).map(|v| v.value)?;
        }

        if let Some(cached_safety_data) = self.cached_safety_data.clone() {
            Ok(cached_safety_data)
        } else {
            let _timer = counters::start_timer("get", SAFETY_DATA);
            let safety_data: SafetyData = self.internal_store.get(SAFETY_DATA).map(|v| v.value)?;
            self.cached_safety_data = Some(safety_data.clone());
            Ok(safety_data)
        }
    }

    pub fn set_safety_data(&mut self, data: SafetyData) -> Result<(), Error> {
        let _timer = counters::start_timer("set", SAFETY_DATA);
        counters::set_state(counters::EPOCH, data.epoch as i64);
        counters::set_state(counters::LAST_VOTED_ROUND, data.last_voted_round as i64);
        counters::set_state(
            counters::HIGHEST_TIMEOUT_ROUND,
            data.highest_timeout_round as i64,
        );
        counters::set_state(counters::PREFERRED_ROUND, data.preferred_round as i64);

        match self.internal_store.set(SAFETY_DATA, data.clone()) {
            Ok(_) => {
                self.cached_safety_data = Some(data);
                Ok(())
            },
            Err(error) => {
                self.cached_safety_data = None;
                Err(Error::SecureStorageUnexpectedError(error.to_string()))
            },
        }
    }

    pub fn waypoint(&self) -> Result<Waypoint, Error> {
        let _timer = counters::start_timer("get", WAYPOINT);
        Ok(self.internal_store.get(WAYPOINT).map(|v| v.value)?)
    }

    pub fn set_waypoint(&mut self, waypoint: &Waypoint) -> Result<(), Error> {
        let _timer = counters::start_timer("set", WAYPOINT);
        counters::set_state(counters::WAYPOINT_VERSION, waypoint.version() as i64);
        self.internal_store.set(WAYPOINT, waypoint)?;
        info!(
            logging::SafetyLogSchema::new(LogEntry::Waypoint, LogEvent::Update).waypoint(*waypoint)
        );
        Ok(())
    }

    pub fn internal_store(&mut self) -> &mut Storage {
        &mut self.internal_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counters;
    use aptos_crypto::hash::HashValue;
    use aptos_secure_storage::InMemoryStorage;
    use aptos_types::{
        block_info::BlockInfo, epoch_state::EpochState, ledger_info::LedgerInfo,
        transaction::Version, validator_signer::ValidatorSigner, waypoint::Waypoint,
    };
    use rusty_fork::rusty_fork_test;

    // Metrics are globally instantiated. We use rusty_fork to prevent concurrent tests
    // from interfering with the metrics while we run this test.
    rusty_fork_test! {
        #[test]
        fn test_counters() {
            let consensus_private_key = ValidatorSigner::from_int(0).private_key().clone();
            let storage = Storage::from(InMemoryStorage::new());
            let mut safety_storage = PersistentSafetyStorage::initialize(
                storage,
                Author::random(),
                consensus_private_key,
                Waypoint::default(),
                true,
            );
            // they both touch the global counters, running it serially to prevent race condition.
            test_safety_data_counters(&mut safety_storage);
            test_waypoint_counters(&mut safety_storage);
        }
    }

    fn test_safety_data_counters(safety_storage: &mut PersistentSafetyStorage) {
        let safety_data = safety_storage.safety_data().unwrap();
        assert_eq!(safety_data.epoch, 1);
        assert_eq!(safety_data.last_voted_round, 0);
        assert_eq!(safety_data.preferred_round, 0);
        assert_eq!(counters::get_state(counters::EPOCH), 1);
        assert_eq!(counters::get_state(counters::LAST_VOTED_ROUND), 0);
        assert_eq!(counters::get_state(counters::PREFERRED_ROUND), 0);

        safety_storage
            .set_safety_data(SafetyData::new(9, 8, 1, 0, None, 0))
            .unwrap();

        let safety_data = safety_storage.safety_data().unwrap();
        assert_eq!(safety_data.epoch, 9);
        assert_eq!(safety_data.last_voted_round, 8);
        assert_eq!(safety_data.preferred_round, 1);
        assert_eq!(counters::get_state(counters::EPOCH), 9);
        assert_eq!(counters::get_state(counters::LAST_VOTED_ROUND), 8);
        assert_eq!(counters::get_state(counters::PREFERRED_ROUND), 1);
    }

    fn test_waypoint_counters(safety_storage: &mut PersistentSafetyStorage) {
        let waypoint = safety_storage.waypoint().unwrap();
        assert_eq!(waypoint.version(), Version::default());
        assert_eq!(
            counters::get_state(counters::WAYPOINT_VERSION) as u64,
            Version::default()
        );

        for expected_version in 1..=10u64 {
            let li = LedgerInfo::new(
                BlockInfo::new(
                    1,
                    10,
                    HashValue::random(),
                    HashValue::random(),
                    expected_version,
                    1000,
                    Some(EpochState::empty()),
                ),
                HashValue::zero(),
            );
            let waypoint = &Waypoint::new_epoch_boundary(&li).unwrap();
            safety_storage.set_waypoint(waypoint).unwrap();

            let waypoint = safety_storage.waypoint().unwrap();
            assert_eq!(waypoint.version(), expected_version);
            assert_eq!(
                counters::get_state(counters::WAYPOINT_VERSION) as u64,
                expected_version
            );
        }
    }
}
