// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    Event, MoveModule, MoveResource, Transaction, TransactionPayload, WriteSetChange,
    WriteSetPayload,
};
use diem_types::{
    access_path::Path, contract_event::ContractEvent, transaction::TransactionInfoTrait,
    write_set::WriteOp,
};
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use resource_viewer::MoveValueAnnotator;

use anyhow::Result;
use std::convert::TryFrom;

pub struct MoveConverter<'a, R> {
    inner: MoveValueAnnotator<'a, R>,
}

impl<'a, R: MoveResolver> MoveConverter<'a, R> {
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
        Ok(self.inner.view_resource(typ, bytes)?.into())
    }

    pub fn try_into_transaction<T: TransactionInfoTrait>(
        &self,
        version: u64,
        submitted: &diem_types::transaction::Transaction,
        info: &T,
        contract_events: &[ContractEvent],
    ) -> Result<Transaction> {
        use diem_types::transaction::Transaction::*;
        let events = self.try_into_events(version, contract_events)?;
        let ret = match submitted {
            UserTransaction(txn) => {
                let payload = self.try_into_transaction_payload(version, txn.payload())?;
                (version, txn, info, payload, events).into()
            }
            GenesisTransaction(write_set) => {
                let payload = self.try_into_write_set_payload(version, write_set)?;
                (version, info, payload, events).into()
            }
            BlockMetadata(txn) => (version, txn, info).into(),
        };
        Ok(ret)
    }

    pub fn try_into_transaction_payload(
        &self,
        txn_version: u64,
        payload: &diem_types::transaction::TransactionPayload,
    ) -> Result<TransactionPayload> {
        use diem_types::transaction::TransactionPayload::*;
        let ret = match payload {
            WriteSet(v) => TransactionPayload::WriteSetPayload(
                self.try_into_write_set_payload(txn_version, v)?,
            ),
            Script(s) => TransactionPayload::ScriptPayload(s.into()),
            Module(m) => TransactionPayload::ModulePayload {
                code: m.code().to_vec().into(),
            },
            ScriptFunction(fun) => TransactionPayload::ScriptFunctionPayload {
                module: fun.module().clone().into(),
                function: fun.function().into(),
                type_arguments: fun.ty_args().iter().map(|arg| arg.clone().into()).collect(),
                arguments: self
                    .inner
                    .view_function_arguments(fun.module(), fun.function(), fun.args())?
                    .iter()
                    .map(|v| v.clone().into())
                    .collect(),
            },
        };
        Ok(ret)
    }

    pub fn try_into_write_set_payload(
        &self,
        txn_version: u64,
        payload: &diem_types::transaction::WriteSetPayload,
    ) -> Result<WriteSetPayload> {
        use diem_types::transaction::WriteSetPayload::*;
        let ret = match payload {
            Script { execute_as, script } => WriteSetPayload::ScriptWriteSet {
                execute_as: (*execute_as).into(),
                script: script.into(),
            },
            Direct(d) => WriteSetPayload::DirectWriteSet {
                changes: d
                    .write_set()
                    .iter()
                    .map(|(access_path, op)| self.try_into_write_set_change(access_path, op))
                    .collect::<Result<_>>()?,
                events: self.try_into_events(txn_version, d.events())?,
            },
        };
        Ok(ret)
    }

    pub fn try_into_write_set_change(
        &self,
        access_path: &diem_types::access_path::AccessPath,
        op: &WriteOp,
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
                    data: MoveModule::try_from(val)?,
                },
                Path::Resource(typ) => WriteSetChange::WriteResource {
                    address: access_path.address.into(),
                    data: self.try_into_resource(&typ, val)?,
                },
            },
        };
        Ok(ret)
    }

    pub fn try_into_events(
        &self,
        txn_version: u64,
        events: &[ContractEvent],
    ) -> Result<Vec<Event>> {
        let mut ret = vec![];
        for event in events {
            let data = self
                .inner
                .view_value(event.type_tag(), event.event_data())?;
            ret.push((txn_version, event, data).into());
        }
        Ok(ret)
    }
}
