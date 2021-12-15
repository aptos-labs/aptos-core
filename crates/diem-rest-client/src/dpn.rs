// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_api_types::U64;
use move_core_types::{identifier::Identifier, language_storage::StructTag};
use serde::{Deserialize, Serialize};

pub use diem_types::account_config::{diem_root_address, BalanceResource, CORE_CODE_ADDRESS};
pub use diem_types::on_chain_config::{
    config_struct_tag, ConfigurationResource, DiemVersion as OnChainDiemVersion, OnChainConfig,
};

use crate::types::EventHandle;

#[derive(Debug, Serialize, Deserialize)]
pub struct Diem {
    pub value: U64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    pub coin: Diem,
}

#[derive(Debug)]
pub struct AccountBalance {
    pub currency: StructTag,
    pub amount: u64,
}

impl AccountBalance {
    pub fn currency_code(&self) -> String {
        self.currency.name.to_string()
    }
}

pub fn diem_version_identifier() -> Identifier {
    Identifier::new(OnChainDiemVersion::IDENTIFIER).unwrap()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiemConfig<T> {
    pub payload: T,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiemVersion {
    pub major: U64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Configuration {
    #[serde(rename = "epoch")]
    pub next_block_epoch: U64,
    pub last_reconfiguration_time: U64,
    pub events: EventHandle,
}
