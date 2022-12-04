// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::StateView;
use aptos_types::{
    account_address::AccountAddress, account_view::AccountView, state_store::state_key::StateKey,
};

pub struct AccountWithStateView<'a, S> {
    account_address: &'a AccountAddress,
    state_view: &'a S,
}

impl<'a, S: StateView<StateKey>> AccountWithStateView<'a, S> {
    pub fn new(account_address: &'a AccountAddress, state_view: &'a S) -> Self {
        Self {
            account_address,
            state_view,
        }
    }
}

impl<'a, S: StateView<StateKey>> AccountView for AccountWithStateView<'a, S> {
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Vec<u8>>> {
        self.state_view.get_state_value(state_key)
    }

    fn get_account_address(&self) -> anyhow::Result<Option<AccountAddress>> {
        Ok(Some(*self.account_address))
    }
}

pub trait AsAccountWithStateView<'a, S: StateView<StateKey>> {
    fn as_account_with_state_view(
        &'a self,
        account_address: &'a AccountAddress,
    ) -> AccountWithStateView<'a, S>;
}
