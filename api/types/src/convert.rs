// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    transaction::{
        DecodedTableData, DeleteModule, DeleteResource, DeleteTableItem, DeletedTableData,
        ModuleBundlePayload, MultisigPayload, MultisigTransactionPayload,
        StateCheckpointTransaction, UserTransactionRequestInner, WriteModule, WriteResource,
        WriteTableItem,
    },
    view::{ViewFunction, ViewRequest},
    Bytecode, DirectWriteSet, EntryFunctionId, EntryFunctionPayload, Event, HexEncodedBytes,
    MoveFunction, MoveModuleBytecode, MoveResource, MoveScriptBytecode, MoveType, MoveValue,
    PendingTransaction, ResourceGroup, ScriptPayload, ScriptWriteSet, SubmitTransactionRequest,
    Transaction, TransactionInfo, TransactionOnChainData, TransactionPayload,
    UserTransactionRequest, VersionedEvent, WriteSet, WriteSetChange, WriteSetPayload,
};
use anyhow::{bail, ensure, format_err, Context as AnyhowContext, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::DbReader;
use aptos_types::{
    access_path::{AccessPath, Path},
    chain_id::ChainId,
    contract_event::{ContractEvent, EventWithVersion},
    state_store::{
        state_key::{StateKey, StateKeyInner},
        table::TableHandle,
    },
    transaction::{
        EntryFunction, ExecutionStatus, ModuleBundle, Multisig, RawTransaction, Script,
        SignedTransaction,
    },
    vm_status::AbortLocation,
    write_set::WriteOp,
};
use move_binary_format::file_format::FunctionHandleIndex;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    resolver::ModuleResolver,
    value::{LayoutTag, MoveStructLayout, MoveTypeLayout},
};
use move_resource_viewer::MoveValueAnnotator;
use serde_json::Value;
use std::{
    convert::{TryFrom, TryInto},
    iter::IntoIterator,
    rc::Rc,
    sync::Arc,
};

const OBJECT_MODULE: &IdentStr = ident_str!("object");
const OBJECT_STRUCT: &IdentStr = ident_str!("Object");

/// The Move converter for converting Move types to JSON
///
/// This reads the underlying BCS types and ABIs to convert them into
/// JSON outputs
pub struct MoveConverter<'a, R: ?Sized> {
    inner: MoveValueAnnotator<'a, R>,
    db: Arc<dyn DbReader>,
}

impl<'a, R: ModuleResolver + ?Sized> MoveConverter<'a, R> {
    pub fn new(inner: &'a R, db: Arc<dyn DbReader>) -> Self {
        Self {
            inner: MoveValueAnnotator::new(inner),
            db,
        }
    }

    pub fn try_into_resources<'b>(
        &self,
        data: impl Iterator<Item = (StructTag, &'b [u8])>,
    ) -> Result<Vec<MoveResource>> {
        data.map(|(typ, bytes)| self.try_into_resource(&typ, bytes))
            .collect()
    }

    pub fn try_into_resource(&self, typ: &StructTag, bytes: &'_ [u8]) -> Result<MoveResource> {
        self.inner.view_resource(typ, bytes)?.try_into()
    }

    pub fn try_into_resources_from_resource_group(
        &self,
        bytes: &[u8],
    ) -> Result<Vec<MoveResource>> {
        let resources_with_tag: Vec<(StructTag, Vec<u8>)> = bcs::from_bytes::<ResourceGroup>(bytes)
            .map(|map| {
                map.into_iter()
                    .map(|(key, value)| (key, value))
                    .collect::<Vec<_>>()
            })?;

        resources_with_tag
            .iter()
            .map(|(struct_tag, value_bytes)| self.try_into_resource(struct_tag, value_bytes))
            .collect::<Result<Vec<_>>>()
    }

    pub fn move_struct_fields(
        &self,
        typ: &StructTag,
        bytes: &'_ [u8],
    ) -> Result<Vec<(Identifier, move_core_types::value::MoveValue)>> {
        self.inner.move_struct_fields(typ, bytes)
    }

    pub fn try_into_pending_transaction(&self, txn: SignedTransaction) -> Result<Transaction> {
        let payload = self.try_into_transaction_payload(txn.payload().clone())?;
        Ok((txn, payload).into())
    }

