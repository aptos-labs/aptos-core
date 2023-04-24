// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    models::processor_status::ProcessorStatus,
    schema::processor_status,
    utils::{
        counters::{GOT_CONNECTION_COUNT, UNABLE_TO_GET_CONNECTION_COUNT},
        database::{execute_with_better_error, PgDbPool, PgPoolConnection},
    },
};
use aptos_protos::transaction::testing1::v1::Transaction as ProtoTransaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, prelude::*};
use std::fmt::Debug;

type StartVersion = u64;
type EndVersion = u64;
pub type ProcessingResult = (StartVersion, EndVersion);

/// Base trait for all processors
#[async_trait]
pub trait ProcessorTrait: Send + Sync + Debug {
    fn name(&self) -> &'static str;

    /// Process all transactions including writing to the database
    async fn process_transactions(
        &self,
        transactions: Vec<ProtoTransaction>,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<ProcessingResult>;

    /// Gets a reference to the connection pool
    /// This is used by the `get_conn()` helper below
    fn connection_pool(&self) -> &PgDbPool;

    //* Below are helper methods that don't need to be implemented *//

    /// Gets the connection.
    /// If it was unable to do so (default timeout: 30s), it will keep retrying until it can.
    fn get_conn(&self) -> PgPoolConnection {
        let pool = self.connection_pool();
        loop {
            match pool.get() {
                Ok(conn) => {
                    GOT_CONNECTION_COUNT.inc();
                    return conn;
                },
                Err(err) => {
                    UNABLE_TO_GET_CONNECTION_COUNT.inc();
                    aptos_logger::error!(
                        "Could not get DB connection from pool, will retry in {:?}. Err: {:?}",
                        pool.connection_timeout(),
                        err
                    );
                },
            };
        }
    }

    /// Store last processed version from database. We can assume that all previously processed
    /// versions are successful because any gap would cause the processor to panic
    async fn update_last_processed_version(&self, version: u64) -> anyhow::Result<()> {
        let mut conn = self.get_conn();
        let status = ProcessorStatus {
            processor: self.name().to_string(),
            last_success_version: version as i64,
        };
        execute_with_better_error(
            &mut conn,
            diesel::insert_into(processor_status::table)
                .values(&status)
                .on_conflict(processor_status::processor)
                .do_update()
                .set((
                    processor_status::last_success_version
                        .eq(excluded(processor_status::last_success_version)),
                    processor_status::last_updated.eq(excluded(processor_status::last_updated)),
                )),
            Some(" WHERE processor_status.last_success_version <= EXCLUDED.last_success_version "),
        )?;
        Ok(())
    }
}
