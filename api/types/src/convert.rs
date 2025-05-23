// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    transaction::{
        BlockEpilogueTransaction, BlockMetadataTransaction, DecodedTableData, DeleteModule,
        DeleteResource, DeleteTableItem, DeletedTableData, MultisigPayload,
        MultisigTransactionPayload, StateCheckpointTransaction, UserTransactionRequestInner,
        WriteModule, WriteResource, WriteTableItem,
    },
    view::{ViewFunction, ViewRequest},
    Address, Bytecode, DirectWriteSet, EntryFunctionId, EntryFunctionPayload, Event,
    HexEncodedBytes, MoveFunction, MoveModuleBytecode, MoveResource, MoveScriptBytecode, MoveType,
    MoveValue, PendingTransaction, ResourceGroup, ScriptPayload, ScriptWriteSet,
    SubmitTransactionRequest, Transaction, TransactionInfo, TransactionOnChainData,
    TransactionPayload, VersionedEvent, WriteSet, WriteSetChange, WriteSetPayload,
};
use anyhow::{bail, ensure, format_err, Context as AnyhowContext, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::{sample, sample::SampleRate};
use aptos_resource_viewer::AptosValueAnnotator;
use aptos_storage_interface::DbReader;
use aptos_types::{
    access_path::{AccessPath, Path},
    chain_id::ChainId,
    contract_event::{ContractEvent, EventWithVersion},
    indexer::indexer_db_reader::IndexerReader,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        table::{TableHandle, TableInfo},
        StateView,
    },
    transaction::{
        BlockEndInfo, BlockEpiloguePayload, EntryFunction, ExecutionStatus, Multisig,
        RawTransaction, Script, SignedTransaction, TransactionAuxiliaryData,
    },
    vm::module_metadata::get_metadata,
    vm_status::AbortLocation,
    write_set::WriteOp,
};
use bytes::Bytes;
use move_binary_format::file_format::FunctionHandleIndex;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag, TypeTag},
    transaction_argument::convert_txn_args,
    value::{MoveStructLayout, MoveTypeLayout},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
    iter::IntoIterator,
    sync::Arc,
    time::Duration,
};

const OBJECT_MODULE: &IdentStr = ident_str!("object");
const OBJECT_STRUCT: &IdentStr = ident_str!("Object");

/// The Move converter for converting Move types to JSON
///
/// This reads the underlying BCS types and ABIs to convert them into
/// JSON outputs
pub struct MoveConverter<'a, S> {
    inner: AptosValueAnnotator<'a, S>,
    db: Arc<dyn DbReader>,
    indexer_reader: Option<Arc<dyn IndexerReader>>,
}

impl<'a, S: StateView> MoveConverter<'a, S> {
    pub fn new(
        inner: &'a S,
        db: Arc<dyn DbReader>,
        indexer_reader: Option<Arc<dyn IndexerReader>>,
    ) -> Self {
        Self {
            inner: AptosValueAnnotator::new(inner),
            db,
            indexer_reader,
        }
    }

    pub fn try_into_resources<'b>(
        &self,
        data: impl Iterator<Item = (StructTag, &'b [u8])>,
    ) -> Result<Vec<MoveResource>> {
        data.map(|(typ, bytes)| self.inner.view_resource(&typ, bytes)?.try_into())
            .collect()
    }

    pub fn try_into_resource(&self, tag: &StructTag, bytes: &'_ [u8]) -> Result<MoveResource> {
        self.inner.view_resource(tag, bytes)?.try_into()
    }

    pub fn is_resource_group(&self, tag: &StructTag) -> bool {
        if let Ok(Some(module)) = self.inner.view_module(&tag.module_id()) {
            if let Some(md) = get_metadata(&module.metadata) {
                if let Some(attrs) = md.struct_attributes.get(tag.name.as_ident_str().as_str()) {
                    return attrs
                        .iter()
                        .find(|attr| attr.is_resource_group())
                        .map(|_| true)
                        .unwrap_or(false);
                }
            }
        }
        false
    }

