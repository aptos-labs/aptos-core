// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_access::{Account, CoinStore, DbAccessUtil},
    metrics::TIMER,
};
use anyhow::Result;
use aptos_executor::block_executor::TransactionBlockExecutor;
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{deposit::DepositEvent, primary_apt_store, withdraw::WithdrawEvent, DepositFAEvent, FungibleStoreResource, MoveEventV2, WithdrawFAEvent},
    block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableTransactions},
    contract_event::ContractEvent,
    event::EventKey,
    state_store::state_key::StateKey,
    transaction::{
        ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionOutput,
        TransactionStatus,
    },
    vm_status::AbortLocation,
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::{Lazy, OnceCell};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::collections::{BTreeMap, HashMap};

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

/// Native block executor (replacing both BlockSTM and AptosVM), that is
/// "loose", i.e. doesn't compute outputs correctly.
/// It's loose in multiple ways:
/// - it ignores conflicts. All transactions see the state at the start of the block!
/// - it doesn't put everything in the writeset that should be there
/// - it doesn't compute gas
pub struct NativeLooseBlockExecutor {}

static NATIVE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static NATIVE_EXECUTOR_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPoolBuilder::new()
        .num_threads(NativeLooseBlockExecutor::get_concurrency_level())
        .thread_name(|index| format!("native_exe_{}", index))
        .build()
        .unwrap()
});

impl NativeLooseBlockExecutor {
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


