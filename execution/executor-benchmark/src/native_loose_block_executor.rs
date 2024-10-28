// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_access::{Account, CoinStore, DbAccessUtil},
    metrics::TIMER, native_transaction::{NativeTransaction, NATIVE_EXECUTOR_POOL},
};
use anyhow::Result;
use aptos_executor::{block_executor::TransactionBlockExecutor, metrics::BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK};
use aptos_executor_types::execution_output::ExecutionOutput;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    account_address::AccountAddress, account_config::{deposit::DepositEvent, primary_apt_store, withdraw::WithdrawEvent, DepositFAEvent, FungibleStoreResource, WithdrawFAEvent}, block_executor::{config::BlockExecutorConfigFromOnchain, partitioner::ExecutableTransactions}, contract_event::ContractEvent, event::EventKey, fee_statement::FeeStatement, move_event_v2::MoveEventV2, on_chain_config::{FeatureFlag, Features}, state_store::state_key::StateKey, transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, ExecutionStatus, Transaction, TransactionAuxiliaryData, TransactionOutput, TransactionStatus
    }, vm_status::AbortLocation, write_set::{WriteOp, WriteSet, WriteSetMut}
};
use dashmap::DashMap;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::{Lazy, OnceCell};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use std::collections::{BTreeMap, HashMap};
use aptos_types::on_chain_config::OnChainConfig;

struct IncrementalOutput {
    write_set: Vec<(StateKey, WriteOp)>,
    events: Vec<ContractEvent>,
}

impl IncrementalOutput {
    fn new() -> Self {
        IncrementalOutput { write_set: vec![], events: vec![] }
    }

