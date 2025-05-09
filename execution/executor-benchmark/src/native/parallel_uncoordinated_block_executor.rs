// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::native_transaction::compute_deltas_for_batch;
use crate::{
    db_access::DbAccessUtil,
    metrics::TIMER,
    native::{native_config::NATIVE_EXECUTOR_POOL, native_transaction::NativeTransaction},
};
use anyhow::{bail, Result};
use aptos_block_executor::{
    counters::BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK, txn_provider::default::DefaultTxnProvider,
};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{
        primary_apt_store, AccountResource, CoinInfoResource, CoinRegister, CoinStoreResource,
        ConcurrentSupplyResource, DepositEvent, DepositFAEvent, FungibleStoreResource,
        WithdrawEvent, WithdrawFAEvent,
    },
    block_executor::{
        config::BlockExecutorConfigFromOnchain,
        transaction_slice_metadata::TransactionSliceMetadata,
    },
    contract_event::ContractEvent,
    fee_statement::FeeStatement,
    move_utils::{move_event_v1::MoveEventV1Type, move_event_v2::MoveEventV2Type},
    on_chain_config::{FeatureFlag, Features, OnChainConfig},
    state_store::{state_key::StateKey, StateView},
    transaction::{
        signature_verified_transaction::SignatureVerifiedTransaction, BlockOutput, ExecutionStatus,
        TransactionAuxiliaryData, TransactionOutput, TransactionStatus,
    },
    vm_status::{StatusCode, VMStatus},
    write_set::{WriteOp, WriteSetMut},
    AptosCoinType,
};
use aptos_vm::VMBlockExecutor;
use dashmap::{
    mapref::one::{Ref, RefMut},
    DashMap,
};
use once_cell::sync::OnceCell;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use std::{
    cell::Cell,
    collections::BTreeMap,
    hash::RandomState,
    sync::atomic::{AtomicU64, Ordering},
    u64,
};
use thread_local::ThreadLocal;

/// Executes transactions fully, and produces TransactionOutput (with final WriteSet)
/// (unlike execution within BlockSTM that produces non-materialized VMChangeSet)
pub trait RawTransactionExecutor: Sync {
    type BlockState: Sync;

    fn new() -> Self;

    fn init_block_state(&self, state_view: &(impl StateView + Sync)) -> Self::BlockState;

    fn execute_transaction(
        &self,
        txn: NativeTransaction,
        state_view: &(impl StateView + Sync),
        block_state: &Self::BlockState,
    ) -> Result<TransactionOutput>;
}

pub struct NativeParallelUncoordinatedBlockExecutor<E: RawTransactionExecutor + Sync + Send> {
    executor: E,
}

impl<E: RawTransactionExecutor + Sync + Send> VMBlockExecutor
    for NativeParallelUncoordinatedBlockExecutor<E>
{
    fn new() -> Self {
        Self { executor: E::new() }
    }

    fn execute_block(
        &self,
        txn_provider: &DefaultTxnProvider<SignatureVerifiedTransaction>,
        state_view: &(impl StateView + Sync),
        _onchain_config: BlockExecutorConfigFromOnchain,
        _transaction_slice_metadata: TransactionSliceMetadata,
    ) -> Result<BlockOutput<TransactionOutput>, VMStatus> {
        let native_transactions = NATIVE_EXECUTOR_POOL.install(|| {
            txn_provider
                .get_txns()
                .par_iter()
                .map(NativeTransaction::parse)
                .collect::<Vec<_>>()
        });

        let _timer = BLOCK_EXECUTOR_INNER_EXECUTE_BLOCK.start_timer();

        let state = self.executor.init_block_state(state_view);

        let transaction_outputs = NATIVE_EXECUTOR_POOL
            .install(|| {
                native_transactions
                    .into_par_iter()
                    .map(|txn| self.executor.execute_transaction(txn, state_view, &state))
                    .collect::<Result<Vec<_>>>()
            })
            .map_err(|e| {
                VMStatus::error(
                    StatusCode::DELAYED_FIELD_OR_BLOCKSTM_CODE_INVARIANT_ERROR,
                    Some(format!("{:?}", e).to_string()),
                )
            })?;

        Ok(BlockOutput::new(transaction_outputs, None))
    }
}

