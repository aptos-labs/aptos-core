// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    on_chain_config::{FeatureFlag, Features},
    validator_txn::ValidatorTransaction,
};

pub mod db_tool;
#[cfg(any(test, feature = "fuzzing"))]
pub mod mock_time_service;
pub mod time_service;

pub fn is_vtxn_expected(features: &Features, vtxn: &ValidatorTransaction) -> bool {
    match vtxn {
        ValidatorTransaction::DKGResult(_) => {
            features.is_enabled(FeatureFlag::RECONFIGURE_WITH_DKG)
        },
        ValidatorTransaction::ObservedJWKUpdate(_) => {
            features.is_enabled(FeatureFlag::JWK_CONSENSUS)
        },
    }
}
