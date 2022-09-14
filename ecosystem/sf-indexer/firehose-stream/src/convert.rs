// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::{
    AccountSignature, DeleteModule, DeleteResource, Ed25519Signature, EntryFunctionId, Event,
    GenesisPayload, MoveAbility, MoveFunction, MoveFunctionGenericTypeParam,
    MoveFunctionVisibility, MoveModule, MoveModuleBytecode, MoveModuleId, MoveScriptBytecode,
    MoveStruct, MoveStructField, MoveStructTag, MoveType, MultiEd25519Signature, ScriptPayload,
    Transaction, TransactionInfo, TransactionPayload, TransactionSignature, WriteSet,
    WriteSetChange,
};
use aptos_bitvec::BitVec;
use aptos_logger::warn;
use aptos_protos::extractor::v1 as extractor;
use aptos_protos::util::timestamp;
use hex;
use move_deps::move_binary_format::file_format::Ability;
use std::time::Duration;

pub fn convert_move_module_id(move_module_id: &MoveModuleId) -> extractor::MoveModuleId {
    extractor::MoveModuleId {
        address: move_module_id.address.to_string(),
        name: move_module_id.name.to_string(),
    }
}

pub fn convert_move_ability(move_ability: &MoveAbility) -> extractor::MoveAbility {
    match move_ability.0 {
        Ability::Copy => extractor::MoveAbility::Copy,
        Ability::Drop => extractor::MoveAbility::Drop,
        Ability::Store => extractor::MoveAbility::Store,
        Ability::Key => extractor::MoveAbility::Key,
    }
}

pub fn convert_move_struct_field(msf: &MoveStructField) -> extractor::MoveStructField {
    extractor::MoveStructField {
        name: msf.name.0.to_string(),
        r#type: Some(convert_move_type(&msf.typ)),
    }
}

pub fn convert_move_struct(move_struct: &MoveStruct) -> extractor::MoveStruct {
    extractor::MoveStruct {
        name: move_struct.name.0.to_string(),
        is_native: move_struct.is_native,
        abilities: move_struct
            .abilities
            .iter()
            .map(|i| convert_move_ability(i) as i32)
            .collect(),
        generic_type_params: vec![],
        fields: move_struct
            .fields
            .iter()
            .map(convert_move_struct_field)
            .collect(),
    }
}

pub fn convert_move_function_visibility(
    visibility: &MoveFunctionVisibility,
) -> extractor::move_function::Visibility {
    match visibility {
        MoveFunctionVisibility::Public => extractor::move_function::Visibility::Public,
        MoveFunctionVisibility::Private => extractor::move_function::Visibility::Private,
        MoveFunctionVisibility::Friend => extractor::move_function::Visibility::Friend,
    }
}

pub fn convert_move_function_generic_type_params(
    mfgtp: &MoveFunctionGenericTypeParam,
) -> extractor::MoveFunctionGenericTypeParam {
    extractor::MoveFunctionGenericTypeParam {
        constraints: mfgtp
            .constraints
            .iter()
            .map(|i| convert_move_ability(i) as i32)
            .collect(),
    }
}

pub fn convert_move_function(move_func: &MoveFunction) -> extractor::MoveFunction {
    extractor::MoveFunction {
        name: move_func.name.0.to_string(),
        visibility: convert_move_function_visibility(&move_func.visibility) as i32,
        is_entry: move_func.is_entry,
        generic_type_params: move_func
            .generic_type_params
            .iter()
            .map(convert_move_function_generic_type_params)
            .collect(),
        params: move_func.params.iter().map(convert_move_type).collect(),
        r#return: move_func.return_.iter().map(convert_move_type).collect(),
    }
}

pub fn convert_move_module(move_module: &MoveModule) -> extractor::MoveModule {
    extractor::MoveModule {
        address: move_module.address.to_string(),
        name: move_module.name.0.to_string(),
        friends: move_module
            .friends
            .iter()
            .map(convert_move_module_id)
            .collect(),
        exposed_functions: move_module
            .exposed_functions
            .iter()
            .map(convert_move_function)
            .collect(),
        structs: move_module
            .structs
            .iter()
            .map(convert_move_struct)
            .collect(),
    }
}

