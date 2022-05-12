// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    transaction::{ModuleBundlePayload, StateCheckpointTransaction},
    Bytecode, DirectWriteSet, Event, HexEncodedBytes, MoveFunction, MoveModuleBytecode,
    MoveResource, MoveScriptBytecode, MoveValue, ScriptFunctionId, ScriptFunctionPayload,
    ScriptPayload, ScriptWriteSet, Transaction, TransactionInfo, TransactionOnChainData,
    TransactionPayload, UserTransactionRequest, WriteSet, WriteSetChange, WriteSetPayload,
};
use anyhow::{bail, ensure, format_err, Result};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_transaction_builder::error_explain;
use aptos_types::{
    access_path::{AccessPath, Path},
    chain_id::ChainId,
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    transaction::{
        ExecutionStatus, ModuleBundle, RawTransaction, Script, ScriptFunction, SignedTransaction,
    },
    vm_status::AbortLocation,
    write_set::WriteOp,
};
use aptos_vm::move_vm_ext::MoveResolverExt;
use move_deps::{
    move_binary_format::file_format::FunctionHandleIndex,
    move_core_types,
    move_core_types::{
        identifier::Identifier,
        language_storage::{ModuleId, StructTag, TypeTag},
        value::{MoveStructLayout, MoveTypeLayout},
    },
    move_resource_viewer::MoveValueAnnotator,
};
use serde_json::Value;
use std::{
    convert::{TryFrom, TryInto},
    iter::IntoIterator,
    rc::Rc,
};

pub struct MoveConverter<'a, R: ?Sized> {
    inner: MoveValueAnnotator<'a, R>,
}

impl<'a, R: MoveResolverExt + ?Sized> MoveConverter<'a, R> {
    pub fn new(inner: &'a R) -> Self {
        Self {
            inner: MoveValueAnnotator::new(inner),
        }
    }

    pub fn try_into_resources<'b>(
        &self,
        data: impl Iterator<Item = (StructTag, &'b [u8])>,
    ) -> Result<Vec<MoveResource>> {
        data.map(|(typ, bytes)| self.try_into_resource(&typ, bytes))
            .collect()
    }

