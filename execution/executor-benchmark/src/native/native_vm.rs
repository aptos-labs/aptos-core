// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_access::DbAccessUtil,
    native::{
        native_config::{NativeConfig, NATIVE_EXECUTOR_POOL},
        native_transaction::{compute_deltas_for_batch, NativeTransaction},
    },
};
use aptos_aggregator::{
    bounded_math::SignedU128,
    delayed_change::{DelayedApplyChange, DelayedChange},
    delta_change_set::{DeltaOp, DeltaWithMax},
    delta_math::DeltaHistory,
};
use aptos_block_executor::{
    code_cache_global_manager::AptosModuleCacheManager,
    task::{ExecutionStatus, ExecutorTask},
    txn_commit_hook::NoOpTransactionCommitHook,
    txn_provider::default::DefaultTxnProvider,
};
use aptos_logger::error;
use aptos_mvhashmap::types::TxnIndex;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{
        primary_apt_store, AccountResource, CoinInfoResource, CoinRegister, CoinStoreResource,
        ConcurrentSupplyResource, DepositEvent, DepositFAEvent, FungibleStoreResource,
        WithdrawEvent, WithdrawFAEvent,
    },
    block_executor::{
        config::{BlockExecutorConfig, BlockExecutorConfigFromOnchain, BlockExecutorLocalConfig},
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    move_utils::{move_event_v1::MoveEventV1Type, move_event_v2::MoveEventV2Type},
    on_chain_config::FeatureFlag,
    state_store::{state_key::StateKey, state_value::StateValueMetadata, StateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput, Transaction,
        TransactionOutput, TransactionStatus, WriteSetPayload,
    },
    write_set::WriteOp,
    AptosCoinType,
};
use aptos_vm::{
    block_executor::{AptosBlockExecutorWrapper, AptosTransactionOutput},
    VMBlockExecutor,
};
use aptos_vm_environment::environment::AptosEnvironment;
use aptos_vm_types::{
    abstract_write_op::{
        AbstractResourceWriteOp, GroupWrite, ResourceGroupInPlaceDelayedFieldChangeOp,
    },
    change_set::VMChangeSet,
    module_write_set::ModuleWriteSet,
    output::VMOutput,
    resolver::{ExecutorView, ResourceGroupView},
    resource_group_adapter::group_size_as_sum,
};
use bytes::Bytes;
use move_core_types::{
    language_storage::StructTag,
    value::{IdentifierMappingKind, MoveStructLayout, MoveTypeLayout},
    vm_status::VMStatus,
};
use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;
use serde::{de::DeserializeOwned, Serialize};
use std::{collections::BTreeMap, fmt::Debug, sync::Arc};

pub struct NativeVMBlockExecutor;

// Executor external API
impl VMBlockExecutor for NativeVMBlockExecutor {
    fn new() -> Self {
        Self
    }

    /// Execute a block of `transactions`. The output vector will have the exact same length as the
    /// input vector. The discarded transactions will be marked as `TransactionStatus::Discard` and
    /// have an empty `WriteSet`. Also `state_view` is immutable, and does not have interior
    /// mutability. Writes to be applied to the data view are encoded in the write set part of a
    /// transaction output.
    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &(impl StateView + Sync),
        onchain_config: BlockExecutorConfigFromOnchain,
        transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<StateKey, TransactionOutput>, VMStatus> {
        AptosBlockExecutorWrapper::<NativeVMExecutorTask>::execute_block_on_thread_pool::<
            _,
            NoOpTransactionCommitHook<AptosTransactionOutput, VMStatus>,
            _,
        >(
            Arc::clone(&NATIVE_EXECUTOR_POOL),
            txn_provider,
            state_view,
            &AptosModuleCacheManager::new(),
            BlockExecutorConfig {
                local: BlockExecutorLocalConfig::default_with_concurrency_level(
                    NativeConfig::get_concurrency_level(),
                ),
                onchain: onchain_config,
            },
            transaction_slice_metadata,
            None,
        )
    }
}