pub struct IncrementalOutput {
    write_set: Vec<(StateKey, WriteOp)>,
    events: Vec<ContractEvent>,
}

impl IncrementalOutput {
    fn new() -> Self {
        IncrementalOutput {
            write_set: vec![],
            events: vec![],
        }
    }

    fn into_success_output(mut self, gas: u64) -> Result<TransactionOutput> {
        self.events.push(
            FeeStatement::new(gas, gas, 0, 0, 0)
                .create_event_v2()
                .expect("Creating FeeStatement should always succeed"),
        );

        Ok(TransactionOutput::new(
            WriteSetMut::new(self.write_set).freeze()?,
            self.events,
            /*gas_used=*/ gas,
            TransactionStatus::Keep(ExecutionStatus::Success),
            TransactionAuxiliaryData::default(),
        ))
    }
}

pub trait CommonNativeRawTransactionExecutor: Sync + Send {
    fn new_impl() -> Self;

    fn update_sequence_number(
        &self,
        sender_address: AccountAddress,
        sequence_number: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()>;

    fn reduce_apt_supply(
        &self,
        fa_migration_complete: bool,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        if fa_migration_complete {
            self.reduce_fa_apt_supply(gas, state_view, output)
        } else {
            self.reduce_coin_apt_supply(gas, state_view, output)
        }
    }

    fn reduce_fa_apt_supply(
        &self,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()>;

    fn reduce_coin_apt_supply(
        &self,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()>;

    fn withdraw_apt_from_signer(
        &self,
        fa_migration_complete: bool,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        if fa_migration_complete {
            self.withdraw_fa_apt_from_signer(
                sender_address,
                transfer_amount,
                gas,
                state_view,
                output,
            )
        } else {
            self.withdraw_coin_apt_from_signer(
                sender_address,
                transfer_amount,
                gas,
                state_view,
                output,
            )
        }
    }

    fn withdraw_fa_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()>;

    fn withdraw_coin_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()>;

    fn deposit_apt(
        &self,
        fa_migration_complete: bool,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<bool> {
        if fa_migration_complete {
            self.deposit_fa_apt(recipient_address, transfer_amount, state_view, output)
        } else {
            self.deposit_coin_apt(recipient_address, transfer_amount, state_view, output)
        }
    }

    fn deposit_fa_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<bool>;

    fn deposit_coin_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<bool>;

    fn check_or_create_account(
        &self,
        address: AccountAddress,
        fail_on_account_existing: bool,
        fail_on_account_missing: bool,
        create_account_resource: bool,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()>;
}

impl<T: CommonNativeRawTransactionExecutor> RawTransactionExecutor for T {
    type BlockState = bool;

    fn new() -> Self {
        Self::new_impl()
    }

    fn init_block_state(&self, state_view: &(impl StateView + Sync)) -> bool {
        let features = Features::fetch_config(&state_view).unwrap_or_default();
        let fa_migration_complete =
            features.is_enabled(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);
        let new_accounts_default_to_fa =
            features.is_enabled(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
        assert_eq!(
            fa_migration_complete, new_accounts_default_to_fa,
            "native code only works with both flags either enabled or disabled"
        );

        fa_migration_complete
    }

    fn execute_transaction(
        &self,
        txn: NativeTransaction,
        state_view: &(impl StateView + Sync),
        block_state: &bool,
    ) -> Result<TransactionOutput> {
        let fa_migration_complete = *block_state;

        let gas_unit = 4; // hardcode gas consumed.
        let gas = gas_unit * 100;

        let mut output = IncrementalOutput::new();

        match txn {
            NativeTransaction::Nop {
                sender,
                sequence_number,
            } => {
                self.update_sequence_number(sender, sequence_number, state_view, &mut output)?;

                self.withdraw_apt_from_signer(
                    fa_migration_complete,
                    sender,
                    0,
                    gas,
                    state_view,
                    &mut output,
                )?;
            },
            NativeTransaction::FaTransfer {
                sender,
                sequence_number,
                recipient,
                amount,
            } => {
                self.update_sequence_number(sender, sequence_number, state_view, &mut output)?;

                self.withdraw_fa_apt_from_signer(sender, amount, gas, state_view, &mut output)?;

                let _existed = self.deposit_fa_apt(recipient, amount, state_view, &mut output)?;
            },
            NativeTransaction::Transfer {
                sender,
                sequence_number,
                recipient,
                amount,
                fail_on_recipient_account_existing,
                fail_on_recipient_account_missing,
            } => {
                self.update_sequence_number(sender, sequence_number, state_view, &mut output)?;
                self.withdraw_apt_from_signer(
                    fa_migration_complete,
                    sender,
                    amount,
                    gas,
                    state_view,
                    &mut output,
                )?;

                let existed = self.deposit_apt(
                    fa_migration_complete,
                    recipient,
                    amount,
                    state_view,
                    &mut output,
                )?;

                if !existed || fail_on_recipient_account_existing {
                    self.check_or_create_account(
                        recipient,
                        fail_on_recipient_account_existing,
                        fail_on_recipient_account_missing,
                        !fa_migration_complete,
                        state_view,
                        &mut output,
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
                self.update_sequence_number(sender, sequence_number, state_view, &mut output)?;

                let (deltas, amount_to_sender) =
                    compute_deltas_for_batch(recipients, amounts, sender);

                self.withdraw_apt_from_signer(
                    fa_migration_complete,
                    sender,
                    amount_to_sender,
                    gas,
                    state_view,
                    &mut output,
                )?;

                for (recipient_address, transfer_amount) in deltas.into_iter() {
                    let existed = self.deposit_apt(
                        fa_migration_complete,
                        recipient_address,
                        transfer_amount as u64,
                        state_view,
                        &mut output,
                    )?;

                    if !existed || fail_on_recipient_account_existing {
                        self.check_or_create_account(
                            recipient_address,
                            fail_on_recipient_account_existing,
                            fail_on_recipient_account_missing,
                            true,
                            state_view,
                            &mut output,
                        )?;
                    }
                }
            },
        };

        self.reduce_apt_supply(fa_migration_complete, gas, state_view, &mut output)?;

        output.into_success_output(gas)
    }
}

pub struct NativeRawTransactionExecutor {
    db_util: DbAccessUtil,
}

impl CommonNativeRawTransactionExecutor for NativeRawTransactionExecutor {
    fn new_impl() -> Self {
        Self {
            db_util: DbAccessUtil::new(),
        }
    }

    fn update_sequence_number(
        &self,
        sender_address: AccountAddress,
        sequence_number: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        let sender_account_key = self.db_util.new_state_key_account(&sender_address);
        let mut sender_account = DbAccessUtil::get_account(&sender_account_key, state_view)?
            .unwrap_or_else(|| DbAccessUtil::new_account_resource(sender_address));

        sender_account.sequence_number = sequence_number + 1;

        output.write_set.push((
            sender_account_key,
            WriteOp::legacy_modification(bcs::to_bytes(&sender_account)?.into()),
        ));
        Ok(())
    }

    fn reduce_fa_apt_supply(
        &self,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        let apt_metadata_object_state_key = self
            .db_util
            .new_state_key_object_resource_group(&AccountAddress::TEN);

        let concurrent_supply_rg_tag = &self.db_util.common.concurrent_supply;

        let mut apt_metadata_object =
            DbAccessUtil::get_resource_group(&apt_metadata_object_state_key, state_view)?.unwrap();
        let mut supply = bcs::from_bytes::<ConcurrentSupplyResource>(
            &apt_metadata_object
                .remove(concurrent_supply_rg_tag)
                .unwrap(),
        )?;

        supply.current.set(supply.current.get() - gas as u128);

        apt_metadata_object.insert(concurrent_supply_rg_tag.clone(), bcs::to_bytes(&supply)?);

        output.write_set.push((
            apt_metadata_object_state_key,
            WriteOp::legacy_modification(bcs::to_bytes(&apt_metadata_object)?.into()),
        ));

        Ok(())
    }

    fn reduce_coin_apt_supply(
        &self,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        let coin_info = DbAccessUtil::get_value::<CoinInfoResource<AptosCoinType>>(
            &self.db_util.common.apt_coin_info_resource,
            state_view,
        )?
        .ok_or_else(|| anyhow::anyhow!("no coin info"))?;

        let total_supply_state_key = coin_info.supply_aggregator_state_key();
        let total_supply = DbAccessUtil::get_value::<u128>(&total_supply_state_key, state_view)?
            .ok_or_else(|| anyhow::anyhow!("no total supply"))?;

        output.write_set.push((
            total_supply_state_key,
            WriteOp::legacy_modification(bcs::to_bytes(&(total_supply - gas as u128))?.into()),
        ));

        Ok(())
    }

    fn withdraw_fa_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        let sender_store_address = primary_apt_store(sender_address);

        let sender_fa_store_object_key = self
            .db_util
            .new_state_key_object_resource_group(&sender_store_address);
        let mut sender_fa_store_object = {
            let _timer = TIMER
                .with_label_values(&["read_sender_fa_store"])
                .start_timer();
            match DbAccessUtil::get_resource_group(&sender_fa_store_object_key, state_view)? {
                Some(sender_fa_store_object) => sender_fa_store_object,
                None => bail!("sender fa store missing"),
            }
        };

        let fungible_store_rg_tag = &self.db_util.common.fungible_store;
        let mut sender_fa_store = bcs::from_bytes::<FungibleStoreResource>(
            &sender_fa_store_object
                .remove(fungible_store_rg_tag)
                .unwrap(),
        )?;

        sender_fa_store.balance -= transfer_amount + gas;

        sender_fa_store_object.insert(
            fungible_store_rg_tag.clone(),
            bcs::to_bytes(&sender_fa_store)?,
        );

        output.write_set.push((
            sender_fa_store_object_key,
            WriteOp::legacy_modification(bcs::to_bytes(&sender_fa_store_object)?.into()),
        ));

        if transfer_amount > 0 {
            output.events.push(
                WithdrawFAEvent {
                    store: sender_store_address,
                    amount: transfer_amount,
                }
                .create_event_v2()
                .expect("Creating WithdrawFAEvent should always succeed"),
            );
        }

        Ok(())
    }

    fn withdraw_coin_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        let sender_coin_store_key = self.db_util.new_state_key_aptos_coin(&sender_address);
        let sender_coin_store_opt = {
            let _timer = TIMER
                .with_label_values(&["read_sender_coin_store"])
                .start_timer();
            DbAccessUtil::get_apt_coin_store(&sender_coin_store_key, state_view)?
        };
        let mut sender_coin_store = match sender_coin_store_opt {
            None => {
                return self.withdraw_fa_apt_from_signer(
                    sender_address,
                    transfer_amount,
                    gas,
                    state_view,
                    output,
                )
            },
            Some(sender_coin_store) => sender_coin_store,
        };

        sender_coin_store.set_coin(sender_coin_store.coin() - transfer_amount - gas);

        if transfer_amount != 0 {
            output.events.push(
                WithdrawEvent::new(transfer_amount)
                    .create_event_v1(sender_coin_store.withdraw_events_mut()),
            );
            // Coin doesn't emit Withdraw event for gas
        }

        output.write_set.push((
            sender_coin_store_key,
            WriteOp::legacy_modification(bcs::to_bytes(&sender_coin_store)?.into()),
        ));

        Ok(())
    }

    fn deposit_fa_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<bool> {
        let recipient_store_address = primary_apt_store(recipient_address);
        let recipient_fa_store_object_key = self
            .db_util
            .new_state_key_object_resource_group(&recipient_store_address);
        let fungible_store_rg_tag = &self.db_util.common.fungible_store;

        let (mut recipient_fa_store, mut recipient_fa_store_object, recipient_fa_store_existed) =
            match DbAccessUtil::get_resource_group(&recipient_fa_store_object_key, state_view)? {
                Some(mut recipient_fa_store_object) => {
                    let recipient_fa_store = bcs::from_bytes::<FungibleStoreResource>(
                        &recipient_fa_store_object
                            .remove(fungible_store_rg_tag)
                            .unwrap(),
                    )?;
                    (recipient_fa_store, recipient_fa_store_object, true)
                },
                None => {
                    let receipeint_fa_store =
                        FungibleStoreResource::new(AccountAddress::TEN, 0, false);
                    let receipeint_fa_store_object = BTreeMap::from([(
                        self.db_util.common.object_core.clone(),
                        bcs::to_bytes(&DbAccessUtil::new_object_core(
                            recipient_store_address,
                            recipient_address,
                        ))?,
                    )]);
                    (receipeint_fa_store, receipeint_fa_store_object, false)
                },
            };

        recipient_fa_store.balance += transfer_amount;

        recipient_fa_store_object.insert(
            fungible_store_rg_tag.clone(),
            bcs::to_bytes(&recipient_fa_store)?,
        );

        output.write_set.push((
            recipient_fa_store_object_key,
            if recipient_fa_store_existed {
                WriteOp::legacy_modification(bcs::to_bytes(&recipient_fa_store_object)?.into())
            } else {
                WriteOp::legacy_creation(bcs::to_bytes(&recipient_fa_store_object)?.into())
            },
        ));

        if transfer_amount != 0 {
            output.events.push(
                DepositFAEvent {
                    store: recipient_store_address,
                    amount: transfer_amount,
                }
                .create_event_v2()
                .expect("Creating DepositFAEvent should always succeed"),
            )
        }

        Ok(recipient_fa_store_existed)
    }

    fn deposit_coin_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<bool> {
        let recipient_coin_store_key = self.db_util.new_state_key_aptos_coin(&recipient_address);

        let (mut recipient_coin_store, recipient_coin_store_existed) =
            match DbAccessUtil::get_apt_coin_store(&recipient_coin_store_key, state_view)? {
                Some(recipient_coin_store) => (recipient_coin_store, true),
                None => {
                    output.events.push(
                        CoinRegister {
                            account: AccountAddress::ONE,
                            type_info: DbAccessUtil::new_type_info_resource::<AptosCoinType>()?,
                        }
                        .create_event_v2()
                        .expect("Creating CoinRegister should always succeed"),
                    );
                    (
                        DbAccessUtil::new_apt_coin_store(0, recipient_address),
                        false,
                    )
                },
            };

        recipient_coin_store.set_coin(recipient_coin_store.coin() + transfer_amount);

        // first need to create events, to update the handle, and then serialize sender_coin_store
        if transfer_amount != 0 {
            output.events.push(
                DepositEvent::new(transfer_amount)
                    .create_event_v1(recipient_coin_store.deposit_events_mut()),
            );
        }

        output.write_set.push((
            recipient_coin_store_key,
            if recipient_coin_store_existed {
                WriteOp::legacy_modification(bcs::to_bytes(&recipient_coin_store)?.into())
            } else {
                WriteOp::legacy_creation(bcs::to_bytes(&recipient_coin_store)?.into())
            },
        ));

        Ok(recipient_coin_store_existed)
    }

    fn check_or_create_account(
        &self,
        address: AccountAddress,
        fail_on_account_existing: bool,
        fail_on_account_missing: bool,
        create_account_resource: bool,
        state_view: &(impl StateView + Sync),
        output: &mut IncrementalOutput,
    ) -> Result<()> {
        let account_key = self.db_util.new_state_key_account(&address);
        match DbAccessUtil::get_account(&account_key, state_view)? {
            Some(_) => {
                if fail_on_account_existing {
                    bail!("account exists");
                }
            },
            None => {
                if fail_on_account_missing {
                    bail!("account missing")
                } else if create_account_resource {
                    let account = DbAccessUtil::new_account_resource(address);
                    output.write_set.push((
                        account_key,
                        WriteOp::legacy_creation(bcs::to_bytes(&account)?.into()),
                    ));
                }
            },
        }

        Ok(())
    }
}

const USE_THREAD_LOCAL_SUPPLY: bool = true;
struct CoinSupply {
    pub total_supply: u128,
}

struct SupplyWithDecrement {
    #[allow(dead_code)]
    pub base: u128,
    pub decrement: ThreadLocal<Cell<u128>>,
}

enum CachedResource {
    Account(AccountResource),
    FungibleStore(FungibleStoreResource),
    FungibleSupply(ConcurrentSupplyResource),
    AptCoinStore(CoinStoreResource<AptosCoinType>),
    AptCoinInfo(CoinInfoResource<AptosCoinType>),
    AptCoinSupply(CoinSupply),
    SupplyDecrement(SupplyWithDecrement),
}

pub struct NativeValueCacheRawTransactionExecutor {
    db_util: DbAccessUtil,
    cache: DashMap<StateKey, CachedResource>,
    coin_supply_state_key: OnceCell<StateKey>,
}

impl CommonNativeRawTransactionExecutor for NativeValueCacheRawTransactionExecutor {
    fn new_impl() -> Self {
        Self {
            db_util: DbAccessUtil::new(),
            cache: DashMap::new(),
            coin_supply_state_key: OnceCell::new(),
        }
    }

    fn update_sequence_number(
        &self,
        sender_address: AccountAddress,
        sequence_number: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<()> {
        let sender_account_key = self.db_util.new_state_key_account(&sender_address);

        match self
            .cache_get_mut_or_init(&sender_account_key, |key| {
                CachedResource::Account(
                    DbAccessUtil::get_account(key, state_view)
                        .unwrap()
                        .unwrap_or_else(|| DbAccessUtil::new_account_resource(sender_address)),
                )
            })
            .value_mut()
        {
            CachedResource::Account(account) => {
                account.sequence_number = sequence_number + 1;
            },
            _ => {
                panic!("wrong type")
            },
        };
        Ok(())
    }

    fn check_or_create_account(
        &self,
        address: AccountAddress,
        fail_on_account_existing: bool,
        fail_on_account_missing: bool,
        _create_account_resource: bool,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<()> {
        let account_key = self.db_util.new_state_key_account(&address);
        let mut missing = false;
        self.cache_get_mut_or_init(&account_key, |key| {
            CachedResource::Account(match DbAccessUtil::get_account(key, state_view).unwrap() {
                Some(account) => account,
                None => {
                    missing = true;
                    assert!(!fail_on_account_missing);
                    DbAccessUtil::new_account_resource(address)
                },
            })
        });
        if fail_on_account_existing {
            assert!(missing);
        }
        Ok(())
    }

    fn reduce_fa_apt_supply(
        &self,
        gas: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<()> {
        let cache_key =
            StateKey::resource(&AccountAddress::TEN, &self.db_util.common.concurrent_supply)
                .unwrap();

        if USE_THREAD_LOCAL_SUPPLY {
            let entry = self.cache_get_or_init(&cache_key, |_key| {
                let concurrent_supply = self.fetch_concurrent_supply(state_view);
                CachedResource::SupplyDecrement(SupplyWithDecrement {
                    base: *concurrent_supply.current.get(),
                    decrement: ThreadLocal::new(),
                })
            });
            match entry.value() {
                CachedResource::SupplyDecrement(SupplyWithDecrement { decrement, .. }) => {
                    let decrement_cell = decrement.get_or_default();
                    decrement_cell.set(decrement_cell.get() + gas as u128);
                },
                _ => panic!("wrong type"),
            }
        } else {
            let mut entry = self.cache_get_mut_or_init(&cache_key, |_key| {
                let concurrent_supply = self.fetch_concurrent_supply(state_view);
                CachedResource::FungibleSupply(concurrent_supply)
            });
            match entry.value_mut() {
                CachedResource::FungibleSupply(fungible_supply) => {
                    fungible_supply
                        .current
                        .set(fungible_supply.current.get() - gas as u128);
                },
                _ => panic!("wrong type"),
            };
        }

        Ok(())
    }

    fn reduce_coin_apt_supply(
        &self,
        gas: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<()> {
        let total_supply_state_key = self.coin_supply_state_key.get_or_init(|| {
            let entry =
                self.cache_get_mut_or_init(&self.db_util.common.apt_coin_info_resource, |key| {
                    CachedResource::AptCoinInfo(
                        DbAccessUtil::get_value::<CoinInfoResource<AptosCoinType>>(key, state_view)
                            .unwrap()
                            .unwrap(),
                    )
                });

            let total_supply_state_key = match entry.value() {
                CachedResource::AptCoinInfo(coin_info) => coin_info.supply_aggregator_state_key(),
                _ => panic!("wrong type"),
            };
            total_supply_state_key
        });

        if USE_THREAD_LOCAL_SUPPLY {
            let total_supply_entry = self.cache_get_or_init(total_supply_state_key, |key| {
                CachedResource::SupplyDecrement(SupplyWithDecrement {
                    base: DbAccessUtil::get_value::<u128>(key, state_view)
                        .unwrap()
                        .unwrap(),
                    decrement: ThreadLocal::new(),
                })
            });
            match total_supply_entry.value() {
                CachedResource::SupplyDecrement(SupplyWithDecrement { decrement, .. }) => {
                    let decrement_cell = decrement.get_or_default();
                    decrement_cell.set(decrement_cell.get() + gas as u128);
                },
                _ => panic!("wrong type"),
            }
        } else {
            let mut total_supply_entry =
                self.cache_get_mut_or_init(total_supply_state_key, |key| {
                    CachedResource::AptCoinSupply(CoinSupply {
                        total_supply: DbAccessUtil::get_value::<u128>(key, state_view)
                            .unwrap()
                            .unwrap(),
                    })
                });

            match total_supply_entry.value_mut() {
                CachedResource::AptCoinSupply(coin_supply) => {
                    coin_supply.total_supply -= gas as u128;
                },
                _ => panic!("wrong type"),
            };
        }

        Ok(())
    }

    fn withdraw_fa_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<()> {
        let _existed =
            self.update_fa_balance(sender_address, state_view, 0, transfer_amount + gas, true);
        Ok(())
    }

    fn withdraw_coin_apt_from_signer(
        &self,
        sender_address: AccountAddress,
        transfer_amount: u64,
        gas: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<()> {
        let _existed =
            self.update_coin_balance(sender_address, state_view, 0, transfer_amount + gas, true);
        Ok(())
    }

    fn deposit_fa_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<bool> {
        let existed =
            self.update_fa_balance(recipient_address, state_view, transfer_amount, 0, false);
        Ok(existed)
    }

    fn deposit_coin_apt(
        &self,
        recipient_address: AccountAddress,
        transfer_amount: u64,
        state_view: &(impl StateView + Sync),
        _output: &mut IncrementalOutput,
    ) -> Result<bool> {
        let existed =
            self.update_coin_balance(recipient_address, state_view, transfer_amount, 0, false);
        Ok(existed)
    }
}

impl NativeValueCacheRawTransactionExecutor {
    fn cache_get_or_init<'a>(
        &'a self,
        key: &StateKey,
        init_value: impl FnOnce(&StateKey) -> CachedResource,
    ) -> Ref<'a, StateKey, CachedResource, RandomState> {
        // Data in cache is going to be the hot path, so short-circuit here to avoid cloning the key.
        if let Some(ref_mut) = self.cache.get(key) {
            return ref_mut;
        }

        self.cache
            .entry(key.clone())
            .or_insert(init_value(key))
            .downgrade()
    }

    fn cache_get_mut_or_init<'a>(
        &'a self,
        key: &StateKey,
        init_value: impl FnOnce(&StateKey) -> CachedResource,
    ) -> RefMut<'a, StateKey, CachedResource, RandomState> {
        // Data in cache is going to be the hot path, so short-circuit here to avoid cloning the key.
        if let Some(ref_mut) = self.cache.get_mut(key) {
            return ref_mut;
        }

        self.cache.entry(key.clone()).or_insert(init_value(key))
    }

    fn fetch_concurrent_supply(
        &self,
        state_view: &(impl StateView + Sync),
    ) -> ConcurrentSupplyResource {
        let concurrent_supply_rg_tag = &self.db_util.common.concurrent_supply;

        let apt_metadata_object_state_key = self
            .db_util
            .new_state_key_object_resource_group(&AccountAddress::TEN);

        let mut apt_metadata_object =
            DbAccessUtil::get_resource_group(&apt_metadata_object_state_key, state_view)
                .unwrap()
                .unwrap();

        bcs::from_bytes::<ConcurrentSupplyResource>(
            &apt_metadata_object
                .remove(concurrent_supply_rg_tag)
                .unwrap(),
        )
        .unwrap()
    }

    fn update_fa_balance(
        &self,
        account: AccountAddress,
        state_view: &(impl StateView + Sync),
        increment: u64,
        decrement: u64,
        fail_on_missing: bool,
    ) -> bool {
        let store_address = primary_apt_store(account);
        let fungible_store_rg_tag = &self.db_util.common.fungible_store;
        let cache_key = StateKey::resource(&store_address, fungible_store_rg_tag).unwrap();

        let mut exists = true;
        let mut entry = self.cache.entry(cache_key).or_insert_with(|| {
            let fa_store_object_key = self
                .db_util
                .new_state_key_object_resource_group(&store_address);
            let rg_opt =
                DbAccessUtil::get_resource_group(&fa_store_object_key, state_view).unwrap();
            CachedResource::FungibleStore(match rg_opt {
                Some(mut rg) => {
                    bcs::from_bytes(&rg.remove(fungible_store_rg_tag).unwrap()).unwrap()
                },
                None => {
                    exists = false;
                    assert!(!fail_on_missing);
                    FungibleStoreResource::new(AccountAddress::TEN, 0, false)
                },
            })
        });
        match entry.value_mut() {
            CachedResource::FungibleStore(fungible_store_resource) => {
                fungible_store_resource.balance += increment;
                fungible_store_resource.balance -= decrement;
            },
            _ => panic!("wrong type"),
        };
        exists
    }

    fn update_coin_balance(
        &self,
        account: AccountAddress,
        state_view: &(impl StateView + Sync),
        increment: u64,
        decrement: u64,
        fail_on_missing: bool,
    ) -> bool {
        let coin_store_key = self.db_util.new_state_key_aptos_coin(&account);
        let mut exists = true;

        let mut entry = self.cache_get_mut_or_init(&coin_store_key, |key| {
            CachedResource::AptCoinStore(
                match DbAccessUtil::get_apt_coin_store(key, state_view).unwrap() {
                    Some(store) => store,
                    None => {
                        exists = false;
                        assert!(!fail_on_missing);
                        DbAccessUtil::new_apt_coin_store(0, account)
                    },
                },
            )
        });

        match entry.value_mut() {
            CachedResource::AptCoinStore(coin_store) => {
                coin_store.set_coin(coin_store.coin() + increment - decrement);
            },
            _ => panic!("wrong type"),
        };

        exists
    }
}

pub struct NativeNoStorageRawTransactionExecutor {
    seq_nums: DashMap<AccountAddress, u64>,
    balances: DashMap<AccountAddress, u64>,
    total_supply_decrement: ThreadLocal<Cell<u128>>,
    total_supply: AtomicU64,
}

impl RawTransactionExecutor for NativeNoStorageRawTransactionExecutor {
    type BlockState = ();

    fn new() -> Self {
        Self {
            seq_nums: DashMap::new(),
            balances: DashMap::new(),
            total_supply_decrement: ThreadLocal::new(),
            total_supply: AtomicU64::new(u64::MAX),
        }
    }

    fn init_block_state(&self, _state_view: &(impl StateView + Sync)) {}

    fn execute_transaction(
        &self,
        txn: NativeTransaction,
        _state_view: &(impl StateView + Sync),
        _block_state: &(),
    ) -> Result<TransactionOutput> {
        let gas_units = 4;
        let gas = gas_units * 100;

        if USE_THREAD_LOCAL_SUPPLY {
            let decrement_cell = self.total_supply_decrement.get_or_default();
            decrement_cell.set(decrement_cell.get() + gas as u128);
        } else {
            self.total_supply.fetch_sub(gas, Ordering::Relaxed);
        }

        let output = IncrementalOutput::new();
        let (sender, sequence_number) = match txn {
            NativeTransaction::Nop {
                sender,
                sequence_number,
            } => {
                *self
                    .balances
                    .entry(sender)
                    .or_insert(100_000_000_000_000_000) -= gas;
                (sender, sequence_number)
            },
            NativeTransaction::FaTransfer {
                sender,
                sequence_number,
                recipient,
                amount,
            }
            | NativeTransaction::Transfer {
                sender,
                sequence_number,
                recipient,
                amount,
                ..
            } => {
                *self
                    .balances
                    .entry(sender)
                    .or_insert(100_000_000_000_000_000) -= amount + gas;
                *self
                    .balances
                    .entry(recipient)
                    .or_insert(100_000_000_000_000_000) += amount;
                (sender, sequence_number)
            },
            NativeTransaction::BatchTransfer {
                sender,
                sequence_number,
                recipients,
                amounts,
                ..
            } => {
                let (deltas, amount_from_sender) =
                    compute_deltas_for_batch(recipients, amounts, sender);

                *self
                    .balances
                    .entry(sender)
                    .or_insert(100_000_000_000_000_000) -= amount_from_sender;

                for (recipient, amount) in deltas.into_iter() {
                    *self
                        .balances
                        .entry(recipient)
                        .or_insert(100_000_000_000_000_000) += amount as u64;
                }
                (sender, sequence_number)
            },
        };

        self.seq_nums.insert(sender, sequence_number);
        output.into_success_output(gas)
    }
}