pub fn convert_move_module_bytecode(mmb: &MoveModuleBytecode) -> extractor::MoveModuleBytecode {
    let abi = mmb.clone().try_parse_abi().map_or_else(
        |e| {
            warn!("[fh-stream] Could not decode MoveModuleBytecode ABI: {}", e);
            None
        },
        |mmb| mmb.abi.map(|move_module| convert_move_module(&move_module)),
    );
    extractor::MoveModuleBytecode {
        bytecode: mmb.bytecode.0.clone(),
        abi,
    }
}

pub fn convert_entry_function_id(
    entry_function_id: &EntryFunctionId,
) -> extractor::EntryFunctionId {
    extractor::EntryFunctionId {
        module: Some(convert_move_module_id(&entry_function_id.module)),
        name: entry_function_id.name.to_string(),
    }
}

pub fn convert_transaction_payload(payload: &TransactionPayload) -> extractor::TransactionPayload {
    match payload {
        TransactionPayload::EntryFunctionPayload(sfp) => extractor::TransactionPayload {
            r#type: extractor::transaction_payload::Type::EntryFunctionPayload as i32,
            payload: Some(
                extractor::transaction_payload::Payload::EntryFunctionPayload(
                    extractor::EntryFunctionPayload {
                        function: Some(convert_entry_function_id(&sfp.function)),
                        type_arguments: sfp.type_arguments.iter().map(convert_move_type).collect(),
                        arguments: sfp
                            .arguments
                            .iter()
                            .map(|move_value| move_value.to_string())
                            .collect(),
                    },
                ),
            ),
        },
        TransactionPayload::ScriptPayload(sp) => extractor::TransactionPayload {
            r#type: extractor::transaction_payload::Type::ScriptPayload as i32,
            payload: Some(extractor::transaction_payload::Payload::ScriptPayload(
                convert_script_payload(sp),
            )),
        },
        TransactionPayload::ModuleBundlePayload(mbp) => extractor::TransactionPayload {
            r#type: extractor::transaction_payload::Type::ModuleBundlePayload as i32,
            payload: Some(
                extractor::transaction_payload::Payload::ModuleBundlePayload(
                    extractor::ModuleBundlePayload {
                        modules: mbp
                            .modules
                            .iter()
                            .map(convert_move_module_bytecode)
                            .collect(),
                    },
                ),
            ),
        },
    }
}

#[inline]
pub fn convert_events(events: &[Event]) -> Vec<extractor::Event> {
    events.iter().map(convert_event).collect()
}

pub fn convert_write_set(write_set: &WriteSet) -> extractor::WriteSet {
    let (write_set_type, write_set) = match write_set {
        WriteSet::ScriptWriteSet(sws) => {
            let write_set_type = extractor::write_set::WriteSetType::ScriptWriteSet as i32;

            let write_set =
                extractor::write_set::WriteSet::ScriptWriteSet(extractor::ScriptWriteSet {
                    execute_as: sws.execute_as.to_string(),
                    script: Some(convert_script_payload(&sws.script)),
                });
            (write_set_type, Some(write_set))
        }
        WriteSet::DirectWriteSet(dws) => {
            let write_set_type = extractor::write_set::WriteSetType::DirectWriteSet as i32;

            let write_set =
                extractor::write_set::WriteSet::DirectWriteSet(extractor::DirectWriteSet {
                    write_set_change: convert_write_set_changes(&dws.changes),
                    events: convert_events(&dws.events),
                });
            (write_set_type, Some(write_set))
        }
    };
    extractor::WriteSet {
        write_set_type,
        write_set,
    }
}

pub fn empty_move_type(r#type: extractor::MoveTypes) -> extractor::MoveType {
    extractor::MoveType {
        r#type: r#type as i32,
        content: None,
    }
}