pub(crate) struct NativeVMExecutorTask {
    fa_migration_complete: bool,
    db_util: DbAccessUtil,
}

impl ExecutorTask for NativeVMExecutorTask {
    type Error = VMStatus;
    type Output = AptosTransactionOutput;
    type Txn = SignatureVerifiedTransaction;

    fn init(env: &AptosEnvironment, _state_view: &impl StateView) -> Self {
        let fa_migration_complete = env
            .features()
            .is_enabled(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);
        let new_accounts_default_to_fa = env
            .features()
            .is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        assert_eq!(
            fa_migration_complete, new_accounts_default_to_fa,
            "native code only works with both flags either enabled or disabled"
        );

        Self {
            fa_migration_complete,
            db_util: DbAccessUtil::new(),
        }
    }

    // This function is called by the BlockExecutor for each transaction it intends
    // to execute (via the ExecutorTask trait). It can be as a part of sequential
    // execution, or speculatively as a part of a parallel execution.
    fn execute_transaction(
        &self,
        executor_with_group_view: &(impl ExecutorView + ResourceGroupView),
        txn: &SignatureVerifiedTransaction,
        _txn_idx: TxnIndex,
    ) -> ExecutionStatus<AptosTransactionOutput, VMStatus> {
        match self.execute_transaction_impl(
            executor_with_group_view,
            txn,
            self.fa_migration_complete,
        ) {
            Ok((change_set, gas_units)) => {
                ExecutionStatus::Success(AptosTransactionOutput::new(VMOutput::new(
                    change_set,
                    ModuleWriteSet::empty(),
                    FeeStatement::new(gas_units, gas_units, 0, 0, 0),
                    TransactionStatus::Keep(aptos_types::transaction::ExecutionStatus::Success),
                )))
            },
            Err(_) => ExecutionStatus::SpeculativeExecutionAbortError("something".to_string()),
        }
    }

    fn is_transaction_dynamic_change_set_capable(txn: &Self::Txn) -> bool {
        if txn.is_valid() {
            if let Transaction::GenesisTransaction(WriteSetPayload::Direct(_)) = txn.expect_valid()
            {
                // WriteSetPayload::Direct cannot be handled in mode where delayed_field_optimization or
                // resource_groups_split_in_change_set is enabled.
                return false;
            }
        }
        true
    }
}