    pub fn find_resource(
        &self,
        state_view: &impl StateView,
        address: Address,
        tag: &StructTag,
    ) -> Result<Option<Bytes>> {
        Ok(match self.inner.view_resource_group_member(tag) {
            Some(group_tag) => {
                let key = StateKey::resource_group(&address.into(), &group_tag);
                match state_view.get_state_value_bytes(&key)? {
                    Some(group_bytes) => {
                        let group: BTreeMap<StructTag, Bytes> = bcs::from_bytes(&group_bytes)?;
                        group.get(tag).cloned()
                    },
                    None => None,
                }
            },
            None => {
                let key = StateKey::resource(&address.into(), tag)?;
                state_view.get_state_value_bytes(&key)?
            },
        })
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
    ) -> Result<(
        Option<Identifier>,
        Vec<(Identifier, move_core_types::value::MoveValue)>,
    )> {
        self.inner.view_struct_fields(typ, bytes)
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
        use aptos_types::transaction::Transaction::{
            BlockEpilogue, BlockMetadata, BlockMetadataExt, GenesisTransaction, StateCheckpoint,
            UserTransaction,
        };
        let aux_data = self
            .db
            .get_transaction_auxiliary_data_by_version(data.version)?;
        let info = self.into_transaction_info(
            data.version,
            &data.info,
            data.accumulator_root_hash,
            data.changes,
            aux_data,
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
            BlockMetadata(txn) => Transaction::BlockMetadataTransaction(
                BlockMetadataTransaction::from_internal(txn, info, events),
            ),
            BlockMetadataExt(txn) => Transaction::BlockMetadataTransaction(
                BlockMetadataTransaction::from_internal_ext(txn, info, events),
            ),
            StateCheckpoint(_) => {
                Transaction::StateCheckpointTransaction(StateCheckpointTransaction {
                    info,
                    timestamp: timestamp.into(),
                })
            },
            BlockEpilogue(block_epilogue_payload) => {
                Transaction::BlockEpilogueTransaction(BlockEpilogueTransaction {
                    info,
                    timestamp: timestamp.into(),
                    block_end_info: match block_epilogue_payload {
                        BlockEpiloguePayload::V0 {
                            block_end_info:
                                BlockEndInfo::V0 {
                                    block_gas_limit_reached,
                                    block_output_limit_reached,
                                    block_effective_block_gas_units,
                                    block_approx_output_size,
                                },
                            ..
                        } => Some(crate::transaction::BlockEndInfo {
                            block_gas_limit_reached,
                            block_output_limit_reached,
                            block_effective_block_gas_units,
                            block_approx_output_size,
                        }),
                    },
                })
            },
            aptos_types::transaction::Transaction::ValidatorTransaction(txn) => {
                Transaction::ValidatorTransaction((txn, info, events, timestamp).into())
            },
        })
    }

    pub fn into_transaction_info(
        &self,
        version: u64,
        info: &aptos_types::transaction::TransactionInfo,
        accumulator_root_hash: HashValue,
        write_set: aptos_types::write_set::WriteSet,
        txn_aux_data: Option<TransactionAuxiliaryData>,
    ) -> TransactionInfo {
        TransactionInfo {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_change_hash: info.state_change_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            state_checkpoint_hash: info.state_checkpoint_hash().map(|h| h.into()),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),
            vm_status: self.explain_vm_status(info.status(), txn_aux_data),
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
        use aptos_types::transaction::{EntryFunction, Script, TransactionPayload::*};

        let try_into_script_payload = |s: Script| -> Result<ScriptPayload> {
            let (code, ty_args, args) = s.into_inner();
            let script_args = self.inner.view_script_arguments(&code, &args, &ty_args);

            let json_args = match script_args {
                Ok(values) => values
                    .into_iter()
                    .map(|v| MoveValue::try_from(v)?.json())
                    .collect::<Result<_>>()?,
                Err(_e) => convert_txn_args(&args)
                    .into_iter()
                    .map(|arg| HexEncodedBytes::from(arg).json())
                    .collect::<Result<_>>()?,
            };

            Ok(ScriptPayload {
                code: MoveScriptBytecode::new(code).try_parse_abi(),
                type_arguments: ty_args.iter().map(|arg| arg.into()).collect(),
                arguments: json_args,
            })
        };

        let try_into_entry_function_payload = |fun: EntryFunction| -> Result<EntryFunctionPayload> {
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

            Ok(EntryFunctionPayload {
                arguments: json_args,
                function: EntryFunctionId {
                    module: module.into(),
                    name: function.into(),
                },
                type_arguments: ty_args.iter().map(|arg| arg.into()).collect(),
            })
        };

        let ret = match payload {
            Script(s) => TransactionPayload::ScriptPayload(try_into_script_payload(s)?),
            EntryFunction(fun) => {
                TransactionPayload::EntryFunctionPayload(try_into_entry_function_payload(fun)?)
            },
            Multisig(multisig) => {
                let transaction_payload = if let Some(payload) = multisig.transaction_payload {
                    match payload {
                        aptos_types::transaction::MultisigTransactionPayload::EntryFunction(
                            entry_function,
                        ) => {
                            let entry_function_payload =
                                try_into_entry_function_payload(entry_function)?;
                            Some(MultisigTransactionPayload::EntryFunctionPayload(
                                entry_function_payload,
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
            Payload(aptos_types::transaction::TransactionPayloadInner::V1 {
                executable,
                extra_config,
            }) => match extra_config {
                aptos_types::transaction::TransactionExtraConfig::V1 {
                    multisig_address,
                    replay_protection_nonce: _,
                } => {
                    if let Some(multisig_address) = multisig_address {
                        match executable {
                            aptos_types::transaction::TransactionExecutable::EntryFunction(
                                entry_function,
                            ) => TransactionPayload::MultisigPayload(MultisigPayload {
                                multisig_address: multisig_address.into(),
                                transaction_payload: Some(
                                    MultisigTransactionPayload::EntryFunctionPayload(
                                        try_into_entry_function_payload(entry_function)?,
                                    ),
                                ),
                            }),
                            aptos_types::transaction::TransactionExecutable::Script(_) => {
                                bail!(
                                    "Script executable is not supported for multisig transactions"
                                )
                            },
                            aptos_types::transaction::TransactionExecutable::Empty => {
                                TransactionPayload::MultisigPayload(MultisigPayload {
                                    multisig_address: multisig_address.into(),
                                    transaction_payload: None,
                                })
                            },
                        }
                    } else {
                        match executable {
                            aptos_types::transaction::TransactionExecutable::EntryFunction(
                                entry_function,
                            ) => TransactionPayload::EntryFunctionPayload(
                                try_into_entry_function_payload(entry_function)?,
                            ),
                            aptos_types::transaction::TransactionExecutable::Script(script) => {
                                TransactionPayload::ScriptPayload(try_into_script_payload(script)?)
                            },
                            aptos_types::transaction::TransactionExecutable::Empty => {
                                bail!("Empty executable is not supported for non-multisig transactions")
                            },
                        }
                    }
                },
            },
            // Deprecated.
            ModuleBundle(_) => bail!("Module bundle payload has been removed"),
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
        let state_key = state_key.inner();
        match state_key {
            StateKeyInner::AccessPath(access_path) => {
                self.try_access_path_into_write_set_changes(hash, access_path, op)
            },
            StateKeyInner::TableItem { handle, key } => {
                vec![self.try_table_item_into_write_set_change(hash, *handle, key.to_owned(), op)]
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
        access_path: &AccessPath,
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
        let table_info = match self.get_table_info(handle)? {
            Some(ti) => ti,
            None => {
                log_missing_table_info(handle);
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
        let table_info = match self.get_table_info(handle)? {
            Some(ti) => ti,
            None => {
                log_missing_table_info(handle);
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

    pub fn try_into_signed_transaction_poem(
        &self,
        submit_transaction_request: SubmitTransactionRequest,
        chain_id: ChainId,
    ) -> Result<SignedTransaction> {
        let SubmitTransactionRequest {
            user_transaction_request,
            signature,
        } = submit_transaction_request;

        Ok(SignedTransaction::new_signed_transaction(
            self.try_into_raw_transaction_poem(
                user_transaction_request,
                chain_id,
            )?,
            (&signature).try_into().context("Failed to parse transaction when building SignedTransaction from SubmitTransactionRequest")?,
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
            replay_protection_nonce,
        } = user_transaction_request;
        Ok(RawTransaction::new(
            sender.into(),
            // The `sequence_number` field is not used for processing orderless transactions.
            // However, the `SignedTransaction` strucut has a mandatory sequence_number field.
            // So, for orderless transactions, we chose to set the sequence_number to u64::MAX.
            if replay_protection_nonce.is_none() {
                sequence_number.into()
            } else {
                u64::MAX
            },
            self.try_into_aptos_core_transaction_payload(
                payload,
                replay_protection_nonce.map(|nonce| nonce.into()),
            )
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
        nonce: Option<u64>,
    ) -> Result<aptos_types::transaction::TransactionPayload> {
        use aptos_types::transaction::{
            TransactionExecutable as Executable, TransactionExtraConfig as ExtraConfig,
            TransactionPayload as Target, TransactionPayloadInner as TargetInner,
        };

        let try_into_entry_function =
            |entry_func_payload: EntryFunctionPayload| -> Result<EntryFunction> {
                let EntryFunctionPayload {
                    function,
                    type_arguments,
                    arguments,
                } = entry_func_payload;

                let module_id: ModuleId = function.module.clone().into();
                let code = self.inner.view_existing_module(&module_id)? as Arc<dyn Bytecode>;
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
                    .try_into_vm_values(&func, arguments.as_slice())?
                    .iter()
                    .map(bcs::to_bytes)
                    .collect::<Result<_, bcs::Error>>()?;

                Ok(EntryFunction::new(
                    module_id,
                    function.name.into(),
                    type_arguments
                        .iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_>>()?,
                    args,
                ))
            };

        let try_into_script_payload = |script: ScriptPayload| -> Result<Script> {
            let ScriptPayload {
                code,
                type_arguments,
                arguments,
            } = script;

            let MoveScriptBytecode { bytecode, abi } = code.try_parse_abi();
            match abi {
                Some(func) => {
                    let args = self.try_into_vm_values(&func, arguments.as_slice())?;
                    Ok(Script::new(
                        bytecode.into(),
                        type_arguments
                            .iter()
                            .map(|v| v.try_into())
                            .collect::<Result<_>>()?,
                        args.into_iter()
                            .map(|arg| arg.try_into())
                            .collect::<Result<_>>()?,
                    ))
                },
                None => Err(anyhow::anyhow!("invalid transaction script bytecode")),
            }
        };
        // TODO[Orderless]: After the new TransactionPayload format is fully deployed, output the TransactionPayload in V2 format here.
        let ret = match payload {
            TransactionPayload::EntryFunctionPayload(entry_func_payload) => {
                if let Some(nonce) = nonce {
                    Target::Payload(TargetInner::V1 {
                        executable: Executable::EntryFunction(try_into_entry_function(
                            entry_func_payload,
                        )?),
                        extra_config: ExtraConfig::V1 {
                            multisig_address: None,
                            replay_protection_nonce: Some(nonce),
                        },
                    })
                } else {
                    Target::EntryFunction(try_into_entry_function(entry_func_payload)?)
                }
            },
            TransactionPayload::ScriptPayload(script) => {
                if let Some(nonce) = nonce {
                    Target::Payload(TargetInner::V1 {
                        executable: Executable::Script(try_into_script_payload(script)?),
                        extra_config: ExtraConfig::V1 {
                            multisig_address: None,
                            replay_protection_nonce: Some(nonce),
                        },
                    })
                } else {
                    Target::Script(try_into_script_payload(script)?)
                }
            },

            TransactionPayload::MultisigPayload(multisig) => {
                if let Some(nonce) = nonce {
                    let executable = if let Some(payload) = multisig.transaction_payload {
                        match payload {
                            MultisigTransactionPayload::EntryFunctionPayload(
                                entry_func_payload,
                            ) => Executable::EntryFunction(try_into_entry_function(
                                entry_func_payload,
                            )?),
                        }
                    } else {
                        Executable::Empty
                    };
                    Target::Payload(TargetInner::V1 {
                        executable,
                        extra_config: ExtraConfig::V1 {
                            multisig_address: Some(multisig.multisig_address.into()),
                            replay_protection_nonce: Some(nonce),
                        },
                    })
                } else {
                    let transaction_payload: Option<
                        aptos_types::transaction::MultisigTransactionPayload,
                    > = if let Some(payload) = multisig.transaction_payload {
                        match payload {
                            MultisigTransactionPayload::EntryFunctionPayload(
                                entry_func_payload,
                            ) => {
                                let entry_function: EntryFunction =
                                    try_into_entry_function(entry_func_payload)?;
                                Some(
                                    aptos_types::transaction::MultisigTransactionPayload::EntryFunction(
                                        entry_function,
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
                }
            },
            // Deprecated.
            TransactionPayload::ModuleBundlePayload(_) => {
                bail!("Module bundle payload has been removed")
            },
        };
        Ok(ret)
    }

    pub fn try_into_vm_values(
        &self,
        func: &MoveFunction,
        args: &[serde_json::Value],
    ) -> Result<Vec<move_core_types::value::MoveValue>> {
        let arg_types = func
            .params
            .iter()
            .filter(|p| !p.is_signer())
            .collect::<Vec<_>>();
        ensure!(
            arg_types.len() == args.len(),
            "expected {} arguments [{}], but got {} ({:?})",
            arg_types.len(),
            arg_types
                .iter()
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
                self.try_into_vm_value(&arg_type.try_into()?, arg.clone())
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
                    self.inner.view_fully_decorated_ty_layout(type_tag)?
                }
            },
            _ => self.inner.view_fully_decorated_ty_layout(type_tag)?,
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
            MoveTypeLayout::Function => {
                // TODO(#15664): do we actually need this? It appears the code here is dead and
                //   nowhere used
                bail!("unexpected move type {:?} for value {:?}", layout, val)
            },

            // Some values, e.g., signer or ones with custom serialization
            // (native), are not stored to storage and so we do not expect
            // to see them here.
            MoveTypeLayout::Signer | MoveTypeLayout::Native(..) => {
                bail!("unexpected move type {:?} for value {:?}", layout, val)
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
            if let MoveStructLayout::Decorated { struct_tag, fields } = layout {
                (struct_tag, fields)
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
        let code = self.inner.view_existing_module(&function.module)? as Arc<dyn Bytecode>;
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
        let code = self.inner.view_existing_module(&module.clone().into())? as Arc<dyn Bytecode>;
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
            .try_into_vm_values(&func, &arguments)?
            .iter()
            .map(bcs::to_bytes)
            .collect::<Result<_, bcs::Error>>()?;

        Ok(ViewFunction {
            module: module.into(),
            function: function.name.into(),
            ty_args: type_arguments
                .iter()
                .map(|v| v.try_into())
                .collect::<Result<_>>()?,
            args,
        })
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        if let Some(indexer_reader) = self.indexer_reader.as_ref() {
            return Ok(indexer_reader.get_table_info(handle).unwrap_or(None));
        }
        Ok(None)
    }

    fn explain_vm_status(
        &self,
        status: &ExecutionStatus,
        txn_aux_data: Option<TransactionAuxiliaryData>,
    ) -> String {
        let mut status = status.to_owned();
        status = if let Some(aux_data) = txn_aux_data {
            ExecutionStatus::aug_with_aux_data(status, &aux_data)
        } else {
            status
        };
        match &status {
            ExecutionStatus::MoveAbort {
                location,
                code,
                info,
            } => match &location {
                AbortLocation::Module(_) => info
                    .as_ref()
                    .map(|i| {
                        format!(
                            "Move abort in {}: {}({:#x}): {}",
                            abort_location_to_str(location),
                            i.reason_name,
                            code,
                            i.description
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "Move abort in {}: {:#x}",
                            abort_location_to_str(location),
                            code
                        )
                    }),
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
                    AbortLocation::Module(module_id) => self
                        .explain_function_index(module_id, function)
                        .map(|name| format!("{}::{}", abort_location_to_str(location), name))
                        .unwrap_or_else(|_| {
                            format!(
                                "{}::<#{} function>",
                                abort_location_to_str(location),
                                function
                            )
                        }),
                    AbortLocation::Script => "script".to_owned(),
                };
                format!(
                    "Execution failed in {} at code offset {}",
                    func_name, code_offset
                )
            },
            ExecutionStatus::MiscellaneousError(code) => {
                if code.is_none() {
                    "Execution failed with miscellaneous error and no status code".to_owned()
                } else {
                    format!("{:#?}", code.unwrap())
                }
            },
        }
    }

    fn explain_function_index(&self, module_id: &ModuleId, function: &u16) -> Result<String> {
        let code = self.inner.view_existing_module(module_id)?;
        let func = code.function_handle_at(FunctionHandleIndex::new(*function));
        let id = code.identifier_at(func.name);
        Ok(id.to_string())
    }
}

fn log_missing_table_info(handle: TableHandle) {
    sample!(
        SampleRate::Duration(Duration::from_secs(1)),
        aptos_logger::debug!(
            "Table info not found for handle {:?}, can't decode table item. OK for simulation",
            handle
        )
    );
}

pub trait AsConverter<R> {
    fn as_converter(
        &self,
        db: Arc<dyn DbReader>,
        indexer_reader: Option<Arc<dyn IndexerReader>>,
    ) -> MoveConverter<R>;
}

impl<R: StateView> AsConverter<R> for R {
    fn as_converter(
        &self,
        db: Arc<dyn DbReader>,
        indexer_reader: Option<Arc<dyn IndexerReader>>,
    ) -> MoveConverter<R> {
        MoveConverter::new(self, db, indexer_reader)
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
