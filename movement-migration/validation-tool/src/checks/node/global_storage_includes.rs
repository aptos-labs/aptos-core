// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    checks::error::ValidationError,
    types::storage::{MovementAptosStorage, MovementStorage},
};
use aptos_types::{
    access_path::Path,
    account_config::{AccountResource, CoinStoreResourceUntyped},
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        TStateView,
    },
};
use bytes::Bytes;
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::str::FromStr;
use tracing::{debug, info};

/// This check iterates over all global state keys starting at ledger version 0.
/// For each state key it fetches the state view for the latest ledger version,
/// from the old Movment database and the new Aptos database. The state view bytes
/// from both databases need to match. If the state key has no value in the latest
/// ledger version of the old Movement database then it should also have no value
/// in the new Aptos database.
/// Account Resources and Coin Stores are deserialized from BSC before comparison.
/// In case of Coin Stores, only the balances are compared.
pub struct GlobalStorageIncludes;

impl GlobalStorageIncludes {
    pub fn satisfies(
        movement_storage: &MovementStorage,
        movement_aptos_storage: &MovementAptosStorage,
    ) -> Result<(), ValidationError> {
        let account = StructTag::from_str("0x1::account::Account").unwrap();
        let coin = StructTag::from_str("0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>").unwrap();

        // get the latest ledger version from the movement storage
        let movement_ledger_version = movement_storage
            .latest_ledger_version()
            .map_err(|e| ValidationError::Internal(e.into()))?;

        info!("checking global state keys and values");
        debug!("movement_ledger_version: {:?}", movement_ledger_version);

        // get the latest state view from the movement storage
        let movement_state_view = movement_storage
            .state_view_at_version(Some(movement_ledger_version))
            .map_err(|e| ValidationError::Internal(e.into()))?;

        // get the latest state view from the maptos storage
        let maptos_state_view = movement_aptos_storage
            .state_view_at_version(Some(movement_ledger_version))
            .map_err(|e| ValidationError::Internal(e.into()))?;

        // the movement state view is the domain, so the maptos state view is the codomain
        let movement_global_state_keys_iterator =
            movement_storage.global_state_keys_from_version(None);
        let movement_global_state_keys = movement_global_state_keys_iterator
            .iter()
            .map_err(|e| ValidationError::Internal(e.into()))?;

        let mut count = 0;
        for movement_state_key in movement_global_state_keys {
            debug!(
                "processing movement_state_key {}: {:?}",
                count, movement_state_key
            );

            let movement_state_key =
                movement_state_key.map_err(|e| ValidationError::Internal(e.into()))?;

            let movement_value = movement_state_view
                .get_state_value_bytes(&movement_state_key)
                .map_err(|e| ValidationError::Internal(e.into()))?;

            match movement_value {
                Some(movement_value) => {
                    let maptos_state_value = maptos_state_view
                        .get_state_value_bytes(&movement_state_key)
                        .map_err(|e| ValidationError::Internal(e.into()))?
                        .ok_or(ValidationError::Unsatisfied(
                            format!(
                                "Movement Aptos is missing a value for {:?}",
                                movement_state_key
                            )
                            .into(),
                        ))?;

                    if let StateKeyInner::AccessPath(p) = movement_state_key.inner() {
                        match p.get_path() {
                            Path::Resource(tag) if tag == account => Self::compare_accounts(
                                p.address,
                                movement_value,
                                maptos_state_value,
                            )?,
                            Path::Resource(tag) if tag == coin => Self::compare_balances(
                                p.address,
                                movement_value,
                                maptos_state_value,
                            )?,
                            _ => Self::compare_raw_state(
                                movement_state_key,
                                movement_value,
                                maptos_state_value,
                            )?,
                        }
                    } else {
                        Self::compare_raw_state(
                            movement_state_key,
                            movement_value,
                            maptos_state_value,
                        )?;
                    }
                },
                None => {
                    debug!("Value from a previous version has been removed at the latest ledger version");

                    match maptos_state_view
                        .get_state_value(&movement_state_key)
                        .map_err(|e| ValidationError::Internal(e.into()))?
                    {
                        Some(_) => {
                            return Err(ValidationError::Unsatisfied(
                                format!(
                                    "Movement Aptos is unexpectedly not missing a value for {:?}",
                                    movement_state_key
                                )
                                .into(),
                            ));
                        },
                        None => {},
                    }
                },
            }
            count += 1;
        }

        Ok(())
    }

    fn compare_raw_state(
        movement_state_key: StateKey,
        movement_value: Bytes,
        maptos_state_value: Bytes,
    ) -> Result<(), ValidationError> {
        if movement_value != maptos_state_value {
            Err(ValidationError::Unsatisfied(
                format!(
                    "Movement state value for {:?} is {:?}, while Movement Aptos state value is {:?}",
                    movement_state_key,
                    movement_value,
                    maptos_state_value
                )
                    .into(),
            ))
        } else {
            Ok(())
        }
    }

    fn compare_accounts(
        address: AccountAddress,
        movement_value: Bytes,
        maptos_state_value: Bytes,
    ) -> Result<(), ValidationError> {
        let movement_account = bcs::from_bytes::<AccountResource>(&movement_value)
            .map_err(|e| ValidationError::Internal(e.into()))?;
        let movement_aptos_account = bcs::from_bytes::<AccountResource>(&maptos_state_value)
            .map_err(|e| ValidationError::Internal(e.into()))?;

        debug!(
            "movement account at 0x{}: {:?}",
            address.short_str_lossless(),
            movement_account
        );

        if movement_account != movement_aptos_account {
            Err(ValidationError::Unsatisfied(
                format!(
                    "Movement account for {:?} is {:?}, while Movement Aptos account is {:?}",
                    address.to_standard_string(),
                    movement_account,
                    movement_aptos_account
                )
                .into(),
            ))
        } else {
            Ok(())
        }
    }

    fn compare_balances(
        address: AccountAddress,
        movement_value: Bytes,
        maptos_state_value: Bytes,
    ) -> Result<(), ValidationError> {
        let movement_balance = bcs::from_bytes::<CoinStoreResourceUntyped>(&movement_value)
            .map_err(|e| ValidationError::Internal(e.into()))?
            .coin();
        let movement_aptos_balance =
            bcs::from_bytes::<CoinStoreResourceUntyped>(&maptos_state_value)
                .map_err(|e| ValidationError::Internal(e.into()))?
                .coin();

        debug!(
            "movement balance at 0x{}: {} coins",
            address.short_str_lossless(),
            movement_balance
        );

        if movement_balance != movement_aptos_balance {
            Err(ValidationError::Unsatisfied(
                format!(
                    "Movement balance for 0x{} is {} coin(s), while Movement Aptos balance is {} coin(s)",
                    address.short_str_lossless(),
                    movement_balance,
                    movement_aptos_balance
                )
                    .into(),
            ))
        } else {
            Ok(())
        }
    }
}