impl NativeVMExecutorTask {
    fn execute_transaction_impl(
        &self,
        view: &(impl ExecutorView + ResourceGroupView),
        txn: &SignatureVerifiedTransaction,
        fa_migration_complete: bool,
    ) -> Result<(VMChangeSet, u64), ()> {
        let gas_units = 4;
        let gas = gas_units * 100;

        let mut resource_write_set = BTreeMap::new();
        let mut events = Vec::new();
        let mut delayed_field_change_set = BTreeMap::new();
        let aggregator_v1_write_set = BTreeMap::new();
        let mut aggregator_v1_delta_set = BTreeMap::new();

        self.reduce_apt_supply(
            fa_migration_complete,
            gas,
            view,
            &mut resource_write_set,
            &mut delayed_field_change_set,
            &mut aggregator_v1_delta_set,
        )
        .unwrap();

        match NativeTransaction::parse(txn) {
            NativeTransaction::Nop {
                sender,
                sequence_number,
            } => {
                self.check_and_set_sequence_number(
                    sender,
                    sequence_number,
                    view,
                    &mut resource_write_set,
                )?;
                self.withdraw_apt(
                    fa_migration_complete,
                    sender,
                    0,
                    view,
                    gas,
                    &mut resource_write_set,
                    &mut events,
                )?;
            },
            NativeTransaction::FaTransfer {
                sender,
                sequence_number,
                recipient,
                amount,
            } => {
                self.check_and_set_sequence_number(
                    sender,
                    sequence_number,
                    view,
                    &mut resource_write_set,
                )?;
                self.withdraw_fa_apt_from_signer(
                    sender,
                    amount,
                    view,
                    gas,
                    &mut resource_write_set,
                    &mut events,
                )?;
                if amount > 0 {
                    self.deposit_fa_apt(
                        recipient,
                        amount,
                        view,
                        &mut resource_write_set,
                        &mut events,
                    )?;
                }
            },
            NativeTransaction::Transfer {
                sender,
                sequence_number,
                recipient,
                amount,
                fail_on_recipient_account_existing: fail_on_account_existing,
                fail_on_recipient_account_missing: fail_on_account_missing,
            } => {
                self.check_and_set_sequence_number(
                    sender,
                    sequence_number,
                    view,
                    &mut resource_write_set,
                )?;

                self.withdraw_apt(
                    fa_migration_complete,
                    sender,
                    amount,
                    view,
                    gas,
                    &mut resource_write_set,
                    &mut events,
                )?;

                let exists = self.deposit_apt(
                    fa_migration_complete,
                    recipient,
                    amount,
                    view,
                    &mut resource_write_set,
                    &mut events,
                )?;

                if !exists || fail_on_account_existing {
                    self.check_or_create_account(
                        recipient,
                        fail_on_account_existing,
                        fail_on_account_missing,
                        !fa_migration_complete,
                        view,
                        &mut resource_write_set,
                    )?;
                }
            },
            NativeTransaction::BatchTransfer {
                sender,
                sequence_number,
                recipients,
                amounts,
                fail_on_recipient_account_existing,
                fail_on_recipient_account_missing,
            } => {
                self.check_and_set_sequence_number(
                    sender,
                    sequence_number,
                    view,
                    &mut resource_write_set,
                )?;

                let (deltas, amount_to_sender) =
                    compute_deltas_for_batch(recipients, amounts, sender);

                self.withdraw_apt(
                    fa_migration_complete,
                    sender,
                    amount_to_sender,
                    view,
                    gas,
                    &mut resource_write_set,
                    &mut events,
                )?;

                for (recipient_address, transfer_amount) in deltas.into_iter() {
                    let existed = self.deposit_apt(
                        fa_migration_complete,
                        recipient_address,
                        transfer_amount as u64,
                        view,
                        &mut resource_write_set,
                        &mut events,
                    )?;

                    if !existed || fail_on_recipient_account_existing {
                        self.check_or_create_account(
                            recipient_address,
                            fail_on_recipient_account_existing,
                            fail_on_recipient_account_missing,
                            !fa_migration_complete,
                            view,
                            &mut resource_write_set,
                        )?;
                    }
                }
            },
            NativeTransaction::BlockEpilogue => return Ok((VMChangeSet::empty(), 0)),
        };

        events.push((
            FeeStatement::new(gas_units, gas_units, 0, 0, 0)
                .create_event_v2()
                .expect("Creating FeeStatement should always succeed"),
            None,
        ));

        Ok((
            VMChangeSet::new(
                resource_write_set,
                events,
                delayed_field_change_set,
                aggregator_v1_write_set,
                aggregator_v1_delta_set,
            ),
            gas_units,
        ))
    }

    pub fn get_value<T: DeserializeOwned>(
        state_key: &StateKey,
        view: &(impl ExecutorView + ResourceGroupView),
    ) -> Result<Option<(T, StateValueMetadata)>, ()> {
        view.get_resource_state_value(state_key, None)
            .map_err(hide_error)?
            .map(|value| {
                bcs::from_bytes::<T>(value.bytes()).map(|bytes| (bytes, value.into_metadata()))
            })
            .transpose()
            .map_err(hide_error)
    }

    pub fn get_value_from_group<T: DeserializeOwned>(
        group_key: &StateKey,
        resource_tag: &StructTag,
        view: &(impl ExecutorView + ResourceGroupView),
    ) -> Result<Option<T>, ()> {
        Self::get_value_from_group_with_layout(group_key, resource_tag, view, None)
    }

