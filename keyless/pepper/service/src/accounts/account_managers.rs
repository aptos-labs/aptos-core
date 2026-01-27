// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::error::PepperServiceError;
use aptos_logger::info;
use std::{collections::HashSet, env, str::FromStr};

/// A struct that holds all account recovery managers
pub struct AccountRecoveryManagers {
    account_recovery_managers: HashSet<AccountRecoveryManager>,
}

impl AccountRecoveryManagers {
    pub fn new(account_recovery_managers: Vec<AccountRecoveryManager>) -> Self {
        let mut account_recovery_managers_set = HashSet::new();

        // Collect all account recovery managers
        for account_recovery_manager in account_recovery_managers {
            info!(
                "Adding account recovery manager: issuer={}, aud={}.",
                account_recovery_manager.issuer(),
                account_recovery_manager.aud()
            );
            account_recovery_managers_set.insert(account_recovery_manager);
        }

        Self {
            account_recovery_managers: account_recovery_managers_set,
        }
    }

    /// Creates an empty set of account recovery managers (used for testing)
    #[cfg(test)]
    pub fn new_empty() -> Self {
        Self {
            account_recovery_managers: HashSet::new(),
        }
    }

    /// Check if the given issuer and aud are in the set of account recovery managers
    pub fn contains(&self, issuer: &str, aud: &str) -> bool {
        // If there are no account recovery managers, return false
        if self.account_recovery_managers.is_empty() {
            return false;
        }

        // Otherwise, check if the given issuer and aud are in the set
        let account_recovery_manager = AccountRecoveryManager::new(issuer.into(), aud.into());
        self.account_recovery_managers
            .contains(&account_recovery_manager)
    }
}

/// A struct that represents an account recovery manager
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AccountRecoveryManager {
    issuer: String,
    aud: String,
}

impl AccountRecoveryManager {
    pub fn new(issuer: String, aud: String) -> Self {
        Self { issuer, aud }
    }

    /// Get the issuer of the account manager
    pub fn issuer(&self) -> String {
        self.issuer.clone()
    }

    /// Get the aud of the account manager
    pub fn aud(&self) -> String {
        self.aud.clone()
    }
}

impl FromStr for AccountRecoveryManager {
    type Err = PepperServiceError;

    // Note: this is used to parse each account recovery manager from
    // the command line. The expected format is: "<issuer> <aud>".
    fn from_str(string: &str) -> Result<Self, Self::Err> {
        // Split the string by whitespace
        let mut iterator = string.split_whitespace();

        // Parse the substrings as issuer and aud
        let issuer = iterator.next().ok_or(PepperServiceError::UnexpectedError(
            "Failed to parse issuer for account recovery manager!".into(),
        ))?;
        let aud = iterator.next().ok_or(PepperServiceError::UnexpectedError(
            "Failed to parse aud for account recovery manager!".into(),
        ))?;

        // Verify that there are exactly 2 substrings
        if iterator.next().is_some() {
            return Err(PepperServiceError::UnexpectedError(
                "Too many arguments found for account recovery manager!".into(),
            ));
        }

        // Create the override
        let account_manager_override =
            AccountRecoveryManager::new(issuer.to_string(), aud.to_string());

        Ok(account_manager_override)
    }
}
