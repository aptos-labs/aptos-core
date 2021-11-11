// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_sdk::{client::BlockingClient, types::LocalAccount};
use shuffle::{account, deploy, new, shared::Home};
use std::{path::PathBuf, str::FromStr};
use tempfile::TempDir;
use url::Url;

pub struct ShuffleTestHelper {
    home: Home,
    tmp_dir: TempDir,
}

impl ShuffleTestHelper {
    pub fn new() -> Result<Self> {
        let tmp_dir = TempDir::new()?;
        let home = Home::new(tmp_dir.path())?;
        Ok(Self { tmp_dir, home })
    }

    pub fn home(&self) -> &Home {
        &self.home
    }

    pub fn project_path(&self) -> PathBuf {
        self.tmp_dir.path().join("project")
    }

    pub fn create_accounts(
        &self,
        treasury_account: &mut LocalAccount,
        client: BlockingClient,
    ) -> Result<()> {
        account::create_local_accounts(&self.home, client, treasury_account)
    }

    pub fn create_project(&self) -> Result<()> {
        new::handle(new::DEFAULT_BLOCKCHAIN.to_string(), self.project_path())
    }

    pub async fn deploy_project(&self, dev_api_url: &str) -> Result<()> {
        let url = Url::from_str(dev_api_url)?;
        deploy::handle(&self.home, &self.project_path(), url).await
    }
}
