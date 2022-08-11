// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context, Result};
use aptos_protos::{
    block_output::v1::{
        BlockMetadataTransactionOutput, EventKeyOutput, EventOutput, SignatureOutput,
        TransactionInfoOutput, UserTransactionOutput, WriteSetChangeOutput,
    },
    extractor::v1::{
        account_signature::Signature as AccountSignature,
        signature::{Signature, Type as SignatureType},
        transaction::TransactionType,
        BlockMetadataTransaction, Ed25519Signature, Event, MultiAgentSignature,
        MultiEd25519Signature, Transaction, TransactionInfo, UserTransaction,
        UserTransactionRequest,
    },
};

pub fn get_transaction_info_output(
    txn: &Transaction,
    info: &TransactionInfo,
) -> Result<TransactionInfoOutput> {
    let transaction_info = TransactionInfoOutput {
        hash: info.hash.clone(),
        r#type: get_transaction_type(txn.r#type()),
        version: txn.version,
        state_root_hash: info.state_root_hash.clone(),
        event_root_hash: info.event_root_hash.clone(),
        gas_used: info.gas_used,
        success: info.success,
        epoch: txn.epoch,
        block_height: txn.block_height,
        vm_status: info.vm_status.clone(),
        accumulator_root_hash: info.accumulator_root_hash.clone(),
        timestamp: txn.timestamp.clone(),
    };
    Ok(transaction_info)
}

pub fn get_block_metadata_output(
    bmt: &BlockMetadataTransaction,
    info: &TransactionInfoOutput,
) -> Result<BlockMetadataTransactionOutput> {
    let bmt_output = BlockMetadataTransactionOutput {
        hash: info.hash.clone(),
        id: bmt.id.clone(),
        round: bmt.round,
        previous_block_votes_bitmap: bmt.previous_block_votes.clone(),
        proposer: bmt.proposer.clone(),
        failed_proposer_indices: bmt.failed_proposer_indices.clone(),
        timestamp: info.timestamp.clone(),
        epoch: info.epoch,
    };
    Ok(bmt_output)
}

