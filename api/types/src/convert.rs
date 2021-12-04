// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Bytecode, DirectWriteSet, Event, HexEncodedBytes, MoveFunction, MoveModuleBytecode,
    MoveResource, MoveScriptBytecode, MoveType, MoveValue, ScriptFunctionId, ScriptFunctionPayload,
    ScriptPayload, ScriptWriteSet, Transaction, TransactionInfo, TransactionOnChainData,
    TransactionPayload, UserTransactionRequest, WriteSet, WriteSetChange, WriteSetPayload,
};
use diem_transaction_builder::error_explain;
use diem_types::{
    access_path::{AccessPath, Path},
    chain_id::ChainId,
    contract_event::ContractEvent,
    transaction::{
        ModuleBundle, RawTransaction, Script, ScriptFunction, SignedTransaction,
        TransactionInfoTrait,
    },
    vm_status::{AbortLocation, KeptVMStatus},
    write_set::WriteOp,
};
use move_binary_format::file_format::FunctionHandleIndex;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    resolver::MoveResolver,
};
use move_resource_viewer::MoveValueAnnotator;

use crate::transaction::ModuleBundlePayload;
use anyhow::{ensure, format_err, Result};
use serde_json::Value;
use std::{
    convert::{TryFrom, TryInto},
    rc::Rc,
};

pub struct MoveConverter<'a, R: ?Sized> {
    inner: MoveValueAnnotator<'a, R>,
}

impl<'a, R: MoveResolver + ?Sized> MoveConverter<'a, R> {
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

    pub fn try_into_onchain_transaction<T: TransactionInfoTrait>(
        &self,
        timestamp: u64,
        data: TransactionOnChainData<T>,
    ) -> Result<Transaction> {
        use diem_types::transaction::Transaction::*;
        let info = self.into_transaction_info(data.version, &data.info);
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
        })
    }

    pub fn into_transaction_info<T: TransactionInfoTrait>(
        &self,
        version: u64,
        info: &T,
    ) -> TransactionInfo {
        TransactionInfo {
            version: version.into(),
            hash: info.transaction_hash().into(),
            state_root_hash: info.state_root_hash().into(),
            event_root_hash: info.event_root_hash().into(),
            gas_used: info.gas_used().into(),
            success: info.status().is_success(),
            vm_status: self.explain_vm_status(info.status()),
        }
    }

    pub fn try_into_transaction_payload(
        &self,
        payload: diem_types::transaction::TransactionPayload,
    ) -> Result<TransactionPayload> {
        use diem_types::transaction::TransactionPayload::*;
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
        payload: diem_types::transaction::WriteSetPayload,
    ) -> Result<WriteSetPayload> {
        use diem_types::transaction::WriteSetPayload::*;
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
                        changes: write_set
                            .into_iter()
                            .map(|(access_path, op)| {
                                self.try_into_write_set_change(access_path, op)
                            })
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
        access_path: AccessPath,
        op: WriteOp,
    ) -> Result<WriteSetChange> {
        let ret = match op {
            WriteOp::Deletion => match access_path.get_path() {
                Path::Code(module_id) => WriteSetChange::DeleteModule {
                    address: access_path.address.into(),
                    module: module_id.into(),
                },
                Path::Resource(typ) => WriteSetChange::DeleteResource {
                    address: access_path.address.into(),
                    resource: typ.into(),
                },
            },
            WriteOp::Value(val) => match access_path.get_path() {
                Path::Code(_) => WriteSetChange::WriteModule {
                    address: access_path.address.into(),
                    data: MoveModuleBytecode::new(val).try_parse_abi()?,
                },
                Path::Resource(typ) => WriteSetChange::WriteResource {
                    address: access_path.address.into(),
                    data: self.try_into_resource(&typ, &val)?,
                },
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
            gas_currency_code,
            expiration_timestamp_secs,
            payload,
            signature: _,
        } = txn;
        Ok(RawTransaction::new(
            sender.into(),
            sequence_number.into(),
            self.try_into_diem_core_transaction_payload(payload)?,
            max_gas_amount.into(),
            gas_unit_price.into(),
            gas_currency_code,
            expiration_timestamp_secs.into(),
            chain_id,
        ))
    }

    pub fn try_into_diem_core_transaction_payload(
        &self,
        payload: TransactionPayload,
    ) -> Result<diem_types::transaction::TransactionPayload> {
        use diem_types::transaction::TransactionPayload as Target;

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
                    .try_into_move_values(func, arguments)?
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
                        let args = self.try_into_move_values(func, arguments)?;
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

    pub fn try_into_move_values(
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
                self.try_into_move_value(&arg_type, arg).map_err(|e| {
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

    pub fn try_into_move_value(
        &self,
        typ: &MoveType,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        use move_core_types::value::MoveValue::*;

        Ok(match typ {
            MoveType::Bool => Bool(serde_json::from_value::<bool>(val)?),
            MoveType::U8 => U8(serde_json::from_value::<u8>(val)?),
            MoveType::U64 => serde_json::from_value::<crate::U64>(val)?.into(),
            MoveType::U128 => serde_json::from_value::<crate::U128>(val)?.into(),
            MoveType::Address => serde_json::from_value::<crate::Address>(val)?.into(),
            MoveType::Vector { items } => self.try_into_move_value_vector(&*items, val)?,
            MoveType::Signer
            | MoveType::Struct(_)
            | MoveType::GenericTypeParam { index: _ }
            | MoveType::Reference { mutable: _, to: _ } => {
                return Err(format_err!(
                    "unexpected move type {:?} for value {:?}",
                    &typ,
                    &val
                ))
            }
        })
    }

    pub fn try_into_move_value_vector(
        &self,
        typ: &MoveType,
        val: Value,
    ) -> Result<move_core_types::value::MoveValue> {
        if matches!(typ, MoveType::U8) {
            Ok(serde_json::from_value::<HexEncodedBytes>(val)?.into())
        } else if let Value::Array(list) = val {
            let vals = list
                .into_iter()
                .map(|v| self.try_into_move_value(typ, v))
                .collect::<Result<_>>()?;

            Ok(move_core_types::value::MoveValue::Vector(vals))
        } else {
            Err(format_err!(
                "expected vector<{:?}>, but got: {:?}",
                typ,
                &val
            ))
        }
    }

    fn explain_vm_status(&self, status: &KeptVMStatus) -> String {
        match status {
            KeptVMStatus::MoveAbort(location, abort_code) => match &location {
                AbortLocation::Module(module_id) => {
                    let explanation = error_explain::get_explanation(module_id, *abort_code);
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
                            format!("Move abort: code {} at {}", abort_code, location)
                        })
                }
                AbortLocation::Script => format!("Move abort: code {}", abort_code),
            },
            KeptVMStatus::Executed => "Executed successfully".to_owned(),
            KeptVMStatus::OutOfGas => "Out of gas".to_owned(),
            KeptVMStatus::ExecutionFailure {
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
            KeptVMStatus::MiscellaneousError => {
                "Move bytecode deserialization / verification failed, including script function not found or invalid arguments"
                    .to_owned()
            }
        }
    }

    fn explain_function_index(&self, module_id: &ModuleId, function: &u16) -> Result<String> {
        let code = self.inner.get_module(&module_id.clone())? as Rc<dyn Bytecode>;
        let func = code.function_handle_at(FunctionHandleIndex::new(*function));
        let id = code.identifier_at(func.name);
        Ok(format!("{}", id))
    }
}
