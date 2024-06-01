// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{
        account_address_from_auth_key, account_address_from_public_key,
        AuthenticationKeyInputOptions, CliCommand, CliConfig, CliError, CliTypedResult,
        ConfigSearchMode, EncodingOptions, ExtractPublicKey, HardwareWalletOptions,
        ParsePrivateKey, ProfileConfig, ProfileOptions, PublicKeyInputOptions, RestOptions,
        TransactionOptions, TransactionSummary,
    },
    utils::{prompt_yes, prompt_yes_with_override, read_line},
};
use aptos_cached_packages::aptos_stdlib;
use aptos_config::config;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
    PrivateKey, SigningKey,
};
use aptos_ledger;
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
///
/// If you wish to rotate from a ledger wallet, it must have its own
/// profile. If you wish to rotate to a ledger wallet, specify the new
/// derivation path or index accordingly.
#[derive(Debug, Parser)]
pub struct RotateKey {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,

    /// File name that contains the new private key encoded in the type from `--encoding`
    #[clap(
        conflicts_with_all = &[
            "derivation_index",
            "derivation_path",
            "new_private_key"
        ],
        long,
        value_parser
    )]
    pub(crate) new_private_key_file: Option<PathBuf>,

    /// New private key encoded in the type from `--encoding`
    #[clap(
        conflicts_with_all = &[
            "derivation_index",
            "derivation_path",
            "new_private_key_file"
        ],
        long
    )]
    pub(crate) new_private_key: Option<String>,

    #[clap(flatten)]
    pub(crate) new_hardware_wallet_options: HardwareWalletOptions,

    /// Skip saving profile(s)
    #[clap(long)]
    pub(crate) skip_saving_profiles: bool,

    /// Name of the profile to save for the new authentication key
    #[clap(conflicts_with = "skip_saving_profiles", long)]
    pub(crate) save_to_profile: Option<String>,

    /// New name for the profile that has just been rendered stale by the rotation operation, when
    /// rotation was from an account with a profile
    #[clap(
        conflicts_with_all = &[
            "new_private_key",
            "new_private_key_file",
            "skip_saving_profiles",
        ],
        long,
    )]
    pub(crate) rename_stale_profile_to: Option<String>,
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
        // Verify profile names before executing rotation operation, to avoid erroring out in a
        // manner that results in corrupted config state.
        if self.save_to_profile.is_some() || self.rename_stale_profile_to.is_some() {
            // Verify no conflict between new and stale profile names, and that neither are empty.
            if let Some(stale_name) = self.rename_stale_profile_to {
                if self.save_to_profile == self.rename_stale_profile_to {
                    return Err(CliError::CommandArgumentError(
                        "New stale profile name and new profile name may not be the same"
                            .to_string(),
                    ));
                };
                if stale_name.is_empty() {
                    return Err(CliError::CommandArgumentError(
                        "New stale profile name may not be empty".to_string(),
                    ));
                }
            };
            if let Some(new_name) = self.save_to_profile {
                if new_name.is_empty() {
                    return Err(CliError::CommandArgumentError(
                        "New profile name may not be empty".to_string(),
                    ));
                }
            };

            // Verify that config exists.
            let mut config = CliConfig::load(ConfigSearchMode::CurrentDirAndParents)?;

            // Verify that the new stale profile name does not already exist in the config, and that
            // the new profile name does not already exist in the config (unless it is the same as
            // the name of a stale profile that will be renamed).
            if let Some(profiles) = config.profiles {
                if let Some(stale_name) = self.rename_stale_profile_to {
                    if profiles.contains_key(&stale_name) {
                        return Err(CliError::CommandArgumentError(format!(
                            "Profile {} already exists",
                            stale_name
                        )));
                    };
                };
                if let Some(new_name) = self.save_to_profile {
                    if profiles.contains_key(&new_name) {
                        if self.rename_stale_profile_to.is_some()
                            && self.save_to_profile == self.txn_options.profile_options.profile
                        {
                            // Do nothing, since new profile is taking the name of a stale profile
                            // that will be renamed.
                        } else {
                            return Err(CliError::CommandArgumentError(format!(
                                "Profile {} already exists",
                                new_name
                            )));
                        }
                    };
                };
            };
        };

        // Get current signer options.
        let current_derivation_path = if self.txn_options.profile_options.profile.is_some() {
            self.txn_options.profile_options.derivation_path()?
        } else {
            None
        };
        let (current_private_key, current_address, current_public_key) = if current_derivation_path
            .is_some()
        {
            (
                None,
                self.txn_options.profile_options.account_address()?,
                self.txn_options.profile_options.public_key()?,
            )
        } else {
            let (current_private_key, current_address) = self.txn_options.get_key_and_address()?;
            (
                Some(current_private_key),
                current_address,
                self.txn_options.get_public_key()?,
            )
        };

        // Get new signer options.
        let new_derivation_path = self.new_hardware_wallet_options.extract_derivation_path()?;
        let (new_private_key, new_public_key) = if new_derivation_path.is_some() {
            (
                None,
                aptos_ledger::get_public_key(new_derivation_path.clone().unwrap().as_str(), false)?,
            )
        } else {
            let new_private_key = self
                .extract_private_key(self.txn_options.encoding_options.encoding)?
                .ok_or_else(|| {
                    CliError::CommandArgumentError("Unable to parse new private key".to_string())
                })?;
            (Some(new_private_key.clone()), new_private_key.public_key())
        };

        // Check that public key is actually changing.
        if new_public_key == current_public_key {
            return Err(CliError::CommandArgumentError(
                "New public key cannot be the same as the current public key".to_string(),
            ));
        }

        // Construct rotation proof challenge.
        let sequence_number = self.txn_options.sequence_number(current_address).await?;
        let auth_key = self.txn_options.auth_key(current_address).await?;
        let rotation_proof = RotationProofChallenge {
            account_address: CORE_CODE_ADDRESS,
            module_name: "account".to_string(),
            struct_name: "RotationProofChallenge".to_string(),
            sequence_number,
            originator: current_address,
            current_auth_key: AccountAddress::from_bytes(auth_key)
                .map_err(|err| CliError::UnableToParse("auth_key", err.to_string()))?,
            new_public_key: new_public_key.to_bytes().to_vec(),
        };
        let rotation_msg =
            bcs::to_bytes(&rotation_proof).map_err(|err| CliError::BCS("rotation_proof", err))?;

        // Sign the struct using both the current private key and the new private key.
        let rotation_proof_signed_by_current_private_key = if current_derivation_path.is_some() {
            aptos_ledger::sign_message(
                current_derivation_path.clone().unwrap().as_str(),
                &rotation_msg.clone(),
            )?
        } else {
            current_private_key
                .unwrap()
                .sign_arbitrary_message(&rotation_msg.clone())
        };
        let rotation_proof_signed_by_new_private_key = if new_derivation_path.is_some() {
            aptos_ledger::sign_message(
                new_derivation_path.clone().unwrap().as_str(),
                &rotation_msg.clone(),
            )?
        } else {
            new_private_key
                .clone()
                .unwrap()
                .sign_arbitrary_message(&rotation_msg.clone())
        };

        // Submit transaction.
        let txn_summary = self
            .txn_options
            .submit_transaction(aptos_stdlib::account_rotate_authentication_key(
                0,
                current_public_key.to_bytes().to_vec(),
                0,
                new_public_key.to_bytes().to_vec(),
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

        if self.skip_saving_profiles {
            return Ok(RotateSummary {
                transaction: txn_summary,
                message: None,
            });
        }

        // If no config exists, then the error should've been caught earlier during the profile
        // name verification step.
        let mut config = CliConfig::load(ConfigSearchMode::CurrentDirAndParents)?;
        if config.profiles.is_none() {
            config.profiles = Some(BTreeMap::new());
        }

        // Create new config.
        let mut new_profile_config = ProfileConfig {
            public_key: Some(new_public_key),
            account: Some(current_address),
            private_key: new_private_key,
            derivation_path: new_derivation_path,
            ..self.txn_options.profile_options.profile()?
        };

        if let Some(url) = self.txn_options.rest_options.url {
            new_profile_config.rest_url = Some(url.into());
        }

        config
            .profiles
            .as_mut()
            .unwrap()
            .insert(new_profile_name.clone(), new_profile_config);
        config.save()?;

        eprintln!("Profile {} is saved.", new_profile_name);

        // Update this
        Ok(RotateSummary {
            transaction: txn_summary,
            message: Some(format!(
                "New profile {} is saved, stale profile {} renamed to {}.",
                new_profile_name
            )),
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
