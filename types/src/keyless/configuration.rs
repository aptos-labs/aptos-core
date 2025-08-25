// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    invalid_signature,
    keyless::{
        circuit_constants, circuit_testcases::SAMPLE_EXP_HORIZON_SECS, KEYLESS_ACCOUNT_MODULE_NAME,
    },
    move_utils::as_move_value::AsMoveValue,
    on_chain_config::OnChainConfig,
};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::MoveStructType,
    value::{MoveStruct, MoveValue},
    vm_status::{StatusCode, VMStatus},
};
use serde::{Deserialize, Serialize};

/// Reflection of aptos_framework::keyless_account::Configuration
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Configuration {
    pub override_aud_vals: Vec<String>,
    pub max_signatures_per_txn: u16,
    pub max_exp_horizon_secs: u64,
    pub training_wheels_pubkey: Option<Vec<u8>>,
    pub max_commited_epk_bytes: u16,
    pub max_iss_val_bytes: u16,
    pub max_extra_field_bytes: u16,
    pub max_jwt_header_b64_bytes: u32,
}

impl AsMoveValue for Configuration {
    fn as_move_value(&self) -> MoveValue {
        let training_wheels_pubkey = match self.training_wheels_pubkey.as_ref() {
            Some(pubkey) => MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![pubkey.as_move_value()])),
            None => MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![])),
        };
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.override_aud_vals.as_move_value(),
            self.max_signatures_per_txn.as_move_value(),
            self.max_exp_horizon_secs.as_move_value(),
            training_wheels_pubkey,
            self.max_commited_epk_bytes.as_move_value(),
            self.max_iss_val_bytes.as_move_value(),
            self.max_extra_field_bytes.as_move_value(),
            self.max_jwt_header_b64_bytes.as_move_value(),
        ]))
    }
}

/// WARNING: This struct uses resource groups on the Move side. Do NOT implement OnChainConfig
/// for it, since `OnChainConfig::fetch_config` does not work with resource groups (yet).
impl MoveStructType for Configuration {
    const MODULE_NAME: &'static IdentStr = ident_str!(KEYLESS_ACCOUNT_MODULE_NAME);
    const STRUCT_NAME: &'static IdentStr = ident_str!("Configuration");
}

impl Configuration {
    /// Should only be used for testing.
    pub const OVERRIDE_AUD_FOR_TESTING: &'static str = "test.recovery.aud";

    pub fn new_for_devnet() -> Configuration {
        Configuration {
            override_aud_vals: vec![Self::OVERRIDE_AUD_FOR_TESTING.to_owned()],
            max_signatures_per_txn: 3,
            max_exp_horizon_secs: 10_000_000, // ~115.74 days
            training_wheels_pubkey: None,
            max_commited_epk_bytes: circuit_constants::MAX_COMMITED_EPK_BYTES,
            max_iss_val_bytes: circuit_constants::MAX_ISS_VAL_BYTES,
            max_extra_field_bytes: circuit_constants::MAX_EXTRA_FIELD_BYTES,
            max_jwt_header_b64_bytes: circuit_constants::MAX_JWT_HEADER_B64_BYTES,
        }
    }

    pub fn new_for_testing() -> Configuration {
        let mut config = Self::new_for_devnet();
        config.max_exp_horizon_secs = SAMPLE_EXP_HORIZON_SECS + 1; // ~31,689 years
        config
    }

    pub fn is_allowed_override_aud(&self, override_aud_val: &String) -> Result<(), VMStatus> {
        let matches = self
            .override_aud_vals
            .iter()
            .filter(|&e| e.eq(override_aud_val))
            .count();

        if matches == 0 {
            Err(invalid_signature!(format!(
                "override aud is not allow-listed in 0x1::{}",
                KEYLESS_ACCOUNT_MODULE_NAME
            )))
        } else {
            Ok(())
        }
    }
}

impl OnChainConfig for Configuration {
    const MODULE_IDENTIFIER: &'static str = KEYLESS_ACCOUNT_MODULE_NAME;
    const TYPE_IDENTIFIER: &'static str = "Configuration";
}
