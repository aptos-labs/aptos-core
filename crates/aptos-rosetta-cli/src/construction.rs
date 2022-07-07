// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, NetworkArgs, UrlArgs};
use anyhow::anyhow;
use aptos::common::types::{EncodingOptions, PrivateKeyInputOptions, ProfileOptions};
use aptos_crypto::{
    ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, ValidCryptoMaterialStringExt,
};
use aptos_logger::info;
use aptos_rosetta::{
    client::RosettaClient,
    common::native_coin,
    types::{
        AccountIdentifier, Amount, ConstructionCombineRequest, ConstructionDeriveRequest,
        ConstructionDeriveResponse, ConstructionMetadata, ConstructionMetadataRequest,
        ConstructionMetadataResponse, ConstructionParseRequest, ConstructionPayloadsRequest,
        ConstructionPayloadsResponse, ConstructionPreprocessRequest, ConstructionSubmitRequest,
        NetworkIdentifier, Operation, PublicKey, Signature, SignatureType, TransactionIdentifier,
    },
};
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};
use std::{collections::HashMap, convert::TryInto};

/// Construction commands
///
/// At a high level, this provides the full E2E commands provided by the construction API for
/// Rosetta.  This can be used for testing to ensure everything works properly
#[derive(Debug, Subcommand)]
pub enum ConstructionCommand {
    CreateAccount(CreateAccountCommand),
    Transfer(TransferCommand),
}

impl ConstructionCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        use ConstructionCommand::*;
        match self {
            CreateAccount(inner) => format_output(inner.execute().await),
            Transfer(inner) => format_output(inner.execute().await),
        }
    }
}

/// Creates an account using Rosetta, no funds will be transferred
///
/// EncodingOptions are here so we can allow using the BCS encoded mint key
#[derive(Debug, Parser)]
pub struct CreateAccountCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    sender: Option<AccountAddress>,
    /// The new account (TODO: Maybe we want to take in the public key instead)
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    new_account: AccountAddress,
}

impl CreateAccountCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Create account: {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        let sender = get_account_address(
            &client,
            network_identifier.clone(),
            &private_key,
            self.sender,
        )
        .await?;
        let mut keys = HashMap::new();
        keys.insert(sender, private_key);

        // A create account transaction is just a Create account operation
        let operations = vec![Operation::create_account(0, None, self.new_account, sender)];

        submit_operations(&client, network_identifier, &keys, operations).await
    }
}

/// Transfer coins via Rosetta
///
/// Only the native coin is allowed for now
#[derive(Debug, Parser)]
pub struct TransferCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    sender: Option<AccountAddress>,
    /// The receiving account
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    receiver: AccountAddress,
    /// The amount of coins to send
    #[clap(long)]
    amount: u64,
}

impl TransferCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Transfer {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;
        let sender = get_account_address(
            &client,
            network_identifier.clone(),
            &private_key,
            self.sender,
        )
        .await?;
        let mut keys = HashMap::new();
        keys.insert(sender, private_key);

        // A transfer operation is made up of a withdraw and a deposit
        let operations = vec![
            Operation::withdraw(0, None, sender, native_coin(), self.amount),
            Operation::deposit(1, None, self.receiver, native_coin(), self.amount),
        ];

        submit_operations(&client, network_identifier, &keys, operations).await
    }
}

/// Retrieves the account address from the derivation path if there isn't an overriding account specified
async fn get_account_address(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    private_key: &Ed25519PrivateKey,
    maybe_sender: Option<AccountAddress>,
) -> anyhow::Result<AccountAddress> {
    if let Some(sender) = maybe_sender {
        Ok(sender)
    } else {
        Ok(derive_account(
            client,
            network_identifier,
            private_key.public_key().try_into()?,
        )
        .await?
        .account_address()?)
    }
}

/// Submits the operations to the blockchain
async fn submit_operations(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    keys: &HashMap<AccountAddress, Ed25519PrivateKey>,
    operations: Vec<Operation>,
) -> anyhow::Result<TransactionIdentifier> {
    // Retrieve txn metadata
    let (metadata, public_keys) = metadata(
        client,
        network_identifier.clone(),
        operations.clone(),
        10000,
        1,
        keys,
    )
    .await?;

    // Build the transaction, sign it, and submit it
    let response = unsigned_transaction(
        client,
        network_identifier.clone(),
        operations.clone(),
        metadata.metadata,
        public_keys,
    )
    .await?;
    let signed_txn = sign_transaction(
        client,
        network_identifier.clone(),
        keys,
        response,
        operations,
    )
    .await?;
    submit_transaction(client, network_identifier, signed_txn).await
}

/// Derives an [`AccountAddress`] from the [`PublicKey`]
async fn derive_account(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    public_key: PublicKey,
) -> anyhow::Result<AccountIdentifier> {
    if let ConstructionDeriveResponse {
        account_identifier: Some(account_id),
    } = client
        .derive(&ConstructionDeriveRequest {
            network_identifier,
            public_key,
        })
        .await?
    {
        Ok(account_id)
    } else {
        return Err(anyhow!("Failed to find account address for key"));
    }
}

