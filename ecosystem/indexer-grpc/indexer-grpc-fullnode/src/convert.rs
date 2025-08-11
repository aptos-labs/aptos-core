// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_api_types::{
    transaction::ValidatorTransaction as ApiValidatorTransactionEnum, AccountSignature,
    DeleteModule, DeleteResource, Ed25519Signature, EntryFunctionId, EntryFunctionPayload, Event,
    GenesisPayload, MoveAbility, MoveFunction, MoveFunctionGenericTypeParam,
    MoveFunctionVisibility, MoveModule, MoveModuleBytecode, MoveModuleId, MoveScriptBytecode,
    MoveStruct, MoveStructField, MoveStructTag, MoveType, MultiEd25519Signature, MultiKeySignature,
    MultisigPayload, MultisigTransactionPayload, PublicKey, ScriptPayload, Signature,
    SingleKeySignature, Transaction, TransactionInfo, TransactionPayload, TransactionSignature,
    WriteSet, WriteSetChange,
};
use aptos_bitvec::BitVec;
use aptos_logger::warn;
use aptos_protos::{
    transaction::v1::{
        self as transaction, any_signature,
        validator_transaction::{
            self,
            observed_jwk_update::exported_provider_jw_ks::{
                jwk::{JwkType, Rsa, UnsupportedJwk},
                Jwk as ProtoJwk,
            },
        },
        Ed25519, Keyless, Secp256k1Ecdsa, TransactionSizeInfo, WebAuthn,
    },
    util::timestamp,
};
use aptos_types::jwks::jwk::JWK;
use hex;
use move_core_types::ability::Ability;
use std::time::Duration;

pub fn convert_move_module_id(move_module_id: &MoveModuleId) -> transaction::MoveModuleId {
    transaction::MoveModuleId {
        address: move_module_id.address.to_string(),
        name: move_module_id.name.to_string(),
    }
}

pub fn convert_move_ability(move_ability: &MoveAbility) -> transaction::MoveAbility {
    match move_ability.0 {
        Ability::Copy => transaction::MoveAbility::Copy,
        Ability::Drop => transaction::MoveAbility::Drop,
        Ability::Store => transaction::MoveAbility::Store,
        Ability::Key => transaction::MoveAbility::Key,
    }
}

pub fn convert_move_struct_field(msf: &MoveStructField) -> transaction::MoveStructField {
    transaction::MoveStructField {
        name: msf.name.0.to_string(),
        r#type: Some(convert_move_type(&msf.typ)),
    }
}

