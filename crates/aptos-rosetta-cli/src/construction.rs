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
        NetworkIdentifier, Operation, OperationIdentifier, OperationSpecificMetadata,
        OperationType, PublicKey, Signature, SignatureType, TransactionIdentifier,
    },
};
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};
use std::{collections::HashMap, convert::TryInto};

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
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    sender: Option<AccountAddress>,
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    new_account: AccountAddress,
}

impl CreateAccountCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Create account: {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let new_account = self.new_account.into();
        let private_key = self.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        let sender_account: AccountIdentifier = if let Some(sender) = self.sender {
            sender.into()
        } else {
            derive_account(
                &client,
                network_identifier.clone(),
                private_key.public_key().try_into()?,
            )
            .await?
        };
        let mut keys = HashMap::new();
        keys.insert(sender_account.account_address()?, private_key);

        let operations = vec![Operation {
            operation_identifier: OperationIdentifier {
                index: 0,
                network_index: None,
            },
            related_operations: None,
            operation_type: OperationType::CreateAccount.to_string(),
            status: None,
            account: Some(new_account),
            amount: None,
            metadata: Some(OperationSpecificMetadata {
                sender: sender_account,
            }),
        }];

        submit_operations(&client, network_identifier, &keys, operations).await
    }
}

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
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    sender: Option<AccountAddress>,
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    receiver: AccountAddress,
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
        let account: AccountIdentifier = if let Some(sender) = self.sender {
            sender.into()
        } else {
            derive_account(
                &client,
                network_identifier.clone(),
                private_key.public_key().try_into()?,
            )
            .await?
        };
        let mut keys = HashMap::new();
        keys.insert(account.account_address()?, private_key);

        let operations = vec![
            Operation {
                operation_identifier: OperationIdentifier {
                    index: 0,
                    network_index: None,
                },
                related_operations: None,
                operation_type: OperationType::Withdraw.to_string(),
                status: None,
                account: Some(account),
                amount: Some(val_to_amount(self.amount, true)),
                metadata: None,
            },
            Operation {
                operation_identifier: OperationIdentifier {
                    index: 1,
                    network_index: None,
                },
                related_operations: None,
                operation_type: OperationType::Deposit.to_string(),
                status: None,
                account: Some(self.receiver.into()),
                amount: Some(val_to_amount(self.amount, false)),
                metadata: None,
            },
        ];

        submit_operations(&client, network_identifier, &keys, operations).await
    }
}

async fn submit_operations(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    keys: &HashMap<AccountAddress, Ed25519PrivateKey>,
    operations: Vec<Operation>,
) -> anyhow::Result<TransactionIdentifier> {
    let public_key: PublicKey = keys.iter().next().unwrap().1.public_key().try_into()?;

    let metadata = metadata(
        client,
        network_identifier.clone(),
        operations.clone(),
        10000,
        1,
        public_key.clone(),
    )
    .await?;

    let response = unsigned_transaction(
        client,
        network_identifier.clone(),
        operations,
        metadata.metadata,
        public_key,
    )
    .await?;
    let signed_txn = sign_transaction(client, network_identifier.clone(), keys, response).await?;
    submit_transaction(client, network_identifier, signed_txn).await
}

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

async fn metadata(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    operations: Vec<Operation>,
    max_fee: u64,
    fee_multiplier: u32,
    public_key: PublicKey,
) -> anyhow::Result<ConstructionMetadataResponse> {
    let amount = val_to_amount(max_fee, false);
    let preprocess_response = client
        .preprocess(&ConstructionPreprocessRequest {
            network_identifier: network_identifier.clone(),
            operations,
            max_fee: Some(vec![amount]),
            suggested_fee_multiplier: Some(fee_multiplier as f64),
        })
        .await?;
    client
        .metadata(&ConstructionMetadataRequest {
            network_identifier,
            options: preprocess_response.options.unwrap(),
            public_keys: vec![public_key],
        })
        .await
}

async fn unsigned_transaction(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    operations: Vec<Operation>,
    metadata: ConstructionMetadata,
    public_key: PublicKey,
) -> anyhow::Result<ConstructionPayloadsResponse> {
    let payloads = client
        .payloads(&ConstructionPayloadsRequest {
            network_identifier: network_identifier.clone(),
            operations,
            metadata: Some(metadata),
            public_keys: Some(vec![public_key]),
        })
        .await?;

    // Verify
    client
        .parse(&ConstructionParseRequest {
            network_identifier,
            signed: false,
            transaction: payloads.unsigned_transaction.clone(),
        })
        .await?;

    Ok(payloads)
}

async fn sign_transaction(
    client: &RosettaClient,
    network_identifier: NetworkIdentifier,
    keys: &HashMap<AccountAddress, Ed25519PrivateKey>,
    unsigned_response: ConstructionPayloadsResponse,
) -> anyhow::Result<String> {
    let mut signatures = Vec::new();
    for payload in unsigned_response.payloads {
        if let Some(ref account) = payload.account_identifier {
            let address = account.account_address()?;
            if let Some(private_key) = keys.get(&address) {
                let signing_bytes = hex::decode(&payload.hex_bytes)?;
                let txn_signature = private_key.sign_arbitrary_message(&signing_bytes);
                signatures.push(Signature {
                    signing_payload: payload,
                    public_key: private_key.public_key().try_into()?,
                    signature_type: SignatureType::Ed25519,
                    hex_bytes: txn_signature.to_encoded_string()?,
                })
            } else {
                return Err(anyhow!("Address in payload is unknown {}", address));
            }
        } else {
            return Err(anyhow!("No account in payload to sign!"));
        }
    }

    let signed_response = client
        .combine(&ConstructionCombineRequest {
            network_identifier: network_identifier.clone(),
            unsigned_transaction: unsigned_response.unsigned_transaction,
            signatures,
        })
        .await?;

    // Verify
    client
        .parse(&ConstructionParseRequest {
            network_identifier,
            signed: true,
            transaction: signed_response.signed_transaction.clone(),
        })
        .await?;

    Ok(signed_response.signed_transaction)
}

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
