// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{velor_debugger::VelorDebugger, common::Opts};
use anyhow::Result;
use velor_crypto::HashValue;
use velor_logger::info;
use velor_rest_client::Client;
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
            VelorDebugger::rest_client(Client::new(Url::parse(&rest_endpoint)?))?
        } else if let Some(db_path) = self.opts.target.db_path {
            VelorDebugger::db(db_path)?
        } else {
            unreachable!("Must provide one target.");
        };

        let user_txns = if let Some(block_rest_endpoint) = self.block_rest_endpoint {
            info!(
                "Getting block {:?} from {block_rest_endpoint:?}.",
                self.block_id
            );
            let base_url =
                Url::parse(&block_rest_endpoint)?.join("/debug/consensus/block?bcs=true")?;
            let url = if let Some(block_id) = self.block_id {
                base_url.join(&format!("&block_id={block_id:?}"))?
            } else {
                base_url
            };
            info!("GET {url:?}...");
            let body = reqwest::get(url).await?.bytes().await?;
            bcs::from_bytes(&body)?
        } else if let Some(consensus_db_path) = self.consensus_db_path {
            info!(
                "Getting block {:?} from {consensus_db_path:?}.",
                self.block_id
            );
            let cmd = velor_consensus::util::db_tool::Command {
                db_dir: consensus_db_path,
                block_id: self.block_id,
            };
            cmd.dump_pending_txns()?
        } else {
            unreachable!("Must provide one target.");
        };

        let block = if self.add_system_txns {
            todo!("Add block metadata txn and state checkpoint txn if necessary.");
        } else {
            user_txns
        };

        let txn_outputs = debugger.execute_transactions_at_version(
            self.begin_version,
            block,
            self.repeat_execution_times.unwrap_or(1),
            &self.opts.concurrency_level,
        )?;
        println!("{txn_outputs:#?}");

        Ok(())
    }
}
