// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_protos::{
    block_output::v1::{
        write_set_change_output::Change as ChangeOutput, BlockMetadataTransactionOutput,
        EventKeyOutput, EventOutput, GenesisTransactionOutput, MoveModuleOutput,
        MoveResourceOutput, SignatureOutput, TableItemOutput, TransactionInfoOutput,
        UserTransactionOutput, WriteSetChangeOutput,
    },
    extractor::v1::{
        account_signature::Signature as AccountSignature,
        signature::{Signature, Type as SignatureType},
        transaction::TransactionType,
        transaction_payload::Payload::EntryFunctionPayload,
        write_set_change::{Change as ChangeInput, Type as WriteSetChangeType},
        BlockMetadataTransaction, Ed25519Signature, Event, GenesisTransaction, MultiAgentSignature,
        MultiEd25519Signature, Transaction, TransactionInfo, UserTransaction,
        UserTransactionRequest,
    },
};

type Result<T> = std::result::Result<T, substreams::errors::Error>;

pub fn get_transaction_info_output(
    txn: &Transaction,
    info: &TransactionInfo,
) -> TransactionInfoOutput {
    TransactionInfoOutput {
        hash: info.hash.clone(),
        r#type: get_transaction_type(txn.r#type()),
        version: txn.version,
        state_change_hash: info.state_change_hash.clone(),
        event_root_hash: info.event_root_hash.clone(),
        state_checkpoint_hash: info.state_checkpoint_hash.clone(),
        gas_used: info.gas_used,
        success: info.success,
        epoch: txn.epoch,
        block_height: txn.block_height,
        vm_status: info.vm_status.clone(),
        accumulator_root_hash: info.accumulator_root_hash.clone(),
        timestamp: txn.timestamp.clone(),
    }
}

pub fn get_block_metadata_output(
    bmt: &BlockMetadataTransaction,
    info: &TransactionInfoOutput,
) -> BlockMetadataTransactionOutput {
    BlockMetadataTransactionOutput {
        version: info.version,
        id: bmt.id.clone(),
        round: bmt.round,
        previous_block_votes_bitvec: bmt.previous_block_votes_bitvec.clone(),
        proposer: bmt.proposer.clone(),
        failed_proposer_indices: bmt.failed_proposer_indices.clone(),
        timestamp: info.timestamp.clone(),
        epoch: info.epoch,
    }
}

