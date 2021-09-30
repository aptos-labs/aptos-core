// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{Event, MoveResource, Transaction, TransactionPayload};
use diem_types::{contract_event::ContractEvent, transaction::TransactionInfoTrait};
use move_core_types::{language_storage::StructTag, resolver::MoveResolver};
use resource_viewer::MoveValueAnnotator;

use anyhow::Result;

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
        data.map(|(typ, bytes)| Ok(self.inner.view_resource(&typ, bytes)?.into()))
            .collect()
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
                let payload = self.try_into_transaction_payload(txn.payload())?;
                (version, txn, info, events, payload).into()
            }
            GenesisTransaction(txn) => (version, txn, info, events).into(),
            BlockMetadata(txn) => (version, txn, info).into(),
        };
        Ok(ret)
    }

    pub fn try_into_transaction_payload(
        &self,
        payload: &diem_types::transaction::TransactionPayload,
    ) -> Result<TransactionPayload> {
        use diem_types::transaction::TransactionPayload::*;
        let payload = match payload {
            WriteSet(_) => TransactionPayload::WriteSetPayload,
            Script(s) => s.into(),
            Module(_) => TransactionPayload::ModulePayload,
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
        Ok(payload)
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
