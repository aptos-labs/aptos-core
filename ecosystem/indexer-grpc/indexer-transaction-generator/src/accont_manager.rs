// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

const FAUCET_URL: &str = "http://localhost:8081";
const REST_URL: &str = "http://localhost:8080";

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountManager {
    pub accounts: HashMap<String, Account>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Account {
    pub public_key: String,
    pub private_key: String,
    pub account: String,
}

impl Account {
    pub async fn save_as_profile_file(&self, profile_file_path: &Path) -> anyhow::Result<()> {
        // TODO: refactor this to use serde to write the file.
        let content = format!(
            "---\nprofiles:\n  default:\n    public_key: {}\n    private_key: {}\n    account: {}\n    rest_url: {REST_URL}\n    faucet_url: {FAUCET_URL}",
            self.public_key, self.private_key, self.account
        );
        // create the folder.
        let account_folder = profile_file_path.join(".velor");
        tokio::fs::create_dir_all(account_folder.clone())
            .await
            .context(format!(
                "[Account] Failed to create account profile folder at path: {:?}",
                profile_file_path
            ))?;
        tokio::fs::write(account_folder.join("config.yaml"), content)
            .await
            .context(format!(
                "[Account] Failed to save account profile to path: {:?}",
                profile_file_path
            ))?;
        Ok(())
    }

    pub async fn delete_profile_file(&self, profile_file_path: &Path) -> anyhow::Result<()> {
        let account_folder = profile_file_path.join(".velor");
        tokio::fs::remove_dir_all(account_folder)
            .await
            .context(format!(
                "[Account] Failed to delete account profile folder at path: {:?}",
                profile_file_path
            ))?;
        Ok(())
    }
}

impl AccountManager {
    pub async fn load(account_manager_file_path: &Path) -> anyhow::Result<Self> {
        let file = tokio::fs::read_to_string(account_manager_file_path)
            .await
            .context(format!(
                "[Account Manager] The account list file is not found or readable at path: {:?}",
                account_manager_file_path
            ))?;
        let account_manager: AccountManager = serde_yaml::from_str(&file)
            .context("[Account Manager] Failed to parse account list file")?;

        Ok(account_manager)
    }

    pub fn get_account(&mut self, account_address: &str) -> anyhow::Result<Account> {
        match self.accounts.get(account_address) {
            Some(account) => Ok(account.clone()),
            None => anyhow::bail!(
                "[Account Manager] Account not found for address: {}",
                account_address
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load() {
        // create a temp file to load.
        let testing_folder_root_path = std::env::temp_dir().join("root_folder");
        let _ = tokio::fs::create_dir(&testing_folder_root_path).await;
        // Example content of the file.
        let content = r#"accounts:
            a531b7fdd7917f73ca216d89a8d9ce0cf7e7cfb9086ca6f6cbf9521532748d16:
              private_key: "0x99978d48e7b2d50d0a7a3273db0929447ae59635e71118fa256af654c0ce56c9"
              public_key: "0x39b4acc85e026dc056464a5ea00b98f858260eaad2b74dd30b86ae0d4d94ddf5"
              account: a531b7fdd7917f73ca216d89a8d9ce0cf7e7cfb9086ca6f6cbf9521532748d16
            501b015c58f2a1a62a330a6da80dfee723f528f719d25a4232751986f9a9f43f:
              private_key: "0xe77498ac20ca67e8f642a6521077c8d5cba54853e7bed1e2c33b67e5a7b6c76e"
              public_key: "0xc92c8e7b4467e629ca8cd201a21564de39eea7cbe45b59bfd37f10b56e0a728c"
              account: 501b015c58f2a1a62a330a6da80dfee723f528f719d25a4232751986f9a9f43f
                "#;
        let account_manager_file_path = testing_folder_root_path
            .clone()
            .join("testing_accounts.yaml");
        tokio::fs::write(&account_manager_file_path, content)
            .await
            .unwrap();
        // create an account manager.
        let account_manager = AccountManager::load(&account_manager_file_path).await;
        assert!(account_manager.is_ok());
        let account_manager = account_manager.unwrap();
        assert_eq!(account_manager.accounts.len(), 2);
    }
}