pub fn convert_move_type(move_type: &MoveType) -> extractor::MoveType {
    let r#type = match move_type {
        MoveType::Bool => extractor::MoveTypes::Bool,
        MoveType::U8 => extractor::MoveTypes::U8,
        MoveType::U64 => extractor::MoveTypes::U64,
        MoveType::U128 => extractor::MoveTypes::U128,
        MoveType::Address => extractor::MoveTypes::Address,
        MoveType::Signer => extractor::MoveTypes::Signer,
        MoveType::Vector { .. } => extractor::MoveTypes::Vector,
        MoveType::Struct(_) => extractor::MoveTypes::Struct,
        MoveType::GenericTypeParam { .. } => extractor::MoveTypes::GenericTypeParam,
        MoveType::Reference { .. } => extractor::MoveTypes::Reference,
        MoveType::Unparsable(_) => extractor::MoveTypes::Unparsable,
    };
    let content = match move_type {
        MoveType::Bool => None,
        MoveType::U8 => None,
        MoveType::U64 => None,
        MoveType::U128 => None,
        MoveType::Address => None,
        MoveType::Signer => None,
        MoveType::Vector { items } => Some(extractor::move_type::Content::Vector(Box::from(
            convert_move_type(items),
        ))),
        MoveType::Struct(struct_tag) => Some(extractor::move_type::Content::Struct(
            convert_move_struct_tag(struct_tag),
        )),
        MoveType::GenericTypeParam { index } => Some(
            extractor::move_type::Content::GenericTypeParamIndex((*index) as u32),
        ),
        MoveType::Reference { mutable, to } => Some(extractor::move_type::Content::Reference(
            Box::new(extractor::move_type::ReferenceType {
                mutable: *mutable,
                to: Some(Box::new(convert_move_type(to))),
            }),
        )),
        MoveType::Unparsable(string) => {
            Some(extractor::move_type::Content::Unparsable(string.clone()))
        }
    };
    extractor::MoveType {
        r#type: r#type as i32,
        content,
    }
}

#[inline]
pub fn convert_write_set_changes(changes: &[WriteSetChange]) -> Vec<extractor::WriteSetChange> {
    changes.iter().map(convert_write_set_change).collect()
}

#[inline]
pub fn convert_hex_string_to_bytes(hex_string: &str) -> Vec<u8> {
    hex::decode(hex_string.strip_prefix("0x").unwrap_or(hex_string))
        .unwrap_or_else(|_| panic!("Could not convert '{}' to bytes", hex_string))
}

pub fn convert_move_struct_tag(struct_tag: &MoveStructTag) -> extractor::MoveStructTag {
    extractor::MoveStructTag {
        address: struct_tag.address.to_string(),
        module: struct_tag.module.to_string(),
        name: struct_tag.name.to_string(),
        generic_type_params: struct_tag
            .generic_type_params
            .iter()
            .map(convert_move_type)
            .collect(),
    }
}

pub fn convert_delete_module(delete_module: &DeleteModule) -> extractor::DeleteModule {
    extractor::DeleteModule {
        address: delete_module.address.to_string(),
        state_key_hash: convert_hex_string_to_bytes(&delete_module.state_key_hash),
        module: Some(extractor::MoveModuleId {
            address: delete_module.module.address.to_string(),
            name: delete_module.module.name.to_string(),
        }),
    }
}

pub fn convert_delete_resource(delete_resource: &DeleteResource) -> extractor::DeleteResource {
    extractor::DeleteResource {
        address: delete_resource.address.to_string(),
        state_key_hash: convert_hex_string_to_bytes(&delete_resource.state_key_hash),
        r#type: Some(convert_move_struct_tag(&delete_resource.resource)),
        type_str: delete_resource.resource.to_string(),
    }
}