    pub fn try_into_resource<'b>(&self, typ: &StructTag, bytes: &'b [u8]) -> Result<MoveResource> {
        self.inner.view_resource(typ, bytes)?.try_into()
    }

    pub fn move_struct_fields<'b>(
        &self,
        typ: &StructTag,
        bytes: &'b [u8],
    ) -> Result<Vec<(Identifier, move_core_types::value::MoveValue)>> {
        self.inner.move_struct_fields(typ, bytes)
    }

    pub fn try_into_pending_transaction(&self, txn: SignedTransaction) -> Result<Transaction> {
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
            }
            GenesisTransaction(write_set) => {
                let payload = self.try_into_write_set_payload(write_set)?;
                (info, payload, events).into()
            }
            BlockMetadata(txn) => (&txn, info).into(),
            StateCheckpoint => {
                Transaction::StateCheckpointTransaction(StateCheckpointTransaction {
                    info,
                    timestamp: timestamp.into(),
                })
            }
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
            state_root_hash: info.state_change_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),
            vm_status: self.explain_vm_status(info.status()),
            accumulator_root_hash: accumulator_root_hash.into(),
            // TODO: the resource value is interpreted by the type definition at the version of the converter, not the version of the tx: must be fixed before we allow module updates
            changes: write_set
                .into_iter()
                .filter_map(|(sk, wo)| self.try_into_write_set_change(sk, wo).ok())
                .collect(),
        }
    }

    pub fn try_into_transaction_payload(
        &self,
        payload: aptos_types::transaction::TransactionPayload,
    ) -> Result<TransactionPayload> {
        use aptos_types::transaction::TransactionPayload::*;
        let ret = match payload {
            WriteSet(v) => TransactionPayload::WriteSetPayload(self.try_into_write_set_payload(v)?),
            Script(s) => TransactionPayload::ScriptPayload(s.try_into()?),
            ModuleBundle(modules) => TransactionPayload::ModuleBundlePayload(ModuleBundlePayload {
                modules: modules
                    .into_iter()
                    .map(|module| MoveModuleBytecode::from(module).try_parse_abi())
                    .collect::<Result<Vec<_>>>()?,
            }),
            ScriptFunction(fun) => {
                let (module, function, ty_args, args) = fun.into_inner();
                let func_args = self
                    .inner
                    .view_function_arguments(&module, &function, &args);
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

                TransactionPayload::ScriptFunctionPayload(ScriptFunctionPayload {
                    arguments: json_args,
                    function: ScriptFunctionId {
                        module: module.into(),
                        name: function,
                    },
                    type_arguments: ty_args.into_iter().map(|arg| arg.into()).collect(),
                })
            }
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
                WriteSetPayload {
                    write_set: WriteSet::DirectWriteSet(DirectWriteSet {
                        // TODO: the resource value is interpreted by the type definition at the version of the converter, not the version of the tx: must be fixed before we allow module updates
                        changes: write_set
                            .into_iter()
                            .map(|(state_key, op)| self.try_into_write_set_change(state_key, op))
                            .collect::<Result<_>>()?,
                        events: self.try_into_events(&events)?,
                    }),
                }
            }
        };
        Ok(ret)
    }

    pub fn try_into_write_set_change(
        &self,
        state_key: StateKey,
        op: WriteOp,
    ) -> Result<WriteSetChange> {
        let hash = state_key.hash().to_hex_literal();

        match state_key {
            StateKey::AccessPath(access_path) => {
                self.try_access_path_into_write_set_change(hash, access_path, op)
            }
            StateKey::TableItem { handle, key } => {
                self.try_table_item_into_write_set_change(hash, handle, key, op)
            }
            StateKey::Raw(_) => Err(format_err!(
                "Can't convert account raw key {:?} to WriteSetChange",
                state_key
            )),
        }
    }

    pub fn try_access_path_into_write_set_change(
        &self,
        state_key_hash: String,
        access_path: AccessPath,
        op: WriteOp,
    ) -> Result<WriteSetChange> {
        let ret = match op {
            WriteOp::Deletion => match access_path.get_path() {
                Path::Code(module_id) => WriteSetChange::DeleteModule {
                    address: access_path.address.into(),
                    state_key_hash,
                    module: module_id.into(),
                },
                Path::Resource(typ) => WriteSetChange::DeleteResource {
                    address: access_path.address.into(),
                    state_key_hash,
                    resource: typ.into(),
                },
            },
            WriteOp::Value(val) => match access_path.get_path() {
                Path::Code(_) => WriteSetChange::WriteModule {
                    address: access_path.address.into(),
                    state_key_hash,
                    data: MoveModuleBytecode::new(val).try_parse_abi()?,
                },
                Path::Resource(typ) => WriteSetChange::WriteResource {
                    address: access_path.address.into(),
                    state_key_hash,
                    data: self.try_into_resource(&typ, &val)?,
                },
            },
        };
        Ok(ret)
    }

    pub fn try_table_item_into_write_set_change(
        &self,
        state_key_hash: String,
        handle: u128,
        key: Vec<u8>,
        op: WriteOp,
    ) -> Result<WriteSetChange> {
        let handle = handle.to_be_bytes().to_vec().into();
        let key = key.into();
        let ret = match op {
            WriteOp::Deletion => WriteSetChange::DeleteTableItem {
                state_key_hash,
                handle,
                key,
            },
            WriteOp::Value(value) => WriteSetChange::WriteTableItem {
                state_key_hash,
                handle,
                key,
                value: value.into(),
            },
        };
        Ok(ret)
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

    pub fn try_into_aptos_core_transaction_payload(
        &self,
        payload: TransactionPayload,
    ) -> Result<aptos_types::transaction::TransactionPayload> {
        use aptos_types::transaction::TransactionPayload as Target;

        let ret = match payload {
            TransactionPayload::ScriptFunctionPayload(script_func_payload) => {
                let ScriptFunctionPayload {
                    function,
                    type_arguments,
                    arguments,
                } = script_func_payload;

                let module = function.module.clone();
                let code = self.inner.get_module(&module.clone().into())? as Rc<dyn Bytecode>;
                let func = code
                    .find_script_function(function.name.as_ident_str())
                    .ok_or_else(|| format_err!("could not find script function by {}", function))?;
                ensure!(
                    func.generic_type_params.len() == type_arguments.len(),
                    "expect {} type arguments for script function {}, but got {}",
                    func.generic_type_params.len(),
                    function,
                    type_arguments.len()
                );
                let args = self
                    .try_into_vm_values(func, arguments)?
                    .iter()
                    .map(bcs::to_bytes)
                    .collect::<Result<_, bcs::Error>>()?;

                Target::ScriptFunction(ScriptFunction::new(
                    module.into(),
                    function.name,
                    type_arguments
                        .into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_>>()?,
                    args,
                ))
            }
            TransactionPayload::ModuleBundlePayload(payload) => {
                Target::ModuleBundle(ModuleBundle::new(
                    payload
                        .modules
                        .into_iter()
                        .map(|m| m.bytecode.into())
                        .collect(),
                ))
            }
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
                    }
                    None => return Err(anyhow::anyhow!("invalid transaction script bytecode")),
                }
            }
            TransactionPayload::WriteSetPayload(_) => {
                return Err(anyhow::anyhow!(
                    "write set transaction payload is not supported yet",
                ))
            }
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
            .zip(args.into_iter())
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
        let layout = self.inner.get_type_layout_with_types(type_tag)?;

        self.try_into_vm_value_from_layout(&layout, val)
    }

    fn try_into_vm_value_from_layout(
        &self,
        layout: &MoveTypeLayout,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        use move_deps::move_core_types::value::MoveValue::*;

        Ok(match layout {
            MoveTypeLayout::Bool => Bool(serde_json::from_value::<bool>(val)?),
            MoveTypeLayout::U8 => U8(serde_json::from_value::<u8>(val)?),
            MoveTypeLayout::U64 => serde_json::from_value::<crate::U64>(val)?.into(),
            MoveTypeLayout::U128 => serde_json::from_value::<crate::U128>(val)?.into(),
            MoveTypeLayout::Address => serde_json::from_value::<crate::Address>(val)?.into(),
            MoveTypeLayout::Vector(item_layout) => {
                self.try_into_vm_value_vector(item_layout.as_ref(), val)?
            }
            MoveTypeLayout::Struct(struct_layout) => {
                self.try_into_vm_value_struct(struct_layout, val)?
            }
            MoveTypeLayout::Signer => {
                bail!("unexpected move type {:?} for value {:?}", layout, val)
            }
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
        if MoveValue::is_ascii_string(struct_tag) {
            let string = val
                .as_str()
                .ok_or_else(|| format_err!("failed to parse ASCII::String."))?;
            return Ok(new_vm_ascii_string(string));
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

    fn explain_vm_status(&self, status: &ExecutionStatus) -> String {
        match status {
            ExecutionStatus::MoveAbort { location, code} => match &location {
                AbortLocation::Module(module_id) => {
                    let explanation = error_explain::get_explanation(module_id, *code);
                    explanation
                        .map(|ec| {
                            format!(
                                "Move abort by {} - {}\n{}\n{}",
                                ec.category.code_name,
                                ec.reason.code_name,
                                ec.category.code_description,
                                ec.reason.code_description
                            )
                        })
                        .unwrap_or_else(|| {
                            format!("Move abort: code {} at {}", code, location)
                        })
                }
                AbortLocation::Script => format!("Move abort: code {}", code),
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
                        .map(|name| format!("{}::{}", location, name))
                        .unwrap_or_else(|_| format!("{}::<#{} function>", location, function)),
                    AbortLocation::Script => "script".to_owned(),
                };
                format!(
                    "Execution failed in {} at code offset {}",
                    func_name, code_offset
                )
            }
            ExecutionStatus::MiscellaneousError( code ) => {
                code.map_or(
                    "Move bytecode deserialization / verification failed, including script function not found or invalid arguments".to_owned(),
                    |e| format!(
                        "Transaction Executed and Committed with Error {:#?}", e
                    )
                )
            }
        }
    }

    pub fn try_into_move_value(&self, typ: &TypeTag, bytes: &[u8]) -> Result<MoveValue> {
        self.inner.view_value(typ, bytes)?.try_into()
    }

    fn explain_function_index(&self, module_id: &ModuleId, function: &u16) -> Result<String> {
        let code = self.inner.get_module(&module_id.clone())? as Rc<dyn Bytecode>;
        let func = code.function_handle_at(FunctionHandleIndex::new(*function));
        let id = code.identifier_at(func.name);
        Ok(format!("{}", id))
    }
}

pub trait AsConverter<R> {
    fn as_converter(&self) -> MoveConverter<R>;
}

impl<R: MoveResolverExt> AsConverter<R> for R {
    fn as_converter(&self) -> MoveConverter<R> {
        MoveConverter::new(self)
    }
}

pub fn new_vm_ascii_string(string: &str) -> move_core_types::value::MoveValue {
    use move_deps::move_core_types::value::{MoveStruct, MoveValue};

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
