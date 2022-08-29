// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use crate::common::{
    types::{
        CliCommand, CliConfig, CliError, CliTypedResult, ConfigSearchMode, EncodingOptions,
        ExtractPublicKey, ProfileConfig, ProfileOptions, PromptOptions, PublicKeyInputOptions,
        RestOptions, TransactionOptions, TransactionSummary,
    },
    utils::{prompt_yes_with_override, read_line},
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey, SigningKey,
};
use aptos_rest_client::Client;
use aptos_types::{
    account_address::AccountAddress, account_config::CORE_CODE_ADDRESS,
    transaction::authenticator::AuthenticationKey,
};
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

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,

    /// Private key encoded in a type as shown in `encoding`
    #[clap(long)]
    pub(crate) new_private_key: String,
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

        if let Err(cli_err) = prompt_yes_with_override(
            "Do you want to create a profile with the new private key?",
            self.prompt_options,
        ) {
            match cli_err {
                CliError::AbortedError => {
                    return Ok(());
                }
                _ => {
                    return Err(cli_err);
                }
            }
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

/// Command to lookup the account adress through on-chain lookup table
///
#[derive(Debug, Parser)]
pub struct LookupAddress {
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,

    #[clap(flatten)]
    pub(crate) public_key_options: PublicKeyInputOptions,

    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,

    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

impl LookupAddress {
    pub(crate) fn public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        self.public_key_options.extract_public_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )
    }

    /// Builds a rest client
    fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options.profile)
    }
}

#[async_trait]
impl CliCommand<String> for LookupAddress {
    fn command_name(&self) -> &'static str {
        "LookupAddress"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let originating_resource = self
            .rest_client()?
            .get_account_resource(CORE_CODE_ADDRESS, "0x1::account::OriginatingAddress")
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner()
            .ok_or_else(|| CliError::UnexpectedError("Unable to parse API response.".to_string()))?
            .data;

        let table_handle = originating_resource["address_map"]["handle"]
            .as_str()
            .ok_or_else(|| {
                CliError::UnexpectedError("Unable to parse table handle.".to_string())
            })?;

        // The derived address that can be used to look up the original address
        let address_key = AuthenticationKey::ed25519(&self.public_key()?).derived_address();

        Ok(self
            .rest_client()?
            .get_table_item(
                table_handle,
                "address",
                "address",
                address_key.to_hex_literal(),
            )
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner()
            .as_str()
            .ok_or_else(|| CliError::UnexpectedError("Unable to parse API response.".to_string()))?
            .to_string())
    }
}