pub fn get_user_transaction_output(
    user_txn: &UserTransaction,
    info: &TransactionInfoOutput,
) -> Result<UserTransactionOutput> {
    if let Some(user_request) = &user_txn.request {
        let mut signature_type = String::new();
        let mut signatures = vec![];
        if let Some(signature) = &user_request.signature {
            signature_type = get_signature_type(signature.r#type());
            if let Some(signature) = &signature.signature {
                signatures = get_signature_outputs(signature, user_request, info)?;
            }
        }
        let user_txn_output = UserTransactionOutput {
            hash: info.hash.clone(),
            sender: user_request.sender.clone(),
            sequence_number: user_request.sequence_number,
            max_gas_amount: user_request.max_gas_amount,
            expiration_timestamp_secs: user_request.expiration_timestamp_secs.clone(),
            gas_unit_price: user_request.gas_unit_price,
            timestamp: info.timestamp.clone(),
            parent_signature_type: signature_type,
            signatures,
        };
        Ok(user_txn_output)
    } else {
        bail!("Transaction info missing from Transaction")
    }
}

pub fn get_events_output(
    events: &[Event],
    transaction_info: &TransactionInfoOutput,
) -> Result<Vec<EventOutput>> {
    Ok(events
        .iter()
        .map(|event| {
            let key = event.key.as_ref().map(|k| EventKeyOutput {
                creation_number: k.creation_number,
                account_address: k.account_address.clone(),
            });
            EventOutput {
                transaction_hash: transaction_info.hash.clone(),
                key,
                sequence_number: event.sequence_number,
                move_type: String::from(""),
                data: event.data.clone(),
            }
        })
        .collect())
}

pub fn get_write_set_changes_output(_input_txn: &Transaction) -> Result<Vec<WriteSetChangeOutput>> {
    Ok(vec![])
}

pub fn get_transaction_type(t: TransactionType) -> String {
    match t {
        TransactionType::Genesis => String::from("genesis_transaction"),
        TransactionType::BlockMetadata => String::from("block_metadata_transaction"),
        TransactionType::User => String::from("user_transaction"),
        TransactionType::StateCheckpoint => String::from("state_checkpoint_transaction"),
    }
}

pub fn get_signature_type(t: SignatureType) -> String {
    match t {
        SignatureType::Ed25519 => String::from("ed25519_signature"),
        SignatureType::MultiEd25519 => String::from("multi_ed25519_signature"),
        SignatureType::MultiAgent => String::from("multi_agent_signature"),
    }
}

pub fn get_signature_outputs(
    s: &Signature,
    request: &UserTransactionRequest,
    info: &TransactionInfoOutput,
) -> Result<Vec<SignatureOutput>> {
    match s {
        Signature::Ed25519(sig) => Ok(vec![parse_single_signature(
            sig, request, info, true, 0, None,
        )]),
        Signature::MultiEd25519(sig) => {
            Ok(parse_multi_signature(sig, request, info, true, 0, None))
        }
        Signature::MultiAgent(sig) => parse_multi_agent_signature(sig, request, info),
    }
}

fn parse_single_signature(
    s: &Ed25519Signature,
    request: &UserTransactionRequest,
    info: &TransactionInfoOutput,
    is_sender_primary: bool,
    multi_agent_index: u32,
    override_address: Option<&String>,
) -> SignatureOutput {
    let signer = override_address.unwrap_or_else(|| &request.sender);
    SignatureOutput {
        transaction_hash: info.hash.clone(),
        signer: signer.clone(),
        is_sender_primary,
        signature_type: get_signature_type(SignatureType::Ed25519),
        public_key: s.public_key.clone(),
        signature: s.signature.clone(),
        threshold: 1,
        bitmap: vec![],
        multi_agent_index,
        multi_sig_index: 0,
    }
}

fn parse_multi_signature(
    s: &MultiEd25519Signature,
    request: &UserTransactionRequest,
    info: &TransactionInfoOutput,
    is_sender_primary: bool,
    multi_agent_index: u32,
    override_address: Option<&String>,
) -> Vec<SignatureOutput> {
    let mut signatures = vec![];
    let mut signer = &request.sender;
    if let Some(addr) = override_address {
        signer = addr;
    }
    for (index, key) in s.public_keys.iter().enumerate() {
        let signature = s.signatures.get(index).unwrap();
        signatures.push(SignatureOutput {
            transaction_hash: info.hash.clone(),
            signer: signer.clone(),
            is_sender_primary,
            signature_type: get_signature_type(SignatureType::MultiEd25519),
            public_key: key.clone(),
            signature: signature.clone(),
            threshold: s.threshold,
            bitmap: s.bitmap.clone(),
            multi_agent_index,
            multi_sig_index: index as u32,
        });
    }
    signatures
}

fn parse_multi_agent_signature(
    s: &MultiAgentSignature,
    request: &UserTransactionRequest,
    info: &TransactionInfoOutput,
) -> Result<Vec<SignatureOutput>> {
    let mut signatures = vec![];
    // process sender signature
    if let Some(signature) = &s.sender {
        match &signature.signature {
            None => {}
            Some(sender_sig) => {
                signatures.append(&mut parse_multi_agent_signature_helper(
                    sender_sig, request, info, true, 0, None,
                ));
            }
        }
    }
    for (index, address) in s.secondary_signer_addresses.iter().enumerate() {
        let secondary_sig = s
            .secondary_signers
            .get(index)
            .unwrap()
            .signature
            .as_ref()
            .context("Failed to parse index {} for multi agent secondary signers")?;
        signatures.append(&mut parse_multi_agent_signature_helper(
            secondary_sig,
            request,
            info,
            false,
            index as u32,
            Some(address),
        ));
    }
    Ok(signatures)
}

fn parse_multi_agent_signature_helper(
    s: &AccountSignature,
    request: &UserTransactionRequest,
    info: &TransactionInfoOutput,
    is_sender_primary: bool,
    multi_agent_index: u32,
    override_address: Option<&String>,
) -> Vec<SignatureOutput> {
    match s {
        AccountSignature::Ed25519(sig) => vec![parse_single_signature(
            sig,
            request,
            info,
            is_sender_primary,
            multi_agent_index,
            override_address,
        )],
        AccountSignature::MultiEd25519(sig) => parse_multi_signature(
            sig,
            request,
            info,
            is_sender_primary,
            multi_agent_index,
            override_address,
        ),
    }
}
