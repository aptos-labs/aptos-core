// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    benchmark_transaction::{AccountCreationInfo, BenchmarkTransaction, ExtraInfo, TransferInfo},
    metrics::TIMER,
};
use anyhow::Result;
use aptos_executor::{
    block_executor::TransactionBlockExecutor, components::chunk_output::ChunkOutput,
};
use aptos_state_view::TStateView;
use aptos_storage_interface::cached_state_view::CachedStateView;
use aptos_types::{
    access_path::AccessPath,
    account_address::AccountAddress,
    account_config::{deposit::DepositEvent, withdraw::WithdrawEvent},
    contract_event::ContractEvent,
    event::EventKey,
    state_store::state_key::StateKey,
    transaction::{ExecutionStatus, Transaction, TransactionOutput, TransactionStatus},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use once_cell::sync::{Lazy, OnceCell};
use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::str::FromStr;

pub struct FakeExecutor {}

type Address = [u8; 32];

static FAKE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
static FAKE_EXECUTOR_POOL: Lazy<ThreadPool> = Lazy::new(|| {
    ThreadPoolBuilder::new()
        .num_threads(FakeExecutor::get_concurrency_level())
        .thread_name(|index| format!("fake_exe_{}", index))
        .build()
        .unwrap()
});

// Note: in case this changes in the future, it doesn't have to be a constant, and can be read from
// genesis directly if necessary.
static TOTAL_SUPPLY_STATE_KEY: Lazy<StateKey> = Lazy::new(|| {
    StateKey::table_item(
        "1b854694ae746cdbd8d44186ca4929b2b337df21d1c74633be19b2710552fdca"
            .parse()
            .unwrap(),
        vec![
            6, 25, 220, 41, 160, 170, 200, 250, 20, 103, 20, 5, 142, 141, 214, 210, 208, 243, 189,
            245, 246, 51, 25, 7, 191, 145, 243, 172, 216, 30, 105, 53,
        ],
    )
});

#[derive(Debug, Default, Deserialize, Serialize)]
struct CoinStore {
    coin: u64,
    _frozen: bool,
    _deposit_events: EventHandle,
    _withdraw_events: EventHandle,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct EventHandle {
    _counter: u64,
    _guid: GUID,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct GUID {
    _creation_num: u64,
    _address: Address,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Account {
    authentication_key: Vec<u8>,
    sequence_number: u64,
    _guid_creation_num: u64,
    _coin_register_events: EventHandle,
    _key_rotation_events: EventHandle,
    _rotation_capability_offer: CapabilityOffer,
    _signer_capability_offer: CapabilityOffer,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct CapabilityOffer {
    _for_address: Option<Address>,
}

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

    fn new_struct_tag(
        address: AccountAddress,
        module: &str,
        name: &str,
        type_params: Vec<TypeTag>,
    ) -> StructTag {
        StructTag {
            address,
            module: Identifier::from_str(module).unwrap(),
            name: Identifier::from_str(name).unwrap(),
            type_params,
        }
    }

    fn new_state_key(
        address: AccountAddress,
        resource_address: AccountAddress,
        module: &str,
        name: &str,
        type_params: Vec<TypeTag>,
    ) -> StateKey {
        StateKey::access_path(AccessPath::new(
            address,
            AccessPath::resource_path_vec(Self::new_struct_tag(
                resource_address,
                module,
                name,
                type_params,
            ))
            .expect("access path in test"),
        ))
    }

    fn new_state_key_account(address: AccountAddress) -> StateKey {
        Self::new_state_key(address, AccountAddress::ONE, "account", "Account", vec![])
    }

    fn new_state_key_aptos_coin(address: AccountAddress) -> StateKey {
        Self::new_state_key(address, AccountAddress::ONE, "coin", "CoinStore", vec![
            TypeTag::Struct(Box::new(Self::new_struct_tag(
                AccountAddress::ONE,
                "aptos_coin",
                "AptosCoin",
                vec![],
            ))),
        ])
    }

    fn get_account(
        account_key: &StateKey,
        state_view: &CachedStateView,
    ) -> Result<Option<Account>> {
        Self::get_value(account_key, state_view)
    }

    fn get_coin_store(
        coin_store_key: &StateKey,
        state_view: &CachedStateView,
    ) -> Result<Option<CoinStore>> {
        Self::get_value(coin_store_key, state_view)
    }

    fn get_value<T: DeserializeOwned>(
        state_key: &StateKey,
        state_view: &CachedStateView,
    ) -> Result<Option<T>> {
        let value = state_view
            .get_state_value_bytes(state_key)?
            .map(move |value| bcs::from_bytes(value.as_slice()));
        value.transpose().map_err(anyhow::Error::msg)
    }

    fn handle_transfer(
        _transfer_info: &TransferInfo,
        _state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        // TODO(grao): Implement this function.
        Ok(TransactionOutput::new(
            Default::default(),
            vec![],
            /*gas_used=*/ 1,
            TransactionStatus::Keep(ExecutionStatus::Success),
        ))
    }

    fn handle_account_creation(
        account_creation_info: &AccountCreationInfo,
        state_view: &CachedStateView,
    ) -> Result<TransactionOutput> {
        let _timer = TIMER.with_label_values(&["account_creation"]).start_timer();
        let sender_address = account_creation_info.sender;
        let new_account_address = account_creation_info.new_account;

        let sender_account_key = Self::new_state_key_account(sender_address);
        let mut sender_account = {
            let _timer = TIMER
                .with_label_values(&["read_sender_account"])
                .start_timer();
            Self::get_account(&sender_account_key, state_view)?.unwrap()
        };
        let sender_coin_store_key = Self::new_state_key_aptos_coin(sender_address);
        let mut sender_coin_store = {
            let _timer = TIMER
                .with_label_values(&["read_sender_coin_store"])
                .start_timer();
            Self::get_coin_store(&sender_coin_store_key, state_view)?.unwrap()
        };

        let new_account_key = Self::new_state_key_account(new_account_address);
        let new_coin_store_key = Self::new_state_key_aptos_coin(new_account_address);

        {
            let _timer = TIMER.with_label_values(&["read_new_account"]).start_timer();
            let new_account_already_exists =
                Self::get_account(&new_account_key, state_view)?.is_some();
            if new_account_already_exists {
                // This is to handle the case that we re-create seed accounts when adding more
                // accounts into an existing db. In real VM this will be an abort, I choose to
                // return Success here, for simplicity.
                return Ok(TransactionOutput::new(
                    Default::default(),
                    vec![],
                    0,
                    TransactionStatus::Keep(ExecutionStatus::Success),
                ));
            }
        }
        {
            let _timer = TIMER
                .with_label_values(&["read_new_coin_store"])
                .start_timer();
            assert!(Self::get_coin_store(&new_coin_store_key, state_view)?.is_none());
        }

        // Note: numbers below may not be real. When runninng in parallel there might be conflicts.
        sender_coin_store.coin -= account_creation_info.initial_balance;

        let gas = 1;
        sender_coin_store.coin -= gas;

        sender_account.sequence_number += 1;

        let new_account = Account {
            authentication_key: new_account_address.to_vec(),
            ..Default::default()
        };

        let new_coin_store = CoinStore {
            coin: account_creation_info.initial_balance,
            ..Default::default()
        };

        let mut total_supply: u128 = Self::get_value(&TOTAL_SUPPLY_STATE_KEY, state_view)?.unwrap();
        total_supply -= gas as u128;

        // TODO(grao): Add other reads to match the read set of the real transaction.

        let write_set = vec![
            (
                sender_account_key,
                WriteOp::Modification(bcs::to_bytes(&sender_account)?),
            ),
            (
                sender_coin_store_key,
                WriteOp::Modification(bcs::to_bytes(&sender_coin_store)?),
            ),
            (
                new_account_key,
                WriteOp::Creation(bcs::to_bytes(&new_account)?),
            ),
            (
                new_coin_store_key,
                WriteOp::Creation(bcs::to_bytes(&new_coin_store)?),
            ),
            (
                TOTAL_SUPPLY_STATE_KEY.clone(),
                WriteOp::Modification(bcs::to_bytes(&total_supply)?),
            ),
        ];

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
                            Self::handle_transfer(transfer_info, &state_view)
                        },
                        ExtraInfo::AccountCreationInfo(account_creation_info) => {
                            Self::handle_account_creation(account_creation_info, &state_view)
                        },
                    },
                    None => match &txn.transaction {
                        Transaction::StateCheckpoint(_) => Self::handle_state_checkpoint(),
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
