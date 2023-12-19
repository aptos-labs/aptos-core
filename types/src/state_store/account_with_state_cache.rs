// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    account_address::AccountAddress,
    account_view::AccountView,
    state_store::{state_key::StateKey, state_value::StateValue},
};
use bytes::Bytes;
use std::collections::HashMap;
pub struct AccountWithStateCache<'a> {
    account_address: &'a AccountAddress,
    state_cache: &'a HashMap<StateKey, StateValue>,
}

impl<'a> AccountWithStateCache<'a> {
    pub fn new(
        account_address: &'a AccountAddress,
        state_cache: &'a HashMap<StateKey, StateValue>,
    ) -> Self {
        Self {
            account_address,
            state_cache,
        }
    }
}

impl<'a> AccountView for AccountWithStateCache<'a> {
    fn get_state_value(&self, state_key: &StateKey) -> anyhow::Result<Option<Bytes>> {
        Ok(self
            .state_cache
            .get(state_key)
            .map(|val| val.bytes().clone()))
    }

    fn get_account_address(&self) -> anyhow::Result<Option<AccountAddress>> {
        Ok(Some(*self.account_address))
    }
}

pub trait AsAccountWithStateCache<'a> {
    fn as_account_with_state_cache(
        &'a self,
        account_address: &'a AccountAddress,
    ) -> AccountWithStateCache;
}

impl<'a> AsAccountWithStateCache<'a> for HashMap<StateKey, StateValue> {
    fn as_account_with_state_cache(
        &'a self,
        account_address: &'a AccountAddress,
    ) -> AccountWithStateCache {
        AccountWithStateCache::new(account_address, self)
    }
}
