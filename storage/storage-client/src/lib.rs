// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_secure_net::NetworkClient;
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    proof::SparseMerkleProof,
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::{TransactionToCommit, Version},
};
use serde::de::DeserializeOwned;
use std::net::SocketAddr;
use storage_interface::{
    DbReader, DbWriter, Error, GetStateValueWithProofByVersionRequest, SaveTransactionsRequest,
    StartupInfo, StorageRequest,
};

pub struct StorageClient {
    network_client: Mutex<NetworkClient>,
}

impl StorageClient {
    pub fn new(server_address: &SocketAddr, timeout: u64) -> Self {
        Self {
            network_client: Mutex::new(NetworkClient::new("storage", *server_address, timeout)),
        }
    }

    fn process_one_message(&self, input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut client = self.network_client.lock();
        client.write(input)?;
        client.read().map_err(|e| e.into())
    }

    fn request<T: DeserializeOwned>(&self, input: StorageRequest) -> std::result::Result<T, Error> {
        let input_message = bcs::to_bytes(&input)?;
        let result = loop {
            match self.process_one_message(&input_message) {
                Err(err) => warn!(
                    error = ?err,
                    request = ?input,
                    "Failed to communicate with storage service.",
                ),
                Ok(value) => break value,
            }
        };
        bcs::from_bytes(&result)?
    }

    pub fn get_state_value_with_proof_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> std::result::Result<(Option<StateValue>, SparseMerkleProof<StateValue>), Error> {
        self.request(StorageRequest::GetStateValueWithProofByVersionRequest(
            Box::new(GetStateValueWithProofByVersionRequest::new(
                state_key.clone(),
                version,
            )),
        ))
    }

    pub fn get_startup_info(&self) -> std::result::Result<Option<StartupInfo>, Error> {
        self.request(StorageRequest::GetStartupInfoRequest)
    }

    pub fn save_transactions(
        &self,
        txns_to_commit: Vec<TransactionToCommit>,
        first_version: Version,
        ledger_info_with_sigs: Option<LedgerInfoWithSignatures>,
    ) -> std::result::Result<(), Error> {
        self.request(StorageRequest::SaveTransactionsRequest(Box::new(
            SaveTransactionsRequest::new(txns_to_commit, first_version, ledger_info_with_sigs),
        )))
    }
}

impl DbReader for StorageClient {
    fn get_state_value_with_proof_by_version(
        &self,
        state_key: &StateKey,
        version: u64,
    ) -> Result<(Option<StateValue>, SparseMerkleProof<StateValue>)> {
        Ok(Self::get_state_value_with_proof_by_version(
            self, state_key, version,
        )?)
    }

    fn get_startup_info(&self) -> Result<Option<StartupInfo>> {
        Ok(Self::get_startup_info(self)?)
    }
}

impl DbWriter for StorageClient {
    fn save_transactions(
        &self,
        txns_to_commit: &[TransactionToCommit],
        first_version: Version,
        ledger_info_with_sigs: Option<&LedgerInfoWithSignatures>,
    ) -> Result<()> {
        Ok(Self::save_transactions(
            self,
            txns_to_commit.to_vec(),
            first_version,
            ledger_info_with_sigs.cloned(),
        )?)
    }
}
