// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{health_checker::HealthChecker, traits::ServiceManager, RunLocalnet};
use anyhow::{anyhow, Result};
use aptos_faucet_core::server::{FunderKeyEnum, RunConfig};
use async_trait::async_trait;
use clap::Parser;
use futures::channel::oneshot;
use maplit::hashset;
use reqwest::Url;
use std::{collections::HashSet, net::Ipv4Addr, path::PathBuf};

/// Args related to running a faucet in the localnet.
#[derive(Debug, Parser)]
pub struct FaucetArgs {
    /// Do not run a faucet alongside the node.
    ///
    /// Running a faucet alongside the node allows you to create and fund accounts
    /// for testing.
    #[clap(long)]
    pub no_faucet: bool,

    /// This does nothing, we already run a faucet by default. We only keep this here
    /// for backwards compatibility with tests. We will remove this once the commit
    /// that added --no-faucet makes its way to the testnet branch.
    #[clap(long, hide = true)]
    pub with_faucet: bool,

    /// Port to run the faucet on.
    ///
    /// When running, you'll be able to use the faucet at `http://127.0.0.1:<port>/mint` e.g.
    /// `http//127.0.0.1:8081/mint`
    #[clap(long, default_value_t = 8081)]
    pub faucet_port: u16,

    /// Disable the delegation of faucet minting to a dedicated account.
    #[clap(long)]
    pub do_not_delegate: bool,
}

#[derive(Clone, Debug)]
pub struct FaucetManager {
    config: RunConfig,
    prerequisite_health_checkers: HashSet<HealthChecker>,
}

impl FaucetManager {
    pub fn new(
        args: &RunLocalnet,
        prerequisite_health_checkers: HashSet<HealthChecker>,
        bind_to: Ipv4Addr,
        test_dir: PathBuf,
        node_api_url: Url,
    ) -> Result<Self> {
        Ok(Self {
            config: RunConfig::build_for_cli(
                node_api_url.clone(),
                bind_to.to_string(),
                args.faucet_args.faucet_port,
                FunderKeyEnum::KeyFile(test_dir.join("mint.key")),
                args.faucet_args.do_not_delegate,
                None,
            ),
            prerequisite_health_checkers,
        })
    }
}

#[async_trait]
impl ServiceManager for FaucetManager {
    fn get_name(&self) -> String {
        "Faucet".to_string()
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        self.prerequisite_health_checkers.iter().collect()
    }

    async fn run_service(
        self: Box<Self>,
        health_chckers_tx: oneshot::Sender<HashSet<HealthChecker>>,
    ) -> Result<()> {
        let name = self.get_name();

        let (port_tx, port_rx) = oneshot::channel();

        let run_fut = self.config.run_and_report_port(port_tx);
        let port = port_rx.await?;

        health_chckers_tx
            .send(hashset! {HealthChecker::http_checker_from_port(
                port,
                name.clone(),
            )})
            .map_err(|_| anyhow!("failed to send health checkers for {}", name));

        run_fut.await
    }
}
