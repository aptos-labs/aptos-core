// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{
        account_address_from_auth_key, account_address_from_public_key,
        AuthenticationKeyInputOptions, CliCommand, CliConfig, CliError, CliTypedResult,
        ConfigSearchMode, EncodingOptions, ExtractPublicKey, ParsePrivateKey, ProfileConfig,
        ProfileOptions, PublicKeyInputOptions, RestOptions, TransactionOptions, TransactionSummary,
    },
    utils::{prompt_yes, prompt_yes_with_override, read_line},
};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
    PrivateKey, SigningKey,
};
use aptos_rest_client::{
    aptos_api_types::{AptosError, AptosErrorCode},
    error::{AptosErrorResponse, RestError},
    Client,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{RotationProofChallenge, CORE_CODE_ADDRESS},
    transaction::authenticator::AuthenticationKey,
};
use async_trait::async_trait;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// Rotate an account's authentication key
///
/// Rotating the account's authentication key allows you to use a new
/// private key.  You must provide a new private key.  Once it is
/// rotated you will need to use the original account address, with the
/// new private key.  There is an interactive prompt to help you add it
/// to a new profile.
#[derive(Debug, Parser)]
pub struct RotateKey {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,

    /// File name that contains the new private key encoded in the type from `--encoding`
    #[clap(long, group = "new_private_key_inputs", value_parser)]
    pub(crate) new_private_key_file: Option<PathBuf>,

    /// New private key encoded in the type from `--encoding`
    #[clap(long, group = "new_private_key_inputs")]
    pub(crate) new_private_key: Option<String>,

    /// Name of the profile to save the new private key
    ///
    /// If not provided, it will interactively have you save a profile,
    /// unless `--skip_saving_profile` is provided
    #[clap(long)]
    pub(crate) save_to_profile: Option<String>,

    /// Skip saving profile
    ///
    /// This skips the interactive profile saving after rotating the authentication key
    #[clap(long)]
    pub(crate) skip_saving_profile: bool,
}

impl ParsePrivateKey for RotateKey {}

impl RotateKey {
    /// Extract private key from CLI args
    pub fn extract_private_key(
        &self,
        encoding: EncodingType,
    ) -> CliTypedResult<Option<Ed25519PrivateKey>> {
        self.parse_private_key(
            encoding,
            self.new_private_key_file.clone(),
            self.new_private_key.clone(),
        )
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RotateSummary {
    message: Option<String>,
    transaction: TransactionSummary,
}

#[async_trait]
impl CliCommand<RotateSummary> for RotateKey {
    fn command_name(&self) -> &'static str {
        "RotateKey"
    }

    async fn execute(self) -> CliTypedResult<RotateSummary> {
        let new_private_key = self
            .extract_private_key(self.txn_options.encoding_options.encoding)?
            .ok_or_else(|| {
                CliError::CommandArgumentError(
                    "One of ['--new-private-key', '--new-private-key-file'] must be used"
                        .to_string(),
                )
            })?;

        let (current_private_key, sender_address) = self.txn_options.get_key_and_address()?;

        if new_private_key == current_private_key {
            return Err(CliError::CommandArgumentError(
                "New private key cannot be the same as the current private key".to_string(),
            ));
        }

        // Get sequence number for account
        let sequence_number = self.txn_options.sequence_number(sender_address).await?;
        let auth_key = self.txn_options.auth_key(sender_address).await?;

        let rotation_proof = RotationProofChallenge {
            account_address: CORE_CODE_ADDRESS,
            module_name: "account".to_string(),
            struct_name: "RotationProofChallenge".to_string(),
            sequence_number,
            originator: sender_address,
            current_auth_key: AccountAddress::from_bytes(auth_key)
                .map_err(|err| CliError::UnableToParse("auth_key", err.to_string()))?,
            new_public_key: new_private_key.public_key().to_bytes().to_vec(),
        };

        let rotation_msg =
            bcs::to_bytes(&rotation_proof).map_err(|err| CliError::BCS("rotation_proof", err))?;

        // Signs the struct using both the current private key and the next private key
        let rotation_proof_signed_by_current_private_key =
            current_private_key.sign_arbitrary_message(&rotation_msg.clone());
        let rotation_proof_signed_by_new_private_key =
            new_private_key.sign_arbitrary_message(&rotation_msg);

        let txn_summary = self
            .txn_options
            .submit_transaction(aptos_stdlib::account_rotate_authentication_key(
                0,
                // Existing public key
                current_private_key.public_key().to_bytes().to_vec(),
                0,
                // New public key
                new_private_key.public_key().to_bytes().to_vec(),
                rotation_proof_signed_by_current_private_key
                    .to_bytes()
                    .to_vec(),
                rotation_proof_signed_by_new_private_key.to_bytes().to_vec(),
            ))
            .await
            .map(TransactionSummary::from)?;

        let string = serde_json::to_string_pretty(&txn_summary)
            .map_err(|err| CliError::UnableToParse("transaction summary", err.to_string()))?;

        eprintln!("{}", string);

        if let Some(txn_success) = txn_summary.success {
            if !txn_success {
                return Err(CliError::ApiError(
                    "Transaction was not executed successfully".to_string(),
                ));
            }
        } else {
            return Err(CliError::UnexpectedError(
                "Malformed transaction response".to_string(),
            ));
        }

        let mut profile_name: String;

        if self.save_to_profile.is_none() {
            if self.skip_saving_profile
                || !prompt_yes("Do you want to create a profile for the new key?")
            {
                return Ok(RotateSummary {
                    transaction: txn_summary,
                    message: None,
                });
            }

            eprintln!("Enter the name for the profile");
            profile_name = read_line("Profile name")?.trim().to_string();
        } else {
            // We can safely unwrap here
            profile_name = self.save_to_profile.unwrap();
        }

        // Check if profile name exists
        let mut config = CliConfig::load(ConfigSearchMode::CurrentDirAndParents)?;

        if let Some(ref profiles) = config.profiles {
            if profiles.contains_key(&profile_name) {
                if let Err(cli_err) = prompt_yes_with_override(
                    format!(
                        "Profile {} exits. Do you want to provide a new profile name?",
                        profile_name
                    )
                    .as_str(),
                    self.txn_options.prompt_options,
                ) {
                    match cli_err {
                        CliError::AbortedError => {
                            return Ok(RotateSummary {
                                transaction: txn_summary,
                                message: None,
                            });
                        },
                        _ => {
                            return Err(cli_err);
                        },
                    }
                }

                eprintln!("Enter the name for the profile");
                profile_name = read_line("Profile name")?.trim().to_string();
            }
        }

        if profile_name.is_empty() {
            return Err(CliError::AbortedError);
        }

        let mut profile_config = ProfileConfig {
            private_key: Some(new_private_key.clone()),
            public_key: Some(new_private_key.public_key()),
            account: Some(sender_address),
            ..self.txn_options.profile_options.profile()?
        };

        if let Some(url) = self.txn_options.rest_options.rpc_url {
            profile_config.rest_url = Some(url.into());
        }

        if config.profiles.is_none() {
            config.profiles = Some(BTreeMap::new());
        }

        config
            .profiles
            .as_mut()
            .unwrap()
            .insert(profile_name.clone(), profile_config);
        config.save()?;

        eprintln!("Profile {} is saved.", profile_name);

        Ok(RotateSummary {
            transaction: txn_summary,
            message: Some(format!("Profile {} is saved.", profile_name)),
        })
    }
}

/// Lookup the account address through the on-chain lookup table
///
/// If the account is rotated, it will provide the address accordingly.  If the account was not
/// rotated, it will provide the derived address only if the account exists onchain.
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

