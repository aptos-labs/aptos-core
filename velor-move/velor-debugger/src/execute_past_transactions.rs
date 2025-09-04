// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{velor_debugger::VelorDebugger, common::Opts};
use anyhow::Result;
use velor_rest_client::Client;
use clap::Parser;
use url::Url;

#[derive(Parser)]
pub struct Command {
    #[clap(flatten)]
    opts: Opts,

    #[clap(long)]
    begin_version: u64,

    #[clap(long)]
    limit: u64,

    #[clap(long)]
    skip_result: bool,

    #[clap(long)]
    repeat_execution_times: Option<u64>,

    #[clap(long)]
    use_same_block_boundaries: bool,
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

        let result = debugger
            .execute_past_transactions(
                self.begin_version,
                self.limit,
                self.use_same_block_boundaries,
                self.repeat_execution_times.unwrap_or(1),
                &self.opts.concurrency_level,
            )
            .await?;

        if !self.skip_result {
            println!("{result:#?}",);
        }

        Ok(())
    }
}
