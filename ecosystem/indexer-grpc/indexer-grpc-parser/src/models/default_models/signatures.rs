// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::extra_unused_lifetimes)]

use super::transactions::Transaction;
use crate::{schema::signatures, utils::util::standardize_address};
use anyhow::{Context, Result};
use aptos_protos::transaction::v1::{
    account_signature::Signature as AccountSignatureEnum, signature::Signature as SignatureEnum,
    AccountSignature as ProtoAccountSignature, Ed25519Signature as Ed25519SignaturePB,
    FeePayerSignature as ProtoFeePayerSignature, MultiAgentSignature as ProtoMultiAgentSignature,
    MultiEd25519Signature as ProtoMultiEd25519Signature, Signature as TransactionSignaturePB,
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

#[derive(
    Associations, Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize,
)]
#[diesel(belongs_to(Transaction, foreign_key = transaction_version))]
#[diesel(primary_key(
    transaction_version,
    multi_agent_index,
    multi_sig_index,
    is_sender_primary
))]
#[diesel(table_name = signatures)]
pub struct Signature {
    pub transaction_version: i64,
    pub multi_agent_index: i64,
    pub multi_sig_index: i64,
    pub transaction_block_height: i64,
    pub signer: String,
    pub is_sender_primary: bool,
    pub type_: String,
    pub public_key: String,
    pub signature: String,
    pub threshold: i64,
    pub public_key_indices: serde_json::Value,
}

impl Signature {
    /// Returns a flattened list of signatures. If signature is a Ed25519Signature, then return a vector of 1 signature
    pub fn from_user_transaction(
        s: &TransactionSignaturePB,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Result<Vec<Self>> {
        match s.signature.as_ref().unwrap() {
            SignatureEnum::Ed25519(sig) => Ok(vec![Self::parse_single_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                true,
                0,
                None,
            )]),
            SignatureEnum::MultiEd25519(sig) => Ok(Self::parse_multi_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                true,
                0,
                None,
            )),
            SignatureEnum::MultiAgent(sig) => Self::parse_multi_agent_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
            ),
            SignatureEnum::FeePayer(sig) => Self::parse_fee_payer_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
            ),
        }
    }

    pub fn get_signature_type(t: &TransactionSignaturePB) -> String {
        match t.signature.as_ref().unwrap() {
            SignatureEnum::Ed25519(_) => String::from("ed25519_signature"),
            SignatureEnum::MultiEd25519(_) => String::from("multi_ed25519_signature"),
            SignatureEnum::MultiAgent(_) => String::from("multi_agent_signature"),
            SignatureEnum::FeePayer(_) => String::from("fee_payer_signature"),
        }
    }

    fn parse_single_signature(
        s: &Ed25519SignaturePB,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
        is_sender_primary: bool,
        multi_agent_index: i64,
        override_address: Option<&String>,
    ) -> Self {
        let signer = standardize_address(override_address.unwrap_or(sender));
        Self {
            transaction_version,
            transaction_block_height,
            signer,
            is_sender_primary,
            type_: String::from("ed25519_signature"),
            public_key: format!("0x{}", hex::encode(s.public_key.as_slice())),
            threshold: 1,
            public_key_indices: serde_json::Value::Array(vec![]),
            signature: format!("0x{}", hex::encode(s.signature.as_slice())),
            multi_agent_index,
            multi_sig_index: 0,
        }
    }

    fn parse_multi_signature(
        s: &ProtoMultiEd25519Signature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
        is_sender_primary: bool,
        multi_agent_index: i64,
        override_address: Option<&String>,
    ) -> Vec<Self> {
        let mut signatures = Vec::default();
        let signer = standardize_address(override_address.unwrap_or(sender));

        let public_key_indices: Vec<usize> = s
            .public_key_indices
            .iter()
            .map(|index| *index as usize)
            .collect();
        for (index, signature) in s.signatures.iter().enumerate() {
            let public_key = s
                .public_keys
                .get(public_key_indices.clone()[index])
                .unwrap()
                .clone();
            signatures.push(Self {
                transaction_version,
                transaction_block_height,
                signer: signer.clone(),
                is_sender_primary,
                type_: String::from("multi_ed25519_signature"),
                public_key: format!("0x{}", hex::encode(public_key.as_slice())),
                threshold: s.threshold as i64,
                signature: format!("0x{}", hex::encode(signature.as_slice())),
                public_key_indices: serde_json::Value::Array(
                    public_key_indices
                        .iter()
                        .map(|index| {
                            serde_json::Value::Number(serde_json::Number::from(*index as i64))
                        })
                        .collect(),
                ),
                multi_agent_index,
                multi_sig_index: index as i64,
            });
        }
        signatures
    }

    fn parse_multi_agent_signature(
        s: &ProtoMultiAgentSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Result<Vec<Self>> {
        let mut signatures = Vec::default();
        // process sender signature
        signatures.append(&mut Self::parse_multi_agent_signature_helper(
            s.sender.as_ref().unwrap(),
            sender,
            transaction_version,
            transaction_block_height,
            true,
            0,
            None,
        ));
        for (index, address) in s.secondary_signer_addresses.iter().enumerate() {
            let secondary_sig = s.secondary_signers.get(index).context(format!(
                "Failed to parse index {} for multi agent secondary signers",
                index
            ))?;
            signatures.append(&mut Self::parse_multi_agent_signature_helper(
                secondary_sig,
                sender,
                transaction_version,
                transaction_block_height,
                false,
                index as i64,
                Some(&address.to_string()),
            ));
        }
        Ok(signatures)
    }

    fn parse_fee_payer_signature(
        s: &ProtoFeePayerSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Result<Vec<Self>> {
        let mut signatures = Vec::default();
        // process sender signature
        signatures.append(&mut Self::parse_multi_agent_signature_helper(
            s.sender.as_ref().unwrap(),
            sender,
            transaction_version,
            transaction_block_height,
            true,
            0,
            None,
        ));
        for (index, address) in s.secondary_signer_addresses.iter().enumerate() {
            let secondary_sig = s.secondary_signers.get(index).context(format!(
                "Failed to parse index {} for multi agent secondary signers",
                index
            ))?;
            signatures.append(&mut Self::parse_multi_agent_signature_helper(
                secondary_sig,
                sender,
                transaction_version,
                transaction_block_height,
                false,
                index as i64,
                Some(&address.to_string()),
            ));
        }
        signatures.append(&mut Self::parse_multi_agent_signature_helper(
            s.fee_payer_signer.as_ref().unwrap(),
            sender,
            transaction_version,
            transaction_block_height,
            true,
            (s.secondary_signer_addresses.len() + 1) as i64,
            Some(&s.fee_payer_address.to_string()),
        ));
        Ok(signatures)
    }

    fn parse_multi_agent_signature_helper(
        s: &ProtoAccountSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
        is_sender_primary: bool,
        multi_agent_index: i64,
        override_address: Option<&String>,
    ) -> Vec<Self> {
        let signature = s.signature.as_ref().unwrap();
        match signature {
            AccountSignatureEnum::Ed25519(sig) => vec![Self::parse_single_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            )],
            AccountSignatureEnum::MultiEd25519(sig) => Self::parse_multi_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            ),
        }
    }
}