    #[clap(flatten)]
    pub(crate) authentication_key_options: AuthenticationKeyInputOptions,
}

impl LookupAddress {
    pub(crate) fn public_key(&self) -> CliTypedResult<Ed25519PublicKey> {
        self.public_key_options
            .extract_public_key(self.encoding_options.encoding, &self.profile_options)
    }

    pub(crate) fn auth_key(&self) -> CliTypedResult<Option<AuthenticationKey>> {
        self.authentication_key_options
            .extract_auth_key(self.encoding_options.encoding)
    }

    /// Builds a rest client
    fn rest_client(&self) -> CliTypedResult<Client> {
        self.rest_options.client(&self.profile_options)
    }
}

#[async_trait]
impl CliCommand<AccountAddress> for LookupAddress {
    fn command_name(&self) -> &'static str {
        "LookupAddress"
    }

    async fn execute(self) -> CliTypedResult<AccountAddress> {
        let rest_client = self.rest_client()?;

        // TODO: Support arbitrary auth key to support other types like multie25519
        let address = match self.auth_key()? {
            Some(key) => account_address_from_auth_key(&key),
            None => account_address_from_public_key(&self.public_key()?),
        };
        Ok(lookup_address(&rest_client, address, true).await?)
    }
}

pub async fn lookup_address(
    rest_client: &Client,
    address_key: AccountAddress,
    must_exist: bool,
) -> Result<AccountAddress, RestError> {
    let originating_resource: OriginatingResource = rest_client
        .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::account::OriginatingAddress")
        .await?
        .into_inner();

    let table_handle = originating_resource.address_map.handle;

    // The derived address that can be used to look up the original address
    match rest_client
        .get_table_item_bcs(
            table_handle,
            "address",
            "address",
            address_key.to_hex_literal(),
        )
        .await
    {
        Ok(inner) => Ok(inner.into_inner()),
        Err(RestError::Api(AptosErrorResponse {
            error:
                AptosError {
                    error_code: AptosErrorCode::TableItemNotFound,
                    ..
                },
            ..
        })) => {
            // If the table item wasn't found, we may check if the account exists
            if !must_exist {
                Ok(address_key)
            } else {
                rest_client
                    .get_account_bcs(address_key)
                    .await
                    .map(|_| address_key)
            }
        },
        Err(err) => Err(err),
    }
}

#[derive(Deserialize)]
pub struct OriginatingResource {
    pub address_map: Table,
}

#[derive(Deserialize)]
pub struct Table {
    pub handle: AccountAddress,
}
