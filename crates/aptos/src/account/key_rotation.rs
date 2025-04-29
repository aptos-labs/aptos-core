// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    account_address_from_auth_key, account_address_from_public_key, AuthenticationKeyInputOptions,
    CliCommand, CliConfig, CliError, CliTypedResult, ConfigSearchMode, EncodingOptions,
    ExtractEd25519PublicKey, HardwareWalletOptions, ParseEd25519PrivateKey, ProfileConfig,
    ProfileOptions, PublicKeyInputOptions, RestOptions, TransactionOptions, TransactionSummary,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    encoding_type::EncodingType,
    PrivateKey, SigningKey,
};
use aptos_ledger;
use aptos_rest_client::{error::RestError, Client};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{RotationProofChallenge, CORE_CODE_ADDRESS},
    transaction::authenticator::AuthenticationKey,
};
use async_trait::async_trait;
use clap::{Args, Parser};
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

    #[clap(flatten)]
    pub(crate) new_auth_key_options: NewAuthKeyOptions,

    #[clap(flatten)]
    pub(crate) new_profile_options: NewProfileOptions,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub(crate) struct NewAuthKeyOptions {
    /// File name that contains the new private key encoded in the type from `--encoding`
    #[clap(long, value_parser)]
    pub(crate) new_private_key_file: Option<PathBuf>,

    /// New private key encoded in the type from `--encoding`
    #[clap(long)]
    pub(crate) new_private_key: Option<String>,

    /// BIP44 derivation path of hardware wallet account, e.g. `m/44'/637'/0'/0'/0'`
    ///
    /// Note you may need to escape single quotes in your shell, for example
    /// `m/44'/637'/0'/0'/0'` would be `m/44\'/637\'/0\'/0\'/0\'`
    #[clap(long)]
    pub(crate) new_derivation_path: Option<String>,

    /// BIP44 account index of hardware wallet account, e.g. `0`
    ///
    /// Given index `n` maps to BIP44 derivation path `m/44'/637'/n'/0'/0`
    #[clap(long)]
    pub(crate) new_derivation_index: Option<String>,
}

#[derive(Args, Debug)]
#[group(required = true, multiple = false)]
pub(crate) struct NewProfileOptions {
    /// Only specify if you do not want to save a new profile
    #[clap(long)]
    pub(crate) skip_saving_profile: bool,

    /// Name of new the profile to save for the new authentication key
    #[clap(long)]
    pub(crate) save_to_profile: Option<String>,
}

impl ParseEd25519PrivateKey for RotateKey {}

impl RotateKey {
    /// Extract private key from CLI args
    pub fn extract_private_key(
        &self,
        encoding: EncodingType,
    ) -> CliTypedResult<Option<Ed25519PrivateKey>> {
        self.parse_private_key(
            encoding,
            self.new_auth_key_options.new_private_key_file.clone(),
            self.new_auth_key_options.new_private_key.clone(),
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
        // Verify profile name before executing rotation operation, to avoid erroring out in a
        // manner that results in corrupted config state.
        if let Some(ref new_profile_name) = self.new_profile_options.save_to_profile {
            if new_profile_name.is_empty() {
                return Err(CliError::CommandArgumentError(
                    "New profile name may not be empty".to_string(),
                ));
            };

            // Verify that config exists by attempting to load it.
            let config = CliConfig::load(ConfigSearchMode::CurrentDirAndParents)?;

            // Verify that the new profile name does not already exist in the config.
            if let Some(profiles) = config.profiles {
                if profiles.contains_key(new_profile_name) {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} already exists",
                        new_profile_name
                    )));
                };
            }
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
        let new_hardware_wallet_options = HardwareWalletOptions {
            derivation_path: self.new_auth_key_options.new_derivation_path.clone(),
            derivation_index: self.new_auth_key_options.new_derivation_index.clone(),
        };
        let new_derivation_path = new_hardware_wallet_options.extract_derivation_path()?;
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

        // Determine if current and new keys are hardware wallets, for better user feedback.
        let current_is_hardware_wallet = current_derivation_path.is_some();
        let new_is_hardware_wallet = new_derivation_path.is_some();

        // Sign the struct using both the current private key and the new private key.
        let rotation_proof_signed_by_current_private_key =
            if let Some(current_derivation_path) = current_derivation_path.clone() {
                eprintln!("Sign rotation proof challenge on your Ledger device (current key)");
                let challenge_signature = aptos_ledger::sign_message(
                    current_derivation_path.as_str(),
                    &rotation_msg.clone(),
                )?;
                eprintln!("Rotation proof challenge successfully signed (current key)");
                if !new_is_hardware_wallet {
                    eprintln!("You will still need to sign the transaction on your Ledger device");
                }
                challenge_signature
            } else {
                current_private_key
                    .unwrap()
                    .sign_arbitrary_message(&rotation_msg.clone())
            };
        let rotation_proof_signed_by_new_private_key =
            if let Some(new_derivation_path) = new_derivation_path.clone() {
                eprintln!("Sign rotation proof challenge on your Ledger device (new key)");
                let challenge_signature = aptos_ledger::sign_message(
                    new_derivation_path.clone().as_str(),
                    &rotation_msg.clone(),
                )?;
                eprintln!("Rotation proof challenge successfully signed (new key)");
                if current_is_hardware_wallet {
                    eprintln!("You will still need to sign the transaction on your Ledger device");
                }
                challenge_signature
            } else {
                new_private_key
                    .clone()
                    .unwrap()
                    .sign_arbitrary_message(&rotation_msg.clone())
            };

        // Submit transaction.
        if current_derivation_path.is_some() {
            eprintln!("Approve transaction on your Ledger device");
        };
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

        let txn_string = serde_json::to_string_pretty(&txn_summary)
            .map_err(|err| CliError::UnableToParse("transaction summary", err.to_string()))?;
        eprintln!("{}", txn_string);

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

        if self.new_profile_options.skip_saving_profile {
            return Ok(RotateSummary {
                transaction: txn_summary,
                message: None,
            });
        }

        // Can safe unwrap here since NewProfileOptions arg group requires either that
        // skip_saving_profile is set, or that a new profile name is specified. If a new profile is
        // specified, then it will have already been error checked above.
        let new_profile_name = self.new_profile_options.save_to_profile.unwrap();

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

        Ok(RotateSummary {
            transaction: txn_summary,
            message: Some(format!("Saved new profile {}", new_profile_name)),
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
    Ok(rest_client
        .lookup_address(address_key, must_exist)
        .await?
        .into_inner())
}