    pub fn try_into_pending_transaction_poem(
        &self,
        txn: SignedTransaction,
    ) -> Result<PendingTransaction> {
        let payload = self.try_into_transaction_payload(txn.payload().clone())?;
        Ok((txn, payload).into())
    }

    pub fn try_into_onchain_transaction(
        &self,
        timestamp: u64,
        data: TransactionOnChainData,
    ) -> Result<Transaction> {
        use aptos_types::transaction::Transaction::*;
        let info = self.into_transaction_info(
            data.version,
            &data.info,
            data.accumulator_root_hash,
            data.changes,
        );
        let events = self.try_into_events(&data.events)?;
        Ok(match data.transaction {
            UserTransaction(txn) => {
                let payload = self.try_into_transaction_payload(txn.payload().clone())?;
                (&txn, info, payload, events, timestamp).into()
            },
            GenesisTransaction(write_set) => {
                let payload = self.try_into_write_set_payload(write_set)?;
                (info, payload, events).into()
            },
            BlockMetadata(txn) => (&txn, info, events).into(),
            StateCheckpoint(_) => {
                Transaction::StateCheckpointTransaction(StateCheckpointTransaction {
                    info,
                    timestamp: timestamp.into(),
                })
            },
            ValidatorTransaction(_txn) => (info, events, timestamp).into(),
        })
    }

