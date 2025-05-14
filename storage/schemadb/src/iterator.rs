// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    IntoDbResult, KeyCodec, Schema, SeekKeyCodec, ValueCodec, APTOS_SCHEMADB_ITER_BYTES,
    APTOS_SCHEMADB_ITER_LATENCY_SECONDS, APTOS_SCHEMADB_SEEK_LATENCY_SECONDS,
};
use std::marker::PhantomData;

#[derive(PartialEq)]
pub enum ScanDirection {
    Forward,
    Backward,
}

enum Status {
    Initialized,
    DoneSeek,
    Advancing,
    Invalid,
}

/// DB Iterator parameterized on [`Schema`] that seeks with [`Schema::Key`] and yields
/// [`Schema::Key`] and [`Schema::Value`]
pub struct SchemaIterator<'a, S> {
    db_iter: rocksdb::DBRawIterator<'a>,
    direction: ScanDirection,
    status: Status,
    phantom: PhantomData<S>,
}

impl<'a, S> SchemaIterator<'a, S>
where
    S: Schema,
{
    pub(crate) fn new(db_iter: rocksdb::DBRawIterator<'a>, direction: ScanDirection) -> Self {
        SchemaIterator {
            db_iter,
            direction,
            status: Status::Initialized,
            phantom: PhantomData,
        }
    }

    /// Seeks to the first key.
    pub fn seek_to_first(&mut self) {
        let _timer = APTOS_SCHEMADB_SEEK_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME, "seek_to_first"])
            .start_timer();
        self.db_iter.seek_to_first();
        self.status = Status::DoneSeek;
    }

    /// Seeks to the last key.
    pub fn seek_to_last(&mut self) {
        let _timer = APTOS_SCHEMADB_SEEK_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME, "seek_to_last"])
            .start_timer();
        self.db_iter.seek_to_last();
        self.status = Status::DoneSeek;
    }

    /// Seeks to the first key whose binary representation is equal to or greater than that of the
    /// `seek_key`.
    pub fn seek<SK>(&mut self, seek_key: &SK) -> aptos_storage_interface::Result<()>
    where
        SK: SeekKeyCodec<S>,
    {
        let _timer = APTOS_SCHEMADB_SEEK_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME, "seek"])
            .start_timer();
        let key = <SK as SeekKeyCodec<S>>::encode_seek_key(seek_key)?;
        self.db_iter.seek(&key);
        self.status = Status::DoneSeek;
        Ok(())
    }

    /// Seeks to the last key whose binary representation is less than or equal to that of the
    /// `seek_key`.
    ///
    /// See example in [`RocksDB doc`](https://github.com/facebook/rocksdb/wiki/SeekForPrev).
    pub fn seek_for_prev<SK>(&mut self, seek_key: &SK) -> aptos_storage_interface::Result<()>
    where
        SK: SeekKeyCodec<S>,
    {
        let _timer = APTOS_SCHEMADB_SEEK_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME, "seek_for_prev"])
            .start_timer();
        let key = <SK as SeekKeyCodec<S>>::encode_seek_key(seek_key)?;
        self.db_iter.seek_for_prev(&key);
        self.status = Status::DoneSeek;
        Ok(())
    }

    fn next_impl(&mut self) -> aptos_storage_interface::Result<Option<(S::Key, S::Value)>> {
        let _timer = APTOS_SCHEMADB_ITER_LATENCY_SECONDS
            .with_label_values(&[S::COLUMN_FAMILY_NAME])
            .start_timer();

        if let Status::Advancing = self.status {
            match self.direction {
                ScanDirection::Forward => self.db_iter.next(),
                ScanDirection::Backward => self.db_iter.prev(),
            }
        } else {
            self.status = Status::Advancing;
        }

        if !self.db_iter.valid() {
            self.db_iter.status().into_db_res()?;
            // advancing an invalid raw iter results in seg fault
            self.status = Status::Invalid;
            return Ok(None);
        }

        let raw_key = self.db_iter.key().expect("db_iter.key() failed.");
        let raw_value = self.db_iter.value().expect("db_iter.value(0 failed.");
        APTOS_SCHEMADB_ITER_BYTES
            .with_label_values(&[S::COLUMN_FAMILY_NAME])
            .observe((raw_key.len() + raw_value.len()) as f64);

        let key = <S::Key as KeyCodec<S>>::decode_key(raw_key);
        let value = <S::Value as ValueCodec<S>>::decode_value(raw_value);

        Ok(Some((key?, value?)))
    }
}

impl<S> Iterator for SchemaIterator<'_, S>
where
    S: Schema,
{
    type Item = aptos_storage_interface::Result<(S::Key, S::Value)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