pub fn convert_write_set_change(change: &WriteSetChange) -> extractor::WriteSetChange {
    match change {
        WriteSetChange::DeleteModule(delete_module) => extractor::WriteSetChange {
            r#type: extractor::write_set_change::Type::DeleteModule as i32,
            change: Some(extractor::write_set_change::Change::DeleteModule(
                convert_delete_module(delete_module),
            )),
        },
        WriteSetChange::DeleteResource(delete_resource) => extractor::WriteSetChange {
            r#type: extractor::write_set_change::Type::DeleteResource as i32,
            change: Some(extractor::write_set_change::Change::DeleteResource(
                convert_delete_resource(delete_resource),
            )),
        },
        WriteSetChange::DeleteTableItem(delete_table_item) => {
            let data = delete_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "Could not extract data from DeletedTableItem '{:?}'",
                    delete_table_item
                )
            });

            extractor::WriteSetChange {
                r#type: extractor::write_set_change::Type::DeleteTableItem as i32,
                change: Some(extractor::write_set_change::Change::DeleteTableItem(
                    extractor::DeleteTableItem {
                        state_key_hash: convert_hex_string_to_bytes(
                            &delete_table_item.state_key_hash,
                        ),
                        handle: delete_table_item.handle.to_string(),
                        key: delete_table_item.key.to_string(),
                        data: Some(extractor::DeleteTableData {
                            key: data.key.to_string(),
                            key_type: data.key_type.clone(),
                        }),
                    },
                )),
            }
        }
        WriteSetChange::WriteModule(write_module) => extractor::WriteSetChange {
            r#type: extractor::write_set_change::Type::WriteModule as i32,
            change: Some(extractor::write_set_change::Change::WriteModule(
                extractor::WriteModule {
                    address: write_module.address.to_string(),
                    state_key_hash: convert_hex_string_to_bytes(&write_module.state_key_hash),
                    data: Some(convert_move_module_bytecode(&write_module.data)),
                },
            )),
        },
        WriteSetChange::WriteResource(write_resource) => extractor::WriteSetChange {
            r#type: extractor::write_set_change::Type::WriteResource as i32,
            change: Some(extractor::write_set_change::Change::WriteResource(
                extractor::WriteResource {
                    address: write_resource.address.to_string(),
                    state_key_hash: convert_hex_string_to_bytes(&write_resource.state_key_hash),
                    r#type: Some(convert_move_struct_tag(&write_resource.data.typ)),
                    type_str: write_resource.data.typ.to_string(),
                    data: serde_json::to_string(&write_resource.data).unwrap_or_else(|_| {
                        panic!(
                            "Could not convert move_resource data to json '{:?}'",
                            write_resource.data
                        )
                    }),
                },
            )),
        },
        WriteSetChange::WriteTableItem(write_table_item) => {
            let data = write_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "Could not extract data from DecodedTableData '{:?}'",
                    write_table_item
                )
            });
            extractor::WriteSetChange {
                r#type: extractor::write_set_change::Type::WriteTableItem as i32,
                change: Some(extractor::write_set_change::Change::WriteTableItem(
                    extractor::WriteTableItem {
                        state_key_hash: convert_hex_string_to_bytes(
                            &write_table_item.state_key_hash,
                        ),
                        handle: write_table_item.handle.to_string(),
                        key: write_table_item.key.to_string(),
                        data: Some(extractor::WriteTableData {
                            key: data.key.to_string(),
                            key_type: data.key_type.clone(),
                            value: data.value.to_string(),
                            value_type: data.value_type.clone(),
                        }),
                    },
                )),
            }
        }
    }
}

pub fn convert_move_script_bytecode(msb: &MoveScriptBytecode) -> extractor::MoveScriptBytecode {
    let abi = msb
        .clone()
        .try_parse_abi()
        .abi
        .map(|move_func| convert_move_function(&move_func));

    extractor::MoveScriptBytecode {
        bytecode: msb.bytecode.0.clone(),
        abi,
    }
}

pub fn convert_script_payload(script_payload: &ScriptPayload) -> extractor::ScriptPayload {
    extractor::ScriptPayload {
        code: Some(convert_move_script_bytecode(&script_payload.code)),
        type_arguments: script_payload
            .type_arguments
            .iter()
            .map(convert_move_type)
            .collect(),
        arguments: script_payload
            .arguments
            .iter()
            .map(|move_value| move_value.to_string())
            .collect(),
    }
}

pub fn convert_event(event: &Event) -> extractor::Event {
    let event_key: aptos_types::event::EventKey = event.guid.into();
    extractor::Event {
        key: Some(extractor::EventKey {
            creation_number: event_key.get_creation_number(),
            account_address: event_key.get_creator_address().to_string(),
        }),
        sequence_number: event.sequence_number.0,
        r#type: Some(convert_move_type(&event.typ)),
        type_str: event.typ.to_string(),
        data: event.data.to_string(),
    }
}