    pub fn get_value_from_group_with_layout<T: DeserializeOwned>(
        group_key: &StateKey,
        resource_tag: &StructTag,
        view: &(impl ExecutorView + ResourceGroupView),
        maybe_layout: Option<&MoveTypeLayout>,
    ) -> Result<Option<T>, ()> {
        view.get_resource_from_group(group_key, resource_tag, maybe_layout)
            .map_err(hide_error)?
            .map(|value| bcs::from_bytes::<T>(&value))
            .transpose()
            .map_err(hide_error)
    }

    fn check_and_set_sequence_number(
        &self,
        sender_address: AccountAddress,
        sequence_number: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
    ) -> Result<(), ()> {
        let sender_account_key = self.db_util.new_state_key_account(&sender_address);

        let value = Self::get_value::<AccountResource>(&sender_account_key, view)?;

        match value {
            Some((mut account, metadata)) => {
                if sequence_number == account.sequence_number {
                    account.sequence_number += 1;
                    resource_write_set.insert(
                        sender_account_key,
                        AbstractResourceWriteOp::Write(WriteOp::modification(
                            Bytes::from(bcs::to_bytes(&account).map_err(hide_error)?),
                            metadata,
                        )),
                    );
                    Ok(())
                } else {
                    error!(
                        "Invalid sequence number: txn: {} vs account: {}",
                        sequence_number, account.sequence_number
                    );
                    Err(())
                }
            },
            None => {
                let mut account = DbAccessUtil::new_account_resource(sender_address);
                if sequence_number == 0 {
                    account.sequence_number = 1;
                    resource_write_set.insert(
                        sender_account_key,
                        AbstractResourceWriteOp::Write(WriteOp::legacy_creation(Bytes::from(
                            bcs::to_bytes(&account).map_err(hide_error)?,
                        ))),
                    );
                    Ok(())
                } else {
                    error!(
                        "Invalid sequence number: txn: {} vs account: {}",
                        sequence_number, account.sequence_number
                    );
                    Err(())
                }
            },
        }
    }

    fn check_or_create_account(
        &self,
        address: AccountAddress,
        fail_on_account_existing: bool,
        fail_on_account_missing: bool,
        create_account_resource: bool,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
    ) -> Result<(), ()> {
        let account_key = self.db_util.new_state_key_account(&address);

        let value = Self::get_value::<AccountResource>(&account_key, view)?;
        match value {
            Some(_) => {
                if fail_on_account_existing {
                    return Err(());
                }
            },
            None => {
                if fail_on_account_missing {
                    return Err(());
                } else if create_account_resource {
                    let account = DbAccessUtil::new_account_resource(address);

                    resource_write_set.insert(
                        account_key,
                        AbstractResourceWriteOp::Write(WriteOp::legacy_creation(Bytes::from(
                            bcs::to_bytes(&account).map_err(hide_error)?,
                        ))),
                    );
                }
            },
        }

        Ok(())
    }

    fn reduce_apt_supply(
        &self,
        fa_migration_complete: bool,
        gas: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        delayed_field_change_set: &mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
        aggregator_v1_delta_set: &mut BTreeMap<StateKey, DeltaOp>,
    ) -> Result<(), ()> {
        if fa_migration_complete {
            self.reduce_fa_apt_supply(gas, view, resource_write_set, delayed_field_change_set)
        } else {
            self.reduce_coin_apt_supply(gas, view, aggregator_v1_delta_set)
        }
    }

