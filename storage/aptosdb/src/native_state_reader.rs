// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Reader over the committed position tip.
//!
//! The query engine itself is [`PositionBase::with_view`], which reads any
//! overlay over the base; this binds it to `bundle.positions`.
//! Consensus-critical consumers want a specific block's overlay from the
//! executor instead, not the tip, which trails execution by however far
//! the commit pipeline lags.

#![forbid(unsafe_code)]

use crate::native_state_store::{NativeStateView, PositionBase, PositionOverlay};
use aptos_infallible::Mutex;
use aptos_types::{state_store::native_position::NativePosition, transaction::Version};
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;

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
    base: Arc<PositionBase>,
    positions: Arc<Mutex<PositionOverlay>>,
}

impl InMemoryNativeStateReader {
    pub fn new(base: Arc<PositionBase>, positions: Arc<Mutex<PositionOverlay>>) -> Self {
        Self { base, positions }
    }

    /// Holds the tip mutex for the scope, so the overlay cannot be
    /// republished underneath the view. Tip then base, everywhere.
    pub fn with_view<R>(&self, f: impl FnOnce(&NativeStateView<'_>) -> R) -> R {
        let tip = self.positions.lock();
        self.base.with_view(&tip, f)
    }

    /// A specific overlay — a block's, say — over the same base.
    pub fn with_view_of<R>(
        &self,
        overlay: &PositionOverlay,
        f: impl FnOnce(&NativeStateView<'_>) -> R,
    ) -> R {
        self.base.with_view(overlay, f)
    }

    /// Version of the published tip.
    pub fn tip_version(&self) -> Option<Version> {
        self.positions.lock().version()
    }
}

impl NativeStateReader for InMemoryNativeStateReader {
    fn iter_position_accounts_for_exchange(&self, exchange: AccountAddress) -> Vec<AccountAddress> {
        self.with_view(|view| view.iter_position_accounts_for_exchange(exchange))
    }

    fn count_positions_for_exchange(&self, exchange: AccountAddress) -> usize {
        self.with_view(|view| view.count_positions_for_exchange(exchange))
    }

    fn get_account_positions(
        &self,
        exchange: AccountAddress,
        account: AccountAddress,
    ) -> Vec<(AccountAddress, NativePosition)> {
        self.with_view(|view| view.get_account_positions(exchange, account))
    }
}
