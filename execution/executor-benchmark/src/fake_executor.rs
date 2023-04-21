// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    benchmark_transaction::{BenchmarkTransaction, ExtraInfo},
    db_access::{Account, CoinStore, DbAccessUtil, TOTAL_SUPPLY_STATE_KEY},
    metrics::TIMER,
};
use anyhow::Result;
use aptos_executor::{
    block_executor::TransactionBlockExecutor, components::chunk_output::ChunkOutput,
};
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{deposit::DepositEvent, withdraw::WithdrawEvent},
    contract_event::ContractEvent,
    event::EventKey,
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    vm_status::AbortLocation,
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::{Lazy, OnceCell};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::str::FromStr;

pub struct FakeExecutor {}

static FAKE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static FAKE_EXECUTOR_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPoolBuilder::new()
        .num_threads(FakeExecutor::get_concurrency_level())
        .thread_name(|index| format!("fake_exe_{}", index))
        .build()
        .unwrap()
});

impl FakeExecutor {
    pub fn set_concurrency_level_once(concurrency_level: usize) {
        FAKE_EXECUTOR_CONCURRENCY_LEVEL.set(concurrency_level).ok();
    }

    pub fn get_concurrency_level() -> usize {
        match FAKE_EXECUTOR_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 32,
        }
    }

    fn handle_account_creation_and_transfer(
        sender_address: AccountAddress,
        new_account_address: AccountAddress,
        initial_balance: u64,
        state_view: &CachedStateView,
        fail_on_existing: bool,
        fail_on_missing: bool,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["account_creation"]).start_timer();
        let sender_account_key = DbAccessUtil::new_state_key_account(sender_address);
        let mut sender_account = {
            let _timer = TIMER
                .with_label_values(&["read_sender_account"])
                .start_timer();
            DbAccessUtil::get_account(&sender_account_key, state_view)?.unwrap()
        };
        let sender_coin_store_key = DbAccessUtil::new_state_key_aptos_coin(sender_address);
        let mut sender_coin_store = {
            let _timer = TIMER
                .with_label_values(&["read_sender_coin_store"])
                .start_timer();
            DbAccessUtil::get_coin_store(&sender_coin_store_key, state_view)?.unwrap()
        };

        let new_account_key = DbAccessUtil::new_state_key_account(new_account_address);
        let new_coin_store_key = DbAccessUtil::new_state_key_aptos_coin(new_account_address);

        let new_account = {
            let _timer = TIMER.with_label_values(&["read_new_account"]).start_timer();
            DbAccessUtil::get_account(&new_account_key, state_view)?
        };

        // Note: numbers below may not be real. When runninng in parallel there might be conflicts.
        sender_coin_store.coin -= initial_balance;

        let gas = 1;
        sender_coin_store.coin -= gas;

        sender_account.sequence_number += 1;

        let mut total_supply: u128 =
            DbAccessUtil::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
        total_supply -= gas as u128;

        // TODO(grao): Add other reads to match the read set of the real transaction.
        let mut write_set = vec![
            (
                sender_account_key,
                WriteOp::Modification(bcs::to_bytes(&sender_account)?),
            ),
            (
                sender_coin_store_key,
                WriteOp::Modification(bcs::to_bytes(&sender_coin_store)?),
            ),
            (
                TOTAL_SUPPLY_STATE_KEY.clone(),
                WriteOp::Modification(bcs::to_bytes(&total_supply)?),
            ),
        ];

        if new_account.is_some() {
            if fail_on_existing {
                return Ok(TransactionOutput::new(
                    Default::default(),
                    vec![],
                    0,
                    TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                        location: AbortLocation::Module(ModuleId::new(
                            AccountAddress::ONE,
                            Identifier::from_str("account").unwrap(),
                        )),
                        code: 7,
                        info: None,
                    }),
                ));
            }

            let mut new_coin_store = {
                let _timer = TIMER
                    .with_label_values(&["read_new_coin_store"])
                    .start_timer();
                DbAccessUtil::get_coin_store(&new_coin_store_key, state_view)?.unwrap()
            };

            if initial_balance != 0 {
                new_coin_store.coin += initial_balance;

                write_set.push((
                    new_coin_store_key,
                    WriteOp::Modification(bcs::to_bytes(&new_coin_store)?),
                ));
            }
        } else {
            if fail_on_missing {
                return Ok(TransactionOutput::new(
                    Default::default(),
                    vec![],
                    0,
                    TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                        location: AbortLocation::Module(ModuleId::new(
                            AccountAddress::ONE,
                            Identifier::from_str("account").unwrap(),
                        )),
                        code: 7,
                        info: None,
                    }),
                ));
            }

            {
                let _timer = TIMER
                    .with_label_values(&["read_new_coin_store"])
                    .start_timer();
                assert!(DbAccessUtil::get_coin_store(&new_coin_store_key, state_view)?.is_none());
            }

            let new_account = Account {
                authentication_key: new_account_address.to_vec(),
                ..Default::default()
            };

            let new_coin_store = CoinStore {
                coin: initial_balance,
                ..Default::default()
            };

            write_set.push((
                new_account_key,
                WriteOp::Creation(bcs::to_bytes(&new_account)?),
            ));
            write_set.push((
                new_coin_store_key,
                WriteOp::Creation(bcs::to_bytes(&new_coin_store)?),
            ));
        }

        // TODO(grao): Some values are fake, because I'm lazy.
        let events = vec![
            ContractEvent::new(
                EventKey::new(0, sender_address),
                0,
                TypeTag::Struct(Box::new(WithdrawEvent::struct_tag())),
                sender_address.to_vec(),
            ),
            ContractEvent::new(
                EventKey::new(0, new_account_address),
                0,
                TypeTag::Struct(Box::new(DepositEvent::struct_tag())),
                new_account_address.to_vec(),
            ),
            // TODO(grao): CoinRegisterEvent
        ];
        Ok(TransactionOutput::new(
            WriteSetMut::new(write_set).freeze()?,
            events,
            /*gas_used=*/ gas,
            TransactionStatus::Keep(ExecutionStatus::Success),
        ))
    }

    fn handle_state_checkpoint() -> Result<TransactionOutput> {
        Ok(TransactionOutput::new(
            WriteSet::default(),
            vec![],
            /*gas_used=*/ 0,
            TransactionStatus::Keep(ExecutionStatus::Success),
        ))
    }
}

