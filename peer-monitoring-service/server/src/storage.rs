// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::Error;
use velor_storage_interface::DbReader;
use velor_types::ledger_info::LedgerInfo;
use std::sync::Arc;

/// The interface into local storage (e.g., the Velor DB) used by the peer
/// monitoring server to handle client requests and responses.
pub trait StorageReaderInterface: Clone + Send + 'static {
    /// Returns the highest synced epoch and version
    fn get_highest_synced_epoch_and_version(&self) -> Result<(u64, u64), Error>;

    /// Returns the ledger timestamp of the blockchain in microseconds
    fn get_ledger_timestamp_usecs(&self) -> Result<u64, Error>;

    /// Returns the lowest available version in storage
    fn get_lowest_available_version(&self) -> Result<u64, Error>;
}

/// The underlying implementation of the StorageReaderInterface, used by the
/// peer monitoring server.
#[derive(Clone)]
pub struct StorageReader {
    storage: Arc<dyn DbReader>,
}

impl StorageReader {
    pub fn new(storage: Arc<dyn DbReader>) -> Self {
        Self { storage }
    }

    /// Returns the latest ledger info in storage
    fn get_latest_ledger_info(&self) -> Result<LedgerInfo, Error> {
        let latest_ledger_info_with_sigs = self
            .storage
            .get_latest_ledger_info()
            .map_err(|err| Error::StorageErrorEncountered(err.to_string()))?;
        Ok(latest_ledger_info_with_sigs.ledger_info().clone())
    }
}

impl StorageReaderInterface for StorageReader {
    fn get_highest_synced_epoch_and_version(&self) -> Result<(u64, u64), Error> {
        let latest_ledger_info = self.get_latest_ledger_info()?;
        Ok((latest_ledger_info.epoch(), latest_ledger_info.version()))
    }

    fn get_ledger_timestamp_usecs(&self) -> Result<u64, Error> {
        let latest_ledger_info = self.get_latest_ledger_info()?;
        Ok(latest_ledger_info.timestamp_usecs())
    }

    fn get_lowest_available_version(&self) -> Result<u64, Error> {
        let maybe_lowest_available_version = self
            .storage
            .get_first_txn_version()
            .map_err(|error| Error::StorageErrorEncountered(error.to_string()))?;
        maybe_lowest_available_version.ok_or_else(|| {
            Error::StorageErrorEncountered("get_first_txn_version() returned None!".into())
        })
    }
}
