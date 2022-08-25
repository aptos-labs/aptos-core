// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use crate::common::{
    types::{
        CliCommand, CliConfig, CliError, CliTypedResult, ConfigSearchMode, ProfileConfig,
        TransactionOptions, TransactionSummary,
    },
    utils::read_line,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey};
use aptos_types::{account_address::AccountAddress, account_config::CORE_CODE_ADDRESS};
use async_trait::async_trait;
use cached_packages::aptos_stdlib;
use clap::Parser;
use serde::{Deserialize, Serialize};

// This struct includes TypeInfo (account_address, module_name, and struct_name)
// and RotationProofChallenge-specific information (sequence_number, originator, current_auth_key, and new_public_key)
// Since the struct RotationProofChallenge is defined in "0x1::account::RotationProofChallenge",
// we will be passing in "0x1" to `account_address`, "account" to `module_name`, and "RotationProofChallenge" to `struct_name`
// Originator refers to the user's address
#[derive(Serialize, Deserialize)]
pub struct RotationProofChallenge {
    // Should be `CORE_CODE_ADDRESS`
    pub account_address: AccountAddress,
    // Should be `account`
    pub module_name: String,
    // Should be `RotationProofChallenge`
    pub struct_name: String,
    pub sequence_number: u64,
    pub originator: AccountAddress,
    pub current_auth_key: AccountAddress,
    pub new_public_key: Vec<u8>,
}

/// Command to rotate the account auth key
///
#[derive(Debug, Parser)]
pub struct RotateKey {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,

    /// Private key encoded in a type as shown in `encoding`
    #[clap(long)]
    new_private_key: String,
}

#[async_trait]
impl CliCommand<()> for RotateKey {
    fn command_name(&self) -> &'static str {
        "RotateKey"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let key = self.new_private_key.as_bytes().to_vec();
        let new_private_key = self
            .txn_options
            .encoding_options
            .encoding
            .decode_key::<Ed25519PrivateKey>("--new-private-key", key)?;

        let sender_address = self.txn_options.sender_address()?;

        // Get sequence number for account
        let sequence_number = self.txn_options.sequence_number(sender_address).await?;
        let auth_key = self.txn_options.auth_key(sender_address).await?;

        let rotation_proof = RotationProofChallenge {
            account_address: CORE_CODE_ADDRESS,
            module_name: "account".to_string(),
            struct_name: "RotationProofChallenge".to_string(),
            sequence_number,
            originator: sender_address,
            current_auth_key: AccountAddress::from_bytes(&auth_key)
                .map_err(|err| CliError::UnableToParse("auth_key", err.to_string()))?,
            new_public_key: new_private_key.public_key().to_bytes().to_vec(),
        };

        let rotation_msg = bcs::to_bytes(&rotation_proof);

        // Signs the struct using both the current private key and the next private key
        let rotation_proof_signed_by_current_private_key = self
            .txn_options
            .private_key()?
            .sign_arbitrary_message(&rotation_msg.clone().unwrap());
        let rotation_proof_signed_by_new_private_key =
            new_private_key.sign_arbitrary_message(&rotation_msg.unwrap());

        let txn_summary = self
            .txn_options
            .submit_transaction(aptos_stdlib::account_rotate_authentication_key_ed25519(
                rotation_proof_signed_by_current_private_key
                    .to_bytes()
                    .to_vec(),
                rotation_proof_signed_by_new_private_key.to_bytes().to_vec(),
                // Existing public key
                self.txn_options
                    .private_key()?
                    .public_key()
                    .to_bytes()
                    .to_vec(),
                // New public key
                new_private_key.public_key().to_bytes().to_vec(),
            ))
            .await
            .map(TransactionSummary::from)?;

        let string = serde_json::to_string_pretty(&txn_summary)
            .map_err(|err| CliError::UnableToParse("trasaction summary", err.to_string()))?;

        eprintln!("{}", string);

        if let Some(txn_success) = txn_summary.success {
            if !txn_success {
                return Err(CliError::ApiError(
                    "transaction was not executed successfully".to_string(),
                ));
            }
        } else {
            return Err(CliError::UnexpectedError(
                "Mailformed transaction response".to_string(),
            ));
        }

        // Asks user if they want to create a new Profile. Overriding profile is a bit of risky, we create a new one
        // instead.

        // Default is Yes
        eprintln!("Do you want to create a profile with the new private key? [Yes | no]");
        let should_create_profile = read_line("Should create a profile")?.trim().to_string();

        if should_create_profile.to_lowercase() == "no" {
            return Ok(());
        }

        eprintln!("Enter the name for the profile");
        let profile_name = read_line("Profile name")?.trim().to_string();

        if profile_name.is_empty() {
            return Ok(());
        }

        let profile_config = ProfileConfig {
            private_key: Some(new_private_key.clone()),
            public_key: Some(new_private_key.public_key()),
            ..self.txn_options.profile_options.profile()?
        };

        let mut config = CliConfig::load(ConfigSearchMode::CurrentDirAndParents)?;

        if config.profiles.is_none() {
            // This should not happen. The command requires a profile exist to rotate key
            config.profiles = Some(BTreeMap::new());
        }
        config
            .profiles
            .as_mut()
            .unwrap()
            .insert(profile_name.clone(), profile_config);
        config.save()?;

        eprintln!("Profile {} is saved.", profile_name);

        Ok(())
    }
}
