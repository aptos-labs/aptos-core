// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::protos::extractor;
use aptos_api_types::{
    DeleteModule, DeleteResource, Event, GenesisPayload, HashValue, MoveAbility, MoveFunction,
    MoveFunctionGenericTypeParam, MoveFunctionVisibility, MoveModule, MoveModuleBytecode,
    MoveModuleId, MoveStruct, MoveStructField, MoveStructTag, MoveType, ScriptPayload, Transaction,
    TransactionInfo, TransactionPayload, WriteSet, WriteSetChange,
};
use aptos_logger::warn;
pub use move_deps::move_binary_format::file_format::Ability;
use protobuf::EnumOrUnknown;
use std::{str::FromStr, time::Duration};

pub fn convert_move_module_id(move_module_id: &MoveModuleId) -> extractor::MoveModuleId {
    extractor::MoveModuleId {
        address: move_module_id.address.to_string(),
        name: move_module_id.name.to_string(),
        special_fields: Default::default(),
    }
}

pub fn convert_move_ability(move_ability: &MoveAbility) -> EnumOrUnknown<extractor::MoveAbility> {
    EnumOrUnknown::new(match move_ability.0 {
        Ability::Copy => extractor::MoveAbility::COPY,
        Ability::Drop => extractor::MoveAbility::DROP,
        Ability::Store => extractor::MoveAbility::STORE,
        Ability::Key => extractor::MoveAbility::KEY,
    })
}
pub fn convert_move_struct_field(msf: &MoveStructField) -> extractor::MoveStructField {
    extractor::MoveStructField {
        name: msf.name.0.to_string(),
        type_: protobuf::MessageField::some(convert_move_type(&msf.typ)),
        special_fields: Default::default(),
    }
}

pub fn convert_move_struct(move_struct: &MoveStruct) -> extractor::MoveStruct {
    extractor::MoveStruct {
        name: move_struct.name.0.to_string(),
        is_native: move_struct.is_native,
        abilities: move_struct
            .abilities
            .iter()
            .map(convert_move_ability)
            .collect(),
        generic_type_params: vec![],
        fields: move_struct
            .fields
            .iter()
            .map(convert_move_struct_field)
            .collect(),
        special_fields: Default::default(),
    }
}

pub fn convert_move_function_visibility(
    visibility: &MoveFunctionVisibility,
) -> EnumOrUnknown<extractor::move_function::Visibility> {
    EnumOrUnknown::new(match visibility {
        MoveFunctionVisibility::Public => extractor::move_function::Visibility::PUBLIC,
        MoveFunctionVisibility::Private => extractor::move_function::Visibility::PRIVATE,
        MoveFunctionVisibility::Friend => extractor::move_function::Visibility::FRIEND,
    })
}

pub fn convert_move_function_generic_type_params(
    mfgtp: &MoveFunctionGenericTypeParam,
) -> extractor::MoveFunctionGenericTypeParam {
    extractor::MoveFunctionGenericTypeParam {
        constraints: mfgtp.constraints.iter().map(convert_move_ability).collect(),
        special_fields: Default::default(),
    }
}

pub fn convert_move_function(move_func: &MoveFunction) -> extractor::MoveFunction {
    extractor::MoveFunction {
        name: move_func.name.0.to_string(),
        visibility: convert_move_function_visibility(&move_func.visibility),
        is_entry: move_func.is_entry,
        generic_type_params: move_func
            .generic_type_params
            .iter()
            .map(convert_move_function_generic_type_params)
            .collect(),
        params: move_func.params.iter().map(convert_move_type).collect(),
        return_: move_func.return_.iter().map(convert_move_type).collect(),
        special_fields: Default::default(),
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
        special_fields: Default::default(),
    }
}

pub fn convert_move_module_bytecode(mmb: &MoveModuleBytecode) -> extractor::MoveModuleBytecode {
    let abi = mmb.clone().try_parse_abi().map_or_else(
        |e| {
            warn!("[sf-stream] Could not decode MoveModuleBytecode ABI: {}", e);
            protobuf::MessageField::none()
        },
        |mmb| match mmb.abi {
            None => protobuf::MessageField::none(),
            Some(move_module) => protobuf::MessageField::some(convert_move_module(&move_module)),
        },
    );
    extractor::MoveModuleBytecode {
        bytecode: mmb.bytecode.0.clone(),
        abi,
        special_fields: Default::default(),
    }
}

