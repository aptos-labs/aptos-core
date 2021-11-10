// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Result;
use diem_infallible::Mutex;
use diem_logger::warn;
use diem_secure_net::NetworkClient;
use diem_types::{
    account_address::AccountAddress,
    account_state_blob::AccountStateBlob,
    ledger_info::LedgerInfoWithSignatures,
    proof::SparseMerkleProof,
    protocol_spec::DpnProto,
    transaction::{TransactionToCommit, Version},
};
use serde::de::DeserializeOwned;
use std::net::SocketAddr;
use storage_interface::{
    DbReader, DbWriter, Error, GetAccountStateWithProofByVersionRequest, SaveTransactionsRequest,
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

    pub fn get_account_state_with_proof_by_version(
        &self,
        address: AccountAddress,
        version: Version,
    ) -> std::result::Result<
        (
            Option<AccountStateBlob>,
            SparseMerkleProof<AccountStateBlob>,
        ),
        Error,
    > {
        self.request(StorageRequest::GetAccountStateWithProofByVersionRequest(
            Box::new(GetAccountStateWithProofByVersionRequest::new(
                address, version,
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

impl DbReader<DpnProto> for StorageClient {
    fn get_account_state_with_proof_by_version(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> Result<(
        Option<AccountStateBlob>,
        SparseMerkleProof<AccountStateBlob>,
    )> {
        Ok(Self::get_account_state_with_proof_by_version(
            self, address, version,
        )?)
    }

    fn get_startup_info(&self) -> Result<Option<StartupInfo>> {
        Ok(Self::get_startup_info(self)?)
    }
}

impl DbWriter<DpnProto> for StorageClient {
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