pub fn convert_timestamp_secs(timestamp: u64) -> timestamp::Timestamp {
    timestamp::Timestamp {
        seconds: timestamp as i64,
        nanos: 0,
    }
}

pub fn convert_timestamp_usecs(timestamp: u64) -> timestamp::Timestamp {
    let ts = Duration::from_nanos(timestamp * 1000);
    timestamp::Timestamp {
        seconds: ts.as_secs() as i64,
        nanos: ts.subsec_nanos() as i32,
    }
}

pub fn convert_transaction_info(transaction_info: &TransactionInfo) -> extractor::TransactionInfo {
    extractor::TransactionInfo {
        hash: transaction_info.hash.0.to_vec(),
        state_checkpoint_hash: transaction_info
            .state_checkpoint_hash
            .map(|hash| hash.0.to_vec()),
        state_change_hash: transaction_info.state_change_hash.0.to_vec(),
        event_root_hash: transaction_info.event_root_hash.0.to_vec(),
        gas_used: transaction_info.gas_used.0,
        success: transaction_info.success,
        vm_status: transaction_info.vm_status.to_string(),
        accumulator_root_hash: transaction_info.accumulator_root_hash.0.to_vec(),
        changes: convert_write_set_changes(&transaction_info.changes),
    }
}

pub fn convert_ed25519_signature(sig: &Ed25519Signature) -> extractor::Ed25519Signature {
    extractor::Ed25519Signature {
        public_key: sig.public_key.0.clone(),
        signature: sig.signature.0.clone(),
    }
}

pub fn convert_multi_ed25519_signature(
    sig: &MultiEd25519Signature,
) -> extractor::MultiEd25519Signature {
    let public_key_indices: Vec<usize> = BitVec::from(sig.bitmap.0.clone()).iter_ones().collect();
    extractor::MultiEd25519Signature {
        public_keys: sig.public_keys.iter().map(|pk| pk.0.clone()).collect(),
        signatures: sig.signatures.iter().map(|sig| sig.0.clone()).collect(),
        threshold: sig.threshold as u32,
        public_key_indices: public_key_indices
            .iter()
            .map(|index| *index as u32)
            .collect(),
    }
}

pub fn convert_account_signature(
    account_signature: &AccountSignature,
) -> extractor::AccountSignature {
    let r#type = match account_signature {
        AccountSignature::Ed25519Signature(_) => extractor::account_signature::Type::Ed25519,
        AccountSignature::MultiEd25519Signature(_) => {
            extractor::account_signature::Type::MultiEd25519
        }
    };
    let signature = match account_signature {
        AccountSignature::Ed25519Signature(s) => {
            extractor::account_signature::Signature::Ed25519(convert_ed25519_signature(s))
        }
        AccountSignature::MultiEd25519Signature(s) => {
            extractor::account_signature::Signature::MultiEd25519(convert_multi_ed25519_signature(
                s,
            ))
        }
    };
    extractor::AccountSignature {
        r#type: r#type as i32,
        signature: Some(signature),
    }
}

pub fn convert_transaction_signature(
    signature: &Option<TransactionSignature>,
) -> Option<extractor::Signature> {
    let signature = match signature {
        None => return None,
        Some(s) => s,
    };
    let r#type = match signature {
        TransactionSignature::Ed25519Signature(_) => extractor::signature::Type::Ed25519,
        TransactionSignature::MultiEd25519Signature(_) => extractor::signature::Type::MultiEd25519,
        TransactionSignature::MultiAgentSignature(_) => extractor::signature::Type::MultiAgent,
    };

    let signature = match signature {
        TransactionSignature::Ed25519Signature(s) => {
            extractor::signature::Signature::Ed25519(convert_ed25519_signature(s))
        }
        TransactionSignature::MultiEd25519Signature(s) => {
            extractor::signature::Signature::MultiEd25519(convert_multi_ed25519_signature(s))
        }
        TransactionSignature::MultiAgentSignature(s) => {
            extractor::signature::Signature::MultiAgent(extractor::MultiAgentSignature {
                sender: Some(convert_account_signature(&s.sender)),
                secondary_signer_addresses: s
                    .secondary_signer_addresses
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                secondary_signers: s
                    .secondary_signers
                    .iter()
                    .map(convert_account_signature)
                    .collect(),
            })
        }
    };

    Some(extractor::Signature {
        r#type: r#type as i32,
        signature: Some(signature),
    })
}

