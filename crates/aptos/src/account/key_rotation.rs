// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliError, CliTypedResult, TransactionOptions, TransactionSummary,
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
impl CliCommand<TransactionSummary> for RotateKey {
    fn command_name(&self) -> &'static str {
        "RotateKey"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
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

        self.txn_options
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
            .map(TransactionSummary::from)
    }
}
