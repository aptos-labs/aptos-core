// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{RocksdbConfig, NO_OP_STORAGE_PRUNER_CONFIG};
use aptos_management::{config::ConfigPath, error::Error, secure_backend::SharedBackend};
use aptos_temppath::TempPath;
use aptos_types::{chain_id::ChainId, transaction::Transaction, waypoint::Waypoint};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use executor::db_bootstrapper;
use storage_interface::DbReaderWriter;
use structopt::StructOpt;

/// Produces a waypoint from Genesis from the shared storage. It then computes the Waypoint and
/// optionally inserts it into another storage, typically the validator storage.
#[derive(Debug, StructOpt)]
pub struct CreateWaypoint {
    #[structopt(flatten)]
    config: ConfigPath,
    #[structopt(long, required_unless("config"))]
    chain_id: Option<ChainId>,
    #[structopt(flatten)]
    shared_backend: SharedBackend,
}

impl CreateWaypoint {
    pub fn execute(self) -> Result<Waypoint, Error> {
        let genesis_helper = crate::genesis::Genesis {
            config: self.config,
            chain_id: self.chain_id,
            backend: self.shared_backend,
            path: None,
        };

        let genesis = genesis_helper.execute()?;

        create_genesis_waypoint(&genesis)
    }
}

pub fn create_genesis_waypoint(genesis: &Transaction) -> Result<Waypoint, Error> {
    let path = TempPath::new();
    let aptosdb = AptosDB::open(
        &path,
        false,
        NO_OP_STORAGE_PRUNER_CONFIG,
        RocksdbConfig::default(),
    )
    .map_err(|e| Error::UnexpectedError(e.to_string()))?;
    let db_rw = DbReaderWriter::new(aptosdb);

    db_bootstrapper::generate_waypoint::<AptosVM>(&db_rw, genesis)
        .map_err(|e| Error::UnexpectedError(e.to_string()))
}
