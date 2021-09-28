// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_sdk::client::BlockingClient;
use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const MAIN_PKG_PATH: &str = "main";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub(crate) blockchain: String,
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

// returns ~/.shuffle
pub fn get_shuffle_dir() -> PathBuf {
    BaseDirs::new().unwrap().home_dir().join(".shuffle")
}

#[cfg(test)]
mod test {
    use crate::shared::get_shuffle_dir;
    use directories::BaseDirs;
    use std::path::PathBuf;

    #[test]
    fn test_get_shuffle_dir() {
        let path = BaseDirs::new()
            .unwrap()
            .home_dir()
            .to_str()
            .unwrap()
            .to_string()
            + "/.shuffle";
        let correct_dir = PathBuf::from(path);
        assert_eq!(get_shuffle_dir(), correct_dir);
    }
}