pub fn convert_transaction_payload(payload: &TransactionPayload) -> extractor::TransactionPayload {
    match payload {
        TransactionPayload::ScriptFunctionPayload(sfp) => extractor::TransactionPayload {
            type_: EnumOrUnknown::new(
                extractor::transaction_payload::PayloadType::SCRIPT_FUNCTION_PAYLOAD,
            ),
            payload: Some(
                extractor::transaction_payload::Payload::ScriptFunctionPayload(
                    extractor::ScriptFunctionPayload {
                        function: protobuf::MessageField::some(extractor::ScriptFunctionId {
                            module: protobuf::MessageField::some(convert_move_module_id(
                                &sfp.function.module,
                            )),
                            name: sfp.function.name.to_string(),
                            special_fields: Default::default(),
                        }),
                        type_arguments: sfp
                            .type_arguments
                            .iter()
                            .map(|move_type| move_type.to_string())
                            .collect(),
                        arguments: sfp
                            .arguments
                            .iter()
                            .map(|move_value| move_value.to_string())
                            .collect(),
                        special_fields: Default::default(),
                    },
                ),
            ),
            special_fields: Default::default(),
        },
        TransactionPayload::ScriptPayload(sp) => extractor::TransactionPayload {
            type_: EnumOrUnknown::new(extractor::transaction_payload::PayloadType::SCRIPT_PAYLOAD),
            payload: Some(extractor::transaction_payload::Payload::ScriptPayload(
                convert_script_payload(sp),
            )),
            special_fields: Default::default(),
        },
        TransactionPayload::ModuleBundlePayload(mbp) => extractor::TransactionPayload {
            type_: EnumOrUnknown::new(
                extractor::transaction_payload::PayloadType::MODULE_BUNDLE_PAYLOAD,
            ),
            payload: Some(
                extractor::transaction_payload::Payload::ModuleBundlePayload(
                    extractor::ModuleBundlePayload {
                        modules: mbp
                            .modules
                            .iter()
                            .map(convert_move_module_bytecode)
                            .collect(),
                        special_fields: Default::default(),
                    },
                ),
            ),
            special_fields: Default::default(),
        },
        TransactionPayload::WriteSetPayload(wsp) => extractor::TransactionPayload {
            type_: EnumOrUnknown::new(
                extractor::transaction_payload::PayloadType::WRITE_SET_PAYLOAD,
            ),
            payload: Some(extractor::transaction_payload::Payload::WriteSetPayload(
                extractor::WriteSetPayload {
                    write_set: convert_write_set(&wsp.write_set),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        },
    }
}

#[inline]
pub fn convert_events(events: &[Event]) -> Vec<extractor::Event> {
    events.iter().map(convert_event).collect()
}

pub fn convert_write_set(write_set: &WriteSet) -> protobuf::MessageField<extractor::WriteSet> {
    let (write_set_type, write_set) = match write_set {
        WriteSet::ScriptWriteSet(sws) => {
            let write_set_type =
                EnumOrUnknown::new(extractor::write_set::WriteSetType::SCRIPT_WRITE_SET);

            let write_set =
                extractor::write_set::Write_set::ScriptWriteSet(extractor::ScriptWriteSet {
                    execute_as: sws.execute_as.to_string(),
                    script: protobuf::MessageField::some(convert_script_payload(&sws.script)),
                    special_fields: Default::default(),
                });
            (write_set_type, Some(write_set))
        }
        WriteSet::DirectWriteSet(dws) => {
            let write_set_type =
                EnumOrUnknown::new(extractor::write_set::WriteSetType::DIRECT_WRITE_SET);

            let write_set =
                extractor::write_set::Write_set::DirectWriteSet(extractor::DirectWriteSet {
                    write_set_change: convert_write_set_changes(&dws.changes),
                    events: convert_events(&dws.events),
                    special_fields: Default::default(),
                });
            (write_set_type, Some(write_set))
        }
    };
    protobuf::MessageField::some(extractor::WriteSet {
        write_set_type,
        write_set,
        special_fields: Default::default(),
    })
}

pub fn empty_move_type(type_: extractor::move_type::Type) -> extractor::MoveType {
    extractor::MoveType {
        type_: EnumOrUnknown::new(type_),
        content: None,
        special_fields: Default::default(),
    }
}

pub fn convert_move_type(move_type: &MoveType) -> extractor::MoveType {
    let type_ = match move_type {
        MoveType::Bool => extractor::move_type::Type::Bool,
        MoveType::U8 => extractor::move_type::Type::U8,
        MoveType::U64 => extractor::move_type::Type::U64,
        MoveType::U128 => extractor::move_type::Type::U128,
        MoveType::Address => extractor::move_type::Type::Address,
        MoveType::Signer => extractor::move_type::Type::Signer,
        MoveType::Vector { .. } => extractor::move_type::Type::Vector,
        MoveType::Struct(_) => extractor::move_type::Type::Struct,
        MoveType::GenericTypeParam { .. } => extractor::move_type::Type::GenericTypeParam,
        MoveType::Reference { .. } => extractor::move_type::Type::Reference,
        MoveType::Unparsable(_) => extractor::move_type::Type::Unparsable,
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
            extractor::move_type::ReferenceType {
                mutable: mutable.clone(),
                to: protobuf::MessageField::some(convert_move_type(&to)),
                special_fields: Default::default(),
            },
        )),
        MoveType::Unparsable(string) => {
            Some(extractor::move_type::Content::Unparsable(string.clone()))
        }
    };
    extractor::MoveType {
        type_: EnumOrUnknown::new(type_),
        content,
        special_fields: Default::default(),
    }
}

#[inline]
pub fn convert_write_set_changes(changes: &[WriteSetChange]) -> Vec<extractor::WriteSetChange> {
    changes.iter().map(convert_write_set_change).collect()
}

#[inline]
pub fn convert_hex_string_to_bytes(hex_string: &str) -> Vec<u8> {
    HashValue::from_str(hex_string)
        .unwrap_or_else(|_| panic!("Could not convert '{}' to HashValue", hex_string))
        .0
        .to_vec()
}

pub fn convert_move_struct_tag(struct_tag: &MoveStructTag) -> extractor::MoveStructTag {
    extractor::MoveStructTag {
        address: struct_tag.address.to_string(),
        module: struct_tag.module.to_string(),
        name: struct_tag.name.to_string(),
        generic_type_params: struct_tag
            .generic_type_params
            .iter()
            .map(|move_type| convert_move_type(move_type))
            .collect(),
        special_fields: Default::default(),
    }
}

pub fn convert_delete_module(delete_module: &DeleteModule) -> extractor::DeleteModule {
    extractor::DeleteModule {
        address: delete_module.address.to_string(),
        state_key_hash: convert_hex_string_to_bytes(&delete_module.state_key_hash),
        module: protobuf::MessageField::some(extractor::MoveModuleId {
            address: delete_module.module.address.to_string(),
            name: delete_module.module.name.to_string(),
            special_fields: Default::default(),
        }),
        special_fields: Default::default(),
    }
}

pub fn convert_delete_resource(delete_resource: &DeleteResource) -> extractor::DeleteResource {
    extractor::DeleteResource {
        address: delete_resource.address.to_string(),
        state_key_hash: convert_hex_string_to_bytes(&delete_resource.state_key_hash),
        resource: protobuf::MessageField::some(extractor::MoveStructTag {
            address: delete_resource.address.to_string(),
            module: delete_resource.resource.module.to_string(),
            name: delete_resource.resource.name.to_string(),
            generic_type_params: delete_resource
                .resource
                .generic_type_params
                .iter()
                .map(|move_type| convert_move_type(move_type))
                .collect(),
            special_fields: Default::default(),
        }),
        special_fields: Default::default(),
    }
}
pub fn convert_write_set_change(change: &WriteSetChange) -> extractor::WriteSetChange {
    match change {
        WriteSetChange::DeleteModule(delete_module) => extractor::WriteSetChange {
            type_: EnumOrUnknown::new(
                extractor::write_set_change::WriteSetChangeType::DELETE_MODULE,
            ),
            change: Some(extractor::write_set_change::Change::DeleteModule(
                convert_delete_module(delete_module),
            )),
            special_fields: Default::default(),
        },
        WriteSetChange::DeleteResource(delete_resource) => extractor::WriteSetChange {
            type_: EnumOrUnknown::new(
                extractor::write_set_change::WriteSetChangeType::DELETE_RESOURCE,
            ),
            change: Some(extractor::write_set_change::Change::DeleteResource(
                convert_delete_resource(delete_resource),
            )),
            special_fields: Default::default(),
        },
        WriteSetChange::DeleteTableItem(delete_table_item) => {
            let data = delete_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "Could not extract data from DeletedTableItem '{:?}'",
                    delete_table_item
                )
            });

            extractor::WriteSetChange {
                type_: EnumOrUnknown::new(
                    extractor::write_set_change::WriteSetChangeType::DELETE_TABLE_ITEM,
                ),
                change: Some(extractor::write_set_change::Change::DeleteTableItem(
                    extractor::DeleteTableItem {
                        state_key_hash: convert_hex_string_to_bytes(
                            &delete_table_item.state_key_hash,
                        ),
                        handle: delete_table_item.handle.to_string(),
                        key: delete_table_item.key.to_string(),
                        data: protobuf::MessageField::some(extractor::DeleteTableData {
                            key: data.key.to_string(),
                            key_type: data.key_type.clone(),
                            special_fields: Default::default(),
                        }),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }
        }
        WriteSetChange::WriteModule(write_module) => extractor::WriteSetChange {
            type_: EnumOrUnknown::new(
                extractor::write_set_change::WriteSetChangeType::DELETE_MODULE,
            ),
            change: Some(extractor::write_set_change::Change::WriteModule(
                extractor::WriteModule {
                    address: write_module.address.to_string(),
                    state_key_hash: convert_hex_string_to_bytes(&write_module.state_key_hash),
                    data: write_module.data.bytecode.to_string(),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        },
        WriteSetChange::WriteResource(write_resource) => extractor::WriteSetChange {
            type_: EnumOrUnknown::new(
                extractor::write_set_change::WriteSetChangeType::WRITE_RESOURCE,
            ),
            change: Some(extractor::write_set_change::Change::WriteResource(
                extractor::WriteResource {
                    address: write_resource.address.to_string(),
                    state_key_hash: convert_hex_string_to_bytes(&write_resource.state_key_hash),
                    data: protobuf::MessageField::some(extractor::MoveResource {
                        type_: protobuf::MessageField::some(extractor::MoveStructTag {
                            address: write_resource.data.typ.address.to_string(),
                            module: write_resource.data.typ.module.to_string(),
                            name: write_resource.data.typ.name.to_string(),
                            generic_type_params: write_resource
                                .data
                                .typ
                                .generic_type_params
                                .iter()
                                .map(|move_type| convert_move_type(move_type))
                                .collect(),
                            special_fields: Default::default(),
                        }),
                        data: serde_json::to_string(&write_resource.data.data).unwrap_or_else(
                            |_| {
                                panic!(
                                    "Could not convert write_resource data to json '{:?}'",
                                    write_resource
                                )
                            },
                        ),
                        special_fields: Default::default(),
                    }),
                    special_fields: Default::default(),
                },
            )),
            special_fields: Default::default(),
        },
        WriteSetChange::WriteTableItem(write_table_item) => {
            let data = write_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "Could not extract data from DecodedTableData '{:?}'",
                    write_table_item
                )
            });
            extractor::WriteSetChange {
                type_: EnumOrUnknown::new(
                    extractor::write_set_change::WriteSetChangeType::WRITE_TABLE_ITEM,
                ),
                change: Some(extractor::write_set_change::Change::WriteTableItem(
                    extractor::WriteTableItem {
                        state_key_hash: convert_hex_string_to_bytes(
                            &write_table_item.state_key_hash,
                        ),
                        handle: write_table_item.handle.to_string(),
                        key: write_table_item.key.to_string(),
                        data: protobuf::MessageField::some(extractor::WriteTableData {
                            key: data.key.to_string(),
                            key_type: data.key_type.clone(),
                            value: data.value.to_string(),
                            value_type: data.value_type.clone(),
                            special_fields: Default::default(),
                        }),
                        special_fields: Default::default(),
                    },
                )),
                special_fields: Default::default(),
            }
        }
    }
}

pub fn convert_script_payload(script_payload: &ScriptPayload) -> extractor::ScriptPayload {
    extractor::ScriptPayload {
        code: script_payload.code.bytecode.to_string(),
        type_arguments: script_payload
            .type_arguments
            .iter()
            .map(|move_type| move_type.to_string())
            .collect(),
        arguments: script_payload
            .arguments
            .iter()
            .map(|move_value| move_value.to_string())
            .collect(),
        special_fields: Default::default(),
    }
}

pub fn convert_event(event: &Event) -> extractor::Event {
    extractor::Event {
        key: protobuf::MessageField::some(extractor::EventKey {
            creation_number: event.key.0.get_creation_number(),
            account_address: event.key.0.get_creator_address().to_string(),
            special_fields: Default::default(),
        }),
        sequence_number: event.sequence_number.0,
        type_: event.typ.to_string(),
        data: event.data.to_string(),
        special_fields: Default::default(),
    }
}

pub fn convert_timestamp_secs(
    timestamp: u64,
) -> protobuf::MessageField<protobuf::well_known_types::timestamp::Timestamp> {
    protobuf::MessageField::some(protobuf::well_known_types::timestamp::Timestamp {
        seconds: timestamp as i64,
        nanos: 0,
        special_fields: Default::default(),
    })
}

pub fn convert_timestamp_usecs(
    timestamp: u64,
) -> protobuf::MessageField<protobuf::well_known_types::timestamp::Timestamp> {
    let ts = Duration::from_nanos(timestamp * 1000);
    protobuf::MessageField::some(protobuf::well_known_types::timestamp::Timestamp {
        seconds: ts.as_secs() as i64,
        nanos: ts.subsec_nanos() as i32,
        special_fields: Default::default(),
    })
}

pub fn convert_transaction_info(transaction_info: &TransactionInfo) -> extractor::TransactionInfo {
    extractor::TransactionInfo {
        hash: transaction_info.hash.0.to_vec(),
        state_root_hash: transaction_info.state_root_hash.0.to_vec(),
        event_root_hash: transaction_info.event_root_hash.0.to_vec(),
        gas_used: transaction_info.gas_used.0,
        success: transaction_info.success,
        vm_status: transaction_info.vm_status.to_string(),
        accumulator_root_hash: transaction_info.accumulator_root_hash.0.to_vec(),
        changes: convert_write_set_changes(&transaction_info.changes),
        special_fields: Default::default(),
    }
}

pub fn convert_transaction(
    transaction: &Transaction,
    block_height: u64,
    current_epoch: u64,
) -> extractor::Transaction {
    let mut timestamp: Option<
        protobuf::MessageField<protobuf::well_known_types::timestamp::Timestamp>,
    > = None;

    let txn_type = match transaction {
        Transaction::UserTransaction(_) => extractor::transaction::TransactionType::USER,
        Transaction::GenesisTransaction(_) => extractor::transaction::TransactionType::GENESIS,
        Transaction::BlockMetadataTransaction(_) => {
            extractor::transaction::TransactionType::BLOCK_METADATA
        }
        Transaction::StateCheckpointTransaction(_) => {
            extractor::transaction::TransactionType::STATE_CHECKPOINT
        }
        Transaction::PendingTransaction(_) => panic!("PendingTransaction is not supported"),
    };

    let txn_data = match &transaction {
        Transaction::UserTransaction(ut) => {
            timestamp = Some(convert_timestamp_usecs(ut.timestamp.0));
            extractor::transaction::Txn_data::User(extractor::UserTransaction {
                request: protobuf::MessageField::some(extractor::UserTransactionRequest {
                    sender: ut.request.sender.to_string(),
                    sequence_number: ut.request.sequence_number.0,
                    max_gas_amount: ut.request.max_gas_amount.0,
                    gas_unit_price: ut.request.gas_unit_price.0,
                    expiration_timestamp_secs: convert_timestamp_secs(
                        ut.request.expiration_timestamp_secs.0,
                    ),
                    payload: protobuf::MessageField::some(convert_transaction_payload(
                        &ut.request.payload,
                    )),
                    signature: Default::default(),
                    special_fields: Default::default(),
                }),
                events: convert_events(&ut.events),
                special_fields: Default::default(),
            })
        }
        Transaction::GenesisTransaction(gt) => {
            let payload = match &gt.payload {
                GenesisPayload::WriteSetPayload(wsp) => convert_write_set(&wsp.write_set),
            };
            extractor::transaction::Txn_data::Genesis(extractor::GenesisTransaction {
                payload,
                events: convert_events(&gt.events),
                special_fields: Default::default(),
            })
        }
        Transaction::BlockMetadataTransaction(bm) => {
            timestamp = Some(convert_timestamp_usecs(bm.timestamp.0));
            extractor::transaction::Txn_data::BlockMetadata(extractor::BlockMetadataTransaction {
                id: bm.id.to_string(),
                events: convert_events(&bm.events),
                previous_block_votes: bm.previous_block_votes.clone(),
                proposer: bm.proposer.to_string(),
                failed_proposer_indices: bm.failed_proposer_indices.clone(),
                round: bm.round.0,
                special_fields: Default::default(),
            })
        }
        Transaction::StateCheckpointTransaction(_st) => {
            extractor::transaction::Txn_data::StateCheckpoint(
                extractor::StateCheckpointTransaction {
                    special_fields: Default::default(),
                },
            )
        }
        Transaction::PendingTransaction(_) => panic!("PendingTransaction not supported"),
    };

    extractor::Transaction {
        timestamp: timestamp.unwrap_or_else(|| convert_timestamp_usecs(transaction.timestamp())),
        version: transaction.version().unwrap_or_else(|| {
            panic!(
                "Could not extract version from Transaction '{:?}'",
                transaction
            )
        }),
        info: protobuf::MessageField::some(convert_transaction_info(
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
        type_: EnumOrUnknown::new(txn_type),
        txn_data: Some(txn_data),
        special_fields: Default::default(),
    }
}