    pub fn into_transaction_info(
        &self,
        version: u64,
        info: &aptos_types::transaction::TransactionInfo,
        accumulator_root_hash: HashValue,
        write_set: aptos_types::write_set::WriteSet,
    ) -> TransactionInfo {
        TransactionInfo {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_change_hash: info.state_change_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            state_checkpoint_hash: info.state_checkpoint_hash().map(|h| h.into()),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),
            vm_status: self.explain_vm_status(info.status()),
            accumulator_root_hash: accumulator_root_hash.into(),
            // TODO: the resource value is interpreted by the type definition at the version of the converter, not the version of the tx: must be fixed before we allow module updates
            changes: write_set
                .into_iter()
                .filter_map(|(sk, wo)| self.try_into_write_set_changes(sk, wo).ok())
                .flatten()
                .collect(),
            block_height: None,
            epoch: None,
        }
    }

    pub fn try_into_transaction_payload(
        &self,
        payload: aptos_types::transaction::TransactionPayload,
    ) -> Result<TransactionPayload> {
        use aptos_types::transaction::TransactionPayload::*;
        let ret = match payload {
            Script(s) => TransactionPayload::ScriptPayload(s.try_into()?),
            EntryFunction(fun) => {
                let (module, function, ty_args, args) = fun.into_inner();
                let func_args = self
                    .inner
                    .view_function_arguments(&module, &function, &ty_args, &args);

                let json_args = match func_args {
                    Ok(values) => values
                        .into_iter()
                        .map(|v| MoveValue::try_from(v)?.json())
                        .collect::<Result<_>>()?,
                    Err(_e) => args
                        .into_iter()
                        .map(|arg| HexEncodedBytes::from(arg).json())
                        .collect::<Result<_>>()?,
                };

                TransactionPayload::EntryFunctionPayload(EntryFunctionPayload {
                    arguments: json_args,
                    function: EntryFunctionId {
                        module: module.into(),
                        name: function.into(),
                    },
                    type_arguments: ty_args.into_iter().map(|arg| arg.into()).collect(),
                })
            },
            Multisig(multisig) => {
                let transaction_payload = if let Some(payload) = multisig.transaction_payload {
                    match payload {
                        aptos_types::transaction::MultisigTransactionPayload::EntryFunction(
                            entry_function,
                        ) => {
                            let (module, function, ty_args, args) = entry_function.into_inner();
                            let func_args = self
                                .inner
                                .view_function_arguments(&module, &function, &ty_args, &args);
                            let json_args = match func_args {
                                Ok(values) => values
                                    .into_iter()
                                    .map(|v| MoveValue::try_from(v)?.json())
                                    .collect::<Result<_>>()?,
                                Err(_e) => args
                                    .into_iter()
                                    .map(|arg| HexEncodedBytes::from(arg).json())
                                    .collect::<Result<_>>()?,
                            };

                            Some(MultisigTransactionPayload::EntryFunctionPayload(
                                EntryFunctionPayload {
                                    arguments: json_args,
                                    function: EntryFunctionId {
                                        module: module.into(),
                                        name: function.into(),
                                    },
                                    type_arguments: ty_args
                                        .into_iter()
                                        .map(|arg| arg.into())
                                        .collect(),
                                },
                            ))
                        },
                    }
                } else {
                    None
                };
                TransactionPayload::MultisigPayload(MultisigPayload {
                    multisig_address: multisig.multisig_address.into(),
                    transaction_payload,
                })
            },

            // Deprecated. Will be removed in the future.
            ModuleBundle(modules) => TransactionPayload::ModuleBundlePayload(ModuleBundlePayload {
                modules: modules
                    .into_iter()
                    .map(|module| MoveModuleBytecode::from(module).try_parse_abi())
                    .collect::<Result<Vec<_>>>()?,
            }),
        };
        Ok(ret)
    }

    pub fn try_into_write_set_payload(
        &self,
        payload: aptos_types::transaction::WriteSetPayload,
    ) -> Result<WriteSetPayload> {
        use aptos_types::transaction::WriteSetPayload::*;
        let ret = match payload {
            Script { execute_as, script } => WriteSetPayload {
                write_set: WriteSet::ScriptWriteSet(ScriptWriteSet {
                    execute_as: execute_as.into(),
                    script: script.try_into()?,
                }),
            },
            Direct(d) => {
                let (write_set, events) = d.into_inner();
                let nested_writeset_changes: Vec<Vec<WriteSetChange>> = write_set
                    .into_iter()
                    .map(|(state_key, op)| self.try_into_write_set_changes(state_key, op))
                    .collect::<Result<Vec<Vec<_>>>>()?;
                WriteSetPayload {
                    write_set: WriteSet::DirectWriteSet(DirectWriteSet {
                        // TODO: the resource value is interpreted by the type definition at the version of the converter, not the version of the tx: must be fixed before we allow module updates
                        changes: nested_writeset_changes
                            .into_iter()
                            .flatten()
                            .collect::<Vec<WriteSetChange>>(),
                        events: self.try_into_events(&events)?,
                    }),
                }
            },
        };
        Ok(ret)
    }

    pub fn try_into_write_set_changes(
        &self,
        state_key: StateKey,
        op: WriteOp,
    ) -> Result<Vec<WriteSetChange>> {
        let hash = state_key.hash().to_hex_literal();
        let state_key = state_key.into_inner();
        match state_key {
            StateKeyInner::AccessPath(access_path) => {
                self.try_access_path_into_write_set_changes(hash, access_path, op)
            },
            StateKeyInner::TableItem { handle, key } => {
                vec![self.try_table_item_into_write_set_change(hash, handle, key, op)]
                    .into_iter()
                    .collect()
            },
            StateKeyInner::Raw(_) => Err(format_err!(
                "Can't convert account raw key {:?} to WriteSetChange",
                state_key
            )),
        }
    }

    pub fn try_access_path_into_write_set_changes(
        &self,
        state_key_hash: String,
        access_path: AccessPath,
        op: WriteOp,
    ) -> Result<Vec<WriteSetChange>> {
        let ret = match op.bytes() {
            None => match access_path.get_path() {
                Path::Code(module_id) => vec![WriteSetChange::DeleteModule(DeleteModule {
                    address: access_path.address.into(),
                    state_key_hash,
                    module: module_id.into(),
                })],
                Path::Resource(typ) => vec![WriteSetChange::DeleteResource(DeleteResource {
                    address: access_path.address.into(),
                    state_key_hash,
                    resource: typ.into(),
                })],
                Path::ResourceGroup(typ) => vec![WriteSetChange::DeleteResource(DeleteResource {
                    address: access_path.address.into(),
                    state_key_hash,
                    resource: typ.into(),
                })],
            },
            Some(bytes) => match access_path.get_path() {
                Path::Code(_) => vec![WriteSetChange::WriteModule(WriteModule {
                    address: access_path.address.into(),
                    state_key_hash,
                    data: MoveModuleBytecode::new(bytes.to_vec()).try_parse_abi()?,
                })],
                Path::Resource(typ) => vec![WriteSetChange::WriteResource(WriteResource {
                    address: access_path.address.into(),
                    state_key_hash,
                    data: self.try_into_resource(&typ, bytes)?,
                })],
                Path::ResourceGroup(_) => self
                    .try_into_resources_from_resource_group(bytes)?
                    .into_iter()
                    .map(|data| {
                        WriteSetChange::WriteResource(WriteResource {
                            address: access_path.address.into(),
                            state_key_hash: state_key_hash.clone(),
                            data,
                        })
                    })
                    .collect::<Vec<_>>(),
            },
        };
        Ok(ret)
    }

    pub fn try_table_item_into_write_set_change(
        &self,
        state_key_hash: String,
        handle: TableHandle,
        key: Vec<u8>,
        op: WriteOp,
    ) -> Result<WriteSetChange> {
        let hex_handle = handle.0.to_vec().into();
        let key: HexEncodedBytes = key.into();
        let ret = match op.bytes() {
            None => {
                let data = self.try_delete_table_item_into_deleted_table_data(handle, &key.0)?;

                WriteSetChange::DeleteTableItem(DeleteTableItem {
                    state_key_hash,
                    handle: hex_handle,
                    key,
                    data,
                })
            },
            Some(bytes) => {
                let data =
                    self.try_write_table_item_into_decoded_table_data(handle, &key.0, bytes)?;

                WriteSetChange::WriteTableItem(WriteTableItem {
                    state_key_hash,
                    handle: hex_handle,
                    key,
                    value: bytes.to_vec().into(),
                    data,
                })
            },
        };
        Ok(ret)
    }

    pub fn try_write_table_item_into_decoded_table_data(
        &self,
        handle: TableHandle,
        key: &[u8],
        value: &[u8],
    ) -> Result<Option<DecodedTableData>> {
        if !self.db.indexer_enabled() && !self.db.indexer_async_v2_enabled() {
            return Ok(None);
        }
        let table_info = match self.db.get_table_info(handle) {
            Ok(ti) => ti,
            Err(_) => {
                aptos_logger::warn!(
                    "Table info not found for handle {:?}, can't decode table item. OK for simulation",
                    handle
                );
                return Ok(None); // if table item not found return None anyway to avoid crash
            },
        };
        let key = self.try_into_move_value(&table_info.key_type, key)?;
        let value = self.try_into_move_value(&table_info.value_type, value)?;

        Ok(Some(DecodedTableData {
            key: key.json().unwrap(),
            key_type: table_info.key_type.to_string(),
            value: value.json().unwrap(),
            value_type: table_info.value_type.to_string(),
        }))
    }

    pub fn try_delete_table_item_into_deleted_table_data(
        &self,
        handle: TableHandle,
        key: &[u8],
    ) -> Result<Option<DeletedTableData>> {
        if !self.db.indexer_enabled() && !self.db.indexer_async_v2_enabled() {
            return Ok(None);
        }
        let table_info = match self.db.get_table_info(handle) {
            Ok(ti) => ti,
            Err(_) => {
                aptos_logger::warn!(
                    "Table info not found for handle {:?}, can't decode table item. OK for simulation",
                    handle
                );
                return Ok(None); // if table item not found return None anyway to avoid crash
            },
        };
        let key = self.try_into_move_value(&table_info.key_type, key)?;

        Ok(Some(DeletedTableData {
            key: key.json().unwrap(),
            key_type: table_info.key_type.to_string(),
        }))
    }

    pub fn try_into_events(&self, events: &[ContractEvent]) -> Result<Vec<Event>> {
        let mut ret = vec![];
        for event in events {
            let data = self
                .inner
                .view_value(event.type_tag(), event.event_data())?;
            ret.push((event, MoveValue::try_from(data)?.json()?).into());
        }
        Ok(ret)
    }

    pub fn try_into_versioned_events(
        &self,
        events: &[EventWithVersion],
    ) -> Result<Vec<VersionedEvent>> {
        let mut ret = vec![];
        for event in events {
            let data = self
                .inner
                .view_value(event.event.type_tag(), event.event.event_data())?;
            ret.push((event, MoveValue::try_from(data)?.json()?).into());
        }
        Ok(ret)
    }

    pub fn try_into_signed_transaction(
        &self,
        txn: UserTransactionRequest,
        chain_id: ChainId,
    ) -> Result<SignedTransaction> {
        let signature = txn
            .signature
            .clone()
            .ok_or_else(|| format_err!("missing signature"))?;
        Ok(SignedTransaction::new_with_authenticator(
            self.try_into_raw_transaction(txn, chain_id)?,
            signature.try_into()?,
        ))
    }

    pub fn try_into_signed_transaction_poem(
        &self,
        submit_transaction_request: SubmitTransactionRequest,
        chain_id: ChainId,
    ) -> Result<SignedTransaction> {
        Ok(SignedTransaction::new_with_authenticator(
            self.try_into_raw_transaction_poem(
                submit_transaction_request.user_transaction_request,
                chain_id,
            )?,
            submit_transaction_request.signature.try_into().context("Failed to parse transaction when building SignedTransaction from SubmitTransactionRequest")?,
        ))
    }

    pub fn try_into_raw_transaction(
        &self,
        txn: UserTransactionRequest,
        chain_id: ChainId,
    ) -> Result<RawTransaction> {
        let UserTransactionRequest {
            sender,
            sequence_number,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            payload,
            signature: _,
        } = txn;
        Ok(RawTransaction::new(
            sender.into(),
            sequence_number.into(),
            self.try_into_aptos_core_transaction_payload(payload)?,
            max_gas_amount.into(),
            gas_unit_price.into(),
            expiration_timestamp_secs.into(),
            chain_id,
        ))
    }

    pub fn try_into_raw_transaction_poem(
        &self,
        user_transaction_request: UserTransactionRequestInner,
        chain_id: ChainId,
    ) -> Result<RawTransaction> {
        let UserTransactionRequestInner {
            sender,
            sequence_number,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            payload,
        } = user_transaction_request;
        Ok(RawTransaction::new(
            sender.into(),
            sequence_number.into(),
            self.try_into_aptos_core_transaction_payload(payload)
                .context("Failed to parse transaction payload")?,
            max_gas_amount.into(),
            gas_unit_price.into(),
            expiration_timestamp_secs.into(),
            chain_id,
        ))
    }

    pub fn try_into_aptos_core_transaction_payload(
        &self,
        payload: TransactionPayload,
    ) -> Result<aptos_types::transaction::TransactionPayload> {
        use aptos_types::transaction::TransactionPayload as Target;

        let ret = match payload {
            TransactionPayload::EntryFunctionPayload(entry_func_payload) => {
                let EntryFunctionPayload {
                    function,
                    type_arguments,
                    arguments,
                } = entry_func_payload;

                let module = function.module.clone();
                let code = self.inner.get_module(&module.clone().into())? as Rc<dyn Bytecode>;
                let func = code
                    .find_entry_function(function.name.0.as_ident_str())
                    .ok_or_else(|| format_err!("could not find entry function by {}", function))?;
                ensure!(
                    func.generic_type_params.len() == type_arguments.len(),
                    "expect {} type arguments for entry function {}, but got {}",
                    func.generic_type_params.len(),
                    function,
                    type_arguments.len()
                );
                let args = self
                    .try_into_vm_values(func, arguments)?
                    .iter()
                    .map(bcs::to_bytes)
                    .collect::<Result<_, bcs::Error>>()?;

                Target::EntryFunction(EntryFunction::new(
                    module.into(),
                    function.name.into(),
                    type_arguments
                        .into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_>>()?,
                    args,
                ))
            },
            TransactionPayload::ScriptPayload(script) => {
                let ScriptPayload {
                    code,
                    type_arguments,
                    arguments,
                } = script;

                let MoveScriptBytecode { bytecode, abi } = code.try_parse_abi();
                match abi {
                    Some(func) => {
                        let args = self.try_into_vm_values(func, arguments)?;
                        Target::Script(Script::new(
                            bytecode.into(),
                            type_arguments
                                .into_iter()
                                .map(|v| v.try_into())
                                .collect::<Result<_>>()?,
                            args.into_iter()
                                .map(|arg| arg.try_into())
                                .collect::<Result<_>>()?,
                        ))
                    },
                    None => return Err(anyhow::anyhow!("invalid transaction script bytecode")),
                }
            },
            TransactionPayload::MultisigPayload(multisig) => {
                let transaction_payload = if let Some(payload) = multisig.transaction_payload {
                    match payload {
                        MultisigTransactionPayload::EntryFunctionPayload(entry_function) => {
                            let EntryFunctionPayload {
                                function,
                                type_arguments,
                                arguments,
                            } = entry_function;

                            let module = function.module.clone();
                            let code =
                                self.inner.get_module(&module.clone().into())? as Rc<dyn Bytecode>;
                            let func = code
                                .find_entry_function(function.name.0.as_ident_str())
                                .ok_or_else(|| {
                                    format_err!("could not find entry function by {}", function)
                                })?;
                            ensure!(
                                func.generic_type_params.len() == type_arguments.len(),
                                "expect {} type arguments for entry function {}, but got {}",
                                func.generic_type_params.len(),
                                function,
                                type_arguments.len()
                            );

                            let args = self
                                .try_into_vm_values(func, arguments)?
                                .iter()
                                .map(bcs::to_bytes)
                                .collect::<Result<_, bcs::Error>>()?;
                            Some(
                                aptos_types::transaction::MultisigTransactionPayload::EntryFunction(
                                    EntryFunction::new(
                                        module.into(),
                                        function.name.into(),
                                        type_arguments
                                            .into_iter()
                                            .map(|v| v.try_into())
                                            .collect::<Result<_>>()?,
                                        args,
                                    ),
                                ),
                            )
                        },
                    }
                } else {
                    None
                };
                Target::Multisig(Multisig {
                    multisig_address: multisig.multisig_address.into(),
                    transaction_payload,
                })
            },

            // Deprecated. Will be removed in the future.
            TransactionPayload::ModuleBundlePayload(payload) => {
                Target::ModuleBundle(ModuleBundle::new(
                    payload
                        .modules
                        .into_iter()
                        .map(|m| m.bytecode.into())
                        .collect(),
                ))
            },
        };
        Ok(ret)
    }

    pub fn try_into_vm_values(
        &self,
        func: MoveFunction,
        args: Vec<serde_json::Value>,
    ) -> Result<Vec<move_core_types::value::MoveValue>> {
        let arg_types = func
            .params
            .into_iter()
            .filter(|p| !p.is_signer())
            .collect::<Vec<_>>();
        ensure!(
            arg_types.len() == args.len(),
            "expected {} arguments [{}], but got {} ({:?})",
            arg_types.len(),
            arg_types
                .into_iter()
                .map(|t| t.json_type_name())
                .collect::<Vec<String>>()
                .join(", "),
            args.len(),
            args,
        );
        arg_types
            .into_iter()
            .zip(args)
            .enumerate()
            .map(|(i, (arg_type, arg))| {
                self.try_into_vm_value(&arg_type.clone().try_into()?, arg)
                    .map_err(|e| {
                        format_err!(
                            "parse arguments[{}] failed, expect {}, caused by error: {}",
                            i,
                            arg_type.json_type_name(),
                            e,
                        )
                    })
            })
            .collect::<Result<_>>()
    }

    // Converts JSON object to `MoveValue`, which can be bcs serialized into the same
    // representation in the DB.
    // Notice that structs are of the `MoveStruct::Runtime` flavor, matching the representation in
    // DB.
    pub fn try_into_vm_value(
        &self,
        type_tag: &TypeTag,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        let layout = match type_tag {
            TypeTag::Struct(boxed_struct) => {
                // The current framework can't handle generics, so we handle this here
                if boxed_struct.address == AccountAddress::ONE
                    && boxed_struct.module.as_ident_str() == OBJECT_MODULE
                    && boxed_struct.name.as_ident_str() == OBJECT_STRUCT
                {
                    // Objects are just laid out as an address
                    MoveTypeLayout::Address
                } else {
                    // For all other structs, use their set layout
                    self.inner.get_type_layout_with_types(type_tag)?
                }
            },
            _ => self.inner.get_type_layout_with_types(type_tag)?,
        };

        self.try_into_vm_value_from_layout(&layout, val)
    }

    fn try_into_vm_value_from_layout(
        &self,
        layout: &MoveTypeLayout,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        use move_core_types::value::MoveValue::*;
        Ok(match layout {
            MoveTypeLayout::Bool => Bool(serde_json::from_value::<bool>(val)?),
            MoveTypeLayout::U8 => U8(serde_json::from_value::<u8>(val)?),
            MoveTypeLayout::U16 => U16(serde_json::from_value::<u16>(val)?),
            MoveTypeLayout::U32 => U32(serde_json::from_value::<u32>(val)?),
            MoveTypeLayout::U64 => serde_json::from_value::<crate::U64>(val)?.into(),
            MoveTypeLayout::U128 => serde_json::from_value::<crate::U128>(val)?.into(),
            MoveTypeLayout::U256 => serde_json::from_value::<crate::U256>(val)?.into(),
            MoveTypeLayout::Address => serde_json::from_value::<crate::Address>(val)?.into(),
            MoveTypeLayout::Vector(item_layout) => {
                self.try_into_vm_value_vector(item_layout.as_ref(), val)?
            },
            MoveTypeLayout::Struct(struct_layout) => {
                self.try_into_vm_value_struct(struct_layout, val)?
            },
            MoveTypeLayout::Signer => {
                bail!("unexpected move type {:?} for value {:?}", layout, val)
            },
            MoveTypeLayout::Tagged(tag, inner_layout) => match tag {
                LayoutTag::IdentifierMapping(_) => {
                    self.try_into_vm_value_from_layout(inner_layout, val)?
                },
            },
        })
    }

    pub fn try_into_vm_value_vector(
        &self,
        layout: &MoveTypeLayout,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        if matches!(layout, MoveTypeLayout::U8) {
            Ok(serde_json::from_value::<HexEncodedBytes>(val)?.into())
        } else if let Value::Array(list) = val {
            let vals = list
                .into_iter()
                .map(|v| self.try_into_vm_value_from_layout(layout, v))
                .collect::<Result<_>>()?;

            Ok(move_core_types::value::MoveValue::Vector(vals))
        } else {
            bail!("expected vector<{:?}>, but got: {:?}", layout, val)
        }
    }

    pub fn try_into_vm_value_struct(
        &self,
        layout: &MoveStructLayout,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        let (struct_tag, field_layouts) =
            if let MoveStructLayout::WithTypes { type_, fields } = layout {
                (type_, fields)
            } else {
                bail!(
                    "Expecting `MoveStructLayout::WithTypes`, getting {:?}",
                    layout
                );
            };
        if MoveValue::is_utf8_string(struct_tag) {
            let string = val
                .as_str()
                .ok_or_else(|| format_err!("failed to parse string::String."))?;
            return Ok(new_vm_utf8_string(string));
        }

        let mut field_values = if let Value::Object(fields) = val {
            fields
        } else {
            bail!("Expecting a JSON Map for struct.");
        };
        let fields = field_layouts
            .iter()
            .map(|field_layout| {
                let name = field_layout.name.as_str();
                let value = field_values
                    .remove(name)
                    .ok_or_else(|| format_err!("field {} not found.", name))?;
                let move_value = self.try_into_vm_value_from_layout(&field_layout.layout, value)?;
                Ok(move_value)
            })
            .collect::<Result<_>>()?;

        Ok(move_core_types::value::MoveValue::Struct(
            move_core_types::value::MoveStruct::Runtime(fields),
        ))
    }

    pub fn try_into_move_value(&self, typ: &TypeTag, bytes: &[u8]) -> Result<MoveValue> {
        self.inner.view_value(typ, bytes)?.try_into()
    }

    pub fn function_return_types(&self, function: &ViewFunction) -> Result<Vec<MoveType>> {
        let module = function.module.clone();
        let code = self.inner.get_module(&module)? as Rc<dyn Bytecode>;
        let func = code
            .find_function(function.function.as_ident_str())
            .ok_or_else(|| format_err!("could not find entry function by {:?}", function))?;

        Ok(func.return_)
    }

    pub fn convert_view_function(&self, view_request: ViewRequest) -> Result<ViewFunction> {
        let ViewRequest {
            function,
            type_arguments,
            arguments,
        } = view_request;

        let module = function.module.clone();
        let code = self.inner.get_module(&module.clone().into())? as Rc<dyn Bytecode>;
        let func = code
            .find_function(function.name.0.as_ident_str())
            .ok_or_else(|| format_err!("could not find entry function by {}", function))?;
        ensure!(
            func.generic_type_params.len() == type_arguments.len(),
            "expected {} type arguments for entry function {}, but got {}",
            func.generic_type_params.len(),
            function,
            type_arguments.len()
        );
        let args = self
            .try_into_vm_values(func, arguments)?
            .iter()
            .map(bcs::to_bytes)
            .collect::<Result<_, bcs::Error>>()?;

        Ok(ViewFunction {
            module: module.into(),
            function: function.name.into(),
            ty_args: type_arguments
                .into_iter()
                .map(|v| v.try_into())
                .collect::<Result<_>>()?,
            args,
        })
    }
}

