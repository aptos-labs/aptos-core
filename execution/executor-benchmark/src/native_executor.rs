// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_access::DbAccessUtil, metrics::TIMER};
use anyhow::Result;
use aptos_executor::{
    block_executor::TransactionBlockExecutor, components::chunk_output::ChunkOutput,
};
use aptos_sdk::types::get_apt_primary_store_address;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{
        lite_account, lite_account::LiteAccountGroup, withdraw::WithdrawEvent,
        FungibleStoreResource, ObjectGroupResource,
    },
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableTransactions},
    contract_event::ContractEvent,
    event::EventKey,
    state_store::state_key::StateKey,
    transaction::{
        ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionOutput,
        TransactionStatus,
    },
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::{language_storage::TypeTag, move_resource::MoveStructType};
use once_cell::sync::{Lazy, OnceCell};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::collections::HashMap;
use aptos_types::account_config::lite_account::AccountResource;

struct IncrementalOutput {
    write_set: Vec<(StateKey, WriteOp)>,
    events: Vec<ContractEvent>,
}

impl IncrementalOutput {
    fn into_success_output(self) -> Result<TransactionOutput> {
        Ok(TransactionOutput::new(
            WriteSetMut::new(self.write_set).freeze()?,
            self.events,
            /*gas_used=*/ 1,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        ))
    }

    fn append(&mut self, mut other: IncrementalOutput) {
        self.write_set.append(&mut other.write_set);
        self.events.append(&mut other.events);
    }

    fn to_abort(status: TransactionStatus) -> TransactionOutput {
        TransactionOutput::new(
            Default::default(),
            vec![],
            0,
            status,
            TransactionAuxiliaryData::default(),
        )
    }
}

pub struct NativeExecutor {}

static NATIVE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static NATIVE_EXECUTOR_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPoolBuilder::new()
        .num_threads(NativeExecutor::get_concurrency_level())
        .thread_name(|index| format!("native_exe_{}", index))
        .build()
        .unwrap()
});

impl NativeExecutor {
    pub fn set_concurrency_level_once(concurrency_level: usize) {
        NATIVE_EXECUTOR_CONCURRENCY_LEVEL
            .set(concurrency_level)
            .ok();
    }

