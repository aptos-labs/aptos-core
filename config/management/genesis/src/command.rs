// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_management::{error::Error, execute_command};
use aptos_types::{transaction::Transaction, waypoint::Waypoint};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Tool used for genesis")]
pub enum Command {
    #[structopt(about = "Create a waypoint")]
    CreateWaypoint(crate::waypoint::CreateWaypoint),
    #[structopt(about = "Retrieves data from a store to produce genesis")]
    Genesis(crate::genesis::Genesis),
    #[structopt(about = "Set the waypoint in the validator storage")]
    InsertWaypoint(aptos_management::waypoint::InsertWaypoint),
    #[structopt(about = "Submits an Ed25519PublicKey for the aptos root")]
    AptosRootKey(crate::key::AptosRootKey),
    #[structopt(about = "Submits an Ed25519PublicKey for the operator")]
    OperatorKey(crate::key::OperatorKey),
    #[structopt(about = "Submits an Ed25519PublicKey for the owner")]
    OwnerKey(crate::key::OwnerKey),
    #[structopt(about = "Submits a Layout doc to a shared storage")]
    SetLayout(crate::layout::SetLayout),
    #[structopt(about = "Submits Move module bytecodes to a shared storage")]
    SetMoveModules(crate::move_modules::SetMoveModules),
    #[structopt(about = "Sets the validator operator chosen by the owner")]
    SetOperator(crate::validator_operator::ValidatorOperator),
    #[structopt(about = "Verifies and prints the current configuration state")]
    Verify(crate::verify::Verify),
}

#[derive(Debug, PartialEq)]
pub enum CommandName {
    CreateWaypoint,
    Genesis,
    InsertWaypoint,
    AptosRootKey,
    OperatorKey,
    OwnerKey,
    SetLayout,
    SetMoveModules,
    SetOperator,
    Verify,
}

impl From<&Command> for CommandName {
    fn from(command: &Command) -> Self {
        match command {
            Command::CreateWaypoint(_) => CommandName::CreateWaypoint,
            Command::Genesis(_) => CommandName::Genesis,
            Command::InsertWaypoint(_) => CommandName::InsertWaypoint,
            Command::AptosRootKey(_) => CommandName::AptosRootKey,
            Command::OperatorKey(_) => CommandName::OperatorKey,
            Command::OwnerKey(_) => CommandName::OwnerKey,
            Command::SetLayout(_) => CommandName::SetLayout,
            Command::SetMoveModules(_) => CommandName::SetMoveModules,
            Command::SetOperator(_) => CommandName::SetOperator,
            Command::Verify(_) => CommandName::Verify,
        }
    }
}

impl std::fmt::Display for CommandName {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = match self {
            CommandName::CreateWaypoint => "create-waypoint",
            CommandName::Genesis => "genesis",
            CommandName::InsertWaypoint => "insert-waypoint",
            CommandName::AptosRootKey => "aptos-root-key",
            CommandName::OperatorKey => "operator-key",
            CommandName::OwnerKey => "owner-key",
            CommandName::SetLayout => "set-layout",
            CommandName::SetMoveModules => "set-move-modules",
            CommandName::SetOperator => "set-operator",
            CommandName::Verify => "verify",
        };
        write!(f, "{}", name)
    }
}

impl Command {
    pub fn execute(self) -> Result<String, Error> {
        match &self {
            Command::CreateWaypoint(_) => {
                self.create_waypoint().map(|w| format!("Waypoint: {}", w))
            }
            Command::Genesis(_) => self.genesis().map(|_| "Success!".to_string()),
            Command::InsertWaypoint(_) => self.insert_waypoint().map(|_| "Success!".to_string()),
            Command::AptosRootKey(_) => self.aptos_root_key().map(|_| "Success!".to_string()),
            Command::OperatorKey(_) => self.operator_key().map(|_| "Success!".to_string()),
            Command::OwnerKey(_) => self.owner_key().map(|_| "Success!".to_string()),
            Command::SetLayout(_) => self.set_layout().map(|_| "Success!".to_string()),
            Command::SetMoveModules(_) => self.set_move_modules().map(|_| "Success!".to_string()),
            Command::SetOperator(_) => self.set_operator().map(|_| "Success!".to_string()),
            Command::Verify(_) => self.verify(),
        }
    }

    pub fn create_waypoint(self) -> Result<Waypoint, Error> {
        execute_command!(self, Command::CreateWaypoint, CommandName::CreateWaypoint)
    }

    pub fn genesis(self) -> Result<Transaction, Error> {
        execute_command!(self, Command::Genesis, CommandName::Genesis)
    }

