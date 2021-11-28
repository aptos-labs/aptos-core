// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_sdk::client::AccountAddress;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// Encapsulates the fields needed to execute on behalf of a user, especially
/// in hand offs to typescript for console REPL and end to end testing.
pub struct UserContext {
    username: String,
    address: AccountAddress,
    private_key_path: PathBuf,
}

impl UserContext {
    pub fn new(username: &str, address: AccountAddress, private_key_path: &Path) -> UserContext {
        UserContext {
            username: username.to_string(),
            address,
            private_key_path: private_key_path.to_owned(),
        }
    }

    #[allow(dead_code)]
    pub fn username(&self) -> &str {
        &self.username
    }

    #[allow(dead_code)]
    pub fn address(&self) -> &AccountAddress {
        &self.address
    }

    #[allow(dead_code)]
    pub fn private_key_path(&self) -> &Path {
        &self.private_key_path
    }

    pub fn to_envs(&self) -> HashMap<String, String> {
        let mut envs: HashMap<String, String> = HashMap::new();
        envs.insert(
            format!("{}_{}", "ADDRESS", self.username.to_uppercase()),
            self.address.to_hex_literal(),
        );
        envs.insert(
            format!("{}_{}", "PRIVATE_KEY_PATH", self.username.to_uppercase()),
            self.private_key_path.to_string_lossy().to_string(),
        );
        envs
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use diem_sdk::types::LocalAccount;

    #[test]
    fn test_to_envs() {
        let mut rng = rand::rngs::OsRng;
        let account = LocalAccount::generate(&mut rng);
        let user = UserContext::new("test", account.address(), Path::new("dev.key"));
        let actual_envs = user.to_envs();
        let mut actual_keys = actual_envs.keys().cloned().collect::<Vec<String>>();
        actual_keys.sort();

        assert_eq!(actual_keys, vec!["ADDRESS_TEST", "PRIVATE_KEY_PATH_TEST"],);
        assert_eq!(
            actual_envs.get("ADDRESS_TEST").unwrap(),
            &account.address().to_hex_literal()
        );
        assert_eq!(actual_envs.get("PRIVATE_KEY_PATH_TEST").unwrap(), "dev.key");
    }
}