impl<'a, R: ModuleResolver + ?Sized> ExplainVMStatus for MoveConverter<'a, R> {
    fn get_module_bytecode(&self, module_id: &ModuleId) -> Result<Rc<dyn Bytecode>> {
        self.inner
            .get_module(module_id)
            .map(|inner| inner as Rc<dyn Bytecode>)
    }
}
pub trait AsConverter<R> {
    fn as_converter(&self, db: Arc<dyn DbReader>) -> MoveConverter<R>;
}

impl<R: ModuleResolver> AsConverter<R> for R {
    fn as_converter(&self, db: Arc<dyn DbReader>) -> MoveConverter<R> {
        MoveConverter::new(self, db)
    }
}

pub fn new_vm_utf8_string(string: &str) -> move_core_types::value::MoveValue {
    use move_core_types::value::{MoveStruct, MoveValue};

    let byte_vector = MoveValue::Vector(
        string
            .as_bytes()
            .iter()
            .map(|byte| MoveValue::U8(*byte))
            .collect(),
    );
    let move_string = MoveStruct::Runtime(vec![byte_vector]);
    MoveValue::Struct(move_string)
}

fn abort_location_to_str(loc: &AbortLocation) -> String {
    match loc {
        AbortLocation::Module(mid) => {
            format!("{}::{}", mid.address().to_hex_literal(), mid.name())
        },
        _ => loc.to_string(),
    }
}

