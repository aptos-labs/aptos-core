// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::transaction::{
    EntryFunction, MultisigTransactionPayload, TransactionExecutableRef, TransactionPayload,
    TransactionPayloadInner,
};
use move_binary_format::deserializer::DeserializerConfig;
use move_core_types::{
    identifier::IdentStr,
    language_storage::TypeTag,
    vm_status::{StatusCode, VMStatus},
};

/// Validates structural limits of a transaction payload:
///   - Identifier lengths inside type tags and entry function module or
///     function names,
///   - Type arguments list size,
///   - Per-tag node count,
///   - Rejects function type tags.
pub fn validate_transaction_payload(
    payload: &TransactionPayload,
    config: &DeserializerConfig,
) -> Result<(), VMStatus> {
    match payload {
        TransactionPayload::EntryFunction(entry_fn) => {
            validate_entry_function(entry_fn, config)?;
        },
        TransactionPayload::Script(script) => {
            validate_type_args(script.ty_args(), config)?;
        },
        TransactionPayload::Multisig(multisig) => {
            if let Some(payload) = &multisig.transaction_payload {
                match payload {
                    MultisigTransactionPayload::EntryFunction(entry_fn) => {
                        validate_entry_function(entry_fn, config)?;
                    },
                    MultisigTransactionPayload::Script(script) => {
                        validate_type_args(script.ty_args(), config)?;
                    },
                }
            }
        },
        TransactionPayload::Payload(inner) => match inner {
            TransactionPayloadInner::V1 { executable, .. } => {
                validate_executable(&executable.as_ref(), config)?;
            },
        },
        TransactionPayload::EncryptedPayload(inner) => {
            if let Ok(executable) = inner.executable_ref() {
                validate_executable(&executable, config)?;
            }
        },
        TransactionPayload::ModuleBundle(_) => {
            // Deprecated: do nothing.
        },
    }
    Ok(())
}

fn validate_executable(
    executable: &TransactionExecutableRef,
    config: &DeserializerConfig,
) -> Result<(), VMStatus> {
    match executable {
        TransactionExecutableRef::Script(script) => {
            validate_type_args(script.ty_args(), config)?;
        },
        TransactionExecutableRef::EntryFunction(entry_fn) => {
            validate_entry_function(entry_fn, config)?;
        },
        TransactionExecutableRef::Empty | TransactionExecutableRef::Encrypted => {},
    }
    Ok(())
}

fn validate_entry_function(
    entry_fn: &EntryFunction,
    config: &DeserializerConfig,
) -> Result<(), VMStatus> {
    validate_identifier(entry_fn.module().name(), config)?;
    validate_identifier(entry_fn.function(), config)?;
    validate_type_args(entry_fn.ty_args(), config)
}

fn validate_type_args(ty_args: &[TypeTag], config: &DeserializerConfig) -> Result<(), VMStatus> {
    if ty_args.len() as u64 > config.max_entry_type_args_count {
        return Err(VMStatus::error(
            StatusCode::MALFORMED_TRANSACTION_PAYLOAD,
            Some(format!(
                "Type arguments count {} exceeds limit {}",
                ty_args.len(),
                config.max_entry_type_args_count
            )),
        ));
    }

    for ty_arg in ty_args {
        validate_type_tag(ty_arg, config)?;
    }
    Ok(())
}

fn validate_type_tag(ty: &TypeTag, config: &DeserializerConfig) -> Result<(), VMStatus> {
    let mut node_count: u64 = 0;
    for tag in ty.preorder_traversal_iter() {
        // Node count check precedes the match below, so an oversized tag
        // containing a function type reports the node-count error rather
        // than the function-type error. Both are rejections.
        node_count += 1;
        if node_count > config.max_entry_type_tag_nodes {
            return Err(VMStatus::error(
                StatusCode::MALFORMED_TRANSACTION_PAYLOAD,
                Some(format!(
                    "Type tag node count exceeds limit {}",
                    config.max_entry_type_tag_nodes
                )),
            ));
        }

        match tag {
            TypeTag::Struct(struct_tag) => {
                validate_identifier(struct_tag.module.as_ident_str(), config)?;
                validate_identifier(struct_tag.name.as_ident_str(), config)?;
            },
            TypeTag::Function(_) => {
                return Err(VMStatus::error(
                    StatusCode::MALFORMED_TRANSACTION_PAYLOAD,
                    Some("Function type tags are not allowed in transaction payloads".to_string()),
                ));
            },
            TypeTag::Bool => {},
            TypeTag::U8 => {},
            TypeTag::U16 => {},
            TypeTag::U32 => {},
            TypeTag::U64 => {},
            TypeTag::U128 => {},
            TypeTag::U256 => {},
            TypeTag::I8
            | TypeTag::I16
            | TypeTag::I32
            | TypeTag::I64
            | TypeTag::I128
            | TypeTag::I256
            | TypeTag::Address
            | TypeTag::Signer
            | TypeTag::Vector(_) => {
                // Allowed.
            },
        }
    }
    Ok(())
}

fn validate_identifier(identifier: &IdentStr, config: &DeserializerConfig) -> Result<(), VMStatus> {
    let len = identifier.as_str().len() as u64;
    if len > config.max_identifier_size {
        return Err(VMStatus::error(
            StatusCode::MALFORMED_TRANSACTION_PAYLOAD,
            Some(format!(
                "Identifier length {} exceeds limit {}",
                len, config.max_identifier_size
            )),
        ));
    }
    if !IdentStr::is_valid(identifier.as_str()) {
        return Err(VMStatus::error(
            StatusCode::MALFORMED_TRANSACTION_PAYLOAD,
            Some(format!(
                "Identifier '{}' contains invalid characters",
                identifier.as_str()
            )),
        ));
    }
    Ok(())
}
