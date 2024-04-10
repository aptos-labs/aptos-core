// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{aptos_debugger::AptosDebugger, common::Opts};
use anyhow::Result;
use aptos_rest_client::Client;
use aptos_vm::AptosVM;
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
}

impl Command {
    pub async fn run(self) -> Result<()> {
        AptosVM::set_concurrency_level_once(self.opts.concurrency_level);

        let debugger = if let Some(rest_endpoint) = self.opts.target.rest_endpoint {
            AptosDebugger::rest_client(Client::new(Url::parse(&rest_endpoint)?))?
        } else if let Some(db_path) = self.opts.target.db_path {
            AptosDebugger::db(db_path)?
        } else {
            unreachable!("Must provide one target.");
        };

        let result = debugger
            .execute_past_transactions(
                self.begin_version,
                self.limit,
                self.repeat_execution_times.unwrap_or(1),
            )
            .await?;

        if !self.skip_result {
            println!("{result:#?}",);
        }

        Ok(())
    }
}