    fn withdraw_fa_apt_from_signer(
        sender_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
        gas: u64,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let sender_account_key = DbAccessUtil::new_state_key_account(sender_address);
        let mut sender_account = {
            let _timer = TIMER
                .with_label_values(&["read_sender_account"])
                .start_timer();
            DbAccessUtil::get_account(&sender_account_key, state_view)?.unwrap()
        };
        let sender_store_address = primary_apt_store(sender_address);

        let sender_fa_store_object_key = DbAccessUtil::new_state_key_object_resource_group(&sender_store_address);
        let mut sender_fa_store_object = {
            let _timer = TIMER
                .with_label_values(&["read_sender_fa_store"])
                .start_timer();
            match DbAccessUtil::get_resource_group( &sender_fa_store_object_key, state_view)? {
                Some(sender_fa_store_object) => sender_fa_store_object,
                None => panic!("couldn't find {:?}", sender_fa_store_object_key)

                // return Ok(Ok(IncrementalOutput { write_set: vec![], events: vec![] }))

                // Ok(Err(TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                //     location: AbortLocation::Module(ModuleId::new(
                //         AccountAddress::ONE,
                //         ident_str!("fungible_asset").into(),
                //     )),
                //     code: 7,
                //     info: None,
                // })))
            }
        };

        let fungible_store_rg_tag = FungibleStoreResource::struct_tag();
        let mut sender_fa_store = bcs::from_bytes::<FungibleStoreResource>(&sender_fa_store_object.remove(&fungible_store_rg_tag).unwrap())?;

        // Note: numbers below may not be real. When runninng in parallel there might be conflicts.
        sender_fa_store.balance -= transfer_amount + gas;

        sender_fa_store_object.insert(fungible_store_rg_tag, bcs::to_bytes(&sender_fa_store)?);

        sender_account.sequence_number += 1;

        // add total supply via aggregators?
        // let mut total_supply: u128 =
        //     DbAccessUtil::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
        // total_supply -= gas as u128;

        // TODO(grao): Add other reads to match the read set of the real transaction.
        let write_set = vec![
            (
                sender_account_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_account)?.into()),
            ),
            (
                sender_fa_store_object_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_fa_store_object)?.into()),
            ),
            // (
            //     TOTAL_SUPPLY_STATE_KEY.clone(),
            //     WriteOp::legacy_modification(bcs::to_bytes(&total_supply)?),
            // ),
        ];

        let event = WithdrawFAEvent {
            store: sender_store_address,
            amount: transfer_amount,
        };

        let events = vec![
            event.create_event_v2()
        ];
        Ok(Ok(IncrementalOutput { write_set, events }))
    }

    fn deposit_fa_apt(
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let recipient_store_address = primary_apt_store(recipient_address);
        let recipient_fa_store_object_key = DbAccessUtil::new_state_key_object_resource_group(&recipient_store_address);
        let fungible_store_rg_tag = FungibleStoreResource::struct_tag();

        let (recipient_fa_store, mut recipient_fa_store_object, recipient_fa_store_existed) = {
            let _timer = TIMER
                .with_label_values(&["read_recipient_fa_store"])
                .start_timer();
            match DbAccessUtil::get_resource_group(&recipient_fa_store_object_key, state_view)? {
                Some(mut recipient_fa_store_object) => {
                    let mut recipient_fa_store = bcs::from_bytes::<FungibleStoreResource>(&recipient_fa_store_object.remove(&fungible_store_rg_tag).unwrap())?;
                    recipient_fa_store.balance += transfer_amount;
                    (recipient_fa_store, recipient_fa_store_object, true)
                },
                None => {
                    let receipeint_fa_store = FungibleStoreResource::new(AccountAddress::TEN, transfer_amount, false);
                    let receipeint_fa_store_object = BTreeMap::new();
                    (receipeint_fa_store, receipeint_fa_store_object, false)
                },
            }
        };

        recipient_fa_store_object.insert(fungible_store_rg_tag, bcs::to_bytes(&recipient_fa_store)?);


        // Note: numbers below may not be real. When runninng in parallel there might be conflicts.

        // add total supply via aggregators?
        // let mut total_supply: u128 =
        //     DbAccessUtil::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
        // total_supply -= gas as u128;

        // TODO(grao): Add other reads to match the read set of the real transaction.
        let write_set = vec![
            (
                recipient_fa_store_object_key,
                if recipient_fa_store_existed {
                    WriteOp::legacy_modification(bcs::to_bytes(&recipient_fa_store_object)?.into())
                } else {
                    WriteOp::legacy_creation(bcs::to_bytes(&recipient_fa_store_object)?.into())
                },
            ),
            // (
            //     TOTAL_SUPPLY_STATE_KEY.clone(),
            //     WriteOp::legacy_modification(bcs::to_bytes(&total_supply)?),
            // ),
        ];

        let event = DepositFAEvent {
            store: recipient_store_address,
            amount: transfer_amount,
        };

        let events = vec![
            event.create_event_v2()
        ];
        Ok(Ok(IncrementalOutput { write_set, events }))
    }

    fn withdraw_from_signer(
        sender_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
        gas: u64,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let sender_account_key = DbAccessUtil::new_state_key_account(sender_address);
        let mut sender_account = {
            let _timer = TIMER
                .with_label_values(&["read_sender_account"])
                .start_timer();
            DbAccessUtil::get_account(&sender_account_key, state_view)?.unwrap()
        };
        let sender_coin_store_key = DbAccessUtil::new_state_key_aptos_coin(&sender_address);
        let mut sender_coin_store = {
            let _timer = TIMER
                .with_label_values(&["read_sender_coin_store"])
                .start_timer();
            DbAccessUtil::get_coin_store(&sender_coin_store_key, state_view)?.unwrap()
        };

        // Note: numbers below may not be real. When runninng in parallel there might be conflicts.
        sender_coin_store.coin -= transfer_amount;

        sender_coin_store.coin -= gas;

        sender_account.sequence_number += 1;

        // add total supply via aggregators?
        // let mut total_supply: u128 =
        //     DbAccessUtil::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
        // total_supply -= gas as u128;

        // TODO(grao): Add other reads to match the read set of the real transaction.
        let write_set = vec![
            (
                sender_account_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_account)?.into()),
            ),
            (
                sender_coin_store_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_coin_store)?.into()),
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
        fail_on_existing: bool,
        fail_on_missing: bool,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let recipient_account_key = DbAccessUtil::new_state_key_account(recipient_address);
        let recipient_coin_store_key = DbAccessUtil::new_state_key_aptos_coin(&recipient_address);

        let recipient_account = {
            let _timer = TIMER.with_label_values(&["read_new_account"]).start_timer();
            DbAccessUtil::get_account(&recipient_account_key, state_view)?
        };

        let mut write_set = Vec::new();
        if recipient_account.is_some() {
            if fail_on_existing {
                return Ok(Err(TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                    location: AbortLocation::Module(ModuleId::new(
                        AccountAddress::ONE,
                        ident_str!("account").into(),
                    )),
                    code: 7,
                    info: None,
                })));
            }

            let mut recipient_coin_store = {
                let _timer = TIMER
                    .with_label_values(&["read_new_coin_store"])
                    .start_timer();
                DbAccessUtil::get_coin_store(&recipient_coin_store_key, state_view)?.unwrap()
            };

            if transfer_amount != 0 {
                recipient_coin_store.coin += transfer_amount;

                write_set.push((
                    recipient_coin_store_key,
                    WriteOp::legacy_modification(bcs::to_bytes(&recipient_coin_store)?.into()),
                ));
            }
        } else {
            if fail_on_missing {
                return Ok(Err(TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                    location: AbortLocation::Module(ModuleId::new(
                        AccountAddress::ONE,
                        ident_str!("account").into(),
                    )),
                    code: 8,
                    info: None,
                })));
            }

            {
                let _timer = TIMER
                    .with_label_values(&["read_new_coin_store"])
                    .start_timer();
                assert!(
                    DbAccessUtil::get_coin_store(&recipient_coin_store_key, state_view)?.is_none()
                );
            }

            let recipient_account = Account {
                authentication_key: recipient_address.to_vec(),
                ..Default::default()
            };

            let recipient_coin_store = CoinStore {
                coin: transfer_amount,
                ..Default::default()
            };

            write_set.push((
                recipient_account_key,
                WriteOp::legacy_creation(bcs::to_bytes(&recipient_account)?.into()),
            ));
            write_set.push((
                recipient_coin_store_key,
                WriteOp::legacy_creation(bcs::to_bytes(&recipient_coin_store)?.into()),
            ));
        }

        let events = vec![
            ContractEvent::new_v1(
                EventKey::new(0, recipient_address),
                0,
                TypeTag::Struct(Box::new(DepositEvent::struct_tag())),
                recipient_address.to_vec(),
            ), // TODO(grao): CoinRegisterEvent
        ];
        Ok(Ok(IncrementalOutput { write_set, events }))
    }

    fn handle_fa_transfer(
        sender_address: AccountAddress,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["fa_transfer"]).start_timer();

        let gas = 500; // hardcode gas consumed.

        let mut output = {
            let output = Self::withdraw_fa_apt_from_signer(sender_address, transfer_amount, state_view, gas)?;
            match output {
                Ok(output) => output,
                Err(status) => return Ok(IncrementalOutput::to_abort(status)),
            }
        };

        let deposit_output = Self::deposit_fa_apt(
            recipient_address,
            transfer_amount,
            state_view,
        )?;

        match deposit_output {
            Ok(deposit_output) => {
                output.append(deposit_output);
                output.into_success_output()
            },
            Err(status) => Ok(IncrementalOutput::to_abort(status)),
        }
    }

    fn handle_account_creation_and_transfer(
        sender_address: AccountAddress,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
        fail_on_existing: bool,
        fail_on_missing: bool,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["account_creation"]).start_timer();

        let gas = 500; // hardcode gas consumed.

        let mut output = {
            let output = Self::withdraw_from_signer(sender_address, transfer_amount, state_view, gas)?;
            match output {
                Ok(output) => output,
                Err(status) => return Ok(IncrementalOutput::to_abort(status)),
            }
        };

        let deposit_output = Self::deposit(
            recipient_address,
            transfer_amount,
            state_view,
            fail_on_existing,
            fail_on_missing,
        )?;

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
        fail_on_existing: bool,
        fail_on_missing: bool,
    ) -> Result<TransactionOutput> {
        let mut deltas = HashMap::new();

        let gas = 5000; // hardcode gas consumed.

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
                Self::withdraw_from_signer(sender_address, amount_to_sender as u64, state_view, gas)?;
            match output {
                Ok(output) => output,
                Err(status) => return Ok(IncrementalOutput::to_abort(status)),
            }
        };

        for (recipient_address, transfer_amount) in deltas.into_iter() {
            output.append({
                let deposit_output = Self::deposit(
                    recipient_address,
                    transfer_amount as u64,
                    state_view,
                    fail_on_existing,
                    fail_on_missing,
                )?;

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

impl TransactionBlockExecutor for NativeLooseBlockExecutor {
    fn execute_transaction_block(
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ExecutionOutput> {
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
                                (AccountAddress::ONE, "aptos_account", "fungible_transfer_only") => {
                                    Self::handle_fa_transfer(
                                        user_txn.sender(),
                                        bcs::from_bytes(&f.args()[0]).unwrap(),
                                        bcs::from_bytes(&f.args()[1]).unwrap(),
                                        &state_view,
                                    )
                                },
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
                                (AccountAddress::ONE, "aptos_account", "batch_transfer") => {
                                    Self::handle_batch_account_creation_and_transfer(
                                        user_txn.sender(),
                                        bcs::from_bytes(&f.args()[0]).unwrap(),
                                        bcs::from_bytes(&f.args()[1]).unwrap(),
                                        &state_view,
                                        false,
                                        true,
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
        Ok(ExecutionOutput {
            transactions: transactions.into_iter().map(|t| t.into_inner()).collect(),
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
            block_end_info: None,
        })
    }
}
