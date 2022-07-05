// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

// FIXME: (gnazario) storage helper doesn't belong in the genesis tool, but it's attached to it right now

use crate::command::Command;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey, Uniform,
};
use aptos_global_constants::{
    APTOS_ROOT_KEY, CONSENSUS_KEY, FULLNODE_NETWORK_KEY, OPERATOR_ACCOUNT, OPERATOR_KEY,
    OWNER_ACCOUNT, OWNER_KEY, SAFETY_DATA, VALIDATOR_NETWORK_KEY, WAYPOINT,
};
use aptos_management::{error::Error, secure_backend::DISK};
use aptos_secure_storage::{CryptoStorage, KVStorage, Namespaced, OnDiskStorage, Storage};
use aptos_types::{account_address, waypoint::Waypoint};
use consensus_types::safety_data::SafetyData;
use std::fs::File;
use structopt::StructOpt;

pub struct StorageHelper {
    temppath: aptos_temppath::TempPath,
}

impl StorageHelper {
    pub fn new() -> Self {
        let temppath = aptos_temppath::TempPath::new();
        temppath.create_as_file().unwrap();
        File::create(temppath.path()).unwrap();
        Self { temppath }
    }

    pub fn storage(&self, namespace: String) -> Storage {
        let storage = OnDiskStorage::new(self.temppath.path().to_path_buf());
        Storage::from(Namespaced::new(namespace, Box::new(Storage::from(storage))))
    }

    pub fn path_string(&self) -> &str {
        self.temppath.path().to_str().unwrap()
    }

    pub fn initialize_by_idx(&self, namespace: String, idx: usize) {
        let partial_seed = bcs::to_bytes(&idx).unwrap();
        let mut seed = [0u8; 32];
        let data_to_copy = 32 - std::cmp::min(32, partial_seed.len());
        seed[data_to_copy..].copy_from_slice(partial_seed.as_slice());
        self.initialize(namespace, seed);
    }

    pub fn initialize(&self, namespace: String, seed: [u8; 32]) {
        let mut rng: rand::rngs::StdRng = rand::SeedableRng::from_seed(seed);
        let mut storage = self.storage(namespace);

        // Initialize all keys in storage
        storage
            .import_private_key(APTOS_ROOT_KEY, Ed25519PrivateKey::generate(&mut rng))
            .unwrap();
        storage
            .set(CONSENSUS_KEY, Ed25519PrivateKey::generate(&mut rng))
            .unwrap();
        storage
            .import_private_key(FULLNODE_NETWORK_KEY, Ed25519PrivateKey::generate(&mut rng))
            .unwrap();
        let owner_key = Ed25519PrivateKey::generate(&mut rng);
        storage
            .set(
                OWNER_ACCOUNT,
                account_address::from_public_key(&owner_key.public_key()),
            )
            .unwrap();
        storage.import_private_key(OWNER_KEY, owner_key).unwrap();
        let operator_key = Ed25519PrivateKey::generate(&mut rng);
        storage
            .set(
                OPERATOR_ACCOUNT,
                account_address::from_public_key(&operator_key.public_key()),
            )
            .unwrap();
        storage
            .import_private_key(OPERATOR_KEY, operator_key)
            .unwrap();
        storage
            .import_private_key(VALIDATOR_NETWORK_KEY, Ed25519PrivateKey::generate(&mut rng))
            .unwrap();

        // Initialize all other data in storage
        storage
            .set(SAFETY_DATA, SafetyData::new(0, 0, 0, 0, None))
            .unwrap();
        storage.set(WAYPOINT, Waypoint::default()).unwrap();
    }

    pub fn operator_key(
        &self,
        validator_ns: &str,
        shared_ns: &str,
    ) -> Result<Ed25519PublicKey, Error> {
        let args = format!(
            "
                aptos-genesis-tool
                operator-key
                --validator-backend backend={backend};\
                    path={path};\
                    namespace={validator_ns}
                --shared-backend backend={backend};\
                    path={path};\
                    namespace={shared_ns}
            ",
            backend = DISK,
            path = self.path_string(),
            validator_ns = validator_ns,
            shared_ns = shared_ns,
        );

        let command = Command::from_iter(args.split_whitespace());
        command.operator_key()
    }

    pub fn owner_key(
        &self,
        validator_ns: &str,
        shared_ns: &str,
    ) -> Result<Ed25519PublicKey, Error> {
        let args = format!(
            "
                aptos-genesis-tool
                owner-key
                --validator-backend backend={backend};\
                    path={path};\
                    namespace={validator_ns}
                --shared-backend backend={backend};\
                    path={path};\
                    namespace={shared_ns}
            ",
            backend = DISK,
            path = self.path_string(),
            validator_ns = validator_ns,
            shared_ns = shared_ns,
        );

        let command = Command::from_iter(args.split_whitespace());
        command.owner_key()
    }

    #[cfg(test)]
    pub fn set_layout(&self, path: &str) -> Result<crate::layout::Layout, Error> {
        let args = format!(
            "
                aptos-genesis-tool
                set-layout
                --path {path}
                --shared-backend backend={backend};\
                    path={storage_path}
            ",
            path = path,
            backend = DISK,
            storage_path = self.path_string(),
        );

        let command = Command::from_iter(args.split_whitespace());
        command.set_layout()
    }

    pub fn set_operator(&self, operator_name: &str, shared_ns: &str) -> Result<String, Error> {
        let args = format!(
            "
                aptos-genesis-tool
                set-operator
                --operator-name {operator_name}
                --shared-backend backend={backend};\
                    path={path};\
                    namespace={shared_ns}
            ",
            operator_name = operator_name,
            backend = DISK,
            path = self.path_string(),
            shared_ns = shared_ns,
        );

        let command = Command::from_iter(args.split_whitespace());
        command.set_operator()
    }

    #[cfg(test)]
    pub fn verify(&self, namespace: &str) -> Result<String, Error> {
        let args = format!(
            "
                aptos-genesis-tool
                verify
                --validator-backend backend={backend};\
                    path={path};\
                    namespace={ns}
            ",
            backend = DISK,
            path = self.path_string(),
            ns = namespace,
        );

        let command = Command::from_iter(args.split_whitespace());
        command.verify()
    }
}
