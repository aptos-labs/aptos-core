// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_sdk::{
    client::BlockingClient,
    crypto::ed25519::Ed25519PrivateKey,
    transaction_builder::TransactionFactory,
    types::{AccountKey, LocalAccount},
};
use forge::ChainInfo;
use shuffle::{
    account, deploy, new, shared,
    shared::{DevApiClient, Home, Network, NetworkHome},
};
use std::{convert::TryFrom, path::PathBuf, str::FromStr};
use tempfile::TempDir;
use url::Url;

pub struct ShuffleTestHelper {
    home: Home,
    network: Network,
    network_home: NetworkHome,
    tmp_dir: TempDir,
}

const FORGE_NETWORK_NAME: &str = "forge";

impl ShuffleTestHelper {
    pub fn new(chain_info: &mut ChainInfo<'_>) -> Result<Self> {
        let tmp_dir = TempDir::new()?;
        let home = Home::new(tmp_dir.path())?;
        let network_home = home.new_network_home(FORGE_NETWORK_NAME);
        network_home.generate_paths_if_nonexistent()?;
        let network = Network::new(
            String::from(FORGE_NETWORK_NAME),
            Url::from_str(chain_info.json_rpc())?,
            Url::from_str(chain_info.rest_api())?,
            None,
        );
        Ok(Self {
            home,
            network,
            network_home,
            tmp_dir,
        })
    }

    pub fn hardcoded_0x2416_account(client: &BlockingClient) -> Result<LocalAccount> {
        let private_key: Ed25519PrivateKey = bcs::from_bytes(shared::NEW_KEY_FILE_CONTENT)?;
        let key = AccountKey::from_private_key(private_key);
        let address = key.authentication_key().derived_address();
        let account_view = client.get_account(address)?.into_inner();
        let seq_num = match account_view {
            Some(account_view) => account_view.sequence_number,
            None => 0,
        };
        Ok(LocalAccount::new(address, key, seq_num))
    }

    pub fn home(&self) -> &Home {
        &self.home
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn network_home(&self) -> &NetworkHome {
        &self.network_home
    }

    pub fn project_path(&self) -> PathBuf {
        self.tmp_dir.path().join("project")
    }

    pub fn create_account(
        &self,
        treasury_account: &mut LocalAccount,
        new_account: &LocalAccount,
        factory: TransactionFactory,
        client: BlockingClient,
    ) -> Result<()> {
        let bytes: &[u8] = &new_account.private_key().to_bytes();
        let private_key = Ed25519PrivateKey::try_from(bytes).map_err(anyhow::Error::new)?;
        self.network_home().save_key_as_latest(private_key)?;
        self.network_home()
            .generate_latest_address_file(new_account.public_key())?;
        account::create_account_onchain(treasury_account, new_account, &factory, &client)
    }

    pub fn create_project(&self) -> Result<()> {
        new::handle(
            &self.home,
            new::DEFAULT_BLOCKCHAIN.to_string(),
            self.project_path(),
        )
    }

    pub async fn deploy_project(
        &self,
        account: &mut LocalAccount,
        dev_api_url: &str,
    ) -> Result<()> {
        let url = Url::from_str(dev_api_url)?;
        let client = DevApiClient::new(reqwest::Client::new(), url)?;
        deploy::deploy(client, account, &self.project_path()).await
    }
}
