// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod config;
pub mod git;
pub mod keys;

use crate::{
    common::{
        types::{CliError, CliTypedResult, PromptOptions},
        utils::{check_if_file_exists, write_to_file},
    },
    genesis::{
        config::{Layout, ValidatorConfiguration},
        git::{GitOptions, LAYOUT_NAME},
    },
    CliCommand, CliResult,
};
use aptos_config::config::{RocksdbConfig, NO_OP_STORAGE_PRUNER_CONFIG};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, transaction::Transaction};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use async_trait::async_trait;
use clap::Parser;
use std::{convert::TryInto, path::PathBuf};
use storage_interface::DbReaderWriter;
use vm_genesis::Validator;

const MIN_PRICE_PER_GAS_UNIT: u64 = 1;
const WAYPOINT_FILE: &str = "waypoint.txt";
const GENESIS_FILE: &str = "genesis.blob";

/// Tool for setting up and building the Genesis transaction
///
#[derive(Parser)]
pub enum GenesisTool {
    GenerateGenesis(GenerateGenesis),
    GenerateKeys(keys::GenerateKeys),
    SetupGit(git::SetupGit),
    SetValidatorConfiguration(keys::SetValidatorConfiguration),
}

impl GenesisTool {
    pub async fn execute(self) -> CliResult {
        match self {
            GenesisTool::GenerateGenesis(tool) => tool.execute_serialized().await,
            GenesisTool::GenerateKeys(tool) => tool.execute_serialized().await,
            GenesisTool::SetupGit(tool) => tool.execute_serialized_success().await,
            GenesisTool::SetValidatorConfiguration(tool) => tool.execute_serialized_success().await,
        }
    }
}

/// Generate genesis from a git repository
#[derive(Parser)]
pub struct GenerateGenesis {
    #[clap(flatten)]
    prompt_options: PromptOptions,
    #[clap(flatten)]
    git_options: GitOptions,
    #[clap(long, parse(from_os_str), default_value = ".")]
    output_dir: PathBuf,
}

impl GenerateGenesis {
    fn generate_genesis_txn(git_options: GitOptions) -> CliTypedResult<Transaction> {
        let genesis_info = fetch_genesis_info(git_options)?;

        Ok(vm_genesis::encode_genesis_transaction(
            genesis_info.root_key.clone(),
            &genesis_info.validators,
            &genesis_info.modules,
            genesis_info.chain_id,
            MIN_PRICE_PER_GAS_UNIT,
        ))
    }
}

#[async_trait]
impl CliCommand<Vec<PathBuf>> for GenerateGenesis {
    fn command_name(&self) -> &'static str {
        "GenerateGenesis"
    }

    async fn execute(self) -> CliTypedResult<Vec<PathBuf>> {
        let genesis_file = self.output_dir.join(GENESIS_FILE);
        let waypoint_file = self.output_dir.join(WAYPOINT_FILE);
        check_if_file_exists(genesis_file.as_path(), self.prompt_options)?;
        check_if_file_exists(waypoint_file.as_path(), self.prompt_options)?;

        // Generate genesis file
        let genesis = Self::generate_genesis_txn(self.git_options)?;
        write_to_file(
            genesis_file.as_path(),
            GENESIS_FILE,
            &bcs::to_bytes(&genesis).map_err(|e| CliError::BCS(GENESIS_FILE, e))?,
        )?;

        // Generate waypoint file
        let path = TempPath::new();
        let aptosdb = AptosDB::open(
            &path,
            false,
            NO_OP_STORAGE_PRUNER_CONFIG,
            RocksdbConfig::default(),
        )
        .map_err(|e| CliError::UnexpectedError(e.to_string()))?;
        let db_rw = DbReaderWriter::new(aptosdb);
        let waypoint = executor::db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, &genesis)
            .map_err(|e| CliError::UnexpectedError(e.to_string()))?;
        write_to_file(
            waypoint_file.as_path(),
            WAYPOINT_FILE,
            waypoint.to_string().as_bytes(),
        )?;
        Ok(vec![genesis_file, waypoint_file])
    }
}

/// Retrieves all information for genesis from the Git repository
pub fn fetch_genesis_info(git_options: GitOptions) -> CliTypedResult<GenesisInfo> {
    let client = git_options.get_client()?;
    let layout: Layout = client.get(LAYOUT_NAME)?;

    let mut validators = Vec::new();
    for user in &layout.users {
        validators.push(client.get::<ValidatorConfiguration>(user)?.try_into()?);
    }

    let modules = client.get_modules("framework")?;

    Ok(GenesisInfo {
        chain_id: layout.chain_id,
        root_key: layout.root_key,
        validators,
        modules,
    })
}

/// Holder object for all pieces needed to generate a genesis transaction
#[derive(Clone)]
pub struct GenesisInfo {
    chain_id: ChainId,
    root_key: Ed25519PublicKey,
    validators: Vec<Validator>,
    modules: Vec<Vec<u8>>,
}
