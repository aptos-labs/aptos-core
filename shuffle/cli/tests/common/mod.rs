// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_sdk::{
    crypto::ed25519::Ed25519PrivateKey, transaction_builder::TransactionFactory,
    types::LocalAccount,
};
use forge::{AdminContext, ChainInfo};
use shuffle::{
    account, deploy,
    dev_api_client::DevApiClient,
    new, shared,
    shared::{Home, Network, NetworkHome, NetworksConfig},
};
use smoke_test::scripts_and_modules::enable_open_publishing;
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};
use tempfile::TempDir;
use tokio::runtime::Runtime;
use url::Url;

const FORGE_NETWORK_NAME: &str = "forge";

#[allow(dead_code)]
pub struct ShuffleTestHelper {
    home: Home,
    network: Network,
    networks_config: NetworksConfig,
    network_home: NetworkHome,
    tmp_dir: TempDir,
}

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
        let mut mapping: BTreeMap<String, Network> = BTreeMap::new();
        mapping.insert(FORGE_NETWORK_NAME.to_string(), network.clone());
        let networks_config = NetworksConfig::new(mapping);
        write_forge_networks_config_into_toml(&home, &networks_config)?;
        Ok(Self {
            home,
            network,
            networks_config,
            network_home,
            tmp_dir,
        })
    }

    #[allow(dead_code)]
    pub fn home(&self) -> &Home {
        &self.home
    }

    #[allow(dead_code)]
    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn network_home(&self) -> &NetworkHome {
        &self.network_home
    }

    #[allow(dead_code)]
    pub fn home_path(&self) -> &Path {
        self.tmp_dir.path()
    }

    #[allow(dead_code)]
    pub fn networks_config(&self) -> &NetworksConfig {
        &self.networks_config
    }

    pub fn project_path(&self) -> PathBuf {
        self.tmp_dir.path().join("project")
    }

    pub async fn create_account(
        &self,
        treasury_account: &mut LocalAccount,
        new_account: &LocalAccount,
        factory: TransactionFactory,
        client: &DevApiClient,
    ) -> Result<()> {
        let bytes: &[u8] = &new_account.private_key().to_bytes();
        let private_key = Ed25519PrivateKey::try_from(bytes).map_err(anyhow::Error::new)?;
        self.network_home().save_key_as_latest(private_key)?;
        self.network_home()
            .generate_latest_address_file(new_account.public_key())?;
        account::create_account_via_dev_api(treasury_account, new_account, &factory, client).await
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

    pub fn codegen_project(&self, account: &LocalAccount) -> Result<()> {
        shared::codegen_typescript_libraries(&self.project_path(), &account.address())
    }
}

fn write_forge_networks_config_into_toml(
    home: &Home,
    networks_config: &NetworksConfig,
) -> Result<()> {
    let network_toml_path = home.get_shuffle_path().join("Networks.toml");
    let networks_config_string = toml::to_string_pretty(networks_config)?;
    fs::write(network_toml_path, networks_config_string)?;
    Ok(())
}

pub fn bootstrap_shuffle_project(ctx: &mut AdminContext<'_>) -> Result<ShuffleTestHelper> {
    let client = ctx.client();
    let dev_client = DevApiClient::new(
        reqwest::Client::new(),
        Url::from_str(ctx.chain_info().rest_api())?,
    )?;
    let factory = ctx.chain_info().transaction_factory();
    enable_open_publishing(&client, &factory, ctx.chain_info().root_account())?;

    let helper = ShuffleTestHelper::new(ctx.chain_info())?;
    helper.create_project()?;

    let rt = Runtime::new().unwrap();
    let handle = rt.handle().clone();

    let mut account = ctx.random_account();
    let tc = ctx.chain_info().treasury_compliance_account();

    handle.block_on(helper.create_account(tc, &account, factory, &dev_client))?;
    handle.block_on(helper.deploy_project(&mut account, ctx.chain_info().rest_api()))?;
    helper.codegen_project(&account)?;
    Ok(helper)
}