impl TransactionBlockExecutor<BenchmarkTransaction> for FakeExecutor {
    fn execute_transaction_block(
        transactions: Vec<BenchmarkTransaction>,
        state_view: CachedStateView,
    ) -> Result<ChunkOutput> {
        let transaction_outputs = FAKE_EXECUTOR_POOL.install(|| {
            transactions
                .par_iter()
                .map(|txn| match &txn.extra_info {
                    Some(extra_info) => match &extra_info {
                        ExtraInfo::TransferInfo(transfer_info) => {
                            Self::handle_account_creation_and_transfer(
                                transfer_info.sender,
                                transfer_info.receiver,
                                transfer_info.amount,
                                &state_view,
                                false,
                                true,
                            )
                        },
                        ExtraInfo::AccountCreationInfo(account_creation_info) => {
                            Self::handle_account_creation_and_transfer(
                                account_creation_info.sender,
                                account_creation_info.new_account,
                                account_creation_info.initial_balance,
                                &state_view,
                                false,
                                false,
                            )
                        },
                    },
                    None => match &txn.transaction {
                        Transaction::StateCheckpoint(_) => Self::handle_state_checkpoint(),
                        Transaction::UserTransaction(user_txn) => match user_txn.payload() {
                            aptos_types::transaction::TransactionPayload::EntryFunction(f) => {
                                match (
                                    *f.module().address(),
                                    f.module().name().as_str(),
                                    f.function().as_str(),
                                ) {
                                    (AccountAddress::ONE, "coin", "transfer") => {
                                        Self::handle_account_creation_and_transfer(
                                            user_txn.sender(),
                                            bcs::from_bytes(&f.args()[0]).unwrap(),
                                            bcs::from_bytes(&f.args()[1]).unwrap(),
                                            &state_view,
                                            false,
                                            true,
                                        )
                                    },
                                    (AccountAddress::ONE, "aptos_account", "transfer") => {
                                        Self::handle_account_creation_and_transfer(
                                            user_txn.sender(),
                                            bcs::from_bytes(&f.args()[0]).unwrap(),
                                            bcs::from_bytes(&f.args()[1]).unwrap(),
                                            &state_view,
                                            false,
                                            false,
                                        )
                                    },
                                    (AccountAddress::ONE, "aptos_account", "create_account") => {
                                        Self::handle_account_creation_and_transfer(
                                            user_txn.sender(),
                                            bcs::from_bytes(&f.args()[0]).unwrap(),
                                            0,
                                            &state_view,
                                            true,
                                            false,
                                        )
                                    },
                                    _ => unimplemented!(
                                        "{} {}::{}",
                                        *f.module().address(),
                                        f.module().name().as_str(),
                                        f.function().as_str()
                                    ),
                                }
                            },
                            _ => unimplemented!(),
                        },
                        _ => unimplemented!(),
                    },
                })
                .collect::<Result<Vec<_>>>()
        })?;
        Ok(ChunkOutput {
            transactions: transactions
                .into_iter()
                .map(|txn| txn.transaction)
                .collect(),
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
        })
    }
}