pub fn convert_transaction(
    transaction: &Transaction,
    block_height: u64,
    current_epoch: u64,
) -> extractor::Transaction {
    let mut timestamp: Option<timestamp::Timestamp> = None;

    let txn_type = match transaction {
        Transaction::UserTransaction(_) => extractor::transaction::TransactionType::User,
        Transaction::GenesisTransaction(_) => extractor::transaction::TransactionType::Genesis,
        Transaction::BlockMetadataTransaction(_) => {
            extractor::transaction::TransactionType::BlockMetadata
        }
        Transaction::StateCheckpointTransaction(_) => {
            extractor::transaction::TransactionType::StateCheckpoint
        }
        Transaction::PendingTransaction(_) => panic!("PendingTransaction is not supported"),
    };

    let txn_data = match &transaction {
        Transaction::UserTransaction(ut) => {
            timestamp = Some(convert_timestamp_usecs(ut.timestamp.0));
            let expiration_timestamp_secs = Some(convert_timestamp_secs(std::cmp::min(
                ut.request.expiration_timestamp_secs.0,
                chrono::NaiveDateTime::MAX.timestamp() as u64,
            )));
            extractor::transaction::TxnData::User(extractor::UserTransaction {
                request: Some(extractor::UserTransactionRequest {
                    sender: ut.request.sender.to_string(),
                    sequence_number: ut.request.sequence_number.0,
                    max_gas_amount: ut.request.max_gas_amount.0,
                    gas_unit_price: ut.request.gas_unit_price.0,
                    expiration_timestamp_secs,
                    payload: Some(convert_transaction_payload(&ut.request.payload)),
                    signature: convert_transaction_signature(&ut.request.signature),
                }),
                events: convert_events(&ut.events),
            })
        }
        Transaction::GenesisTransaction(gt) => {
            let payload = match &gt.payload {
                GenesisPayload::WriteSetPayload(wsp) => convert_write_set(&wsp.write_set),
            };
            extractor::transaction::TxnData::Genesis(extractor::GenesisTransaction {
                payload: Some(payload),
                events: convert_events(&gt.events),
            })
        }
        Transaction::BlockMetadataTransaction(bm) => {
            timestamp = Some(convert_timestamp_usecs(bm.timestamp.0));
            extractor::transaction::TxnData::BlockMetadata(extractor::BlockMetadataTransaction {
                id: bm.id.to_string(),
                events: convert_events(&bm.events),
                previous_block_votes_bitvec: bm.previous_block_votes_bitvec.clone(),
                proposer: bm.proposer.to_string(),
                failed_proposer_indices: bm.failed_proposer_indices.clone(),
                round: bm.round.0,
            })
        }
        Transaction::StateCheckpointTransaction(_st) => {
            extractor::transaction::TxnData::StateCheckpoint(
                extractor::StateCheckpointTransaction {},
            )
        }
        Transaction::PendingTransaction(_) => panic!("PendingTransaction not supported"),
    };

    extractor::Transaction {
        timestamp: Some(
            timestamp.unwrap_or_else(|| convert_timestamp_usecs(transaction.timestamp())),
        ),
        version: transaction.version().unwrap_or_else(|| {
            panic!(
                "Could not extract version from Transaction '{:?}'",
                transaction
            )
        }),
        info: Some(convert_transaction_info(
            transaction.transaction_info().unwrap_or_else(|_| {
                panic!(
                    "Could not extract transaction_info from Transaction '{:?}'",
                    transaction
                )
            }),
        )),
        // TODO: keep track of the epoch as we iterate through BlockMetadata
        epoch: current_epoch,
        block_height,
        r#type: txn_type as i32,
        txn_data: Some(txn_data),
    }
}
