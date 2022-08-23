// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]

use crate::{models::transactions::Transaction, schema::signatures};
use aptos_protos::block_output::v1::SignatureOutput;
use aptos_rest_client::aptos_api_types::HexEncodedBytes;
use field_count::FieldCount;
use serde::Serialize;

#[derive(
    Associations, Clone, Debug, FieldCount, Identifiable, Insertable, Queryable, Serialize,
)]
#[diesel(table_name = "signatures")]
#[belongs_to(Transaction, foreign_key = "transaction_version")]
#[primary_key(transaction_version, multi_agent_index, multi_sig_index)]
pub struct Signature {
    pub transaction_version: i64,
    pub multi_agent_index: i64,
    pub multi_sig_index: i64,
    pub transaction_block_height: i64,
    pub signer: String,
    pub is_sender_primary: bool,
    #[diesel(column_name = type)]
    pub type_: String,
    pub public_key: String,
    pub threshold: i64,
    pub public_key_indices: serde_json::Value,
    // Default time columns
    pub inserted_at: chrono::NaiveDateTime,
}

impl Signature {
    pub fn from_signature(signature: &SignatureOutput, block_height: u64) -> Self {
        Signature {
            transaction_version: signature.version as i64,
            multi_agent_index: signature.multi_agent_index as i64,
            multi_sig_index: signature.multi_sig_index as i64,
            transaction_block_height: block_height as i64,
            signer: signature.signer.clone(),
            is_sender_primary: signature.is_sender_primary,
            type_: signature.signature_type.clone(),
            public_key: HexEncodedBytes::from(signature.public_key.clone()).to_string(),
            threshold: signature.threshold as i64,
            public_key_indices: serde_json::to_value(signature.public_key_indices.clone()).unwrap(),
            inserted_at: chrono::Utc::now().naive_utc(),
        }
    }

    pub fn from_signatures(signatures: &[SignatureOutput], block_height: u64) -> Vec<Self> {
        signatures
            .iter()
            .map(|sig| Self::from_signature(sig, block_height))
            .collect()
    }
}