    pub fn insert_waypoint(self) -> Result<(), Error> {
        execute_command!(self, Command::InsertWaypoint, CommandName::InsertWaypoint)
    }

    pub fn aptos_root_key(self) -> Result<Ed25519PublicKey, Error> {
        execute_command!(self, Command::AptosRootKey, CommandName::AptosRootKey)
    }

    pub fn operator_key(self) -> Result<Ed25519PublicKey, Error> {
        execute_command!(self, Command::OperatorKey, CommandName::OperatorKey)
    }

    pub fn owner_key(self) -> Result<Ed25519PublicKey, Error> {
        execute_command!(self, Command::OwnerKey, CommandName::OwnerKey)
    }

    pub fn set_layout(self) -> Result<crate::layout::Layout, Error> {
        execute_command!(self, Command::SetLayout, CommandName::SetLayout)
    }

    pub fn set_move_modules(self) -> Result<Vec<Vec<u8>>, Error> {
        execute_command!(self, Command::SetMoveModules, CommandName::SetMoveModules)
    }

    pub fn set_operator(self) -> Result<String, Error> {
        execute_command!(self, Command::SetOperator, CommandName::SetOperator)
    }

    pub fn verify(self) -> Result<String, Error> {
        execute_command!(self, Command::Verify, CommandName::Verify)
    }
}

/// These tests depends on running Vault, which can be done by using the provided docker run script
/// in `docker/testutils/start_vault_container.sh`.
/// Note: Some of these tests may fail if you run them too quickly one after another due to data
/// sychronization issues within Vault. It would seem the only way to fix it would be to restart
/// the Vault service between runs.
#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::storage_helper::StorageHelper;
    use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, Uniform};
    use aptos_global_constants::OPERATOR_KEY;
    use aptos_management::constants;
    use aptos_secure_storage::KVStorage;
    use std::{fs::File, io::Write};

    #[test]
    fn test_set_layout() {
        let helper = StorageHelper::new();

        let temppath = aptos_temppath::TempPath::new();
        helper
            .set_layout(temppath.path().to_str().unwrap())
            .unwrap_err();

        temppath.create_as_file().unwrap();
        let mut file = File::create(temppath.path()).unwrap();
        let layout_text = "\
            operators = [\"alice\", \"bob\"]\n\
            owners = [\"carol\"]\n\
            aptos_root = \"dave\"\n\
        ";
        file.write_all(&layout_text.to_string().into_bytes())
            .unwrap();
        file.sync_all().unwrap();

        helper
            .set_layout(temppath.path().to_str().unwrap())
            .unwrap();
        let storage = helper.storage(constants::COMMON_NS.into());
        let stored_layout = storage.get::<String>(constants::LAYOUT).unwrap().value;
        assert_eq!(layout_text, stored_layout);
    }

    #[test]
    fn test_set_operator() {
        let storage_helper = StorageHelper::new();
        let local_owner_ns = "local";
        let remote_owner_ns = "owner";
        storage_helper.initialize_by_idx(local_owner_ns.into(), 0);

        // Upload an operator key to the remote storage
        let operator_name = "operator";
        let operator_key = Ed25519PrivateKey::generate_for_testing().public_key();
        let mut shared_storage = storage_helper.storage(operator_name.into());
        shared_storage
            .set(OPERATOR_KEY, operator_key)
            .map_err(|e| Error::StorageWriteError("shared", OPERATOR_KEY, e.to_string()))
            .unwrap();

        // Owner calls the set-operator command
        let local_operator_name = storage_helper
            .set_operator(operator_name, remote_owner_ns)
            .unwrap();

        // Verify that a file setting the operator was uploaded to the remote storage
        let shared_storage = storage_helper.storage(remote_owner_ns.into());
        let uploaded_operator_name = shared_storage
            .get::<String>(constants::VALIDATOR_OPERATOR)
            .unwrap()
            .value;
        assert_eq!(local_operator_name, uploaded_operator_name);
    }

    #[test]
    fn test_verify() {
        let helper = StorageHelper::new();
        let namespace = "verify";

        let output = helper
            .verify(namespace)
            .unwrap()
            .split("Key not set")
            .count();
        // 9 KeyNotSet results in 10 splits
        assert_eq!(output, 10);

        helper.initialize_by_idx(namespace.into(), 0);

        let output = helper
            .verify(namespace)
            .unwrap()
            .split("Key not set")
            .count();
        // All keys are set now
        assert_eq!(output, 1);
    }
}