    fn reduce_fa_apt_supply(
        &self,
        gas: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        delayed_field_change_set: &mut BTreeMap<DelayedFieldID, DelayedChange<DelayedFieldID>>,
    ) -> Result<(), ()> {
        let apt_metadata_object_state_key = self
            .db_util
            .new_state_key_object_resource_group(&AccountAddress::TEN);

        let concurrent_supply_rg_tag = &self.db_util.common.concurrent_supply;

        let concurrent_supply_layout = MoveTypeLayout::Struct(MoveStructLayout::new(vec![
            MoveTypeLayout::Native(
                IdentifierMappingKind::Aggregator,
                Box::new(MoveTypeLayout::U128),
            ),
            MoveTypeLayout::U128,
        ]));

        let supply = Self::get_value_from_group_with_layout::<ConcurrentSupplyResource>(
            &apt_metadata_object_state_key,
            concurrent_supply_rg_tag,
            view,
            Some(&concurrent_supply_layout),
        )?
        .unwrap();

        let delayed_id = DelayedFieldID::from(*supply.current.get() as u64);
        view.validate_delayed_field_id(&delayed_id).unwrap();
        delayed_field_change_set.insert(
            delayed_id,
            DelayedChange::Apply(DelayedApplyChange::AggregatorDelta {
                delta: DeltaWithMax::new(SignedU128::Negative(gas as u128), u128::MAX),
            }),
        );
        let materialized_size = view
            .get_resource_state_value_size(&apt_metadata_object_state_key)
            .map_err(hide_error)?;
        let metadata = view
            .get_resource_state_value_metadata(&apt_metadata_object_state_key)
            .map_err(hide_error)?
            .unwrap();
        resource_write_set.insert(
            apt_metadata_object_state_key,
            AbstractResourceWriteOp::ResourceGroupInPlaceDelayedFieldChange(
                ResourceGroupInPlaceDelayedFieldChangeOp {
                    materialized_size,
                    metadata,
                },
            ),
        );
        Ok(())
    }

    fn reduce_coin_apt_supply(
        &self,
        gas: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        aggregator_v1_delta_set: &mut BTreeMap<StateKey, DeltaOp>,
    ) -> Result<(), ()> {
        let (sender_coin_store, _metadata) = Self::get_value::<CoinInfoResource<AptosCoinType>>(
            &self.db_util.common.apt_coin_info_resource,
            view,
        )?
        .ok_or(())?;

        let delta_op = DeltaOp::new(SignedU128::Negative(gas as u128), u128::MAX, DeltaHistory {
            max_achieved_positive_delta: 0,
            min_achieved_negative_delta: gas as u128,
            min_overflow_positive_delta: None,
            max_underflow_negative_delta: None,
        });
        aggregator_v1_delta_set.insert(sender_coin_store.supply_aggregator_state_key(), delta_op);
        Ok(())
    }

