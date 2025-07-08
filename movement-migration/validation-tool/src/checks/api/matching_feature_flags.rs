// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::checks::error::ValidationError;
use crate::types::api::{MovementAptosRestClient, MovementRestClient};
use aptos_rest_client::aptos_api_types::{EntryFunctionId, MoveType, ViewRequest};
use serde_json::json;
use std::collections::HashSet;
use std::str::FromStr;

pub struct GlobalFeatureCheck;

impl GlobalFeatureCheck {
    pub async fn satisfies(
        movement_rest_client: &MovementRestClient,
        movement_aptos_rest_client: &MovementAptosRestClient,
    ) -> Result<(), ValidationError> {
        let mut errors = vec![];
        let expected_active = HashSet::from([73]);
        let expected_inactive = HashSet::<u64>::new();

        let mut aptos_request = ViewRequest {
            function: EntryFunctionId::from_str("0x1::features::is_enabled")
                .map_err(|e| ValidationError::Internal(e.into()))?,
            type_arguments: vec![MoveType::U64],
            arguments: vec![],
        };

        let mut maptos_request = ViewRequest {
            function: EntryFunctionId::from_str("0x1::features::is_enabled")
                .map_err(|e| ValidationError::Internal(e.into()))?,
            type_arguments: vec![MoveType::U64],
            arguments: vec![],
        };

        for feature_id in 1u64..=100 {
            aptos_request.arguments = vec![json!(feature_id)];
            maptos_request.arguments = vec![json!(feature_id)];

            // Check feature for Aptos executor
            let aptos_active = movement_aptos_rest_client
                .view(&aptos_request, None)
                .await
                .map_err(|e| {
                    ValidationError::Internal(
                        format!("failed to get Aptos feature flag {}: {:?}", feature_id, e).into(),
                    )
                })?
                .into_inner();

            let aptos_active = aptos_active.get(0).ok_or_else(|| {
                ValidationError::Internal(
                    format!(
                        "failed to get Aptos feature flag {}: response is empty",
                        feature_id
                    )
                    .into(),
                )
            })?;

            let aptos_active = aptos_active.as_bool().ok_or_else(|| {
                ValidationError::Internal(
                    format!(
                        "failed to get Aptos feature flag {}: can't convert {:?} into a bool",
                        feature_id, aptos_active
                    )
                    .into(),
                )
            })?;

            // Check feature for Maptos executor
            let maptos_active = movement_rest_client
                .view(&maptos_request, None)
                .await
                .map_err(|e| {
                    ValidationError::Internal(
                        format!(
                            "failed to get Movement feature flag {}: {:?}",
                            feature_id, e
                        )
                        .into(),
                    )
                })?
                .into_inner();

            let maptos_active = maptos_active.get(0).ok_or_else(|| {
                ValidationError::Internal(
                    format!(
                        "failed to get Movement feature flag {}: response is empty",
                        feature_id
                    )
                    .into(),
                )
            })?;

            let maptos_active = maptos_active.as_bool().ok_or_else(|| {
                ValidationError::Internal(
                    format!(
                        "failed to get Movement feature flag {}: can't convert {:?} into a bool",
                        feature_id, aptos_active
                    )
                    .into(),
                )
            })?;

            if !expected_active.contains(&feature_id) {
                if !maptos_active {
                    errors.push(format!(
                        "Feature {}: Aptos={}, Maptos={} — expected to be active",
                        feature_id, aptos_active, maptos_active
                    ));
                }
            } else if !expected_inactive.contains(&feature_id) {
                if maptos_active {
                    errors.push(format!(
                        "Feature {}: Aptos={}, Maptos={} — expected to be inactive",
                        feature_id, aptos_active, maptos_active
                    ));
                }
            } else if aptos_active != maptos_active {
                errors.push(format!(
                    "Feature {}: Aptos={}, Maptos={} — expected to match",
                    feature_id, aptos_active, maptos_active
                ));
            }
        }

        if !errors.is_empty() {
            return Err(ValidationError::Unsatisfied(errors.join("\n").into()));
        }

        Ok(())
    }
}
