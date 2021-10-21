// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Bytecode, Event, HexEncodedBytes, MoveFunction, MoveModuleBytecode, MoveResource, MoveType,
    MoveValue, ScriptFunctionPayload, ScriptPayload, Transaction, TransactionData,
    TransactionOnChainData, TransactionPayload, UserTransactionRequest, WriteSetChange,
    WriteSetPayload,
};
use diem_types::{
    access_path::{AccessPath, Path},
    chain_id::ChainId,
    contract_event::ContractEvent,
    transaction::{
        Module, RawTransaction, Script, ScriptFunction, SignedTransaction, TransactionInfoTrait,
    },
    write_set::WriteOp,
};
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use resource_viewer::MoveValueAnnotator;

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

    pub fn try_into_pending_transaction(&self, txn: SignedTransaction) -> Result<Transaction> {
        let payload = self.try_into_transaction_payload(txn.payload().clone())?;
        Ok((txn, payload).into())
    }

    pub fn try_into_transaction<T: TransactionInfoTrait>(
        &self,
        data: TransactionData<T>,
    ) -> Result<Transaction> {
        match data {
            TransactionData::OnChain(txn) => self.try_into_onchain_transaction(txn),
            TransactionData::Pending(txn) => self.try_into_pending_transaction(*txn),
        }
    }

    pub fn try_into_onchain_transaction<T: TransactionInfoTrait>(
        &self,
        data: TransactionOnChainData<T>,
    ) -> Result<Transaction> {
        use diem_types::transaction::Transaction::*;
        let events = self.try_into_events(&data.events)?;
        let ret = match data.transaction {
            UserTransaction(txn) => {
                let payload = self.try_into_transaction_payload(txn.payload().clone())?;
                (data.version, &txn, &data.info, payload, events).into()
            }
            GenesisTransaction(write_set) => {
                let payload = self.try_into_write_set_payload(write_set)?;
                (data.version, &data.info, payload, events).into()
            }
            BlockMetadata(txn) => (data.version, &txn, &data.info).into(),
        };
        Ok(ret)
    }

    pub fn try_into_transaction_payload(
        &self,
        payload: diem_types::transaction::TransactionPayload,
    ) -> Result<TransactionPayload> {
        use diem_types::transaction::TransactionPayload::*;
        let ret = match payload {
            WriteSet(v) => TransactionPayload::WriteSetPayload(self.try_into_write_set_payload(v)?),
            Script(s) => TransactionPayload::ScriptPayload(s.try_into()?),
            Module(m) => {
                TransactionPayload::ModulePayload(MoveModuleBytecode::from(m).ensure_abi()?)
            }
            ScriptFunction(fun) => {
                let (module, function, ty_args, args) = fun.into_inner();
                TransactionPayload::ScriptFunctionPayload(ScriptFunctionPayload {
                    arguments: self
                        .inner
                        .view_function_arguments(&module, &function, &args)?
                        .into_iter()
                        .map(|v| MoveValue::try_from(v)?.json())
                        .collect::<Result<_>>()?,
                    module: module.into(),
                    function,
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
            Script { execute_as, script } => WriteSetPayload::ScriptWriteSet {
                execute_as: execute_as.into(),
                script: script.try_into()?,
            },
            Direct(d) => {
                let (write_set, events) = d.into_inner();
                WriteSetPayload::DirectWriteSet {
                    changes: write_set
                        .into_iter()
                        .map(|(access_path, op)| self.try_into_write_set_change(access_path, op))
                        .collect::<Result<_>>()?,
                    events: self.try_into_events(&events)?,
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
                    data: MoveModuleBytecode::new(val).ensure_abi()?,
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
                    module,
                    function,
                    type_arguments,
                    arguments,
                } = script_func_payload;

                let code = self.inner.get_module(&module.clone().into())? as Rc<dyn Bytecode>;
                let func = code
                    .find_script_function(function.as_ident_str())
                    .ok_or_else(|| format_err!("could not find script function by {}", function))?;
                let args = self
                    .try_into_move_values(func, arguments)?
                    .iter()
                    .map(bcs::to_bytes)
                    .collect::<Result<_, bcs::Error>>()?;

                Target::ScriptFunction(ScriptFunction::new(
                    module.into(),
                    function,
                    type_arguments
                        .into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_>>()?,
                    args,
                ))
            }
            TransactionPayload::ModulePayload(module) => {
                Target::Module(Module::new(module.bytecode.into()))
            }
            TransactionPayload::ScriptPayload(script) => {
                let ScriptPayload {
                    code,
                    type_arguments,
                    arguments,
                } = script;

                let args = self.try_into_move_values(code.clone().into_abi()?, arguments)?;
                Target::Script(Script::new(
                    code.bytecode.into(),
                    type_arguments
                        .into_iter()
                        .map(|v| v.try_into())
                        .collect::<Result<_>>()?,
                    args.into_iter()
                        .map(|arg| arg.try_into())
                        .collect::<Result<_>>()?,
                ))
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
            "invalid arguments: expected arguments {:?}, but got {:?}",
            arg_types,
            args
        );
        arg_types
            .into_iter()
            .zip(args.into_iter())
            .enumerate()
            .map(|(i, (arg_type, arg))| {
                self.try_into_move_value(&arg_type, arg)
                    .map_err(|e| format_err!("parse #{} argument failed: {:?}", i + 1, e))
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
}