    fn withdraw_apt(
        &self,
        fa_migration_complete: bool,
        sender: AccountAddress,
        amount: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        gas: u64,
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: &mut Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Result<(), ()> {
        if fa_migration_complete {
            self.withdraw_fa_apt_from_signer(
                sender,
                amount,
                view,
                gas,
                resource_write_set,
                events,
            )?;
        } else {
            self.withdraw_coin_apt_from_signer(
                sender,
                amount,
                view,
                gas,
                resource_write_set,
                events,
            )?;
        }
        Ok(())
    }

    fn withdraw_fa_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        gas: u64,
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: &mut Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Result<(), ()> {
        let sender_store_address = primary_apt_store(sender_address);
        let sender_fa_store_object_key = self
            .db_util
            .new_state_key_object_resource_group(&sender_store_address);
        let fungible_store_rg_tag = &self.db_util.common.fungible_store;

        match Self::get_value_from_group::<FungibleStoreResource>(
            &sender_fa_store_object_key,
            fungible_store_rg_tag,
            view,
        )? {
            Some(mut fa_store) => {
                if fa_store.balance >= transfer_amount + gas {
                    fa_store.balance -= transfer_amount + gas;
                    let fa_store_write = Self::create_single_resource_in_group_modification(
                        &fa_store,
                        &sender_fa_store_object_key,
                        fungible_store_rg_tag.clone(),
                        view,
                    )?;
                    resource_write_set.insert(sender_fa_store_object_key, fa_store_write);

                    if transfer_amount > 0 {
                        events.push((
                            WithdrawFAEvent {
                                store: sender_store_address,
                                amount: transfer_amount,
                            }
                            .create_event_v2()
                            .expect("Creating WithdrawFAEvent should always succeed"),
                            None,
                        ));
                    }
                    Ok(())
                } else {
                    Err(())
                }
            },
            None => Err(()),
        }
    }

    fn withdraw_coin_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        gas: u64,
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: &mut Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Result<(), ()> {
        let sender_coin_store_key = self.db_util.new_state_key_aptos_coin(&sender_address);

        let sender_coin_store_opt =
            Self::get_value::<CoinStoreResource<AptosCoinType>>(&sender_coin_store_key, view)?;

        let (mut sender_coin_store, metadata) = match sender_coin_store_opt {
            None => {
                return self.withdraw_fa_apt_from_signer(
                    sender_address,
                    transfer_amount,
                    view,
                    gas,
                    resource_write_set,
                    events,
                )
            },
            Some((sender_coin_store, metadata)) => (sender_coin_store, metadata),
        };

        sender_coin_store.set_coin(sender_coin_store.coin() - transfer_amount - gas);

        // first need to create events, to update the handle, and then serialize sender_coin_store
        if transfer_amount > 0 {
            events.push((
                WithdrawEvent::new(transfer_amount)
                    .create_event_v1(sender_coin_store.withdraw_events_mut()),
                None,
            ));
        }

        // coin doesn't emit WithdrawEvent for gas.
        resource_write_set.insert(
            sender_coin_store_key,
            AbstractResourceWriteOp::Write(WriteOp::modification(
                Bytes::from(bcs::to_bytes(&sender_coin_store).map_err(hide_error)?),
                metadata,
            )),
        );

        Ok(())
    }

    /// Returns bool whether FungibleStore existed.
    fn deposit_apt(
        &self,
        fa_migration_complete: bool,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: &mut Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Result<bool, ()> {
        if fa_migration_complete {
            self.deposit_fa_apt(
                recipient_address,
                transfer_amount,
                view,
                resource_write_set,
                events,
            )
        } else {
            self.deposit_coin_apt(
                recipient_address,
                transfer_amount,
                view,
                resource_write_set,
                events,
            )
        }
    }

    /// Returns bool whether FungibleStore existed.
    fn deposit_fa_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: &mut Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Result<bool, ()> {
        let recipient_store_address = primary_apt_store(recipient_address);
        let recipient_fa_store_object_key = self
            .db_util
            .new_state_key_object_resource_group(&recipient_store_address);
        let fungible_store_rg_tag = &self.db_util.common.fungible_store;

        let (mut fa_store, rest_to_create, existed) =
            match Self::get_value_from_group::<FungibleStoreResource>(
                &recipient_fa_store_object_key,
                fungible_store_rg_tag,
                view,
            )? {
                Some(fa_store) => (fa_store, None, true),
                None => (
                    FungibleStoreResource::new(AccountAddress::TEN, 0, false),
                    Some(BTreeMap::from([(
                        self.db_util.common.object_core.clone(),
                        bcs::to_bytes(&DbAccessUtil::new_object_core(
                            recipient_store_address,
                            recipient_address,
                        ))
                        .map_err(hide_error)?,
                    )])),
                    false,
                ),
            };

        fa_store.balance += transfer_amount;

        let fa_store_write = if existed {
            Self::create_single_resource_in_group_modification(
                &fa_store,
                &recipient_fa_store_object_key,
                fungible_store_rg_tag.clone(),
                view,
            )?
        } else {
            let mut rg = rest_to_create.unwrap();
            rg.insert(
                fungible_store_rg_tag.clone(),
                bcs::to_bytes(&fa_store).map_err(hide_error)?,
            );
            Self::create_resource_in_group_creation(&recipient_fa_store_object_key, rg, view)?
        };
        resource_write_set.insert(recipient_fa_store_object_key, fa_store_write);

        if transfer_amount > 0 {
            let event = DepositFAEvent {
                store: recipient_store_address,
                amount: transfer_amount,
            };
            events.push((
                event
                    .create_event_v2()
                    .expect("Creating DepositFAEvent should always succeed"),
                None,
            ));
        }
        Ok(existed)
    }

    fn deposit_coin_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        view: &(impl ExecutorView + ResourceGroupView),
        resource_write_set: &mut BTreeMap<StateKey, AbstractResourceWriteOp>,
        events: &mut Vec<(ContractEvent, Option<MoveTypeLayout>)>,
    ) -> Result<bool, ()> {
        let recipient_coin_store_key = self.db_util.new_state_key_aptos_coin(&recipient_address);
        let (mut recipient_coin_store, recipient_coin_store_metadata, existed) =
            match Self::get_value::<CoinStoreResource<AptosCoinType>>(
                &recipient_coin_store_key,
                view,
            )? {
                Some((recipient_coin_store, metadata)) => {
                    (recipient_coin_store, Some(metadata), true)
                },
                None => {
                    events.push((
                        CoinRegister {
                            account: AccountAddress::ONE,
                            type_info: DbAccessUtil::new_type_info_resource::<AptosCoinType>()
                                .map_err(hide_error)?,
                        }
                        .create_event_v2()
                        .expect("Creating CoinRegister should always succeed"),
                        None,
                    ));
                    (
                        DbAccessUtil::new_apt_coin_store(0, recipient_address),
                        None,
                        false,
                    )
                },
            };

        recipient_coin_store.set_coin(recipient_coin_store.coin() + transfer_amount);

        // first need to create events, to update the handle, and then serialize sender_coin_store
        if transfer_amount > 0 {
            events.push((
                DepositEvent::new(transfer_amount)
                    .create_event_v1(recipient_coin_store.deposit_events_mut()),
                None,
            ))
        }
        let write_op = if existed {
            WriteOp::modification(
                Bytes::from(bcs::to_bytes(&recipient_coin_store).map_err(hide_error)?),
                recipient_coin_store_metadata.unwrap(),
            )
        } else {
            WriteOp::legacy_creation(Bytes::from(
                bcs::to_bytes(&recipient_coin_store).map_err(hide_error)?,
            ))
        };
        resource_write_set.insert(
            recipient_coin_store_key,
            AbstractResourceWriteOp::Write(write_op),
        );

        Ok(existed)
    }

    fn create_single_resource_in_group_modification<T: Serialize>(
        value: &T,
        group_key: &StateKey,
        resource_tag: StructTag,
        view: &(impl ExecutorView + ResourceGroupView),
    ) -> Result<AbstractResourceWriteOp, ()> {
        let metadata = view
            .get_resource_state_value_metadata(group_key)
            .map_err(hide_error)?
            .unwrap();
        let size = view.resource_group_size(group_key).map_err(hide_error)?;
        let value_bytes = Bytes::from(bcs::to_bytes(value).map_err(hide_error)?);
        let group_write = AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
            WriteOp::modification(Bytes::new(), metadata),
            BTreeMap::from([(
                resource_tag,
                (WriteOp::legacy_modification(value_bytes), None),
            )]),
            size,
            size.get(),
        ));
        Ok(group_write)
    }

    fn create_resource_in_group_creation(
        group_key: &StateKey,
        resources: BTreeMap<StructTag, Vec<u8>>,
        view: &(impl ExecutorView + ResourceGroupView),
    ) -> Result<AbstractResourceWriteOp, ()> {
        let size = view.resource_group_size(group_key).map_err(hide_error)?;
        assert_eq!(size.get(), 0);
        let inner_ops = resources
            .into_iter()
            .map(|(resource_tag, value)| -> Result<_, ()> {
                Ok((
                    resource_tag,
                    (WriteOp::legacy_creation(Bytes::from(value)), None),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, ()>>()?;

        let new_size = group_size_as_sum(
            inner_ops
                .iter()
                .map(|(resource_tag, (value, _layout))| (resource_tag, value.bytes_size())),
        )
        .map_err(hide_error)?;

        let group_write = AbstractResourceWriteOp::WriteResourceGroup(GroupWrite::new(
            WriteOp::legacy_creation(Bytes::new()),
            inner_ops,
            new_size,
            size.get(),
        ));
        Ok(group_write)
    }
}

fn hide_error<E: Debug>(e: E) {
    error!("encountered error {:?}, hiding", e);
}