/// Retrieves the metadata for the set of operations
async fn metadata(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    operations: Vec<Operation>,
    max_fee: u64,
    fee_multiplier: u32,
    keys: &HashMap<AccountAddress, Ed25519PrivateKey>,
) -> anyhow::Result<(ConstructionMetadataResponse, Vec<PublicKey>)> {
    // Request the given operation with the given gas constraints
    let amount = val_to_amount(max_fee, false);
    let preprocess_response = client
        .preprocess(&ConstructionPreprocessRequest {
            network_identifier: network_identifier.clone(),
            operations,
            max_fee: Some(vec![amount]),
            suggested_fee_multiplier: Some(fee_multiplier as f64),
        })
        .await?;

    // Process the required public keys
    let mut public_keys = Vec::new();
    if let Some(accounts) = preprocess_response.required_public_keys {
        for account in accounts {
            if let Some(key) = keys.get(&account.account_address()?) {
                public_keys.push(key.public_key().try_into()?);
            } else {
                return Err(anyhow!("No public key found for account"));
            }
        }
    } else {
        return Err(anyhow!("No public keys found required for transaction"));
    };

    // Request the metadata
    if let Some(options) = preprocess_response.options {
        client
            .metadata(&ConstructionMetadataRequest {
                network_identifier,
                options,
                public_keys: public_keys.clone(),
            })
            .await
            .map(|response| (response, public_keys))
    } else {
        Err(anyhow!(
            "No metadata options returned from preprocess response"
        ))
    }
}

/// Build an unsigned transaction
async fn unsigned_transaction(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    operations: Vec<Operation>,
    metadata: ConstructionMetadata,
    public_keys: Vec<PublicKey>,
) -> anyhow::Result<ConstructionPayloadsResponse> {
    // Build the unsigned transaction
    let payloads = client
        .payloads(&ConstructionPayloadsRequest {
            network_identifier: network_identifier.clone(),
            operations: operations.clone(),
            metadata: Some(metadata),
            public_keys: Some(public_keys),
        })
        .await?;

    // Verify that we can parse the transaction
    let response = client
        .parse(&ConstructionParseRequest {
            network_identifier,
            signed: false,
            transaction: payloads.unsigned_transaction.clone(),
        })
        .await?;

    if response.account_identifier_signers.is_some() {
        Err(anyhow!("Signers were in the unsigned transaction!"))
    } else if operations != response.operations {
        Err(anyhow!(
            "Operations were not parsed to be the same as input! Expected {:?} Got {:?}",
            operations,
            response.operations
        ))
    } else {
        Ok(payloads)
    }
}

/// Signs a transaction and combines it with an unsigned transaction
async fn sign_transaction(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    keys: &HashMap<AccountAddress, Ed25519PrivateKey>,
    unsigned_response: ConstructionPayloadsResponse,
    operations: Vec<Operation>,
) -> anyhow::Result<String> {
    let mut signatures = Vec::new();
    let mut signers: Vec<AccountIdentifier> = Vec::new();

    // Go through all payloads, and sign them accordingly
    for payload in unsigned_response.payloads {
        // Payloads must have a signer and an associated key
        if let Some(ref account) = payload.account_identifier {
            let address = account.account_address()?;

            if let Some(private_key) = keys.get(&address) {
                // Sign the message
                let signing_bytes = hex::decode(&payload.hex_bytes)?;
                let txn_signature = private_key.sign_arbitrary_message(&signing_bytes);

                signers.push(address.into());
                signatures.push(Signature {
                    signing_payload: payload,
                    public_key: private_key.public_key().try_into()?,
                    signature_type: SignatureType::Ed25519,
                    hex_bytes: txn_signature.to_encoded_string()?,
                })
            } else {
                return Err(anyhow!(
                    "Address in payload doesn't have an associated key {}",
                    address
                ));
            }
        } else {
            return Err(anyhow!("No account in payload to sign!"));
        }
    }

    // Build the signed transaction
    let signed_response = client
        .combine(&ConstructionCombineRequest {
            network_identifier: network_identifier.clone(),
            unsigned_transaction: unsigned_response.unsigned_transaction,
            signatures,
        })
        .await?;

    // Verify transaction can be parsed properly
    let response = client
        .parse(&ConstructionParseRequest {
            network_identifier,
            signed: true,
            transaction: signed_response.signed_transaction.clone(),
        })
        .await?;

    // Signers must match exactly
    if let Some(parsed_signers) = response.account_identifier_signers {
        if signers != parsed_signers {
            return Err(anyhow!(
                "Signers don't match Expected: {:?} Got: {:?}",
                signers,
                parsed_signers
            ));
        }
    } else {
        return Err(anyhow!("Signers were in the unsigned transaction!"));
    }

    // Operations must match exactly
    if operations != response.operations {
        Err(anyhow!(
            "Operations were not parsed to be the same as input! Expected {:?} Got {:?}",
            operations,
            response.operations
        ))
    } else {
        Ok(signed_response.signed_transaction)
    }
}

/// Submit a transaction to the blockchain
async fn submit_transaction(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    signed_transaction: String,
) -> anyhow::Result<TransactionIdentifier> {
    Ok(client
        .submit(&ConstructionSubmitRequest {
            network_identifier,
            signed_transaction,
        })
        .await?
        .transaction_identifier)
}

/// Converts a value to a Rosetta [`Amount`]
///
/// Only works with the native coin
fn val_to_amount(amount: u64, withdraw: bool) -> Amount {
    let value = if withdraw {
        format!("-{}", amount)
    } else {
        amount.to_string()
    };
    Amount {
        value,
        currency: native_coin(),
    }
}