pub fn get_user_transaction_output(
    user_txn: &UserTransaction,
    info: &TransactionInfoOutput,
) -> Result<UserTransactionOutput> {
    if let Some(user_request) = &user_txn.request {
        let mut signature_type = String::new();
        let mut signatures = Vec::default();
        if let Some(signature) = &user_request.signature {
            signature_type = get_signature_type(signature.r#type());
            if let Some(signature) = &signature.signature {
                signatures = get_signature_outputs(signature, user_request, info)?;
            }
        }
        let mut entry_function_id_str = String::default();
        if let Some(payload) = &user_request.payload {
            if let Some(EntryFunctionPayload(entry_fn_payload)) = &payload.payload {
                let entry_function = entry_fn_payload.function.as_ref().unwrap();
                let module = entry_function.module.as_ref().unwrap();
                entry_function_id_str = format!(
                    "{}::{}::{}",
                    &module.address, &module.name, entry_function.name
                );
            }
        }
        let user_txn_output = UserTransactionOutput {
            version: info.version,
            sender: user_request.sender.clone(),
            sequence_number: user_request.sequence_number,
            max_gas_amount: user_request.max_gas_amount,
            expiration_timestamp_secs: user_request.expiration_timestamp_secs.clone(),
            gas_unit_price: user_request.gas_unit_price,
            timestamp: info.timestamp.clone(),
            parent_signature_type: signature_type,
            signatures,
            payload: serde_json::to_string(&user_request.payload).unwrap_or_default(),
            entry_function_id_str,
        };
        Ok(user_txn_output)
    } else {
        panic!("Transaction info missing from Transaction")
    }
}

pub fn get_genesis_output(genesis_txn: &GenesisTransaction) -> GenesisTransactionOutput {
    GenesisTransactionOutput {
        payload: serde_json::to_string(&genesis_txn.payload).unwrap_or_default(),
    }
}

pub fn get_events_output(
    events: &[Event],
    transaction_info: &TransactionInfoOutput,
) -> Vec<EventOutput> {
    events
        .iter()
        .map(|event| {
            let key = event.key.as_ref().map(|k| EventKeyOutput {
                creation_number: k.creation_number,
                account_address: k.account_address.clone(),
            });
            EventOutput {
                version: transaction_info.version,
                key,
                sequence_number: event.sequence_number,
                r#type: serde_json::to_string(&event.r#type).unwrap_or_default(),
                type_str: event.type_str.clone(),
                data: event.data.clone(),
            }
        })
        .collect()
}

pub fn get_write_set_changes_output(
    transaction_info: &TransactionInfo,
    version: u64,
) -> Vec<WriteSetChangeOutput> {
    let mut wsc_out = Vec::default();
    for (index, wsc) in transaction_info.changes.iter().enumerate() {
        if let Some(c) = &wsc.change {
            let hash = get_state_key_hash(c);
            let change = get_change_output(c, index as u64);
            wsc_out.push(WriteSetChangeOutput {
                version,
                hash,
                r#type: get_write_set_change_type(wsc.r#type()),
                change: Some(change),
            });
        }
    }
    wsc_out
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

pub fn get_write_set_change_type(t: WriteSetChangeType) -> String {
    match t {
        WriteSetChangeType::DeleteModule => String::from("delete_module"),
        WriteSetChangeType::DeleteResource => String::from("delete_resource"),
        WriteSetChangeType::DeleteTableItem => String::from("delete_table_item"),
        WriteSetChangeType::WriteModule => String::from("write_module"),
        WriteSetChangeType::WriteResource => String::from("write_resource"),
        WriteSetChangeType::WriteTableItem => String::from("write_table_item"),
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
    let signer = override_address.unwrap_or(&request.sender);
    SignatureOutput {
        version: info.version,
        signer: signer.clone(),
        is_sender_primary,
        signature_type: get_signature_type(SignatureType::Ed25519),
        public_key: s.public_key.clone(),
        signature: s.signature.clone(),
        threshold: 1,
        public_key_indices: Vec::default(),
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
    let mut signatures = Vec::default();
    let mut signer = &request.sender;
    if let Some(addr) = override_address {
        signer = addr;
    }
    for (index, signature) in s.signatures.iter().enumerate() {
        let public_key = s
            .public_keys
            .get(s.public_key_indices.clone()[index] as usize)
            .unwrap()
            .clone();
        signatures.push(SignatureOutput {
            version: info.version,
            signer: signer.clone(),
            is_sender_primary,
            signature_type: get_signature_type(SignatureType::MultiEd25519),
            public_key,
            signature: signature.clone(),
            threshold: s.threshold,
            public_key_indices: s.public_key_indices.clone(),
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
    let mut signatures = Vec::default();
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
            .expect("Failed to parse index {} for multi agent secondary signers");
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

fn get_change_output(change: &ChangeInput, index: u64) -> ChangeOutput {
    match change {
        ChangeInput::DeleteModule(item) => ChangeOutput::MoveModule(MoveModuleOutput {
            address: item.address.clone(),
            name: item.module.clone().unwrap_or_default().name,
            bytecode: Vec::default(),
            friends: Vec::default(),
            exposed_functions: Vec::default(),
            structs: Vec::default(),
            is_deleted: true,
            wsc_index: index,
        }),
        ChangeInput::WriteModule(item) => {
            let abi = item
                .data
                .clone()
                .unwrap_or_default()
                .abi
                .unwrap_or_default();
            let friends = match abi.friends.iter().map(serde_json::to_string).collect() {
                Ok(res) => res,
                _ => Vec::default(),
            };
            let exposed_functions = match abi
                .exposed_functions
                .iter()
                .map(serde_json::to_string)
                .collect()
            {
                Ok(res) => res,
                _ => Vec::default(),
            };
            let structs = match abi.structs.iter().map(serde_json::to_string).collect() {
                Ok(res) => res,
                _ => Vec::default(),
            };
            ChangeOutput::MoveModule(MoveModuleOutput {
                address: item.address.clone(),
                name: abi.name,
                bytecode: item.data.clone().unwrap_or_default().bytecode,
                friends,
                exposed_functions,
                structs,
                is_deleted: false,
                wsc_index: index,
            })
        }
        ChangeInput::DeleteResource(item) => ChangeOutput::MoveResource(MoveResourceOutput {
            address: item.address.clone(),
            type_str: item.type_str.clone(),
            name: item
                .r#type
                .as_ref()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            module: item
                .r#type
                .as_ref()
                .map(|a| a.module.clone())
                .unwrap_or_default(),
            generic_type_params: Vec::default(),
            data: String::default(),
            is_deleted: true,
            wsc_index: index,
        }),
        ChangeInput::WriteResource(item) => {
            let struct_tag = item.r#type.clone().unwrap_or_default();
            ChangeOutput::MoveResource(MoveResourceOutput {
                address: item.address.clone(),
                module: struct_tag.module.clone(),
                type_str: item.type_str.clone(),
                name: struct_tag.name.clone(),
                generic_type_params: struct_tag
                    .generic_type_params
                    .iter()
                    .map(|param| serde_json::to_string(param).unwrap_or_default())
                    .collect(),
                data: item.data.clone(),
                is_deleted: false,
                wsc_index: index,
            })
        }
        ChangeInput::DeleteTableItem(item) => {
            let data = item.data.clone().unwrap_or_default();
            ChangeOutput::TableItem(TableItemOutput {
                handle: item.handle.clone(),
                key: item.key.clone(),
                decoded_key: data.key,
                key_type: data.key_type,
                decoded_value: String::default(),
                value_type: String::default(),
                is_deleted: true,
                wsc_index: index,
            })
        }
        ChangeInput::WriteTableItem(item) => {
            let data = item.data.clone().unwrap_or_default();
            ChangeOutput::TableItem(TableItemOutput {
                handle: item.handle.clone(),
                key: item.key.clone(),
                decoded_key: data.key,
                key_type: data.key_type,
                decoded_value: data.value,
                value_type: data.value_type,
                is_deleted: false,
                wsc_index: index,
            })
        }
    }
}

fn get_state_key_hash(change: &ChangeInput) -> Vec<u8> {
    match change {
        ChangeInput::DeleteModule(item) => item.state_key_hash.clone(),
        ChangeInput::DeleteResource(item) => item.state_key_hash.clone(),
        ChangeInput::DeleteTableItem(item) => item.state_key_hash.clone(),
        ChangeInput::WriteModule(item) => item.state_key_hash.clone(),
        ChangeInput::WriteResource(item) => item.state_key_hash.clone(),
        ChangeInput::WriteTableItem(item) => item.state_key_hash.clone(),
    }
}