pub trait ExplainVMStatus {
    fn get_module_bytecode(&self, module_id: &ModuleId) -> Result<Rc<dyn Bytecode>>;

    fn explain_vm_status(&self, status: &ExecutionStatus) -> String {
        match status {
            ExecutionStatus::MoveAbort { location, code, info } => match &location {
                AbortLocation::Module(_) => {
                    info.as_ref().map(|i| {
                        format!("Move abort in {}: {}({:#x}): {}", abort_location_to_str(location), i.reason_name, code, i.description)
                    }).unwrap_or_else(|| {
                        format!("Move abort in {}: {:#x}", abort_location_to_str(location), code)
                    })
                }
                AbortLocation::Script => format!("Move abort: code {:#x}", code),
            },
            ExecutionStatus::Success => "Executed successfully".to_owned(),
            ExecutionStatus::OutOfGas => "Out of gas".to_owned(),
            ExecutionStatus::ExecutionFailure {
                location,
                function,
                code_offset,
            } => {
                let func_name = match location {
                    AbortLocation::Module(module_id) => self.explain_function_index(module_id, function)
                        .map(|name| format!("{}::{}", abort_location_to_str(location), name))
                        .unwrap_or_else(|_| format!("{}::<#{} function>", abort_location_to_str(location), function)),
                    AbortLocation::Script => "script".to_owned(),
                };
                format!(
                    "Execution failed in {} at code offset {}",
                    func_name, code_offset
                )
            }
            ExecutionStatus::MiscellaneousError(code) => {
                code.map_or(
                    "Move bytecode deserialization / verification failed, including entry function not found or invalid arguments".to_owned(),
                    |e| format!(
                        "Transaction Executed and Committed with Error {:#?}", e
                    ),
                )
            }
        }
    }

    fn explain_function_index(&self, module_id: &ModuleId, function: &u16) -> Result<String> {
        let code = self.get_module_bytecode(module_id)?;
        let func = code.function_handle_at(FunctionHandleIndex::new(*function));
        let id = code.identifier_at(func.name);
        Ok(id.to_string())
    }
}

// TODO: add caching?