    fn into_success_output(mut self, gas: u64) -> Result<TransactionOutput> {
        self.events.push(FeeStatement::new(gas, gas, 0, 0, 0).create_event_v2());

        Ok(TransactionOutput::new(
            WriteSetMut::new(self.write_set).freeze()?,
            self.events,
            /*gas_used=*/ gas,
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

#[macro_export]
macro_rules! merge_output {
    ($output:ident, $new_output:expr) => {
        match $new_output {
            Ok(new_output) => {
                $output.append(new_output);
            },
            Err(status) => return Ok(IncrementalOutput::to_abort(status)),
        }
    }
}

#[macro_export]
macro_rules! merge_output_in_partial {
    ($output:ident, $new_output:expr) => {
        match $new_output {
            Ok(new_output) => {
                $output.append(new_output);
            },
            Err(status) => return Ok(Err(status)),
        }
    }
}

/// Native block executor (replacing both BlockSTM and AptosVM), that is
/// "loose", i.e. doesn't compute outputs correctly.
/// It's loose in multiple ways:
/// - it ignores conflicts. All transactions see the state at the start of the block!
/// - it doesn't put everything in the writeset that should be there
/// - it doesn't compute gas
pub struct NativeLooseSpeculativeBlockExecutor;

impl NativeLooseSpeculativeBlockExecutor {
    fn increment_sequence_number(
        sender_address: AccountAddress,
        state_view: &CachedStateView
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let sender_account_key = DbAccessUtil::new_state_key_account(sender_address);
        let mut sender_account = {
            let _timer = TIMER
                .with_label_values(&["read_sender_account"])
                .start_timer();
            DbAccessUtil::get_account(&sender_account_key, state_view)?.unwrap()
        };

        sender_account.sequence_number += 1;

        let write_set = vec![
            (
                sender_account_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_account)?.into()),
            ),
        ];
        let events = vec![];
        Ok(Ok(IncrementalOutput { write_set, events }))
    }

    // add total supply via aggregators?
    // let mut total_supply: u128 =
    //     DbAccessUtil::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
    // total_supply -= gas as u128;
    // (
    //     TOTAL_SUPPLY_STATE_KEY.clone(),
    //     WriteOp::legacy_modification(bcs::to_bytes(&total_supply)?),
    // ),

    fn withdraw_fa_apt_from_signer(
        sender_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
        gas: u64,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let sender_store_address = primary_apt_store(sender_address);

        let sender_fa_store_object_key = DbAccessUtil::new_state_key_object_resource_group(&sender_store_address);
        let mut sender_fa_store_object = {
            let _timer = TIMER
                .with_label_values(&["read_sender_fa_store"])
                .start_timer();
            match DbAccessUtil::get_resource_group( &sender_fa_store_object_key, state_view)? {
                Some(sender_fa_store_object) => sender_fa_store_object,
                None => return Ok(Err(TransactionStatus::Keep(ExecutionStatus::MoveAbort {
                    location: AbortLocation::Module(ModuleId::new(
                        AccountAddress::ONE,
                        ident_str!("fungible_asset").into(),
                    )),
                    code: 7,
                    info: None,
                })))
            }
        };

        let fungible_store_rg_tag = FungibleStoreResource::struct_tag();
        let mut sender_fa_store = bcs::from_bytes::<FungibleStoreResource>(&sender_fa_store_object.remove(&fungible_store_rg_tag).unwrap())?;

        sender_fa_store.balance -= transfer_amount + gas;

        sender_fa_store_object.insert(fungible_store_rg_tag, bcs::to_bytes(&sender_fa_store)?);

        let write_set = vec![
            (
                sender_fa_store_object_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_fa_store_object)?.into()),
            ),
        ];

        let mut events = Vec::new();
        if transfer_amount > 0 {
            events.push(WithdrawFAEvent {
                store: sender_store_address,
                amount: transfer_amount,
            }.create_event_v2());
        }

        events.push(WithdrawFAEvent {
            store: sender_store_address,
            amount: gas,
        }.create_event_v2());
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

        let write_set = vec![
            (
                recipient_fa_store_object_key,
                if recipient_fa_store_existed {
                    WriteOp::legacy_modification(bcs::to_bytes(&recipient_fa_store_object)?.into())
                } else {
                    WriteOp::legacy_creation(bcs::to_bytes(&recipient_fa_store_object)?.into())
                },
            ),
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

    fn withdraw_coin_apt_from_signer(
        sender_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
        gas: u64,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let sender_coin_store_key = DbAccessUtil::new_state_key_aptos_coin(&sender_address);
        let sender_coin_store_opt = {
            let _timer = TIMER
                .with_label_values(&["read_sender_coin_store"])
                .start_timer();
            DbAccessUtil::get_coin_store(&sender_coin_store_key, state_view)?
        };
        let mut sender_coin_store = match sender_coin_store_opt {
            None => return Self::withdraw_fa_apt_from_signer(sender_address, transfer_amount, state_view, gas),
            Some(sender_coin_store) => sender_coin_store,
        };

        sender_coin_store.coin -= transfer_amount;
        sender_coin_store.coin -= gas;

        let write_set = vec![
            (
                sender_coin_store_key,
                WriteOp::legacy_modification(bcs::to_bytes(&sender_coin_store)?.into()),
            ),
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

    fn create_non_existing_account(
        recipient_address: AccountAddress,
        recipient_account_key: StateKey,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let mut output = IncrementalOutput::new();

        let recipient_account = Account {
            authentication_key: recipient_address.to_vec(),
            ..Default::default()
        };

        output.write_set.push((
            recipient_account_key,
            WriteOp::legacy_creation(bcs::to_bytes(&recipient_account)?.into()),
        ));

        Ok(Ok(output))
    }

    fn deposit_coin_apt(
        recipient_address: AccountAddress,
        transfer_amount: u64,
        fail_on_existing: bool,
        fail_on_missing: bool,
        state_view: &CachedStateView,
        new_accounts_default_to_fa: bool,
    ) -> Result<Result<IncrementalOutput, TransactionStatus>> {
        let recipient_account_key = DbAccessUtil::new_state_key_account(recipient_address);
        let recipient_coin_store_key = DbAccessUtil::new_state_key_aptos_coin(&recipient_address);

        let recipient_account = {
            let _timer = TIMER.with_label_values(&["read_new_account"]).start_timer();
            DbAccessUtil::get_account(&recipient_account_key, state_view)?
        };

        let mut output = IncrementalOutput::new();
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

                output.write_set.push((
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

            merge_output_in_partial!(output, Self::create_non_existing_account(recipient_address, recipient_account_key)?);

            if new_accounts_default_to_fa {
                merge_output_in_partial!(output, Self::deposit_fa_apt(recipient_address, transfer_amount, state_view)?);
                return Ok(Ok(output));
            }

            {
                let _timer = TIMER
                    .with_label_values(&["read_new_coin_store"])
                    .start_timer();
                assert!(
                    DbAccessUtil::get_coin_store(&recipient_coin_store_key, state_view)?.is_none()
                );
            }

            let recipient_coin_store = CoinStore {
                coin: transfer_amount,
                ..Default::default()
            };

            output.write_set.push((
                recipient_coin_store_key,
                WriteOp::legacy_creation(bcs::to_bytes(&recipient_coin_store)?.into()),
            ));
        }

        output.events.push(
            ContractEvent::new_v1(
                EventKey::new(0, recipient_address),
                0,
                TypeTag::Struct(Box::new(DepositEvent::struct_tag())),
                recipient_address.to_vec(),
            ), // TODO(grao): CoinRegisterEvent
        );
        Ok(Ok(output))
    }

    fn handle_fa_transfer(
        sender_address: AccountAddress,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["fa_transfer"]).start_timer();

        let gas = 500; // hardcode gas consumed.

        let mut output = IncrementalOutput::new();

        merge_output!(output, Self::increment_sequence_number(sender_address, state_view)?);
        merge_output!(output, Self::withdraw_fa_apt_from_signer(sender_address, transfer_amount, state_view, gas)?);

        merge_output!(output, Self::deposit_fa_apt(
            recipient_address,
            transfer_amount,
            state_view,
        )?);

        output.into_success_output(gas)
    }

    fn handle_account_creation_and_transfer(
        sender_address: AccountAddress,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        fail_on_existing: bool,
        fail_on_missing: bool,
        state_view: &CachedStateView,
        new_accounts_default_to_fa: bool,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["account_creation"]).start_timer();

        let gas = 500; // hardcode gas consumed.

        let mut output = IncrementalOutput::new();
        merge_output!(output, Self::increment_sequence_number(sender_address, state_view)?);
        merge_output!(output, Self::withdraw_coin_apt_from_signer(sender_address, transfer_amount, state_view, gas)?);
        merge_output!(output, Self::deposit_coin_apt(
            recipient_address,
            transfer_amount,
            fail_on_existing,
            fail_on_missing,
            state_view,
            new_accounts_default_to_fa,
        )?);

        output.into_success_output(gas)
    }

    fn handle_batch_account_creation_and_transfer(
        sender_address: AccountAddress,
        recipient_addresses: Vec<AccountAddress>,
        transfer_amounts: Vec<u64>,
        fail_on_existing: bool,
        fail_on_missing: bool,
        state_view: &CachedStateView,
        new_accounts_default_to_fa: bool,
    ) -> Result<TransactionOutput> {
        let gas = 5000; // hardcode gas consumed.

        let mut deltas = compute_deltas_for_batch(recipient_addresses, transfer_amounts, sender_address);

        let amount_to_sender = -deltas.remove(&sender_address).unwrap_or(0);
        assert!(amount_to_sender >= 0);


        let mut output = IncrementalOutput::new();
        merge_output!(output, Self::increment_sequence_number(sender_address, state_view)?);
        merge_output!(output, Self::withdraw_coin_apt_from_signer(sender_address, amount_to_sender as u64, state_view, gas)?);

        for (recipient_address, transfer_amount) in deltas.into_iter() {
            merge_output!(output, Self::deposit_coin_apt(
                recipient_address,
                transfer_amount as u64,
                fail_on_existing,
                fail_on_missing,
                state_view,
                new_accounts_default_to_fa,
            )?);
        }

        output.into_success_output(gas)
    }

    fn handle_nop(
        sender_address: AccountAddress,
        fa_migration_complete: bool,
        state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["nop"]).start_timer();

        let gas = 4; // hardcode gas consumed.

        let mut output = IncrementalOutput::new();

        merge_output!(output, Self::increment_sequence_number(sender_address, state_view)?);
        if fa_migration_complete {
            merge_output!(output, Self::withdraw_fa_apt_from_signer(sender_address, 0, state_view, gas)?);
        } else {
            merge_output!(output, Self::withdraw_coin_apt_from_signer(sender_address, 0, state_view, gas)?);
        }
        output.into_success_output(gas)
    }
}

fn compute_deltas_for_batch(recipient_addresses: Vec<AccountAddress>, transfer_amounts: Vec<u64>, sender_address: AccountAddress) -> HashMap<AccountAddress, i64> {
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
    deltas
}

impl TransactionBlockExecutor for NativeLooseSpeculativeBlockExecutor {
    fn new() -> Self {
        Self
    }

    fn execute_transaction_block(
        &self,
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ExecutionOutput> {
        let _timer = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.start_timer();

        let features = Features::fetch_config(&state_view).unwrap_or_default();
        let fa_migration_complete = features.is_enabled(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);
        let new_accounts_default_to_fa = features.is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);

        let transactions = match transactions {
            ExecutableTransactions::Unsharded(txns) => txns,
            _ => todo!("sharded execution not yet supported"),
        };
        let transaction_outputs = NATIVE_EXECUTOR_POOL.install(|| {
            transactions
                .par_iter()
                .map(|txn| {
                    // since we don't handle conflicts, we ignore current sequence number
                    match NativeTransaction::parse(txn) {
                        NativeTransaction::Nop { sender, sequence_number: _ } => {
                            Self::handle_nop(sender, fa_migration_complete, &state_view)
                        },
                        NativeTransaction::FaTransfer { sender, sequence_number: _, recipient, amount } => {
                            Self::handle_fa_transfer(sender, recipient, amount, &state_view)
                        },
                        NativeTransaction::Transfer { sender, sequence_number: _, recipient, amount, fail_on_account_existing, fail_on_account_missing } => {
                            Self::handle_account_creation_and_transfer(sender, recipient, amount, fail_on_account_existing, fail_on_account_missing, &state_view, new_accounts_default_to_fa)
                        },
                        NativeTransaction::BatchTransfer { sender, sequence_number: _, recipients, amounts, fail_on_account_existing, fail_on_account_missing } => {
                            Self::handle_batch_account_creation_and_transfer(sender, recipients, amounts, fail_on_account_existing, fail_on_account_missing, &state_view, new_accounts_default_to_fa)
                        },
                    }
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


pub struct NativeNoStorageLooseSpeculativeBlockExecutor {
    seq_nums: DashMap<AccountAddress, u64>,
    balances: DashMap<AccountAddress, u64>,
}

impl TransactionBlockExecutor  for NativeNoStorageLooseSpeculativeBlockExecutor {
    fn new() -> Self {
        Self {
            seq_nums: DashMap::new(),
            balances: DashMap::new(),
        }
    }

    fn execute_transaction_block(
        &self,
        transactions: ExecutableTransactions,
        state_view: CachedStateView,
        _onchain_config: BlockExecutorConfigFromOnchain,
    ) -> Result<ExecutionOutput> {
        let transactions = match transactions {
            ExecutableTransactions::Unsharded(txns) => txns,
            _ => todo!("sharded execution not yet supported"),
        };
        let native_transactions = NATIVE_EXECUTOR_POOL.install(|| {
            transactions
                .par_iter()
                .map(NativeTransaction::parse)
                .collect::<Vec<_>>()
        });

        let timer = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.start_timer();

        let transaction_outputs = NATIVE_EXECUTOR_POOL.install(|| {
            native_transactions
                .into_par_iter()
                .map(|txn| {
                    match txn {
                        NativeTransaction::Nop { sender, sequence_number } => {
                            self.seq_nums.insert(sender, sequence_number);
                        },
                        NativeTransaction::FaTransfer { sender, sequence_number, recipient, amount }
                        | NativeTransaction::Transfer { sender, sequence_number, recipient, amount, .. } => {
                            self.seq_nums.insert(sender, sequence_number);
                            *self.balances.entry(sender).or_insert(100_000_000_000_000_000) -= amount;
                            *self.balances.entry(recipient).or_insert(100_000_000_000_000_000) += amount;
                        },
                        NativeTransaction::BatchTransfer { sender, sequence_number, recipients, amounts, .. } => {
                            self.seq_nums.insert(sender, sequence_number);

                            let mut deltas = compute_deltas_for_batch(recipients, amounts, sender);

                            let amount_from_sender = -deltas.remove(&sender).unwrap_or(0);
                            assert!(amount_from_sender >= 0);

                            *self.balances.entry(sender).or_insert(100_000_000_000_000_000) -= amount_from_sender as u64;

                            for (recipient, amount) in deltas.into_iter() {
                                *self.balances.entry(recipient).or_insert(100_000_000_000_000_000) += amount as u64;
                            }
                        },
                    }
                    TransactionOutput::new(
                        Default::default(),
                        vec![],
                        0,
                        TransactionStatus::Keep(ExecutionStatus::Success),
                        TransactionAuxiliaryData::default(),
                    )
                })
                .collect::<Vec<_>>()
        });

        drop(timer);

        Ok(ExecutionOutput {
            transactions: transactions.into_iter().map(|t| t.into_inner()).collect(),
            transaction_outputs,
            state_cache: state_view.into_state_cache(),
            block_end_info: None,
        })
    }
}
