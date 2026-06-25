// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Validator-side reader API for native-position data.
//!
//! At block boundaries off-Move consumers query the layered
//! per-account [`UserPositions`] held at the `PositionBundle` level
//! via [`NativeStateReader`] to inspect committed Position state
//! without going through the VM session cache.

#![forbid(unsafe_code)]

use crate::native_state_store::{UserPositionKey, UserPositions};
use aptos_infallible::Mutex;
use aptos_types::state_store::native_position::NativePosition;
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

static LATEST_READER: std::sync::Mutex<Option<Arc<InMemoryNativeStateReader>>> =
    std::sync::Mutex::new(None);

pub fn install_global_reader(reader: Arc<InMemoryNativeStateReader>) {
    if let Ok(mut guard) = LATEST_READER.lock() {
        *guard = Some(reader);
    }
}

pub fn global_reader() -> Option<Arc<InMemoryNativeStateReader>> {
    LATEST_READER.lock().ok().and_then(|g| g.clone())
}

pub trait NativeStateReader: Send + Sync {
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress>;

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)>;

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize;
}

pub struct InMemoryNativeStateReader {
    user_positions: Arc<Mutex<UserPositions>>,
}

impl InMemoryNativeStateReader {
    pub fn new(user_positions: Arc<Mutex<UserPositions>>) -> Self {
        Self { user_positions }
    }

    pub fn snapshot(&self) -> NativeStateView {
        NativeStateView {
            user_positions: self.user_positions.lock().clone(),
        }
    }
}

impl NativeStateReader for InMemoryNativeStateReader {
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        self.snapshot().iter_position_accounts_for_exchange(exchange)
    }

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.snapshot().count_positions_for_exchange(exchange)
    }

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        self.snapshot().get_account_positions(exchange, account)
    }
}

pub struct NativeStateView {
    user_positions: UserPositions,
}

impl NativeStateView {
    pub fn iter_position_accounts_for_exchange(
        &self,
        exchange: AccountAddress,
    ) -> Vec<AccountAddress> {
        let view = self
            .user_positions
            .top()
            .view_layers_after(self.user_positions.family_root());
        view.iter()
            .filter_map(|(position_key, state)| {
                (position_key.exchange == exchange && !state.is_empty())
                    .then_some(position_key.account)
            })
            .collect()
    }

    pub fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        let view = self
            .user_positions
            .top()
            .view_layers_after(self.user_positions.family_root());
        view.iter()
            .filter(|(position_key, _)| position_key.exchange == exchange)
            .map(|(_, state)| state.positions.len())
            .sum()
    }

    pub fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        let key = UserPositionKey { exchange, account };
        self.user_positions
            .get(&key)
            .map(|us| us.positions.iter().map(|(k, v)| (*k, v.clone())).collect())
            .unwrap_or_default()
    }
}