pub fn convert_move_struct(move_struct: &MoveStruct) -> transaction::MoveStruct {
    transaction::MoveStruct {
        name: move_struct.name.0.to_string(),
        is_native: move_struct.is_native,
        is_event: move_struct.is_event,
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
) -> transaction::move_function::Visibility {
    match visibility {
        MoveFunctionVisibility::Public => transaction::move_function::Visibility::Public,
        MoveFunctionVisibility::Private => transaction::move_function::Visibility::Private,
        MoveFunctionVisibility::Friend => transaction::move_function::Visibility::Friend,
    }
}

pub fn convert_move_function_generic_type_params(
    mfgtp: &MoveFunctionGenericTypeParam,
) -> transaction::MoveFunctionGenericTypeParam {
    transaction::MoveFunctionGenericTypeParam {
        constraints: mfgtp
            .constraints
            .iter()
            .map(|i| convert_move_ability(i) as i32)
            .collect(),
    }
}

pub fn convert_move_function(move_func: &MoveFunction) -> transaction::MoveFunction {
    transaction::MoveFunction {
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

pub fn convert_move_module(move_module: &MoveModule) -> transaction::MoveModule {
    transaction::MoveModule {
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

pub fn convert_move_module_bytecode(mmb: &MoveModuleBytecode) -> transaction::MoveModuleBytecode {
    let abi = mmb.clone().try_parse_abi().map_or_else(
        |e| {
            warn!("[fh-stream] Could not decode MoveModuleBytecode ABI: {}", e);
            None
        },
        |mmb| mmb.abi.map(|move_module| convert_move_module(&move_module)),
    );
    transaction::MoveModuleBytecode {
        bytecode: mmb.bytecode.0.clone(),
        abi,
    }
}

pub fn convert_entry_function_id(
    entry_function_id: &EntryFunctionId,
) -> transaction::EntryFunctionId {
    transaction::EntryFunctionId {
        module: Some(convert_move_module_id(&entry_function_id.module)),
        name: entry_function_id.name.to_string(),
    }
}

pub fn convert_transaction_payload(
    payload: &TransactionPayload,
    nonce: Option<u64>,
) -> transaction::TransactionPayload {
    match payload {
        TransactionPayload::EntryFunctionPayload(sfp) => transaction::TransactionPayload {
            r#type: transaction::transaction_payload::Type::EntryFunctionPayload as i32,
            payload: Some(
                transaction::transaction_payload::Payload::EntryFunctionPayload(
                    convert_entry_function_payload(sfp),
                ),
            ),
            extra_config: Some(
                transaction::transaction_payload::ExtraConfig::ExtraConfigV1(
                    transaction::ExtraConfigV1 {
                        multisig_address: None,
                        replay_protection_nonce: nonce,
                    },
                ),
            ),
        },
        TransactionPayload::ScriptPayload(sp) => transaction::TransactionPayload {
            r#type: transaction::transaction_payload::Type::ScriptPayload as i32,
            payload: Some(transaction::transaction_payload::Payload::ScriptPayload(
                convert_script_payload(sp),
            )),
            extra_config: Some(
                transaction::transaction_payload::ExtraConfig::ExtraConfigV1(
                    transaction::ExtraConfigV1 {
                        multisig_address: None,
                        replay_protection_nonce: nonce,
                    },
                ),
            ),
        },
        TransactionPayload::MultisigPayload(mp) => transaction::TransactionPayload {
            r#type: transaction::transaction_payload::Type::MultisigPayload as i32,
            payload: Some(transaction::transaction_payload::Payload::MultisigPayload(
                convert_multisig_payload(mp),
            )),
            extra_config: Some(
                transaction::transaction_payload::ExtraConfig::ExtraConfigV1(
                    transaction::ExtraConfigV1 {
                        multisig_address: Some(mp.multisig_address.to_string()),
                        replay_protection_nonce: nonce,
                    },
                ),
            ),
        },

        // Deprecated.
        TransactionPayload::ModuleBundlePayload(_) => {
            unreachable!("Module bundle payload has been removed")
        },
    }
}

#[inline]
pub fn convert_events(events: &[Event]) -> Vec<transaction::Event> {
    events.iter().map(convert_event).collect()
}

pub fn convert_write_set(write_set: &WriteSet) -> transaction::WriteSet {
    let (write_set_type, write_set) = match write_set {
        WriteSet::ScriptWriteSet(sws) => {
            let write_set_type = transaction::write_set::WriteSetType::ScriptWriteSet as i32;

            let write_set =
                transaction::write_set::WriteSet::ScriptWriteSet(transaction::ScriptWriteSet {
                    execute_as: sws.execute_as.to_string(),
                    script: Some(convert_script_payload(&sws.script)),
                });
            (write_set_type, Some(write_set))
        },
        WriteSet::DirectWriteSet(dws) => {
            let write_set_type = transaction::write_set::WriteSetType::DirectWriteSet as i32;

            let write_set =
                transaction::write_set::WriteSet::DirectWriteSet(transaction::DirectWriteSet {
                    write_set_change: convert_write_set_changes(&dws.changes),
                    events: convert_events(&dws.events),
                });
            (write_set_type, Some(write_set))
        },
    };
    transaction::WriteSet {
        write_set_type,
        write_set,
    }
}

pub fn empty_move_type(r#type: transaction::MoveTypes) -> transaction::MoveType {
    transaction::MoveType {
        r#type: r#type as i32,
        content: None,
    }
}

pub fn convert_move_type(move_type: &MoveType) -> transaction::MoveType {
    let r#type = match move_type {
        MoveType::Bool => transaction::MoveTypes::Bool,
        MoveType::U8 => transaction::MoveTypes::U8,
        MoveType::U16 => transaction::MoveTypes::U16,
        MoveType::U32 => transaction::MoveTypes::U32,
        MoveType::U64 => transaction::MoveTypes::U64,
        MoveType::U128 => transaction::MoveTypes::U128,
        MoveType::U256 => transaction::MoveTypes::U256,
        MoveType::Address => transaction::MoveTypes::Address,
        MoveType::Signer => transaction::MoveTypes::Signer,
        MoveType::Vector { .. } => transaction::MoveTypes::Vector,
        MoveType::Struct(_) => transaction::MoveTypes::Struct,
        MoveType::GenericTypeParam { .. } => transaction::MoveTypes::GenericTypeParam,
        MoveType::Reference { .. } => transaction::MoveTypes::Reference,
        MoveType::Function { .. } => transaction::MoveTypes::Unparsable,
        MoveType::Unparsable(_) => transaction::MoveTypes::Unparsable,
    };
    let content = match move_type {
        MoveType::Bool => None,
        MoveType::U8 => None,
        MoveType::U16 => None,
        MoveType::U32 => None,
        MoveType::U64 => None,
        MoveType::U128 => None,
        MoveType::U256 => None,
        MoveType::Address => None,
        MoveType::Signer => None,
        MoveType::Vector { items } => Some(transaction::move_type::Content::Vector(Box::from(
            convert_move_type(items),
        ))),
        MoveType::Struct(struct_tag) => Some(transaction::move_type::Content::Struct(
            convert_move_struct_tag(struct_tag),
        )),
        MoveType::GenericTypeParam { index } => Some(
            transaction::move_type::Content::GenericTypeParamIndex((*index) as u32),
        ),
        MoveType::Reference { mutable, to } => Some(transaction::move_type::Content::Reference(
            Box::new(transaction::move_type::ReferenceType {
                mutable: *mutable,
                to: Some(Box::new(convert_move_type(to))),
            }),
        )),
        MoveType::Function { .. } => Some(transaction::move_type::Content::Unparsable(
            "function".to_string(),
        )),
        MoveType::Unparsable(string) => {
            Some(transaction::move_type::Content::Unparsable(string.clone()))
        },
    };
    transaction::MoveType {
        r#type: r#type as i32,
        content,
    }
}

#[inline]
pub fn convert_write_set_changes(changes: &[WriteSetChange]) -> Vec<transaction::WriteSetChange> {
    changes.iter().map(convert_write_set_change).collect()
}

#[inline]
pub fn convert_hex_string_to_bytes(hex_string: &str) -> Vec<u8> {
    hex::decode(hex_string.strip_prefix("0x").unwrap_or(hex_string))
        .unwrap_or_else(|_| panic!("Could not convert '{}' to bytes", hex_string))
}

pub fn convert_move_struct_tag(struct_tag: &MoveStructTag) -> transaction::MoveStructTag {
    transaction::MoveStructTag {
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

pub fn convert_delete_module(delete_module: &DeleteModule) -> transaction::DeleteModule {
    transaction::DeleteModule {
        address: delete_module.address.to_string(),
        state_key_hash: convert_hex_string_to_bytes(&delete_module.state_key_hash),
        module: Some(transaction::MoveModuleId {
            address: delete_module.module.address.to_string(),
            name: delete_module.module.name.to_string(),
        }),
    }
}

pub fn convert_delete_resource(delete_resource: &DeleteResource) -> transaction::DeleteResource {
    transaction::DeleteResource {
        address: delete_resource.address.to_string(),
        state_key_hash: convert_hex_string_to_bytes(&delete_resource.state_key_hash),
        r#type: Some(convert_move_struct_tag(&delete_resource.resource)),
        type_str: delete_resource.resource.to_string(),
    }
}

pub fn convert_write_set_change(change: &WriteSetChange) -> transaction::WriteSetChange {
    match change {
        WriteSetChange::DeleteModule(delete_module) => transaction::WriteSetChange {
            r#type: transaction::write_set_change::Type::DeleteModule as i32,
            change: Some(transaction::write_set_change::Change::DeleteModule(
                convert_delete_module(delete_module),
            )),
        },
        WriteSetChange::DeleteResource(delete_resource) => transaction::WriteSetChange {
            r#type: transaction::write_set_change::Type::DeleteResource as i32,
            change: Some(transaction::write_set_change::Change::DeleteResource(
                convert_delete_resource(delete_resource),
            )),
        },
        WriteSetChange::DeleteTableItem(delete_table_item) => {
            let data = delete_table_item.data.as_ref().unwrap_or_else(|| {
                panic!(
                    "Could not extract data from DeletedTableItem '{:?}' with handle '{:?}'",
                    delete_table_item,
                    delete_table_item.handle.to_string()
                )
            });

            transaction::WriteSetChange {
                r#type: transaction::write_set_change::Type::DeleteTableItem as i32,
                change: Some(transaction::write_set_change::Change::DeleteTableItem(
                    transaction::DeleteTableItem {
                        state_key_hash: convert_hex_string_to_bytes(
                            &delete_table_item.state_key_hash,
                        ),
                        handle: delete_table_item.handle.to_string(),
                        key: delete_table_item.key.to_string(),
                        data: Some(transaction::DeleteTableData {
                            key: data.key.to_string(),
                            key_type: data.key_type.clone(),
                        }),
                    },
                )),
            }
        },
        WriteSetChange::WriteModule(write_module) => transaction::WriteSetChange {
            r#type: transaction::write_set_change::Type::WriteModule as i32,
            change: Some(transaction::write_set_change::Change::WriteModule(
                transaction::WriteModule {
                    address: write_module.address.to_string(),
                    state_key_hash: convert_hex_string_to_bytes(&write_module.state_key_hash),
                    data: Some(convert_move_module_bytecode(&write_module.data)),
                },
            )),
        },
        WriteSetChange::WriteResource(write_resource) => transaction::WriteSetChange {
            r#type: transaction::write_set_change::Type::WriteResource as i32,
            change: Some(transaction::write_set_change::Change::WriteResource(
                transaction::WriteResource {
                    address: write_resource.address.to_string(),
                    state_key_hash: convert_hex_string_to_bytes(&write_resource.state_key_hash),
                    r#type: Some(convert_move_struct_tag(&write_resource.data.typ)),
                    type_str: write_resource.data.typ.to_string(),
                    data: serde_json::to_string(&write_resource.data.data).unwrap_or_else(|_| {
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
                    "Could not extract data from DecodedTableData '{:?}' with handle '{:?}'",
                    write_table_item,
                    write_table_item.handle.to_string(),
                )
            });
            transaction::WriteSetChange {
                r#type: transaction::write_set_change::Type::WriteTableItem as i32,
                change: Some(transaction::write_set_change::Change::WriteTableItem(
                    transaction::WriteTableItem {
                        state_key_hash: convert_hex_string_to_bytes(
                            &write_table_item.state_key_hash,
                        ),
                        handle: write_table_item.handle.to_string(),
                        key: write_table_item.key.to_string(),
                        data: Some(transaction::WriteTableData {
                            key: data.key.to_string(),
                            key_type: data.key_type.clone(),
                            value: data.value.to_string(),
                            value_type: data.value_type.clone(),
                        }),
                    },
                )),
            }
        },
    }
}

pub fn convert_move_script_bytecode(msb: &MoveScriptBytecode) -> transaction::MoveScriptBytecode {
    let abi = msb
        .clone()
        .try_parse_abi()
        .abi
        .map(|move_func| convert_move_function(&move_func));

    transaction::MoveScriptBytecode {
        bytecode: msb.bytecode.0.clone(),
        abi,
    }
}

pub fn convert_entry_function_payload(
    entry_function_payload: &EntryFunctionPayload,
) -> transaction::EntryFunctionPayload {
    transaction::EntryFunctionPayload {
        function: Some(convert_entry_function_id(&entry_function_payload.function)),
        type_arguments: entry_function_payload
            .type_arguments
            .iter()
            .map(convert_move_type)
            .collect(),
        arguments: entry_function_payload
            .arguments
            .iter()
            .map(|move_value| move_value.to_string())
            .collect(),
        entry_function_id_str: entry_function_payload.function.to_string(),
    }
}

pub fn convert_script_payload(script_payload: &ScriptPayload) -> transaction::ScriptPayload {
    transaction::ScriptPayload {
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

pub fn convert_multisig_payload(
    multisig_payload: &MultisigPayload,
) -> transaction::MultisigPayload {
    let transaction_payload = multisig_payload
        .transaction_payload
        .as_ref()
        .map(|p| match p {
            MultisigTransactionPayload::EntryFunctionPayload(entry_function_payload) => {
                transaction::MultisigTransactionPayload {
                    r#type: transaction::multisig_transaction_payload::Type::EntryFunctionPayload
                        as i32,
                    payload: Some(
                        transaction::multisig_transaction_payload::Payload::EntryFunctionPayload(
                            convert_entry_function_payload(entry_function_payload),
                        ),
                    ),
                }
            },
        });
    transaction::MultisigPayload {
        multisig_address: multisig_payload.multisig_address.to_string(),
        transaction_payload,
    }
}

pub fn convert_event(event: &Event) -> transaction::Event {
    let event_key: aptos_types::event::EventKey = event.guid.into();
    transaction::Event {
        key: Some(transaction::EventKey {
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
    let timestamp = std::cmp::min(timestamp, i64::MAX as u64);
    timestamp::Timestamp {
        seconds: timestamp as i64,
        nanos: 0,
    }
}

pub fn convert_timestamp_usecs(timestamp: u64) -> timestamp::Timestamp {
    let ts = Duration::from_micros(timestamp);
    timestamp::Timestamp {
        seconds: ts.as_secs() as i64,
        nanos: ts.subsec_nanos() as i32,
    }
}

pub fn convert_transaction_info(
    transaction_info: &TransactionInfo,
) -> transaction::TransactionInfo {
    transaction::TransactionInfo {
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

pub fn convert_ed25519_signature(sig: &Ed25519Signature) -> transaction::Ed25519Signature {
    transaction::Ed25519Signature {
        public_key: sig.public_key.0.clone(),
        signature: sig.signature.0.clone(),
    }
}

pub fn convert_multi_ed25519_signature(
    sig: &MultiEd25519Signature,
) -> transaction::MultiEd25519Signature {
    let public_key_indices: Vec<usize> = BitVec::from(sig.bitmap.0.clone()).iter_ones().collect();
    transaction::MultiEd25519Signature {
        public_keys: sig.public_keys.iter().map(|pk| pk.0.clone()).collect(),
        signatures: sig.signatures.iter().map(|sig| sig.0.clone()).collect(),
        threshold: sig.threshold as u32,
        public_key_indices: public_key_indices
            .iter()
            .map(|index| *index as u32)
            .collect(),
    }
}

pub fn convert_single_key_signature(sig: &SingleKeySignature) -> transaction::SingleKeySignature {
    transaction::SingleKeySignature {
        public_key: Some(convert_public_key(&sig.public_key)),
        signature: Some(convert_signature(&sig.signature)),
    }
}

pub fn convert_multi_key_signature(sig: &MultiKeySignature) -> transaction::MultiKeySignature {
    transaction::MultiKeySignature {
        public_keys: sig.public_keys.iter().map(convert_public_key).collect(),
        signatures: sig
            .signatures
            .iter()
            .map(|signature| transaction::IndexedSignature {
                index: signature.index as u32,
                signature: Some(convert_signature(&signature.signature)),
            })
            .collect(),
        signatures_required: sig.signatures_required as u32,
    }
}

#[allow(deprecated)]
fn convert_signature(signature: &Signature) -> transaction::AnySignature {
    match signature {
        Signature::Ed25519(s) => transaction::AnySignature {
            r#type: transaction::any_signature::Type::Ed25519 as i32,
            signature: s.value.clone().into(),
            signature_variant: Some(any_signature::SignatureVariant::Ed25519(Ed25519 {
                signature: s.value.clone().into(),
            })),
        },
        Signature::Secp256k1Ecdsa(s) => transaction::AnySignature {
            r#type: transaction::any_signature::Type::Secp256k1Ecdsa as i32,
            signature: s.value.clone().into(),
            signature_variant: Some(any_signature::SignatureVariant::Secp256k1Ecdsa(
                Secp256k1Ecdsa {
                    signature: s.value.clone().into(),
                },
            )),
        },
        Signature::WebAuthn(s) => transaction::AnySignature {
            r#type: transaction::any_signature::Type::Webauthn as i32,
            signature: s.value.clone().into(),
            signature_variant: Some(any_signature::SignatureVariant::Webauthn(WebAuthn {
                signature: s.value.clone().into(),
            })),
        },
        Signature::Keyless(s) => transaction::AnySignature {
            r#type: transaction::any_signature::Type::Keyless as i32,
            signature: s.value.clone().into(),
            signature_variant: Some(any_signature::SignatureVariant::Keyless(Keyless {
                signature: s.value.clone().into(),
            })),
        },
    }
}

fn convert_public_key(public_key: &PublicKey) -> transaction::AnyPublicKey {
    match public_key {
        PublicKey::Ed25519(p) => transaction::AnyPublicKey {
            r#type: transaction::any_public_key::Type::Ed25519 as i32,
            public_key: p.value.clone().into(),
        },
        PublicKey::Secp256k1Ecdsa(p) => transaction::AnyPublicKey {
            r#type: transaction::any_public_key::Type::Secp256k1Ecdsa as i32,
            public_key: p.value.clone().into(),
        },
        PublicKey::Secp256r1Ecdsa(p) => transaction::AnyPublicKey {
            r#type: transaction::any_public_key::Type::Secp256r1Ecdsa as i32,
            public_key: p.value.clone().into(),
        },
        PublicKey::Keyless(p) => transaction::AnyPublicKey {
            r#type: transaction::any_public_key::Type::Keyless as i32,
            public_key: p.value.clone().into(),
        },
        PublicKey::FederatedKeyless(p) => transaction::AnyPublicKey {
            r#type: transaction::any_public_key::Type::FederatedKeyless as i32,
            public_key: p.value.clone().into(),
        },
    }
}

pub fn convert_account_signature(
    account_signature: &AccountSignature,
) -> transaction::AccountSignature {
    let (r#type, signature) = match account_signature {
        AccountSignature::Ed25519Signature(s) => (
            transaction::account_signature::Type::Ed25519,
            transaction::account_signature::Signature::Ed25519(convert_ed25519_signature(s)),
        ),
        AccountSignature::MultiEd25519Signature(s) => (
            transaction::account_signature::Type::MultiEd25519,
            transaction::account_signature::Signature::MultiEd25519(
                convert_multi_ed25519_signature(s),
            ),
        ),
        AccountSignature::SingleKeySignature(s) => (
            transaction::account_signature::Type::SingleKey,
            transaction::account_signature::Signature::SingleKeySignature(
                convert_single_key_signature(s),
            ),
        ),
        AccountSignature::MultiKeySignature(s) => (
            transaction::account_signature::Type::MultiKey,
            transaction::account_signature::Signature::MultiKeySignature(
                convert_multi_key_signature(s),
            ),
        ),
        AccountSignature::NoAccountSignature(_) => {
            unreachable!(
                "[Indexer Fullnode] Indexer should never see transactions with NoAccountSignature"
            )
        },
        AccountSignature::AbstractSignature(s) => (
            transaction::account_signature::Type::Abstraction,
            transaction::account_signature::Signature::Abstraction(
                transaction::AbstractSignature {
                    function_info: s.function_info.to_owned(),
                    signature: s.auth_data.inner().to_owned(),
                },
            ),
        ),
    };

    transaction::AccountSignature {
        r#type: r#type as i32,
        signature: Some(signature),
    }
}

pub fn convert_transaction_signature(
    signature: &Option<TransactionSignature>,
) -> Option<transaction::Signature> {
    let signature = match signature {
        None => return None,
        Some(s) => s,
    };
    let r#type = match signature {
        TransactionSignature::Ed25519Signature(_) => transaction::signature::Type::Ed25519,
        TransactionSignature::MultiEd25519Signature(_) => {
            transaction::signature::Type::MultiEd25519
        },
        TransactionSignature::MultiAgentSignature(_) => transaction::signature::Type::MultiAgent,
        TransactionSignature::FeePayerSignature(_) => transaction::signature::Type::FeePayer,
        TransactionSignature::SingleSender(_) => transaction::signature::Type::SingleSender,
        TransactionSignature::NoAccountSignature(_) => {
            unreachable!("No account signature can't be committed onchain")
        },
    };

    let signature = match signature {
        TransactionSignature::Ed25519Signature(s) => Some(
            transaction::signature::Signature::Ed25519(convert_ed25519_signature(s)),
        ),
        TransactionSignature::MultiEd25519Signature(s) => Some(
            transaction::signature::Signature::MultiEd25519(convert_multi_ed25519_signature(s)),
        ),
        TransactionSignature::MultiAgentSignature(s) => Some(
            transaction::signature::Signature::MultiAgent(transaction::MultiAgentSignature {
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
            }),
        ),
        TransactionSignature::FeePayerSignature(s) => Some(
            transaction::signature::Signature::FeePayer(transaction::FeePayerSignature {
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
                fee_payer_address: s.fee_payer_address.to_string(),
                fee_payer_signer: Some(convert_account_signature(&s.fee_payer_signer)),
            }),
        ),
        TransactionSignature::SingleSender(s) => Some(
            transaction::signature::Signature::SingleSender(transaction::SingleSender {
                sender: Some(convert_account_signature(s)),
            }),
        ),
        TransactionSignature::NoAccountSignature(_) => None,
    };

    Some(transaction::Signature {
        r#type: r#type as i32,
        signature,
    })
}

pub fn convert_transaction(
    transaction: &Transaction,
    block_height: u64,
    epoch: u64,
    size_info: TransactionSizeInfo,
) -> transaction::Transaction {
    let mut timestamp: Option<timestamp::Timestamp> = None;

    let txn_type = match transaction {
        Transaction::UserTransaction(_) => transaction::transaction::TransactionType::User,
        Transaction::GenesisTransaction(_) => transaction::transaction::TransactionType::Genesis,
        Transaction::BlockMetadataTransaction(_) => {
            transaction::transaction::TransactionType::BlockMetadata
        },
        Transaction::StateCheckpointTransaction(_) => {
            transaction::transaction::TransactionType::StateCheckpoint
        },
        Transaction::BlockEpilogueTransaction(_) => {
            transaction::transaction::TransactionType::BlockEpilogue
        },
        Transaction::PendingTransaction(_) => panic!("PendingTransaction is not supported"),
        Transaction::ValidatorTransaction(_) => {
            transaction::transaction::TransactionType::Validator
        },
    };

    let txn_data = match &transaction {
        Transaction::UserTransaction(ut) => {
            timestamp = Some(convert_timestamp_usecs(ut.timestamp.0));
            let expiration_timestamp_secs = Some(convert_timestamp_secs(
                ut.request.expiration_timestamp_secs.0,
            ));
            transaction::transaction::TxnData::User(transaction::UserTransaction {
                request: Some(transaction::UserTransactionRequest {
                    sender: ut.request.sender.to_string(),
                    sequence_number: ut.request.sequence_number.0,
                    max_gas_amount: ut.request.max_gas_amount.0,
                    gas_unit_price: ut.request.gas_unit_price.0,
                    expiration_timestamp_secs,
                    payload: Some(convert_transaction_payload(
                        &ut.request.payload,
                        ut.request.replay_protection_nonce.map(|n| n.into()),
                    )),
                    signature: convert_transaction_signature(&ut.request.signature),
                }),
                events: convert_events(&ut.events),
            })
        },
        Transaction::GenesisTransaction(gt) => {
            let payload = match &gt.payload {
                GenesisPayload::WriteSetPayload(wsp) => convert_write_set(&wsp.write_set),
            };
            transaction::transaction::TxnData::Genesis(transaction::GenesisTransaction {
                payload: Some(payload),
                events: convert_events(&gt.events),
            })
        },
        Transaction::BlockMetadataTransaction(bm) => {
            timestamp = Some(convert_timestamp_usecs(bm.timestamp.0));
            transaction::transaction::TxnData::BlockMetadata(
                transaction::BlockMetadataTransaction {
                    id: bm.id.to_string(),
                    events: convert_events(&bm.events),
                    previous_block_votes_bitvec: bm.previous_block_votes_bitvec.clone(),
                    proposer: bm.proposer.to_string(),
                    failed_proposer_indices: bm.failed_proposer_indices.clone(),
                    round: bm.round.0,
                },
            )
        },
        Transaction::StateCheckpointTransaction(_st) => {
            transaction::transaction::TxnData::StateCheckpoint(
                transaction::StateCheckpointTransaction {},
            )
        },
        Transaction::BlockEpilogueTransaction(block_epilogue) => {
            transaction::transaction::TxnData::BlockEpilogue(
                transaction::BlockEpilogueTransaction {
                    block_end_info: block_epilogue
                        .block_end_info
                        .as_ref()
                        .map(|block_end_info| transaction::BlockEndInfo {
                            block_gas_limit_reached: block_end_info.block_gas_limit_reached,
                            block_output_limit_reached: block_end_info.block_output_limit_reached,
                            block_effective_block_gas_units: block_end_info
                                .block_effective_block_gas_units,
                            block_approx_output_size: block_end_info.block_approx_output_size,
                        }),
                },
            )
        },
        Transaction::PendingTransaction(_) => panic!("PendingTransaction not supported"),
        Transaction::ValidatorTransaction(api_validator_txn) => {
            convert_validator_transaction(api_validator_txn)
        },
    };

    transaction::Transaction {
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
        epoch,
        block_height,
        r#type: txn_type as i32,
        txn_data: Some(txn_data),
        size_info: Some(size_info),
    }
}

fn convert_validator_transaction(
    api_validator_txn: &aptos_api_types::transaction::ValidatorTransaction,
) -> transaction::transaction::TxnData {
    transaction::transaction::TxnData::Validator(transaction::ValidatorTransaction {
        validator_transaction_type: match api_validator_txn {
            ApiValidatorTransactionEnum::DkgResult(dgk_result) => {
                Some(
                    validator_transaction::ValidatorTransactionType::DkgUpdate(
                        validator_transaction::DkgUpdate {
                            dkg_transcript: Some(validator_transaction::dkg_update::DkgTranscript {
                                author: dgk_result.dkg_transcript.author.to_string(),
                                epoch: dgk_result.dkg_transcript.epoch.0,
                                payload: dgk_result.dkg_transcript.payload.0.clone(),
                            }),
                        },
                    )
                )
            },
            ApiValidatorTransactionEnum::ObservedJwkUpdate(observed_jwk_update) => {
                Some(
                    validator_transaction::ValidatorTransactionType::ObservedJwkUpdate(
                        validator_transaction::ObservedJwkUpdate {
                            quorum_certified_update: Some(
                                validator_transaction::observed_jwk_update::QuorumCertifiedUpdate {
                                    update: Some(
                                        validator_transaction::observed_jwk_update::ExportedProviderJwKs {
                                            issuer: observed_jwk_update.quorum_certified_update.update.issuer.clone(),
                                            version: observed_jwk_update.quorum_certified_update.update.version,
                                            jwks: observed_jwk_update.quorum_certified_update.update.jwks.iter().map(|jwk| {
                                                match jwk {
                                                    JWK::RSA(rsa) => {
                                                        ProtoJwk {
                                                            jwk_type: Some(
                                                                JwkType::Rsa(
                                                                    Rsa {
                                                                        kid: rsa.kid.clone(),
                                                                        n: rsa.n.clone(),
                                                                        e: rsa.e.clone(),
                                                                        kty: rsa.kty.clone(),
                                                                        alg: rsa.alg.clone(),
                                                                    }
                                                                )
                                                            )
                                                        }
                                                    },
                                                    JWK::Unsupported(unsupported) => {
                                                        ProtoJwk {
                                                            jwk_type: Some(
                                                                JwkType::UnsupportedJwk(
                                                                    UnsupportedJwk {
                                                                        id: unsupported.id.clone(),
                                                                        payload: unsupported.payload.clone()
                                                                    }
                                                                )
                                                            )
                                                        }
                                                    }
                                                }
                                            }).collect(),
                                        }
                                    ),
                                    multi_sig: Some(aptos_protos::transaction::v1::validator_transaction::observed_jwk_update::ExportedAggregateSignature {
                                        signer_indices: observed_jwk_update.quorum_certified_update.multi_sig.signer_indices.clone().into_iter().map(|i| i as u64).collect(),
                                        sig: match &observed_jwk_update.quorum_certified_update.multi_sig.sig {
                                            Some(sig) =>  sig.0.clone(),
                                            None => vec![],
                                        },
                                    }),
                                }
                            )
                        },
                    )
                )
            },
        },
        events: convert_events(api_validator_txn.events()),
    })
}
