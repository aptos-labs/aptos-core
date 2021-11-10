// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{DbReader, DbWriter};
use anyhow::Result;
use diem_types::{
    account_address::AccountAddress, account_config::AccountResource, account_state::AccountState,
    account_state_blob::AccountStateBlob, event::EventHandle, protocol_spec::DpnProto,
};
use move_core_types::move_resource::MoveResource;
use std::convert::TryFrom;

/// This is a mock of the DbReaderWriter in tests.
pub struct MockDbReaderWriter;

impl DbReader<DpnProto> for MockDbReaderWriter {
    fn get_latest_account_state(
        &self,
        _address: AccountAddress,
    ) -> Result<Option<AccountStateBlob>> {
        Ok(Some(get_mock_account_state_blob()))
    }
}

fn get_mock_account_state_blob() -> AccountStateBlob {
    let account_resource = AccountResource::new(
        0,
        vec![],
        None,
        None,
        EventHandle::random_handle(0),
        EventHandle::random_handle(0),
    );

    let mut account_state = AccountState::default();
    account_state.insert(
        AccountResource::resource_path(),
        bcs::to_bytes(&account_resource).unwrap(),
    );

    AccountStateBlob::try_from(&account_state).unwrap()
}

impl DbWriter<DpnProto> for MockDbReaderWriter {}
