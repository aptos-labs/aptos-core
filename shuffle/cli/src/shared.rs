// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_sdk::client::BlockingClient;
use diem_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub(crate) blockchain: String,
    pub(crate) named_addresses: BTreeMap<String, AccountAddress>,
}

pub fn read_config(project_path: &Path) -> Result<Config> {
    let config_string =
        fs::read_to_string(project_path.join("Shuffle").with_extension("toml")).unwrap();
    let read_config: Config = toml::from_str(config_string.as_str())?;
    Ok(read_config)
}

/// Send a transaction to the blockchain through the blocking client.
pub fn send(client: &BlockingClient, tx: diem_types::transaction::SignedTransaction) -> Result<()> {
    use diem_json_rpc_types::views::VMStatusView;

    client.submit(&tx)?;
    assert_eq!(
        client
            .wait_for_signed_transaction(&tx, Some(std::time::Duration::from_secs(60)), None)?
            .into_inner()
            .vm_status,
        VMStatusView::Executed,
    );
    Ok(())
}
