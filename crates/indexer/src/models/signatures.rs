// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
#![allow(clippy::extra_unused_lifetimes)]

use crate::{models::transactions::Transaction, schema::signatures, util::standardize_address};
use anyhow::{Context, Result};
use aptos_api_types::{
    AccountSignature as APIAccountSignature, Ed25519Signature as APIEd25519Signature,
    FeePayerSignature as APIFeePayerSignature, MultiAgentSignature as APIMultiAgentSignature,
    MultiEd25519Signature as APIMultiEd25519Signature, MultiKeySignature as APIMultiKeySignature,
    NoAccountSignature as APINoAccountSignature, SingleKeySignature as APISingleKeySignature,
    TransactionSignature as APITransactionSignature,
};
use aptos_bitvec::BitVec;
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
        s: &APITransactionSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Result<Vec<Self>> {
        match s {
            APITransactionSignature::Ed25519Signature(sig) => {
                Ok(vec![Self::parse_ed25519_signature(
                    sig,
                    sender,
                    transaction_version,
                    transaction_block_height,
                    true,
                    0,
                    None,
                )])
            },
            APITransactionSignature::MultiEd25519Signature(sig) => Ok(Self::parse_multi_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                true,
                0,
                None,
            )),
            APITransactionSignature::MultiAgentSignature(sig) => Self::parse_multi_agent_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
            ),
            APITransactionSignature::FeePayerSignature(sig) => Self::parse_fee_payer_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
            ),
            APITransactionSignature::SingleSender(sig) => {
                Ok(Self::parse_multi_agent_signature_helper(
                    sig,
                    sender,
                    transaction_version,
                    transaction_block_height,
                    true,
                    0,
                    None,
                ))
            },
        }
    }

    pub fn get_signature_type(t: &APITransactionSignature) -> String {
        match t {
            APITransactionSignature::Ed25519Signature(_) => String::from("ed25519_signature"),
            APITransactionSignature::MultiEd25519Signature(_) => {
                String::from("multi_ed25519_signature")
            },
            APITransactionSignature::MultiAgentSignature(_) => {
                String::from("multi_agent_signature")
            },
            APITransactionSignature::FeePayerSignature(_) => String::from("fee_payer_signature"),
            APITransactionSignature::SingleSender(_sig) => String::from("single_sender"),
        }
    }

    fn parse_ed25519_signature(
        s: &APIEd25519Signature,
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
            public_key: s.public_key.to_string(),
            threshold: 1,
            public_key_indices: serde_json::Value::Array(vec![]),
            signature: s.signature.to_string(),
            multi_agent_index,
            multi_sig_index: 0,
        }
    }

    fn parse_multi_signature(
        s: &APIMultiEd25519Signature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
        is_sender_primary: bool,
        multi_agent_index: i64,
        override_address: Option<&String>,
    ) -> Vec<Self> {
        let mut signatures = Vec::default();
        let signer = standardize_address(override_address.unwrap_or(sender));

        let public_key_indices: Vec<usize> = BitVec::from(s.bitmap.0.clone()).iter_ones().collect();
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
                public_key: public_key.to_string(),
                threshold: s.threshold as i64,
                signature: signature.to_string(),
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
        s: &APIMultiAgentSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Result<Vec<Self>> {
        let mut signatures = Vec::default();
        // process sender signature
        signatures.append(&mut Self::parse_multi_agent_signature_helper(
            &s.sender,
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
        s: &APIFeePayerSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
    ) -> Result<Vec<Self>> {
        let mut signatures = Vec::default();
        // process sender signature
        signatures.append(&mut Self::parse_multi_agent_signature_helper(
            &s.sender,
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
            &s.fee_payer_signer,
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
        s: &APIAccountSignature,
        sender: &String,
        transaction_version: i64,
        transaction_block_height: i64,
        is_sender_primary: bool,
        multi_agent_index: i64,
        override_address: Option<&String>,
    ) -> Vec<Self> {
        match s {
            APIAccountSignature::Ed25519Signature(sig) => vec![Self::parse_ed25519_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            )],
            APIAccountSignature::MultiEd25519Signature(sig) => Self::parse_multi_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            ),
            APIAccountSignature::SingleKeySignature(sig) => vec![Self::parse_single_key_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            )],
            APIAccountSignature::MultiKeySignature(sig) => vec![Self::parse_multi_key_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            )],
            APIAccountSignature::NoAccountSignature(sig) => vec![Self::parse_no_account_signature(
                sig,
                sender,
                transaction_version,
                transaction_block_height,
                is_sender_primary,
                multi_agent_index,
                override_address,
            )],
        }
    }

    fn parse_single_key_signature(
        _s: &APISingleKeySignature,
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
            type_: String::from("single_key_signature"),
            public_key: "Not implemented".into(),
            threshold: 1,
            public_key_indices: serde_json::Value::Array(vec![]),
            signature: "Not implemented".into(),
            multi_agent_index,
            multi_sig_index: 0,
        }
    }

    fn parse_multi_key_signature(
        _s: &APIMultiKeySignature,
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
            type_: String::from("multi_key_signature"),
            public_key: "Not implemented".into(),
            threshold: 1,
            public_key_indices: serde_json::Value::Array(vec![]),
            signature: "Not implemented".into(),
            multi_agent_index,
            multi_sig_index: 0,
        }
    }

    fn parse_no_account_signature(
        _s: &APINoAccountSignature,
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
            type_: String::from("no_account_signature"),
            public_key: "Not implemented".into(),
            threshold: 1,
            public_key_indices: serde_json::Value::Array(vec![]),
            signature: "Not implemented".into(),
            multi_agent_index,
            multi_sig_index: 0,
        }
    }
}
