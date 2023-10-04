// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;

/// Args related to running an indexer API for the local testnet.
#[derive(Debug, Parser)]
pub struct IndexerApiArgs {
    /// If set, we will run a postgres DB using Docker (unless
    /// --use-host-postgres is set), run the standard set of indexer processors (see
    /// --processors) and configure them to write to this DB, and run an API that lets
    /// you access the data they write to storage. This is opt in because it requires
    /// Docker to be installed in the host system.
    #[clap(long, conflicts_with = "no_txn_stream")]
    pub with_indexer_api: bool,
}
