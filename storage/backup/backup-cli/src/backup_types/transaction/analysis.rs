// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_types::{
    contract_event::ContractEvent,
    transaction::{PersistedAuxiliaryInfo, Transaction, TransactionInfo, Version},
    write_set::{TransactionWrite, WriteSet},
};
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

#[derive(Serialize)]
struct TransactionRow {
    version: Version,
    transaction_size: usize,
    events_size: usize,
    write_set_size: usize,
}

#[derive(Serialize)]
struct WriteOpRow {
    version: Version,
    index: usize,
    write_op_size: usize,
}

#[derive(Serialize)]
struct EventRow {
    version: Version,
    index: usize,
    event_size: usize,
}

pub struct TransactionAnalysis {
    txn_writer: csv::Writer<File>,
    event_writer: csv::Writer<File>,
    write_op_writer: csv::Writer<File>,
}

impl TransactionAnalysis {
    pub fn new(output_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(output_dir)?;
        let txn_writer = Self::open_csv_writer(output_dir, "transaction.csv")?;
        let event_writer = Self::open_csv_writer(output_dir, "event.csv")?;
        let write_op_writer = Self::open_csv_writer(output_dir, "write_op.csv")?;

        Ok(Self {
            txn_writer,
            event_writer,
            write_op_writer,
        })
    }

    fn open_csv_writer(output_dir: &Path, filename: &str) -> Result<csv::Writer<File>> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output_dir.join(filename))?;

        Ok(csv::Writer::from_writer(file))
    }

    pub fn add_transaction(
        &mut self,
        version: Version,
        txn: &Transaction,
        _persisted_aux_info: &PersistedAuxiliaryInfo,
        _txn_info: &TransactionInfo,
        events: &[ContractEvent],
        write_set: &WriteSet,
    ) -> Result<()> {
        let mut events_size = 0;
        for (index, event) in events.iter().enumerate() {
            let event_size = event.size();
            events_size += event_size;

            self.event_writer.serialize(EventRow {
                version,
                index,
                event_size,
            })?;
        }

        let mut write_set_size = 0;
        for (index, (key, op)) in write_set.write_op_iter().enumerate() {
            let write_op_size = key.size() + op.as_state_value().map_or(0, |value| value.size());
            write_set_size += write_op_size;

            self.write_op_writer.serialize(WriteOpRow {
                version,
                index,
                write_op_size,
            })?;
        }

        let transaction_size = Self::txn_size(txn);
        self.txn_writer.serialize(TransactionRow {
            version,
            transaction_size,
            events_size,
            write_set_size,
        })?;

        Ok(())
    }

    fn txn_size(txn: &Transaction) -> usize {
        use Transaction::*;

        match txn {
            UserTransaction(signed_txn) => signed_txn.raw_txn_bytes_len(),
            GenesisTransaction(_)
            | BlockMetadata(_)
            | BlockMetadataExt(_)
            | StateCheckpoint(_)
            | BlockEpilogue(_)
            | ValidatorTransaction(_) => bcs::serialized_size(txn).expect("Txn should serialize"),
        }
    }
}
