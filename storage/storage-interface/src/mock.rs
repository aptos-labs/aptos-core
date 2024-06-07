// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module provides mock dbreader for tests.

use crate::{errors::AptosDbError, DbReader, DbWriter, Result};
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, NewBlockEvent},
    account_state::AccountState,
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_state::EpochState,
    event::EventHandle,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    proof::SparseMerkleProofExt,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        state_value::StateValue,
    },
    transaction::Version,
    validator_verifier::ValidatorVerifier,
};
use move_core_types::move_resource::MoveResource;

/// This is a mock of the DbReaderWriter in tests.
pub struct MockDbReaderWriter;

impl DbReader for MockDbReaderWriter {
    fn get_latest_state_checkpoint_version(&self) -> Result<Option<Version>> {
        // return a dummy version for tests
        Ok(Some(1))
    }

    fn get_state_proof_by_version_ext(
        &self,
        _state_key: &StateKey,
        _version: Version,
    ) -> Result<SparseMerkleProofExt> {
        Ok(SparseMerkleProofExt::new(None, vec![]))
    }

    fn get_state_value_by_version(
        &self,
        state_key: &StateKey,
        _: Version,
    ) -> Result<Option<StateValue>> {
        match state_key.inner() {
            StateKeyInner::AccessPath(access_path) => {
                let account_state = get_mock_account_state();
                Ok(account_state
                    .get(&access_path.path)
                    .cloned()
                    .map(StateValue::from))
            },
            StateKeyInner::Raw(raw_key) => Ok(Some(StateValue::from(raw_key.to_owned()))),
            _ => Err(AptosDbError::Other(format!(
                "Not supported state key type {:?}",
                state_key
            ))),
        }
    }

    fn get_state_value_with_version_by_version(
        &self,
        state_key: &StateKey,
        version: Version,
    ) -> Result<Option<(Version, StateValue)>> {
        Ok(self
            .get_state_value_by_version(state_key, version)?
            .map(|value| (version, value)))
    }

    fn get_epoch_ending_ledger_info(&self, known_version: u64) -> Result<LedgerInfoWithSignatures> {
        if known_version == 0 {
            let next_epoch_state = EpochState {
                epoch: 1,
                verifier: ValidatorVerifier::new(vec![]),
            };
            let block_info = BlockInfo::new(
                0,
                0,
                HashValue::new([1; HashValue::LENGTH]),
                HashValue::zero(),
                1,
                1717757545265,
                Some(next_epoch_state),
            );
            Ok(LedgerInfoWithSignatures::new(
                LedgerInfo::new(block_info, HashValue::zero()),
                AggregateSignature::empty(),
            ))
        } else {
            Err(AptosDbError::NotFound(format!(
                "mock ledger info for version {known_version}"
            )))
        }
    }

    fn get_block_info_by_height(&self, height: u64) -> Result<(Version, Version, NewBlockEvent)> {
        Ok((
            0,
            1,
            NewBlockEvent::new(
                AccountAddress::ONE,
                0,
                0,
                height,
                vec![],
                AccountAddress::TWO,
                vec![],
                0,
            ),
        ))
    }

    fn get_accumulator_root_hash(&self, _version: Version) -> Result<HashValue> {
        Ok(HashValue::zero())
    }
}

fn get_mock_account_state() -> AccountState {
    let account_resource =
        AccountResource::new(0, vec![], EventHandle::random(0), EventHandle::random(0));

    AccountState::new(
        AccountAddress::random(),
        std::collections::BTreeMap::from([(
            AccountResource::resource_path(),
            bcs::to_bytes(&account_resource).unwrap(),
        )]),
    )
}

impl DbWriter for MockDbReaderWriter {}
