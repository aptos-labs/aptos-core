// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::StateView;
use aptos_types::{
    account_address::AccountAddress, account_view::AccountView, state_store::state_key::StateKey,
};

pub struct AccountWithStateView<'a> {
    account_address: &'a AccountAddress,
    state_view: &'a dyn StateView,
}

impl<'a> AccountWithStateView<'a> {
    pub fn new(account_address: &'a AccountAddress, state_view: &'a dyn StateView) -> Self {
        Self {
            account_address,
            state_view,
        }
    }
}

impl<'a> AccountView for AccountWithStateView<'a> {
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
        self.state_view.get_state_value(state_key)
    }

    fn get_account_address(&self) -> anyhow::Result<Option<AccountAddress>> {
        Ok(Some(*self.account_address))
    }
}

pub trait AsAccountWithStateView<'a> {
    fn as_account_with_state_view(
        &'a self,
        account_address: &'a AccountAddress,
    ) -> AccountWithStateView;
}