    pub fn get_concurrency_level() -> usize {
        match NATIVE_EXECUTOR_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 32,
        }
    }

    fn withdraw_from_signer(
        sender_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let mut sender_fungible_group = {
            let _timer = TIMER
                .with_label_values(&["read_sender_fungible_store_resource_group"])
                .start_timer();
            DbAccessUtil::get_fungible_store_group(&sender_address, state_view)?.unwrap()
        };

        let mut fungible_store = if let Some(fungible_store_resource) = sender_fungible_group
            .group
            .get_mut(&FungibleStoreResource::struct_tag())
            .and_then(|bytes| Some(bcs::from_bytes(bytes).unwrap()))
        {
            fungible_store_resource
        } else {
            FungibleStoreResource::new(sender_address, 0, false)
        };

        let mut lite_account = if let Some(lite_account_resource) = sender_fungible_group
            .group
            .get_mut(&AccountResource::struct_tag())
            .and_then(|bytes| Some(bcs::from_bytes(bytes).unwrap()))
        {
            lite_account_resource
        } else {
            AccountResource { sequence_number: 0 }
        };
        lite_account.sequence_number += 1;

        // Note: numbers below may not be real. When runninng in parallel there might be conflicts.
        fungible_store.balance -= transfer_amount;

        let gas = 1;
        fungible_store.balance -= gas;

        sender_fungible_group.group.insert(AccountResource::struct_tag(), bcs::to_bytes(&lite_account).unwrap());
        sender_fungible_group.group.insert(FungibleStoreResource::struct_tag(), bcs::to_bytes(&fungible_store).unwrap());

        // add total supply via aggregators?
        // let mut total_supply: u128 =
        //     DbAccessUtil::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
        // total_supply -= gas as u128;

        // TODO(grao): Add other reads to match the read set of the real transaction.
        let write_set = vec![
            (
                StateKey::resource_group(
                    &get_apt_primary_store_address(sender_address),
                    &ObjectGroupResource::struct_tag(),
                ),
                WriteOp::legacy_modification(bcs::to_bytes(&sender_fungible_group)?.into()),
            ),
            // (
            //     TOTAL_SUPPLY_STATE_KEY.clone(),
            //     WriteOp::legacy_modification(bcs::to_bytes(&total_supply)?),
            // ),
        ];

        // TODO(grao): Some values are fake, because I'm lazy.
        let events = vec![ContractEvent::new_v1(
            EventKey::new(0, sender_address),
            0,
            TypeTag::Struct(Box::new(WithdrawEvent::struct_tag())),
            sender_address.to_vec(),
        )];
        Ok(Ok(IncrementalOutput { write_set, events }))
    }

    fn deposit(
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let mut write_set = Vec::new();

        let mut recipient_fungible_store_group = {
            let _timer = TIMER
                .with_label_values(&["read_new_fungible_store_resource_group"])
                .start_timer();
            DbAccessUtil::get_fungible_store_group(&recipient_address, state_view)?
                .unwrap_or_default()
        };

        let mut fungible_store = if let Some(fungible_store_resource) =
            recipient_fungible_store_group
                .group
                .get(&FungibleStoreResource::struct_tag())
                .and_then(|bytes| Some(bcs::from_bytes(bytes).unwrap()))
        {
            fungible_store_resource
        } else {
            FungibleStoreResource::new(recipient_address, 0, false)
        };

        if transfer_amount != 0 {
            fungible_store.balance += transfer_amount;

            recipient_fungible_store_group.insert(
                FungibleStoreResource::struct_tag(),
                bcs::to_bytes(&fungible_store).unwrap(),
            );
            write_set.push((
                StateKey::resource_group(
                    &get_apt_primary_store_address(recipient_address),
                    &ObjectGroupResource::struct_tag(),
                ),
                WriteOp::legacy_modification(recipient_fungible_store_group.to_bytes()?.into()),
            ));
        }

        Ok(Ok(IncrementalOutput {
            write_set,
            events: vec![],
        }))
    }

    fn handle_account_creation_and_transfer(
        sender_address: AccountAddress,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["account_creation"]).start_timer();

        let mut output = {
            let output = Self::withdraw_from_signer(sender_address, transfer_amount, state_view)?;
            match output {
                Ok(output) => output,
                Err(status) => return Ok(IncrementalOutput::to_abort(status)),
            }
        };

        let deposit_output = Self::deposit(recipient_address, transfer_amount, state_view)?;

        match deposit_output {
            Ok(deposit_output) => {
                output.append(deposit_output);
                output.into_success_output()
            },
            Err(status) => Ok(IncrementalOutput::to_abort(status)),
        }
    }

    fn handle_batch_account_creation_and_transfer(
        sender_address: AccountAddress,
        recipient_addresses: Vec<AccountAddress>,
        transfer_amounts: Vec<u64>,
        state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        let mut deltas = HashMap::new();
        for (recipient, amount) in recipient_addresses
            .into_iter()
            .zip(transfer_amounts.into_iter())
        {
            let amount = amount as i64;
            deltas
                .entry(recipient)
                .and_modify(|counter| *counter += amount)
                .or_insert(amount);
            deltas
                .entry(sender_address)
                .and_modify(|counter| *counter -= amount)
                .or_insert(-amount);
        }

        let amount_to_sender = -deltas.remove(&sender_address).unwrap_or(0);

        assert!(amount_to_sender >= 0);
        let mut output = {
            let output =
                Self::withdraw_from_signer(sender_address, amount_to_sender as u64, state_view)?;
            match output {
                Ok(output) => output,
                Err(status) => return Ok(IncrementalOutput::to_abort(status)),
            }
        };

        for (recipient_address, transfer_amount) in deltas.into_iter() {
            output.append({
                let deposit_output =
                    Self::deposit(recipient_address, transfer_amount as u64, state_view)?;

                match deposit_output {
                    Ok(deposit_output) => deposit_output,
                    Err(status) => return Ok(IncrementalOutput::to_abort(status)),
                }
            });
        }

        output.into_success_output()
    }

    fn handle_state_checkpoint() -> Result<TransactionOutput> {
        Ok(TransactionOutput::new(
            WriteSet::default(),
            vec![],
            /*gas_used=*/ 0,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        ))
    }
}

impl TransactionBlockExecutor for NativeExecutor {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ChunkOutput> {
        let transactions = match transactions {
            ExecutableTransactions::Unsharded(txns) => txns,
            _ => todo!("sharded execution not yet supported"),
        };
        let transaction_outputs = NATIVE_EXECUTOR_POOL.install(|| {
            transactions
                .par_iter()
                .map(|txn| match &txn.expect_valid() {
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
                                    )
                                },
                                (AccountAddress::ONE, "aptos_account", "transfer") => {
                                    Self::handle_account_creation_and_transfer(
                                        user_txn.sender(),
                                        bcs::from_bytes(&f.args()[0]).unwrap(),
                                        bcs::from_bytes(&f.args()[1]).unwrap(),
                                        &state_view,
                                    )
                                },
                                (AccountAddress::ONE, "aptos_account", "create_account") => {
                                    Self::handle_account_creation_and_transfer(
                                        user_txn.sender(),
                                        bcs::from_bytes(&f.args()[0]).unwrap(),
                                        0,
                                        &state_view,
                                    )
                                },
                                (AccountAddress::ONE, "aptos_account", "batch_transfer") => {
                                    Self::handle_batch_account_creation_and_transfer(
                                        user_txn.sender(),
                                        bcs::from_bytes(&f.args()[0]).unwrap(),
                                        bcs::from_bytes(&f.args()[1]).unwrap(),
                                        &state_view,
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
                })
                .collect::<Result<Vec<_>>>()
        })?;
        Ok(ChunkOutput {
            transactions: transactions.into_iter().map(|t| t.into_inner()).collect(),
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
            block_end_info: None,
        })
    }
}
