// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::Context,
    failpoint::fail_point,
    metrics::metrics,
    param::{AddressParam, LedgerVersionParam, MoveIdentifierParam, MoveStructTagParam},
    version::Version,
};
use std::cmp::min;

use aptos_api_types::{
    AccountData, Address, AsConverter, Error, HashValue, LedgerInfo, MoveModuleBytecode, Response,
    TransactionId, TransactionOnChainData,
};
use aptos_types::{
    account_config::AccountResource,
    account_state::AccountState,
    event::{EventHandle, EventKey},
};

use crate::param::Param;
use anyhow::Result;
use aptos_types::{
    access_path::AccessPath, state_store::state_key::StateKey, transaction::Transaction,
};
use move_deps::move_core_types::{
    identifier::Identifier,
    language_storage::{ResourceKey, StructTag},
    move_resource::MoveStructType,
    value::MoveValue,
};
use std::convert::TryInto;
use warp::{filters::BoxedFilter, Filter, Rejection, Reply};

pub type BlockParam = Param<u64>;

// GET /blocks/<address>
pub fn get_block(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("blocks" / BlockParam)
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_get_block())
        .with(metrics("get_block"))
        .boxed()
}

// GET /genesis
pub fn get_genesis_ledger_info(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("genesis")
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_get_genesis_ledger_info())
        .with(metrics("get_genesis_ledger_info"))
        .boxed()
}

async fn handle_get_block(
    ledger_version: Option<LedgerVersionParam>,
    context: Context,
) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_block")?;
    Ok(Block::new(ledger_version, context)?.account()?)
}

async fn handle_get_genesis_ledger_info(context: Context) -> Result<impl Reply, Rejection> {
    fail_point("endpoint_get_genesis_ledger_info")?;
    Ok(context.get_genesis_ledger_info()?)
}

pub struct BlockDescription {
    starting_version: u64,
    ending_version: u64,
    num_transactions: u64,
    hash: HashValue,
}

pub(crate) struct Block {
    ledger_version: u64,
    latest_ledger_info: LedgerInfo,
    context: Context,
}

impl Block {
    pub fn new(
        ledger_version: Option<LedgerVersionParam>,
        context: Context,
    ) -> Result<Self, Error> {
        let latest_ledger_info = context.get_latest_ledger_info()?;
        let ledger_version = ledger_version
            .map(|v| v.parse("ledger version"))
            .unwrap_or_else(|| Ok(latest_ledger_info.version()))?;

        if ledger_version > latest_ledger_info.version() {
            return Err(Error::not_found(
                "ledger",
                TransactionId::Version(ledger_version),
                latest_ledger_info.version(),
            ));
        }

        Ok(Self {
            ledger_version,
            latest_ledger_info,
            context,
        })
    }

    /// Scans the DB
    pub fn find_block(self) -> Result<BlockDescription, Error> {
        // Genesis is as special case, always version 0
        if self.ledger_version == 0 {
            let hash = self.context.get_genesis_accumulator_hash()?;
            return Ok(BlockDescription {
                starting_version: 0,
                ending_version: 0,
                num_transactions: 1,
                hash: hash.into(),
            });
        }

        // Every other block needs to be found
        const MAX_BLOCK_SIZE: u16 = 10000;
        let search_start = self.ledger_version.saturating_sub(MAX_BLOCK_SIZE as u64);
        let txns =
            self.context
                .get_transactions(search_start, MAX_BLOCK_SIZE * 2, self.ledger_version)?;

        // Search for the closest block boundaries to the version
        let mut start = search_start;
        let mut end = search_start.saturating_add((MAX_BLOCK_SIZE * 2) as u64);
        let mut hash = None;
        for txn in txns {
            let txn_version = txn.version;
            match &txn.transaction {
                Transaction::BlockMetadata(_) => {
                    // BlockMetadata is the beginning of the block
                    if start < txn_version && txn_version <= self.ledger_version {
                        start = txn_version;
                    }
                }
                Transaction::StateCheckpoint => {
                    // StateCheckpoint is at the end of the block
                    if end > txn_version && txn_version >= self.ledger_version {
                        end = txn_version;
                        hash = Some(txn.accumulator_root_hash.into());
                        break;
                    }
                }
                _ => {}
            }
        }

        // If there isn't a hash, we didn't find it!
        if let Some(hash) = hash {
            Ok(BlockDescription {
                starting_version: start,
                ending_version: end,
                num_transactions: end.saturating_sub(start).saturating_add(1),
                hash,
            })
        } else {
            Err(Error::not_found("block", "block", self.ledger_version))
        }
    }
}
