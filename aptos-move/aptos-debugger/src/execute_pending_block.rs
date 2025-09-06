// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{aptos_debugger::AptosDebugger, common::Opts};
use anyhow::Result;
use aptos_crypto::HashValue;
use aptos_logger::info;
use aptos_rest_client::Client;
use clap::Parser;
use std::path::PathBuf;
use url::Url;

#[derive(Parser)]
#[clap(group(clap::ArgGroup::new("block_target")
        .required(true)
        .multiple(false)
        .args(&["block_rest_endpoint", "consensus_db_path"]),
))]
pub struct Command {
    #[clap(flatten)]
    opts: Opts,

    #[clap(long, group = "block_target")]
    block_rest_endpoint: Option<String>,

    #[clap(long, group = "block_target")]
    consensus_db_path: Option<PathBuf>,

    #[clap(long)]
    begin_version: u64,

    #[clap(long)]
    block_id: Option<HashValue>,

    #[clap(long)]
    add_system_txns: bool,

    #[clap(long)]
    repeat_execution_times: Option<u64>,
}

impl Command {
    pub async fn run(self) -> Result<()> {
        let debugger = if let Some(rest_endpoint) = self.opts.target.rest_endpoint {
            AptosDebugger::rest_client(Client::new(Url::parse(&rest_endpoint)?))?
        } else if let Some(db_path) = self.opts.target.db_path {
            AptosDebugger::db(db_path)?
        } else {
            unreachable!("Must provide one target.");
        };

        let (user_txns, auxiliary_infos) =
            if let Some(block_rest_endpoint) = self.block_rest_endpoint {
                info!(
                    "Getting block {:?} from {block_rest_endpoint:?}.",
                    self.block_id
                );

                // Get the pending transactions from the REST endpoint
                let base_url =
                    Url::parse(&block_rest_endpoint)?.join("/debug/consensus/block?bcs=true")?;
                let url = if let Some(block_id) = self.block_id {
                    base_url.join(&format!("&block_id={block_id:?}"))?
                } else {
                    base_url
                };
                info!("GET {url:?}...");
                let body = reqwest::get(url).await?.bytes().await?;
                let pending_txns: Vec<_> = bcs::from_bytes(&body)?;

                // Question[MI Counter]: Is it okay to use these auxiliary infos here?
                let pending_aux_infos = (0..pending_txns.len())
                    .map(|i| aptos_types::transaction::PersistedAuxiliaryInfo::V1 {
                        transaction_index: i as u32,
                    })
                    .collect::<Vec<_>>();

                (pending_txns, pending_aux_infos)
            } else if let Some(consensus_db_path) = self.consensus_db_path {
                info!(
                    "Getting block {:?} from {consensus_db_path:?}.",
                    self.block_id
                );
                let cmd = aptos_consensus::util::db_tool::Command {
                    db_dir: consensus_db_path,
                    block_id: self.block_id,
                };
                let txns = cmd.dump_pending_txns()?;

                // For consensus DB path, create auxiliary infos with sequential indices
                let aux_infos = (0..txns.len())
                    .map(|i| aptos_types::transaction::PersistedAuxiliaryInfo::V1 {
                        transaction_index: i as u32,
                    })
                    .collect::<Vec<_>>();

                (txns, aux_infos)
            } else {
                unreachable!("Must provide one target.");
            };

        let (block, block_auxiliary_infos) = if self.add_system_txns {
            todo!("Add block metadata txn and state checkpoint txn if necessary.");
        } else {
            (user_txns, auxiliary_infos)
        };

        let txn_outputs = debugger.execute_transactions_at_version(
            self.begin_version,
            block,
            block_auxiliary_infos,
            self.repeat_execution_times.unwrap_or(1),
            &self.opts.concurrency_level,
        )?;
        println!("{txn_outputs:#?}");

        Ok(())
    }
}
